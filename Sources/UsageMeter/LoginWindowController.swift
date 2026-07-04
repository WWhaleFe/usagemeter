import AppKit

/// 앱 안 로그인 창을 관리한다. 사용자가 창 안에서 claude.ai에 로그인하면
/// 세션 쿠키가 자동으로 잡힌다(sessionKey를 손으로 붙여넣을 필요 없음).
///
/// 로그인 감지는 1.2초마다 폴링(네비게이션 이벤트만으론 놓칠 수 있어서).
@MainActor
final class LoginWindowController: NSObject, NSWindowDelegate {

    private let session: WebSession
    /// 로그인 완료(true) 또는 취소(false) 시 호출.
    private let onDone: (Bool) -> Void
    private var finished = false
    private var pollTask: Task<Void, Never>?

    init(session: WebSession, onDone: @escaping (Bool) -> Void) {
        self.session = session
        self.onDone = onDone
        super.init()
    }

    func show() {
        Task {
            if await session.probeLoggedIn() {   // 이미 로그인돼 있으면 창 없이 완료
                finish(true, present: false)
                return
            }
            session.presentLoginWindow(delegate: self)
            pollTask = Task { [weak self] in
                while let self, !self.finished {
                    if await self.session.probeLoggedIn() { self.finish(true, present: true); break }
                    try? await Task.sleep(nanoseconds: 1_200_000_000)
                }
            }
        }
    }

    private func finish(_ success: Bool, present: Bool) {
        guard !finished else { return }
        finished = true
        pollTask?.cancel()
        if present { session.dismissLoginWindow() }
        onDone(success)
    }

    // 사용자가 로그인 없이 창을 닫으면 취소로 처리(웹뷰는 유지).
    func windowShouldClose(_ sender: NSWindow) -> Bool {
        finish(false, present: true)
        return false
    }
}
