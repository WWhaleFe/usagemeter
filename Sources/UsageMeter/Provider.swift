import Foundation

/// AI별 사용량 데이터 수집 계층의 **공통 출력 포맷** (PoC 트랙 ②에서 확정).
///
/// 계획서 3-4장의 Provider 패턴: Claude·Gemini·ChatGPT 등 서로 다른 소스를
/// 이 하나의 `UsageSnapshot` 모양으로 변환해서 오버레이(띠 렌더링)에 넘긴다.
/// 오버레이는 어떤 AI인지 몰라도 `remainingRatio` 하나만 보고 띠 길이를 그린다.

/// 인증/조회 상태. 띠 색을 흐리게 하거나 "만료" 경고를 띄우는 판단에 쓴다.
enum UsageStatus: Equatable {
    case ok                 // 정상 조회됨
    case stale              // 이전 값은 있으나 최근 갱신 실패(네트워크 등) → 흐리게 표시
    case authExpired        // 세션 키 만료/무효 → 재로그인 필요
    case unavailable(String) // 이 소스는 사용량을 못 준다(예: ChatGPT) 또는 기타 오류
}

/// 한 AI의 "지금 남은 사용량" 스냅샷. 오버레이가 소비하는 최종 단위.
///
/// 2026-07-03 실제 claude.ai `/usage` 응답으로 검증한 필드 구성:
/// - `five_hour.utilization` → 주(primary) 비율의 원천 (remaining = 1 - used)
/// - `seven_day.utilization` → 보조(secondary) 비율의 원천
/// - `resets_at`(ISO-8601 UTC) → `resetAt`
struct UsageSnapshot: Equatable {
    /// Provider 식별자. 예: "claude", "gemini", "chatgpt".
    let id: String

    /// 주(primary) 잔여 비율 0.0~1.0. Claude 기본값 = 5시간 세션 한도의 잔여.
    /// 오버레이 띠 길이는 이 값에 비례한다. (used% 를 1-used 로 뒤집은 값)
    let remainingRatio: Double

    /// 보조 잔여 비율 0.0~1.0 (없으면 nil). Claude 기본값 = 주간 한도의 잔여.
    let secondaryRatio: Double?

    /// Opus 주간 잔여 비율 0.0~1.0 (Claude, 없으면 nil).
    let opusRatio: Double?

    /// 주 한도가 리셋되는 시각(UTC). "임박 경고"와 카운트다운 표시에 쓴다.
    let resetAt: Date?

    /// 보조 한도 리셋 시각(UTC, 없으면 nil).
    let secondaryResetAt: Date?

    /// Opus 주간 리셋 시각(UTC, 없으면 nil).
    let opusResetAt: Date?

    /// 조회 상태.
    let status: UsageStatus

    /// 이 스냅샷을 만든 시각(로컬). stale 판정·표시에 쓴다.
    let lastUpdated: Date
}

/// 각 AI 수집기가 구현하는 프로토콜. 폴링 주기마다 `fetch()`가 호출된다.
protocol UsageProvider {
    /// Provider 식별자 (UsageSnapshot.id 와 동일).
    var id: String { get }

    /// 현재 사용량을 조회해 공통 포맷으로 반환. 실패 시 status로 표현(throw 대신).
    func fetch() async -> UsageSnapshot
}
