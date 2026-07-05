import AppKit
import SwiftUI

/// 화면 위에서 메뉴바선/Dock선을 **직접 드래그로 조정**하는 임시 전체 화면 창.
/// 설정 창의 "화면에서 드래그로 조정" 버튼으로 열고, 완료를 누르면 닫힌다.
/// 조정 중에도 오버레이가 실시간으로 따라 움직인다(설정 @Published 바인딩).
@MainActor
final class LineDragOverlay {
    static let shared = LineDragOverlay()
    private var window: NSWindow?

    func show(settings: OverlaySettings) {
        guard window == nil, let screen = NSScreen.main else { return }
        let win = NSWindow(contentRect: screen.frame, styleMask: [.borderless],
                           backing: .buffered, defer: false)
        win.backgroundColor = .clear
        win.isOpaque = false
        win.hasShadow = false
        win.level = .screenSaver
        win.ignoresMouseEvents = false
        win.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        win.contentView = NSHostingView(rootView: LineDragView(settings: settings) { [weak self] in
            self?.hide()
        })
        win.orderFrontRegardless()
        window = win
    }

    func hide() {
        window?.orderOut(nil)
        window = nil
    }
}

/// 드래그 조정 화면: 살짝 어두운 배경 위에 경계선 핸들 + 완료 버튼.
private struct LineDragView: View {
    @ObservedObject var settings: OverlaySettings
    let onDone: () -> Void

    var body: some View {
        GeometryReader { geo in
            let H = geo.size.height, W = geo.size.width
            ZStack {
                Color.black.opacity(0.22)
                if settings.menuLineEnabled {
                    handle(label: "\(settings.t("part.menuLine"))  \(Int(settings.menuLineHeight))pt",
                           color: .orange, y: settings.menuLineHeight, width: W)
                        .gesture(DragGesture(coordinateSpace: .global).onChanged { v in
                            settings.menuLineHeight = min(max(5, v.location.y), H * 0.6)
                        })
                }
                if settings.dockLineEnabled {
                    handle(label: "\(settings.t("part.dockLine"))  \(Int(settings.dockLineHeight))pt",
                           color: .cyan, y: H - settings.dockLineHeight, width: W)
                        .gesture(DragGesture(coordinateSpace: .global).onChanged { v in
                            settings.dockLineHeight = min(max(5, H - v.location.y), H * 0.6)
                        })
                }
                VStack {
                    Button {
                        onDone()
                    } label: {
                        Text(settings.t("part.dragDone"))
                            .font(.headline)
                            .padding(.horizontal, 22).padding(.vertical, 8)
                    }
                    .buttonStyle(.borderedProminent)
                    .padding(.top, 64)
                    Spacer()
                }
            }
        }
        .ignoresSafeArea()
    }

    /// 드래그 가능한 경계선 핸들(선 + 라벨). 위아래 넓은 히트 영역.
    private func handle(label: String, color: Color, y: CGFloat, width: CGFloat) -> some View {
        ZStack {
            Rectangle().fill(color).frame(height: 2)
            Text(label)
                .font(.caption).bold().foregroundStyle(.black)
                .padding(.horizontal, 8).padding(.vertical, 3)
                .background(color.opacity(0.95), in: Capsule())
                .offset(y: -17)
        }
        .frame(width: width, height: 30)
        .contentShape(Rectangle())
        .position(x: width / 2, y: y)
    }
}
