import AppKit

/// 앱 수명주기를 담당. 시작 시 연결된 각 화면마다 오버레이 창을 하나씩 띄운다.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {

    private var overlays: [OverlayWindow] = []
    private var statusBar: StatusBarController?
    private var refreshScheduler: RefreshScheduler?

    /// 오버레이 설정(오버레이·메뉴바 공유).
    private let settings = OverlaySettings()
    /// 여러 AI 세션 관리자.
    private let manager = ProviderManager()

    func applicationDidFinishLaunching(_ notification: Notification) {
        for screen in NSScreen.screens {
            let window = OverlayWindow(screen: screen, settings: settings, manager: manager)
            window.orderFrontRegardless()
            overlays.append(window)
        }

        statusBar = StatusBarController(settings: settings, manager: manager)
        manager.start(clearFirst: !settings.keepLoggedIn)
        refreshScheduler = RefreshScheduler(settings: settings, manager: manager)

        print("[UsageMeter] 오버레이 \(overlays.count)개 + 메뉴바 표시됨.")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }
}
