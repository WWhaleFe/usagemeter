import Foundation
import CoreServices

/// Google Antigravity CLI (`agy`) 쿼터 조회 및 실시간 활동 감지기.
///
/// - 왜 필요한가:
///   `gemini.google.com/usage` 웹 화면은 일반 Gemini 웹 대화만 반영하거나 집계가 매우 느려서,
///   `agy`에서 Gemini 4 Argon / Gemini Pro / Claude 등을 사용할 때의 실제 쿼터 소모가 반영되지 않는다.
///   반면 `agy --log-file /dev/null -p /usage --output-format json`은 Antigravity 서버의 실시간
///   쿼터(`remaining_fraction`, 소수점 정밀도 + `reset_time`)를 토큰 소모 없이 즉시 반환한다.
actor AgyQuotaFetcher {
    static let shared = AgyQuotaFetcher()

    private var inFlight: Task<UsageSnapshot?, Never>?
    private var lastActivityAt: Date = .distantPast
    /// 동일 리셋 주기 안에서 백엔드 레플리카 간 캐시 차이로 잔여율이 잠깐 튀어 오르는 현상(깜빡임·중복 집계)을
    /// 막기 위해 버킷별 (최소 잔여율, reset_time)을 기억한다.
    private var monotonicBuckets: [String: (ratio: Double, resetTime: String)] = [:]

    /// 최근 10분 이내에 agy 활동이 있었는지(빠른 보조 폴링 여부 판단용).
    func noteActivity() {
        lastActivityAt = Date()
    }

    func isRecentlyActive(within seconds: TimeInterval = 600) -> Bool {
        Date().timeIntervalSince(lastActivityAt) < seconds
    }

    /// 시스템에 설치된 `agy` 실행파일 경로를 찾는다(GUI .app은 PATH가 좁으므로 주요 경로 직접 탐색).
    nonisolated static func findAgyExecutable() -> String? {
        let fm = FileManager.default
        let home = fm.homeDirectoryForCurrentUser.path
        var candidates = [
            "\(home)/.local/bin/agy",
            "\(home)/.gemini/antigravity-cli/bin/agy",
            "/opt/homebrew/bin/agy",
            "/usr/local/bin/agy",
            "/usr/bin/agy"
        ]
        if let pathEnv = ProcessInfo.processInfo.environment["PATH"] {
            for dir in pathEnv.split(separator: ":") {
                candidates.append("\(dir)/agy")
            }
        }
        return candidates.first { fm.isExecutableFile(atPath: $0) }
    }

    /// `agy` 쿼터를 조회해 `UsageSnapshot`으로 변환한다.
    /// `agy`가 설치돼 있지 않거나 미로그인/실패 시 `nil`을 반환해 기존 웹뷰 조회로 폴백하게 한다.
    func fetch(providerID: String = "gemini") async -> UsageSnapshot? {
        if let existing = inFlight {
            return await existing.value
        }
        let task = Task<UsageSnapshot?, Never> {
            await self.runQuery(providerID: providerID)
        }
        inFlight = task
        let result = await task.value
        inFlight = nil
        return result
    }

    private func runQuery(providerID: String) async -> UsageSnapshot? {
        guard let exe = Self.findAgyExecutable() else { return nil }
        guard let data = await Self.runProcess(executable: exe) else { return nil }
        return parseUsageJSON(data, providerID: providerID)
    }

    private static func runProcess(executable: String, timeoutSeconds: Double = 12) async -> Data? {
        await withCheckedContinuation { (cont: CheckedContinuation<Data?, Never>) in
            DispatchQueue.global(qos: .utility).async {
                let proc = Process()
                proc.executableURL = URL(fileURLWithPath: executable)
                // --log-file /dev/null: 조회용 실행이 ~/.gemini/antigravity-cli/log 에 로그 파일을 남기지 않게 한다.
                proc.arguments = ["--log-file", "/dev/null", "-p", "/usage", "--output-format", "json"]
                proc.currentDirectoryURL = URL(fileURLWithPath: "/tmp")

                var env = ProcessInfo.processInfo.environment
                let home = FileManager.default.homeDirectoryForCurrentUser.path
                env["HOME"] = home
                let extraPath = "\(home)/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin"
                if let existing = env["PATH"], !existing.isEmpty {
                    env["PATH"] = "\(extraPath):\(existing)"
                } else {
                    env["PATH"] = extraPath
                }
                proc.environment = env

                let outPipe = Pipe()
                proc.standardOutput = outPipe
                proc.standardError = FileHandle.nullDevice
                proc.standardInput = FileHandle.nullDevice

                do {
                    try proc.run()
                } catch {
                    cont.resume(returning: nil)
                    return
                }

                let deadline = DispatchTime.now() + timeoutSeconds
                let timer = DispatchSource.makeTimerSource(queue: .global(qos: .utility))
                timer.schedule(deadline: deadline)
                timer.setEventHandler {
                    if proc.isRunning { proc.terminate() }
                }
                timer.resume()

                let outData = outPipe.fileHandleForReading.readDataToEndOfFile()
                proc.waitUntilExit()
                timer.cancel()

                guard proc.terminationStatus == 0, !outData.isEmpty else {
                    cont.resume(returning: nil)
                    return
                }
                cont.resume(returning: outData)
            }
        }
    }

    /// `agy -p /usage --output-format json` 출력을 파싱해 `UsageSnapshot`을 만든다.
    private func parseUsageJSON(_ data: Data, providerID: String) -> UsageSnapshot? {
        guard let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              (root["status"] as? String) == "SUCCESS",
              let cmd = root["command"] as? [String: Any],
              let cmdData = cmd["data"] as? [String: Any],
              let groups = cmdData["groups"] as? [[String: Any]], !groups.isEmpty else {
            return nil
        }

        let isoWithFrac = ISO8601DateFormatter()
        isoWithFrac.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let isoPlain = ISO8601DateFormatter()
        isoPlain.formatOptions = [.withInternetDateTime]
        func parseDate(_ str: String?) -> Date? {
            guard let s = str, !s.isEmpty else { return nil }
            return isoWithFrac.date(from: s) ?? isoPlain.date(from: s)
        }

        var gemini5h: (ratio: Double, reset: Date?)?
        var geminiWeekly: (ratio: Double, reset: Date?)?
        var modelBuckets: [UsageModelBucket] = []
        var rawBuckets: [String: Double] = [:]
        var urgentCandidates: [(ratio: Double, reset: Date?)] = []

        for g in groups {
            let groupName = (g["name"] as? String) ?? ""
            guard let buckets = g["buckets"] as? [[String: Any]] else { continue }
            let isGeminiCoreGroup = groupName.lowercased().contains("gemini models")

            for b in buckets {
                guard let rem = (b["remaining_fraction"] as? NSNumber)?.doubleValue else { continue }
                let rawRatio = max(0.0, min(1.0, rem))
                let bid = (b["id"] as? String) ?? ""
                let window = ((b["window"] as? String) ?? "").lowercased()
                let resetStr = (b["reset_time"] as? String) ?? ""
                let reset = parseDate(resetStr)
                let is5h = (window == "5h" || bid.hasSuffix("-5h"))

                // 같은 리셋 주기(reset_time 동일) 안에서는 백엔드 서버 레플리카 간 미세한 동기화 차이로
                // 직전보다 살짝 높은 값이 내려오더라도 직전 최소값을 유지한다(리셋되어 큰 폭 상승한 경우는 제외).
                let ratio: Double
                if !bid.isEmpty, let prev = monotonicBuckets[bid],
                   prev.resetTime == resetStr, rawRatio > prev.ratio, (rawRatio - prev.ratio) < 0.12 {
                    ratio = prev.ratio
                } else {
                    ratio = rawRatio
                    if !bid.isEmpty {
                        monotonicBuckets[bid] = (ratio, resetStr)
                    }
                }

                if !bid.isEmpty {
                    rawBuckets[bid] = ratio
                }

                if bid == "gemini-5h" || (isGeminiCoreGroup && is5h) {
                    gemini5h = (ratio, reset)
                    urgentCandidates.append((ratio, reset))
                } else if bid == "gemini-weekly" || (isGeminiCoreGroup && !is5h) {
                    geminiWeekly = (ratio, reset)
                    urgentCandidates.append((ratio, reset))
                } else {
                    // 별도 모델 그룹(예: "Gemini 4 Argon", "Claude and GPT models")
                    let label: String
                    if groupName.lowercased().contains("claude") {
                        label = "Claude·GPT (agy)"
                    } else if !groupName.isEmpty {
                        label = groupName
                    } else {
                        label = (b["name"] as? String) ?? bid
                    }
                    modelBuckets.append(UsageModelBucket(
                        label: label,
                        remainingRatio: ratio,
                        resetAt: reset,
                        isFiveHour: is5h
                    ))
                    // Gemini 계열 버킷(예: Gemini 4 Argon)은 Gemini 대표 잔여율(가장 급한 비율) 후보에 포함.
                    if bid.lowercased().contains("gemini") || groupName.lowercased().contains("gemini") {
                        urgentCandidates.append((ratio, reset))
                    }
                }
            }
        }

        // 만약 Gemini 후보가 하나도 없으면 전체 버킷 중 가장 급한 것을 사용.
        if urgentCandidates.isEmpty {
            for b in modelBuckets {
                urgentCandidates.append((b.remainingRatio, b.resetAt))
            }
        }
        guard let mostUrgent = urgentCandidates.min(by: { $0.ratio < $1.ratio }) else {
            return nil
        }

        let primary5h = gemini5h ?? mostUrgent
        let now = Date()
        return UsageSnapshot(
            id: providerID,
            remainingRatio: mostUrgent.ratio,
            secondaryRatio: geminiWeekly?.ratio,
            opusRatio: nil,
            resetAt: mostUrgent.reset ?? primary5h.reset,
            secondaryResetAt: geminiWeekly?.reset,
            opusResetAt: nil,
            status: .ok,
            lastUpdated: now,
            plan: "Antigravity",
            modelBuckets: modelBuckets.isEmpty ? nil : modelBuckets,
            fiveHourRatio: primary5h.ratio,
            fiveHourResetAt: primary5h.reset,
            rawBuckets: rawBuckets.isEmpty ? nil : rawBuckets
        )
    }
}

/// `~/.gemini/antigravity-cli` 내부의 대화 DB·히스토리 변경을 FSEvents로 감시해
/// `agy`에서 토큰이 소모되는 즉시(1~3초 내) 사용량 갱신을 트리거한다.
@MainActor
final class AgyActivityWatcher {
    private let onActivity: () -> Void
    nonisolated(unsafe) private var stream: FSEventStreamRef?
    private var leadingWorkItem: DispatchWorkItem?
    private var trailingWorkItem: DispatchWorkItem?
    private var followUpWorkItem: DispatchWorkItem?
    private var lastTriggeredAt: Date = .distantPast

    init(onActivity: @escaping @MainActor () -> Void) {
        self.onActivity = onActivity
        startWatching()
    }

    deinit {
        if let s = stream {
            FSEventStreamStop(s)
            FSEventStreamInvalidate(s)
            FSEventStreamRelease(s)
        }
    }

    private func startWatching() {
        let fm = FileManager.default
        let base = fm.homeDirectoryForCurrentUser
            .appendingPathComponent(".gemini/antigravity-cli", isDirectory: true)
        guard fm.fileExists(atPath: base.path) else { return }

        // 실제 대화/턴 진행 시에만 갱신되는 하위 경로들을 감시한다.
        // (`agy -p /usage`는 conversation_id가 비어 있어 conversations/ 와 brain/ 을 건드리지 않음 → 무한루프 방지)
        let watchPaths: [String] = [
            base.appendingPathComponent("conversations").path,
            base.appendingPathComponent("brain").path,
            base.appendingPathComponent("history.jsonl").path
        ].filter { fm.fileExists(atPath: $0) }

        let pathsToWatch = (watchPaths.isEmpty ? [base.path] : watchPaths) as CFArray

        var ctx = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(self).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )

        let callback: FSEventStreamCallback = { _, info, numEvents, eventPaths, _, _ in
            guard let info else { return }
            let watcher = Unmanaged<AgyActivityWatcher>.fromOpaque(info).takeUnretainedValue()
            guard let paths = unsafeBitCast(eventPaths, to: NSArray.self) as? [String] else { return }
            let matched = paths.prefix(numEvents).contains { p in
                p.contains("/conversations/") && (p.hasSuffix(".db-wal") || p.hasSuffix(".db"))
                    || p.hasSuffix("/transcript.jsonl")
                    || p.hasSuffix("/transcript_full.jsonl")
                    || p.hasSuffix("/history.jsonl")
            }
            guard matched else { return }
            Task { @MainActor in
                watcher.handleActivityDetected()
            }
        }

        let flags = FSEventStreamCreateFlags(
            kFSEventStreamCreateFlagUseCFTypes
            | kFSEventStreamCreateFlagFileEvents
            | kFSEventStreamCreateFlagNoDefer
        )
        guard let s = FSEventStreamCreate(
            kCFAllocatorDefault,
            callback,
            &ctx,
            pathsToWatch,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            0.4,
            flags
        ) else { return }

        stream = s
        FSEventStreamSetDispatchQueue(s, DispatchQueue.main)
        FSEventStreamStart(s)
    }

    private func handleActivityDetected() {
        Task { await AgyQuotaFetcher.shared.noteActivity() }

        let now = Date()
        // 1) 진행 중 빠른 반영: 마지막 트리거로부터 7초 이상 지났다면 1.2초 후 즉시 한 번 갱신.
        if now.timeIntervalSince(lastTriggeredAt) >= 7.0, leadingWorkItem == nil {
            let item = DispatchWorkItem { [weak self] in
                guard let self else { return }
                self.leadingWorkItem = nil
                self.lastTriggeredAt = Date()
                self.onActivity()
            }
            leadingWorkItem = item
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.2, execute: item)
        }

        // 2) 턴 완료 직후(파일 쓰기가 멈춘 뒤 2.5초) 최종 정밀 갱신 + 서버 집계 지연 대비 7초 후 재확인.
        trailingWorkItem?.cancel()
        let trailing = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.trailingWorkItem = nil
            self.lastTriggeredAt = Date()
            self.onActivity()

            self.followUpWorkItem?.cancel()
            let followUp = DispatchWorkItem { [weak self] in
                guard let self else { return }
                self.followUpWorkItem = nil
                self.lastTriggeredAt = Date()
                self.onActivity()
            }
            self.followUpWorkItem = followUp
            DispatchQueue.main.asyncAfter(deadline: .now() + 6.5, execute: followUp)
        }
        trailingWorkItem = trailing
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5, execute: trailing)
    }
}
