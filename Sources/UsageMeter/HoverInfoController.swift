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

    /// 메뉴바 상태 아이콘 창의 화면 좌표 프레임(없으면 메뉴바 호버 비활성).
    var menuBarAnchor: (() -> NSRect?)?
    /// 드롭다운 메뉴가 열려 있는 동안엔 패널을 띄우지 않는다.
    var suppressed = false {
        didSet { if suppressed { hide() } }
    }

    init(settings: OverlaySettings) {
        self.settings = settings

        label = NSTextField(labelWithString: "")
        label.font = .systemFont(ofSize: 12, weight: .medium)
        // 라이트/다크에 맞춰 자동으로 글자색이 바뀐다(#테마 대응).
        label.textColor = .labelColor
        label.usesSingleLineMode = false
        label.maximumNumberOfLines = 0
        label.lineBreakMode = .byWordWrapping
        label.translatesAutoresizingMaskIntoConstraints = false

        // 반투명 재질 배경(라이트/다크 자동 대응) — 어떤 화면 위에서도 대비 유지.
        let bg = NSVisualEffectView()
        bg.material = .hudWindow
        bg.state = .active
        bg.blendingMode = .behindWindow
        bg.wantsLayer = true
        bg.layer?.cornerRadius = 7
        bg.layer?.masksToBounds = true
        bg.addSubview(label)

        panel = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 10, height: 10),
                         styleMask: .borderless, backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        // 오버레이 선(.statusBar)보다 한 단계 위 — 선에 가려지지 않게. 단, 팝업 메뉴·시스템
        // 보안 인증 UI는 막지 않도록 보안 레벨(.screenSaver)보다는 낮게 유지한다.
        panel.level = NSWindow.Level(rawValue: NSWindow.Level.statusBar.rawValue + 1)
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

    /// 커서가 메뉴바 아이콘 또는 선택된 변 근처인지 확인해 패널을 띄우거나 숨긴다.
    /// 상태 아이콘 위의 이동은 전역 모니터에 안 잡히므로 버튼 트래킹 영역에서도 호출된다.
    func handleMove() {
        guard !suppressed else { hide(); return }
        let p = NSEvent.mouseLocation
        if settings.menuBarHoverInfoEnabled, let anchor = menuBarAnchor?(),
           let (zone, screen) = Self.menuBarZone(around: anchor), zone.contains(p) {
            showBelowMenuBar(zone: zone, cursorX: p.x, on: screen)
            return
        }
        guard settings.hoverInfoEnabled else { hide(); return }
        guard let screen = NSScreen.screens.first(where: { $0.frame.contains(p) }) else { hide(); return }
        let f = screen.frame
        let th = max(6, settings.thickness) + 7      // 선 근처로 볼 여유
        // 세그먼트 모델 기준: 어느 AI든 그 변에 선이 있어야 호버 대상(#스키마 일원화).
        // 그 위에서 사용자가 고른 호버 영역(변 전체 / 꼭짓점 구간)만 반응한다.
        let cs = settings.hoverCornerSize
        // (변, 커서가 그 변 위인지, 변 시작/끝 쪽 꼭짓점, 시작·끝에서의 거리)
        let checks: [(BorderEdge, Bool, BorderCorner, BorderCorner, CGFloat, CGFloat)] = [
            (.top,    abs(p.y - f.maxY) <= th && (f.minX...f.maxX).contains(p.x), .topLeft, .topRight, p.x - f.minX, f.maxX - p.x),
            (.bottom, abs(p.y - f.minY) <= th && (f.minX...f.maxX).contains(p.x), .bottomLeft, .bottomRight, p.x - f.minX, f.maxX - p.x),
            (.left,   abs(p.x - f.minX) <= th && (f.minY...f.maxY).contains(p.y), .bottomLeft, .topLeft, p.y - f.minY, f.maxY - p.y),
            (.right,  abs(p.x - f.maxX) <= th && (f.minY...f.maxY).contains(p.y), .bottomRight, .topRight, p.y - f.minY, f.maxY - p.y),
        ]
        let near = checks.contains { edge, onEdge, c1, c2, d1, d2 in
            guard onEdge, settings.edgeActive(edge) else { return false }
            if settings.hoverEdges.contains(edge) { return true }
            return (settings.hoverCorners.contains(c1) && d1 <= cs)
                || (settings.hoverCorners.contains(c2) && d2 <= cs)
        }
        near ? show(at: p, on: screen) : hide()
    }

    /// 메뉴바 아이콘 호버 영역. 아이콘이 있는 디스플레이 기준으로 계산한다:
    /// 세로 = 화면 맨 위 ~ 실제 메뉴바 아래 끝(노치 모델은 메뉴바가 더 높음),
    /// 가로 = 아이콘 폭 + 화면 폭에 비례한 여유(고해상도·큰 화면일수록 넓게).
    static func menuBarZone(around anchor: NSRect) -> (NSRect, NSScreen)? {
        guard let screen = NSScreen.screens.first(where: { $0.frame.intersects(anchor) })
                ?? NSScreen.screens.first(where: { $0.frame.contains(NSPoint(x: anchor.midX, y: anchor.minY)) })
        else { return nil }
        let f = screen.frame
        // 메뉴바 자동 숨김 등으로 visibleFrame이 메뉴바를 반영하지 않으면 상태 아이콘 창 높이로 대체.
        let measured = f.maxY - screen.visibleFrame.maxY
        let barH = max(measured > 0 ? measured : 0, anchor.height, NSStatusBar.system.thickness)
        let padX = min(24, max(8, f.width * 0.006))
        let padBelow = max(4, barH * 0.25)       // 메뉴바 아래로 살짝 벗어나도 유지
        let zone = NSRect(x: anchor.minX - padX, y: f.maxY - barH - padBelow,
                          width: anchor.width + padX * 2, height: barH + padBelow + 1)
        return (zone, screen)
    }

    /// 메뉴바 아래, 커서 x 기준으로 패널을 띄운다(화면 밖으로 안 나가게 보정).
    private func showBelowMenuBar(zone: NSRect, cursorX: CGFloat, on screen: NSScreen) {
        label.stringValue = infoText
        let size = label.intrinsicContentSize
        let winW = size.width + 20, winH = size.height + 12
        let f = screen.frame
        let ox = min(max(f.minX + 4, cursorX - winW / 2), f.maxX - winW - 4)
        let oy = (screen.visibleFrame.maxY < f.maxY ? screen.visibleFrame.maxY : zone.minY) - winH - 6
        panel.setFrame(NSRect(x: ox, y: oy, width: winW, height: winH), display: true)
        panel.orderFrontRegardless()
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
