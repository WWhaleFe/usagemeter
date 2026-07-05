import AppKit
import SwiftUI

/// 설정 창(SwiftUI `SettingsView`를 담은 일반 창)을 관리한다. 한 번 만들고 재사용.
@MainActor
final class SettingsWindowController: NSObject {
    private var window: NSWindow?
    private let settings: OverlaySettings
    private let manager: ProviderManager

    init(settings: OverlaySettings, manager: ProviderManager) {
        self.settings = settings
        self.manager = manager
        super.init()
    }

    /// 설정 창을 앞으로 가져온다(없으면 생성).
    func show() {
        if window == nil {
            let hosting = NSHostingController(rootView: SettingsView(settings: settings, manager: manager))
            let win = NSWindow(contentViewController: hosting)
            win.title = "UsageMeter 설정"
            win.styleMask = [.titled, .closable, .miniaturizable]
            win.isReleasedWhenClosed = false     // 닫아도 파괴하지 않고 재사용
            win.center()
            window = win
        }
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
    }
}
