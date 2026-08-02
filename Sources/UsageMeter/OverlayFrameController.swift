import AppKit
import SwiftUI
import Combine

/// 오버레이를 **화면 중앙을 덮지 않는 얇은 스트립 창들의 집합**으로 구성한다.
///
/// 배경: macOS 15.3.2+에는 다른 앱의 창(투명이라도)이 App Store 등 시스템 창의
/// 결제·설치 "확인 버튼" 위를 덮으면 그 버튼/Touch ID를 숨기는 오버레이 방지 동작이 있다.
/// 전체 화면 한 장짜리 오버레이는 화면 중앙(버튼이 뜨는 곳)을 항상 덮으므로 이 동작에 걸린다.
///
/// 근본 해결: 테두리 잉크는 화면 가장자리와 몇 개의 가로/세로 선 위에만 존재하므로,
/// **그 선들을 감싸는 얇은 스트립 창**만 만들고 중앙에는 창을 두지 않는다. 각 스트립 창은
/// 동일한 `BorderView`를 전체 화면 크기로 담되 위치를 어긋나게(offset) 두고 창 경계로 클리핑해,
/// 자기 구역의 잉크만 보여준다 → 렌더링 결과는 기존과 동일하고 중앙만 비게 된다.
@MainActor
final class OverlayFrameController {

    /// 클릭 통과·키 불가 스트립 창.
    private final class StripWindow: NSWindow {
        override var canBecomeKey: Bool { false }
        override var canBecomeMain: Bool { false }
    }

    private let screen: NSScreen
    private let settings: OverlaySettings
    private let manager: ProviderManager

    /// 세로 좌·우(항상) + 가로 상·하(항상) + 메뉴선·Dock선(옵션).
    private enum Strip: CaseIterable { case vLeft, vRight, hTop, hBottom, hMenu, hDock }
    private var windows: [Strip: StripWindow] = [:]
    private var hosts: [Strip: NSHostingView<BorderView>] = [:]
    private var cancellable: AnyCancellable?

    /// 스트립 두께(잉크가 가장자리에서 뻗는 최대치 + 여유).
    /// 세로 폭: inset(≤4)+선(≤8)+곡률(≤80) ≈ 92 → 150. 가로 높이: +노치(≤100) ≈ 112 → 170.
    private let vThick: CGFloat = 150
    private let hThick: CGFloat = 170
    /// 스트립 경계(직선 구간)에서 이음매가 안 보이게 하는 미세 겹침.
    private let seam: CGFloat = 2

    init(screen: NSScreen, settings: OverlaySettings, manager: ProviderManager) {
        self.screen = screen
        self.settings = settings
        self.manager = manager
        for s in Strip.allCases {
            let w = StripWindow(contentRect: .zero, styleMask: [.borderless], backing: .buffered, defer: false)
            w.backgroundColor = .clear
            w.isOpaque = false
            w.hasShadow = false
            w.isReleasedWhenClosed = false
            w.level = .statusBar   // 메뉴바·일반 앱 위, 팝업/시스템 인증 UI 아래.
            w.ignoresMouseEvents = true
            w.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]
            let container = NSView()
            container.wantsLayer = true
            container.layer?.masksToBounds = true   // 창 밖(다른 스트립 구역)은 클리핑.
            let host = NSHostingView(rootView: BorderView(settings: settings, manager: manager))
            container.addSubview(host)
            w.contentView = container
            windows[s] = w
            hosts[s] = host
        }
        reconfigure()   // 유효한 스트립만 앞으로, 비활성 선(메뉴/Dock 꺼짐)은 내린다.

        // 선 위치·표시 여부가 바뀌면(메뉴선/Dock선 드래그·토글 등) 스트립 배치를 갱신.
        cancellable = settings.objectWillChange
            .receive(on: RunLoop.main)
            .sink { [weak self] in self?.reconfigure() }
    }

    /// 현재 설정 기준으로 각 스트립 창의 프레임과 내부 호스팅 뷰 오프셋을 다시 계산한다.
    func reconfigure() {
        // 임시 숨김이면 모든 스트립 창을 내린다(팝업 메뉴 '테두리 숨기기').
        if settings.hideOverlay {
            for w in windows.values { w.orderOut(nil) }
            return
        }
        let S = screen.frame
        // 가로 스트립은 세로 스트립 사이(좌우 vThick)만 덮되, 이음매 방지로 seam 만큼 겹친다.
        let hx = S.minX + vThick - seam
        let hw = S.width - 2 * (vThick - seam)

        func frame(_ s: Strip) -> CGRect? {
            switch s {
            case .vLeft:   return CGRect(x: S.minX, y: S.minY, width: vThick, height: S.height)
            case .vRight:  return CGRect(x: S.maxX - vThick, y: S.minY, width: vThick, height: S.height)
            case .hTop:    return CGRect(x: hx, y: S.maxY - hThick, width: hw, height: hThick)
            case .hBottom: return CGRect(x: hx, y: S.minY, width: hw, height: hThick)
            case .hMenu:
                guard settings.menuLineEnabled else { return nil }
                let yCenter = S.maxY - settings.menuLineHeight          // 메뉴선: 위(top)에서 menuH.
                return CGRect(x: hx, y: yCenter - hThick / 2, width: hw, height: hThick)
            case .hDock:
                guard settings.dockLineEnabled else { return nil }
                let yCenter = S.minY + settings.dockLineHeight          // Dock선: 아래(bottom)에서 dockH.
                return CGRect(x: hx, y: yCenter - hThick / 2, width: hw, height: hThick)
            }
        }

        for s in Strip.allCases {
            guard let w = windows[s], let host = hosts[s] else { continue }
            if let f = frame(s) {
                w.setFrame(f, display: true)
                // 전체 화면 크기의 BorderView를, 화면 좌표가 일치하도록 창 안에서 어긋나게 배치.
                host.frame = CGRect(x: S.minX - f.minX, y: S.minY - f.minY, width: S.width, height: S.height)
                if !w.isVisible { w.orderFrontRegardless() }
            } else {
                w.orderOut(nil)   // 비활성 선(메뉴/Dock 꺼짐)은 창을 내린다.
            }
        }
    }

    /// 해상도·모니터 구성 변경 등으로 이 컨트롤러를 폐기할 때 창과 구독을 정리한다.
    func invalidate() {
        cancellable?.cancel()
        cancellable = nil
        for w in windows.values { w.orderOut(nil); w.close() }
        windows.removeAll()
        hosts.removeAll()
    }
}
