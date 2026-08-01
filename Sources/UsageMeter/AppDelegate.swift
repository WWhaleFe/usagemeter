import AppKit
import Combine

/// 앱 수명주기를 담당. 시작 시 연결된 각 화면마다 오버레이 창을 하나씩 띄운다.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {

    private var overlays: [OverlayWindow] = []
    private var statusBar: StatusBarController?
    private var refreshScheduler: RefreshScheduler?
    private var historyStore: HistoryStore?
    private var notificationManager: NotificationManager?
    private var cancellables: Set<AnyCancellable> = []

    /// 오버레이 설정(오버레이·메뉴바 공유).
    private let settings = OverlaySettings()
    /// 여러 AI 세션 관리자.
    private let manager = ProviderManager()

    func applicationDidFinishLaunching(_ notification: Notification) {
        // 외형(라이트/다크/시스템) 적용 + 변경 구독.
        applyAppearance()
        settings.$appearance
            .receive(on: RunLoop.main)
            .sink { [weak self] app in self?.applyAppearance(app) }
            .store(in: &cancellables)

        for screen in NSScreen.screens {
            let window = OverlayWindow(screen: screen, settings: settings, manager: manager)
            window.orderFrontRegardless()
            overlays.append(window)
        }

        let history = HistoryStore(manager: manager)
        historyStore = history
        notificationManager = NotificationManager(settings: settings, manager: manager)
        statusBar = StatusBarController(settings: settings, manager: manager, history: history)
        manager.start(clearFirst: !settings.keepLoggedIn)
        refreshScheduler = RefreshScheduler(settings: settings, manager: manager)

        print("[UsageMeter] 오버레이 \(overlays.count)개 + 메뉴바 표시됨.")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    /// Dock 아이콘 클릭 시 이미 열린 설정·분석 창을 모두 앞으로 가져온다.
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows: Bool) -> Bool {
        DockPresence.shared.bringOpenToFront()
        return true
    }

    /// 외형 설정을 NSApp에 반영(nil = 기기 설정 따름).
    private func applyAppearance(_ app: AppAppearance? = nil) {
        NSApp.appearance = (app ?? settings.appearance).nsAppearance
    }
}
