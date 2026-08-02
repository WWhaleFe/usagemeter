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

        // App Store 등 특정 앱이 전면일 때는 오버레이를 숨긴다(아래 observeFrontmostApp 참고).
        observeFrontmostApp()

        let history = HistoryStore(manager: manager)
        historyStore = history
        notificationManager = NotificationManager(settings: settings, manager: manager)
        statusBar = StatusBarController(settings: settings, manager: manager, history: history)
        manager.start(clearFirst: !settings.keepLoggedIn)
        refreshScheduler = RefreshScheduler(settings: settings, manager: manager)

        print("[UsageMeter] 오버레이 \(overlays.count)개 + 메뉴바 표시됨.")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }

    // MARK: - App Store 전면일 때 오버레이 뒤로 보내기
    //
    // macOS 15.3.2+에는 다른 앱의 창(투명이라도)이 App Store 창 위(구매/설치 버튼 영역)를 덮고
    // 있으면 그 버튼과 Touch ID 인증을 숨기는 시스템 동작이 있다(오버레이 피싱 방지 추정,
    // Apple FB20444423). 트리거는 "버튼 위에 겹침"이므로, 해당 앱이 전면이면 오버레이를 숨기는
    // 대신 레벨을 일반 창 아래로 낮춰 App Store 창 뒤로 보낸다("위에 덮음"을 해소). 이러면 App
    // Store가 가리지 않는 화면 가장자리 등에는 테두리가 그대로 보인다. 벗어나면 원래 레벨로 복원.

    /// 전면일 때 오버레이를 뒤로 보낼 앱들의 번들 ID.
    private let sendBackBundleIDs: Set<String> = ["com.apple.AppStore"]
    private var overlaySentBack = false
    /// 평소 레벨(일반 앱 위) / 뒤로 보낼 때 레벨(일반 창 아래).
    private let frontLevel: NSWindow.Level = .statusBar
    private let backLevel = NSWindow.Level(rawValue: NSWindow.Level.normal.rawValue - 1)

    private func observeFrontmostApp() {
        NSWorkspace.shared.notificationCenter.addObserver(
            self, selector: #selector(frontmostAppChanged(_:)),
            name: NSWorkspace.didActivateApplicationNotification, object: nil)
        // 시작 시 현재 전면 앱 기준으로 즉시 반영.
        updateOverlayStacking(for: NSWorkspace.shared.frontmostApplication)
    }

    @objc private func frontmostAppChanged(_ note: Notification) {
        let app = note.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication
        updateOverlayStacking(for: app)
    }

    private func updateOverlayStacking(for app: NSRunningApplication?) {
        let sendBack = (app?.bundleIdentifier).map(sendBackBundleIDs.contains) ?? false
        guard sendBack != overlaySentBack else { return }
        overlaySentBack = sendBack
        for w in overlays {
            w.level = sendBack ? backLevel : frontLevel
            if sendBack { w.orderBack(nil) } else { w.orderFrontRegardless() }
        }
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
