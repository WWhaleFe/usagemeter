import AppKit
import Combine

/// 앱 수명주기를 담당. 시작 시 연결된 각 화면마다 오버레이 창을 하나씩 띄운다.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {

    private var overlayFrames: [OverlayFrameController] = []
    private var statusBar: StatusBarController?
    private var refreshScheduler: RefreshScheduler?
    private var historyStore: HistoryStore?
    private var notificationManager: NotificationManager?
    private var updateChecker: UpdateChecker?
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

        // 오버레이를 화면 중앙을 덮지 않는 얇은 스트립 창들로 구성한다(OverlayFrameController).
        // 중앙에 창이 없으므로 App Store 등 시스템 인증 UI(구매/설치 버튼·Touch ID)를 가리지 않는다.
        rebuildOverlays()

        // 해상도·모니터 구성이 바뀌면 오버레이를 새 화면에 맞게 다시 구성한다.
        NotificationCenter.default.addObserver(
            self, selector: #selector(screenParametersChanged),
            name: NSApplication.didChangeScreenParametersNotification, object: nil)

        let history = HistoryStore(manager: manager)
        historyStore = history
        notificationManager = NotificationManager(settings: settings, manager: manager)
        let updater = UpdateChecker(settings: settings)
        updateChecker = updater
        statusBar = StatusBarController(settings: settings, manager: manager, history: history, updater: updater)
        manager.start(clearFirst: !settings.keepLoggedIn)
        refreshScheduler = RefreshScheduler(settings: settings, manager: manager)

        print("[UsageMeter] 오버레이 \(overlayFrames.count)개 화면 + 메뉴바 표시됨.")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    /// 연결된 각 화면마다 오버레이 스트립 프레임을 (재)생성한다.
    private func rebuildOverlays() {
        overlayFrames.forEach { $0.invalidate() }
        overlayFrames = NSScreen.screens.map {
            OverlayFrameController(screen: $0, settings: settings, manager: manager)
        }
    }

    /// 해상도·모니터 구성이 바뀌면: 노치·곡률을 새 화면에 맞게 조정하고 오버레이를 다시 만든다.
    @objc private func screenParametersChanged() {
        settings.adaptToScreenChange()
        rebuildOverlays()
    }

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
