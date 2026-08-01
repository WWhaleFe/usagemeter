import AppKit

/// 설정·분석 같은 "실제 창"이 하나라도 열려 있으면 Dock 아이콘을 띄우고(.regular),
/// 모두 닫히면 다시 숨긴다(.accessory). 평소엔 메뉴바 아이콘만 있는 보조 앱이지만,
/// 창이 떠 있는 동안엔 Dock 아이콘을 눌러 이미 열린 창을 앞으로 가져올 수 있다.
@MainActor
final class DockPresence: NSObject, NSWindowDelegate {
    static let shared = DockPresence()

    private var open = Set<ObjectIdentifier>()
    private var windows: [ObjectIdentifier: NSWindow] = [:]

    /// 표시할 창을 등록(창 컨트롤러의 show()에서 호출). 닫힘 감지를 위해 delegate를 잡는다.
    func register(_ w: NSWindow) {
        let key = ObjectIdentifier(w)
        windows[key] = w
        w.delegate = self
        open.insert(key)
        update()
    }

    func windowWillClose(_ n: Notification) {
        guard let w = n.object as? NSWindow else { return }
        let key = ObjectIdentifier(w)
        open.remove(key)
        windows[key] = nil
        // 닫히는 도중 정책을 바꾸지 않도록 다음 런루프에서 갱신.
        DispatchQueue.main.async { [weak self] in self?.update() }
    }

    /// Dock 아이콘 클릭(reopen) 시 열려 있는 창을 모두 최상단으로.
    func bringOpenToFront() {
        for key in open { windows[key]?.makeKeyAndOrderFront(nil) }
        if !open.isEmpty { NSApp.activate(ignoringOtherApps: true) }
    }

    private func update() {
        let policy: NSApplication.ActivationPolicy = open.isEmpty ? .accessory : .regular
        if NSApp.activationPolicy() != policy {
            NSApp.setActivationPolicy(policy)
        }
        if !open.isEmpty { NSApp.activate(ignoringOtherApps: true) }
    }
}
