import Foundation
import UserNotifications
import Combine

/// 사용률이 임계치(예: 75/90/95%)에 도달하면 macOS 알림을 보낸다.
/// 같은 임계치는 리셋 주기당 한 번만(중복 방지). 리셋되면(resetAt 변경) 다시 알림 가능.
@MainActor
final class NotificationManager {
    private let settings: OverlaySettings
    private let manager: ProviderManager
    private var cancellable: AnyCancellable?
    private var notified: Set<String> = []      // "id|threshold|resetEpoch"

    /// UNUserNotificationCenter는 번들 앱(.app)에서만 동작한다. `swift run`(번들 ID 없음)에선
    /// 크래시하므로, 번들 ID가 있을 때만 알림을 사용한다(개발 실행에선 조용히 비활성).
    private let available: Bool = Bundle.main.bundleIdentifier != nil

    init(settings: OverlaySettings, manager: ProviderManager) {
        self.settings = settings
        self.manager = manager
        guard available else { return }
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound]) { _, _ in }
        cancellable = manager.objectWillChange
            .merge(with: settings.objectWillChange)
            .debounce(for: .milliseconds(500), scheduler: RunLoop.main)
            .sink { [weak self] in self?.check() }
    }

    private func check() {
        guard available, settings.notifyEnabled else { return }
        for a in manager.active {
            let usedPct = Int(((1 - a.snap.remainingRatio) * 100).rounded())
            let resetKey = a.snap.resetAt.map { String(Int($0.timeIntervalSince1970)) } ?? "na"
            for th in settings.notifyThresholds.sorted() where usedPct >= th {
                let key = "\(a.spec.id)|\(th)|\(resetKey)"
                guard !notified.contains(key) else { continue }
                notified.insert(key)
                fire(name: a.spec.name, threshold: th)
            }
        }
        if notified.count > 200 { notified.removeAll() }   // 과도 성장 방지
    }

    private func fire(name: String, threshold: Int) {
        let content = UNMutableNotificationContent()
        content.title = settings.t("notify.title")
        content.body = settings.tf("notify.body", name).replacingOccurrences(of: "%d", with: String(threshold))
        content.sound = .default
        let req = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
        UNUserNotificationCenter.current().add(req)
    }
}
