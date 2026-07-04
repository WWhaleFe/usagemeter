import AppKit
import SwiftUI

/// 화면 위에 떠 있지만 클릭은 아래 앱으로 통과시키는 투명한 오버레이 창.
///
/// PoC 1단계의 핵심 검증 대상. 계획서 5장의 설정을 그대로 적용한다:
/// - borderless: 제목표시줄/테두리 없음
/// - 투명 배경: 가운데는 아래 앱이 그대로 보이고, 테두리 띠만 그려진다
/// - always-on-top: 다른 앱 위에 항상 보임
/// - click-through: 창 위를 클릭해도 아래 앱이 클릭됨
final class OverlayWindow: NSWindow {

    /// 지정한 화면(screen)의 전체 영역을 덮는 오버레이 창을 만든다.
    init(screen: NSScreen, settings: OverlaySettings, manager: ProviderManager) {
        super.init(
            contentRect: screen.frame,
            styleMask: [.borderless],   // 제목표시줄/테두리 없음
            backing: .buffered,
            defer: false
        )

        // --- 투명 배경 ---
        backgroundColor = .clear
        isOpaque = false
        hasShadow = false

        // --- 항상 위에 (always-on-top) ---
        // 스크린세이버 수준으로 올려 일반 앱 창들 위에 뜨게 한다.
        level = .screenSaver

        // --- 클릭 통과 (click-through) ---
        // 창을 마우스 이벤트에 투명하게 만들어 아래 앱을 그대로 클릭할 수 있게 한다.
        ignoresMouseEvents = true

        // --- 모든 데스크톱(Spaces)에서 보이게 ---
        collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]

        // 화면 전체 프레임에 맞추고, 테두리 띠를 그리는 SwiftUI 뷰를 얹는다.
        setFrame(screen.frame, display: true)
        let hosting = NSHostingView(rootView: BorderView(settings: settings, manager: manager))
        hosting.frame = NSRect(origin: .zero, size: screen.frame.size)
        hosting.autoresizingMask = [.width, .height]
        contentView = hosting
    }

    // borderless 창은 기본적으로 키 창이 될 수 없다. 오버레이는 포커스를 가로채면
    // 안 되므로 그대로 두는 게 맞다(클릭 통과와도 일관).
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}
