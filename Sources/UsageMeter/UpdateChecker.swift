import AppKit
import Combine
import UserNotifications

/// 앱 버전 표시 + 새 버전 확인.
///
/// 배포가 GitHub Releases라, 인증 없이 공개 API로 최신 릴리스 태그만 읽어
/// 현재 번들 버전과 비교한다. **다운로드·설치는 하지 않는다** — 새 버전이 있으면
/// 알리고 릴리스 페이지를 열어줄 뿐이다(ad-hoc 서명 앱이라 자동 교체는 위험).
@MainActor
final class UpdateChecker: NSObject, ObservableObject {

    /// 확인 결과.
    enum State: Equatable {
        case idle
        case checking
        case upToDate
        case available(version: String, url: URL)
        case failed(String)
    }

    @Published private(set) var state: State = .idle
    /// 마지막으로 확인을 끝낸 시각(성공·실패 모두).
    @Published private(set) var lastCheckedAt: Date?

    /// 현재 앱 버전. `.app` 번들이 아니면(개발 중 `swift run`) nil.
    let currentVersion: String?

    private let settings: OverlaySettings
    private let releasesAPI = URL(string: "https://api.github.com/repos/WWhaleFe/usagemeter/releases/latest")!
    private let releasesPage = URL(string: "https://github.com/WWhaleFe/usagemeter/releases/latest")!
    /// 같은 버전으로 두 번 알리지 않기 위해 마지막으로 알린 버전을 기억한다.
    private let notifiedKey = "usagemeter.lastNotifiedVersion"
    private var timer: Timer?

    /// 표시용 버전 문자열(번들이 아니면 "dev").
    var displayVersion: String { currentVersion ?? "dev" }

    init(settings: OverlaySettings) {
        self.settings = settings
        self.currentVersion = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String
        super.init()
        // 알림을 눌렀을 때 릴리스 페이지를 열려면 델리게이트가 필요하다.
        if Bundle.main.bundleIdentifier != nil {
            UNUserNotificationCenter.current().delegate = self
        }
        scheduleAuto()
    }

    /// 자동 확인: 시작 직후 1회 + 24시간마다. 설정이 꺼져 있으면 아무것도 하지 않는다.
    private func scheduleAuto() {
        timer?.invalidate()
        guard settings.autoCheckUpdate else { return }
        Task { await check(manual: false) }
        timer = Timer.scheduledTimer(withTimeInterval: 24 * 60 * 60, repeats: true) { [weak self] _ in
            Task { @MainActor in await self?.check(manual: false) }
        }
    }

    /// 설정에서 자동 확인을 켜고 끌 때 호출.
    func autoCheckSettingChanged() { scheduleAuto() }

    /// 릴리스 페이지 열기(메뉴·설정의 '다운로드' 동작).
    func openDownloadPage() {
        if case .available(_, let url) = state { NSWorkspace.shared.open(url) }
        else { NSWorkspace.shared.open(releasesPage) }
    }

    /// 최신 릴리스를 조회해 현재 버전과 비교한다.
    /// - Parameter manual: 사용자가 직접 눌렀는지(최신일 때도 결과를 보여주기 위함).
    func check(manual: Bool) async {
        guard state != .checking else { return }
        state = .checking
        defer { lastCheckedAt = Date() }
        do {
            var req = URLRequest(url: releasesAPI)
            req.timeoutInterval = 15
            req.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
            req.setValue("UsageMeter/\(displayVersion)", forHTTPHeaderField: "User-Agent")
            let (data, resp) = try await URLSession.shared.data(for: req)
            let code = (resp as? HTTPURLResponse)?.statusCode ?? 0
            guard code == 200 else { state = .failed("HTTP \(code)"); return }
            guard let obj = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let tag = obj["tag_name"] as? String else {
                state = .failed("bad_response"); return
            }
            let pageURL = (obj["html_url"] as? String).flatMap(URL.init(string:)) ?? releasesPage
            let latest = Self.normalize(tag)

            // 번들 버전을 모르면(개발 실행) 비교할 수 없다.
            guard let current = currentVersion else { state = .failed("no_bundle_version"); return }

            let newer = Self.isNewer(latest, than: Self.normalize(current))
            NSLog("[UsageMeter] update check: current=%@ latest=%@ newer=%@ manual=%@",
                  current, latest, newer ? "yes" : "no", manual ? "yes" : "no")
            if newer {
                state = .available(version: latest, url: pageURL)
                notifyIfNeeded(version: latest, url: pageURL)
            } else {
                state = .upToDate
            }
        } catch {
            state = .failed(error.localizedDescription)
        }
    }

    // MARK: - 버전 비교

    /// "v1.2.5" → "1.2.5" (앞의 v와 공백 제거).
    static func normalize(_ s: String) -> String {
        var t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.hasPrefix("v") || t.hasPrefix("V") { t.removeFirst() }
        return t
    }

    /// 점으로 나눈 숫자 비교("1.2.10" > "1.2.9"). 숫자가 아닌 조각은 0으로 본다.
    static func isNewer(_ a: String, than b: String) -> Bool {
        let x = a.split(separator: ".").map { Int($0.prefix(while: \.isNumber)) ?? 0 }
        let y = b.split(separator: ".").map { Int($0.prefix(while: \.isNumber)) ?? 0 }
        for i in 0..<max(x.count, y.count) {
            let l = i < x.count ? x[i] : 0
            let r = i < y.count ? y[i] : 0
            if l != r { return l > r }
        }
        return false
    }

    // MARK: - 알림

    private func notifyIfNeeded(version: String, url: URL) {
        guard Bundle.main.bundleIdentifier != nil else { return }
        // 같은 버전은 한 번만 알린다(하루마다 같은 알림이 반복되지 않게).
        guard UserDefaults.standard.string(forKey: notifiedKey) != version else { return }
        UserDefaults.standard.set(version, forKey: notifiedKey)

        let content = UNMutableNotificationContent()
        content.title = settings.t("update.notifyTitle")
        content.body = settings.tf("update.notifyBody", version)
        content.sound = .default
        content.userInfo = ["url": url.absoluteString]
        let req = UNNotificationRequest(identifier: "update-\(version)", content: content, trigger: nil)
        UNUserNotificationCenter.current().add(req)
    }
}

// 알림을 클릭하면 릴리스 페이지를 연다.
extension UpdateChecker: UNUserNotificationCenterDelegate {
    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            didReceive response: UNNotificationResponse) async {
        let info = response.notification.request.content.userInfo
        guard let s = info["url"] as? String, let url = URL(string: s) else { return }
        await MainActor.run { NSWorkspace.shared.open(url) }
    }

    /// 앱이 떠 있을 때도 알림 배너를 보여준다(기본은 표시 안 됨).
    nonisolated func userNotificationCenter(_ center: UNUserNotificationCenter,
                                            willPresent notification: UNNotification) async
        -> UNNotificationPresentationOptions { [.banner, .sound] }
}
