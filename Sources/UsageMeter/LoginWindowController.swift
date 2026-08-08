import AppKit

/// 앱 안 로그인 창을 관리한다. 사용자가 창 안에서 각 서비스에 로그인하면
/// 세션 쿠키가 자동으로 잡힌다(키를 손으로 붙여넣을 필요 없음).
///
/// 로그인 감지는 1.2초마다 폴링(네비게이션 이벤트만으론 놓칠 수 있어서).
/// 판정 기준은 WebSession.probeLoginDone() — 서비스마다 다르다.
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
            // 이미 조회가 되면(=확실히 로그인됨) 창 없이 완료.
            // 제미나이 조회 JS는 실패할 때 최대 6초까지 DOM을 뒤지므로, 여기서 다 기다리면
            // 버튼을 눌러도 몇 초간 아무 일도 안 일어난 것처럼 보인다 → 짧게 끊는다.
            if await probeUsageOK(timeout: 2.0) {
                finish(true, present: false)
                return
            }
            session.presentLoginWindow(delegate: self)
            pollTask = Task { [weak self] in
                while let self, !self.finished {
                    if await self.session.probeLoginDone() { self.finish(true, present: true); break }
                    try? await Task.sleep(nanoseconds: 1_200_000_000)
                }
            }
        }
    }

    /// 조회 성공 여부를 제한 시간 안에서만 확인(초과하면 false로 보고 로그인 창을 연다).
    private func probeUsageOK(timeout: Double) async -> Bool {
        await withTaskGroup(of: Bool.self) { group in
            group.addTask { await self.session.probeUsageOK() }
            group.addTask {
                try? await Task.sleep(nanoseconds: UInt64(timeout * 1_000_000_000))
                return false
            }
            let first = await group.next() ?? false
            group.cancelAll()
            return first
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
