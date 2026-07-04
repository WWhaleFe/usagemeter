import AppKit

/// 커서가 테두리(색 선) 근처에 오면 현재 사용량 정보를 작은 패널로 보여준다.
///
/// 오버레이 창은 클릭 통과(`ignoresMouseEvents`)라 마우스 이벤트를 못 받으므로,
/// 전역/로컬 마우스 이동 모니터로 커서 위치를 추적한다(마우스 모니터는 접근성
/// 권한이 필요 없다). 선택된 변의 화면 가장자리 근처면 패널을 띄운다.
@MainActor
final class HoverInfoController: NSObject {

    private let settings: OverlaySettings
    /// 표시할 현재 정보(사용량 새로고침 때 갱신).
    var infoText: String = "로그인이 필요합니다"

    private let panel: NSWindow
    private let label: NSTextField
    private var globalMonitor: Any?
    private var localMonitor: Any?

    init(settings: OverlaySettings) {
        self.settings = settings

        label = NSTextField(labelWithString: "")
        label.font = .systemFont(ofSize: 12, weight: .medium)
        label.textColor = .white
        label.usesSingleLineMode = false
        label.maximumNumberOfLines = 0
        label.lineBreakMode = .byWordWrapping
        label.translatesAutoresizingMaskIntoConstraints = false

        let bg = NSView()
        bg.wantsLayer = true
        bg.layer?.backgroundColor = NSColor.black.withAlphaComponent(0.82).cgColor
        bg.layer?.cornerRadius = 7
        bg.addSubview(label)

        panel = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 10, height: 10),
                         styleMask: .borderless, backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .popUpMenu                        // 오버레이보다 위
        panel.ignoresMouseEvents = true
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]
        panel.contentView = bg

        super.init()

        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: bg.leadingAnchor, constant: 10),
            label.trailingAnchor.constraint(equalTo: bg.trailingAnchor, constant: -10),
            label.topAnchor.constraint(equalTo: bg.topAnchor, constant: 6),
            label.bottomAnchor.constraint(equalTo: bg.bottomAnchor, constant: -6),
        ])

        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .mouseMoved) { [weak self] _ in
            self?.handleMove()
        }
        localMonitor = NSEvent.addLocalMonitorForEvents(matching: .mouseMoved) { [weak self] e in
            self?.handleMove(); return e
        }
    }

    /// 커서가 선택된 변 근처인지 확인해 패널을 띄우거나 숨긴다.
    private func handleMove() {
        let p = NSEvent.mouseLocation
        guard let screen = NSScreen.screens.first(where: { $0.frame.contains(p) }) else { hide(); return }
        let f = screen.frame
        let th = max(6, settings.thickness) + 7      // 선 근처로 볼 여유
        var near = false
        if settings.edges.contains(.top),    abs(p.y - f.maxY) <= th, (f.minX...f.maxX).contains(p.x) { near = true }
        if settings.edges.contains(.bottom), abs(p.y - f.minY) <= th, (f.minX...f.maxX).contains(p.x) { near = true }
        if settings.edges.contains(.left),   abs(p.x - f.minX) <= th, (f.minY...f.maxY).contains(p.y) { near = true }
        if settings.edges.contains(.right),  abs(p.x - f.maxX) <= th, (f.minY...f.maxY).contains(p.y) { near = true }
        near ? show(at: p, on: screen) : hide()
    }

    private func show(at p: NSPoint, on screen: NSScreen) {
        label.stringValue = infoText
        let size = label.intrinsicContentSize
        let winW = size.width + 20, winH = size.height + 12
        // 커서 우상단에 띄우되 화면 밖으로 나가지 않게 보정.
        var ox = p.x + 14, oy = p.y + 14
        let f = screen.frame
        if ox + winW > f.maxX { ox = p.x - winW - 14 }
        if oy + winH > f.maxY { oy = p.y - winH - 14 }
        panel.setFrame(NSRect(x: ox, y: oy, width: winW, height: winH), display: true)
        panel.orderFrontRegardless()
    }

    private func hide() { panel.orderOut(nil) }
}
