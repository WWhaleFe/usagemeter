import SwiftUI

/// 여러 AI(클로드·제미나이)의 세션·로그인·사용량을 함께 관리한다.
/// 각 provider는 자기 WebSession을 갖고, 최신 스냅샷을 여기서 보관·발행한다.
@MainActor
final class ProviderManager: ObservableObject {

    struct ProviderState {
        let spec: ProviderSpec
        let session: WebSession
        var snapshot: UsageSnapshot?
        var loggedIn: Bool
    }

    /// 표시 순서(스펙 순서).
    let order: [String]
    @Published private(set) var states: [String: ProviderState] = [:]

    private var loginControllers: [String: LoginWindowController] = [:]

    init() {
        var s: [String: ProviderState] = [:]
        var o: [String] = []
        for spec in ProviderSpec.all {
            s[spec.id] = ProviderState(spec: spec, session: WebSession(spec: spec), snapshot: nil, loggedIn: false)
            o.append(spec.id)
        }
        states = s
        order = o
    }

    /// 앱 시작. clearFirst면(로그인 유지 옵션 꺼짐) 먼저 로그아웃해 재로그인을 요구.
    func start(clearFirst: Bool) {
        Task {
            if clearFirst {
                for id in order { await states[id]?.session.logout() }
            }
            for id in order { states[id]?.session.load() }
            for id in order { refreshWithRetry(id) }
        }
    }

    func refreshAll() { for id in order { refresh(id) } }

    func refresh(_ id: String) {
        guard let session = states[id]?.session else { return }
        Task { apply(await session.fetchUsage(), to: id) }
    }

    /// 시작 시 로드 타이밍 때문에 놓칠 수 있어 몇 번 재시도.
    private func refreshWithRetry(_ id: String) {
        guard let session = states[id]?.session else { return }
        Task {
            for _ in 0..<4 {
                let snap = await session.fetchUsage()
                apply(snap, to: id)
                if snap.status == .ok { break }
                try? await Task.sleep(nanoseconds: 2_000_000_000)
            }
        }
    }

    /// 조회 결과를 상태에 반영한다.
    ///
    /// 핵심: **조회 실패 ≠ 로그아웃**. 제미나이처럼 화면을 파싱해 읽는 서비스는 구글이 DOM을
    /// 조금만 바꿔도 조회가 실패하는데, 예전엔 그때마다 loggedIn을 false로 내려서
    /// "로그인이 저절로 풀린다"처럼 보였다. 이제 로그아웃은 서버/페이지가 실제로
    /// 미로그인을 알려준 경우(.authExpired)에만 표시하고, 단순 읽기 실패면 직전 정상값을
    /// 그대로 유지한다(오버레이도 마지막 값을 계속 보여준다).
    private func apply(_ snap: UsageSnapshot, to id: String) {
        guard var st = states[id] else { return }
        switch snap.status {
        case .ok:
            st.snapshot = snap
            st.loggedIn = true
        case .authExpired:
            st.snapshot = snap
            st.loggedIn = false
        case .stale, .unavailable:
            // 한 번이라도 제대로 읽었으면 그 값을 지키고, 아니면 실패 상태를 그대로 반영.
            if st.snapshot?.status == .ok { return }
            st.snapshot = snap
        }
        states[id] = st
    }

    func login(_ id: String) {
        guard let session = states[id]?.session else { return }
        let ctrl = LoginWindowController(session: session) { [weak self] ok in
            if ok { self?.refresh(id) }
        }
        loginControllers[id] = ctrl
        ctrl.show()
    }

    func logout(_ id: String) {
        guard let session = states[id]?.session else { return }
        Task {
            await session.logout()
            states[id]?.loggedIn = false
            states[id]?.snapshot = nil
        }
    }

    func logoutAll() { for id in order { logout(id) } }

    func isLoggedIn(_ id: String) -> Bool { states[id]?.loggedIn ?? false }
    var anyLoggedIn: Bool { states.values.contains { $0.loggedIn } }

    /// 로그인된 provider들의 (스펙, 스냅샷). 표시 순서.
    var active: [(spec: ProviderSpec, snap: UsageSnapshot)] {
        order.compactMap { id in
            guard let st = states[id], st.loggedIn, let snap = st.snapshot, snap.status == .ok else { return nil }
            return (st.spec, snap)
        }
    }
}
