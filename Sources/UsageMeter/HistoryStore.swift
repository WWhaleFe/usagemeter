import Foundation
import Combine

/// 시각별 잔여율 표본 하나(모든 로그인 AI의 잔여율).
struct UsageSample: Codable {
    let t: Date
    let ratios: [String: Double]   // providerId -> 잔여 비율(0~1)
}

/// 소진 예측 결과.
struct PaceInfo {
    let depletion: Date        // 이 속도면 잔여 0이 되는 예상 시각
    let ratePerHour: Double     // 시간당 잔여율 하락(양수 = 소진 중)
}

/// 사용량 잔여율 이력을 로컬 JSON에 저장하고, 미니차트·소진예측(pace)에 제공한다.
/// (민감정보 아님: 잔여 비율·시각만. 쿠키·키는 저장 안 함.)
@MainActor
final class HistoryStore: ObservableObject {
    private let manager: ProviderManager
    @Published private(set) var samples: [UsageSample] = []

    private var cancellable: AnyCancellable?
    private var lastRecord: Date = .distantPast
    private let url: URL
    private let maxAge: TimeInterval = 48 * 3600   // 48시간 보관
    private let minGap: TimeInterval = 45          // 표본 간 최소 간격(초)

    init(manager: ProviderManager) {
        self.manager = manager
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)
            .first!.appendingPathComponent("UsageMeter", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        url = dir.appendingPathComponent("history.json")
        load()
        // 사용량이 갱신되면(디바운스 후 상태 반영됨) 표본 기록.
        cancellable = manager.objectWillChange
            .debounce(for: .milliseconds(500), scheduler: RunLoop.main)
            .sink { [weak self] in self?.record() }
        record()
    }

    private func record() {
        let active = manager.active
        guard !active.isEmpty else { return }
        let now = Date()
        guard now.timeIntervalSince(lastRecord) >= minGap else { return }
        lastRecord = now
        var ratios: [String: Double] = [:]
        for a in active { ratios[a.spec.id] = a.snap.remainingRatio }
        samples.append(UsageSample(t: now, ratios: ratios))
        let cutoff = now.addingTimeInterval(-maxAge)
        samples.removeAll { $0.t < cutoff }
        save()
    }

    /// 최근 `hours`시간 동안 특정 AI의 (시각, 잔여율) 시계열.
    func series(for id: String, hours: Double = 24) -> [(Date, Double)] {
        let cutoff = Date().addingTimeInterval(-hours * 3600)
        return samples.compactMap { s in
            guard s.t >= cutoff, let r = s.ratios[id] else { return nil }
            return (s.t, r)
        }
    }

    /// 최근 추세로 소진 예측. 잔여가 감소 중일 때만 값을 준다.
    func pace(for id: String) -> PaceInfo? {
        let s = series(for: id, hours: 3)        // 최근 3시간 추세
        guard let first = s.first, let last = s.last else { return nil }
        let dtHours = last.0.timeIntervalSince(first.0) / 3600
        guard dtHours > 0.05 else { return nil }
        let ratePerHour = (first.1 - last.1) / dtHours    // 양수면 소진 중
        guard ratePerHour > 0.0005 else { return nil }
        let hoursToEmpty = last.1 / ratePerHour
        return PaceInfo(depletion: Date().addingTimeInterval(hoursToEmpty * 3600), ratePerHour: ratePerHour)
    }

    // MARK: - 영속화
    private func load() {
        guard let data = try? Data(contentsOf: url),
              let arr = try? JSONDecoder().decode([UsageSample].self, from: data) else { return }
        let cutoff = Date().addingTimeInterval(-maxAge)
        samples = arr.filter { $0.t >= cutoff }
    }
    private func save() {
        if let data = try? JSONEncoder().encode(samples) {
            try? data.write(to: url, options: .atomic)
        }
    }
}
