import AppKit
import SwiftUI

/// 분석 대시보드 창(SwiftUI `AnalyticsView`). 한 번 만들고 재사용.
@MainActor
final class AnalyticsWindowController: NSObject {
    private var window: NSWindow?
    private let settings: OverlaySettings
    private let manager: ProviderManager
    private let history: HistoryStore

    init(settings: OverlaySettings, manager: ProviderManager, history: HistoryStore) {
        self.settings = settings
        self.manager = manager
        self.history = history
        super.init()
    }

    func show() {
        if window == nil {
            let hosting = NSHostingController(rootView:
                AnalyticsView(settings: settings, manager: manager, history: history))
            // 자동 사이징을 끄지 않으면 SwiftUI가 창 min/max를 덮어써 리사이즈 한계가 안 먹는다.
            hosting.sizingOptions = []
            let win = NSWindow(contentViewController: hosting)
            win.title = settings.t("analytics.title")
            // .resizable 제외 = 크기 완전 고정(사용자가 조절 불가).
            win.styleMask = [.titled, .closable, .miniaturizable]
            win.isReleasedWhenClosed = false
            let size = NSSize(width: 1000, height: 720)
            win.contentMinSize = size
            win.contentMaxSize = size
            win.setContentSize(size)
            win.center()
            window = win
        }
        window?.title = settings.t("analytics.title")
        if let win = window { DockPresence.shared.register(win) }   // 열려 있는 동안 Dock 아이콘 표시
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
    }
}
