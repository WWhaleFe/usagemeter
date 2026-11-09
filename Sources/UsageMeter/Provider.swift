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

/// 모델별 한도 버킷(예: Claude "Fable", Antigravity "Gemini 4 Argon"). 서버가 주는 display_name과 잔여율.
struct UsageModelBucket: Equatable {
    let label: String
    let remainingRatio: Double   // 0.0~1.0
    let resetAt: Date?
    /// true면 5시간 한도 버킷, false면 주간 한도 버킷(기본).
    var isFiveHour: Bool = false
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

    /// 구독 플랜/티어(예: "Max (5x)", "Pro"). 없으면 nil.
    var plan: String? = nil

    /// 모델별 주간/5시간 한도 버킷(예: Claude "Fable", Antigravity "Gemini 4 Argon"). 없으면 nil.
    var modelBuckets: [UsageModelBucket]? = nil

    /// `remainingRatio`가 가장 급한 모델 버킷(예: Gemini 4 Argon)을 가리킬 때,
    /// 드롭다운 '5시간 잔여' 줄에 원래 5시간 버킷 값을 그대로 표시하기 위한 값. nil이면 `remainingRatio` 사용.
    var fiveHourRatio: Double? = nil

    /// 원래 5시간 버킷의 리셋 시각. nil이면 `resetAt` 사용.
    var fiveHourResetAt: Date? = nil

    /// 세부 버킷별 잔여 비율(id -> 0.0~1.0). HistoryStore가 다중 버킷 소모량을 놓치지 않고 집계하는 데 쓴다.
    var rawBuckets: [String: Double]? = nil
}

/// 한 AI의 세부 사용량 항목(5시간·주간·모델별 버킷). 설정 탭·드롭다운·분할 띠가 공유한다.
struct UsageSubItem: Equatable, Identifiable {
    /// 설정 저장 키로 쓰는 안정적인 식별자(예: "5h", "model:Gemini 4 Argon:weekly").
    let id: String
    /// 모델 버킷 이름(서버 제공). 대표 5시간·주간 항목은 nil.
    let modelLabel: String?
    let isFiveHour: Bool
    let remainingRatio: Double
    let resetAt: Date?
}

extension UsageSnapshot {
    /// 표시 순서: 대표 5시간 → 대표 주간 → 모델별 버킷(서버가 준 순서).
    var subItems: [UsageSubItem] {
        var out = [UsageSubItem(id: "5h", modelLabel: nil, isFiveHour: true,
                                remainingRatio: fiveHourRatio ?? remainingRatio,
                                resetAt: fiveHourResetAt ?? resetAt)]
        if let w = secondaryRatio {
            out.append(UsageSubItem(id: "weekly", modelLabel: nil, isFiveHour: false,
                                    remainingRatio: w, resetAt: secondaryResetAt))
        }
        for b in modelBuckets ?? [] {
            out.append(UsageSubItem(id: "model:\(b.label):\(b.isFiveHour ? "5h" : "weekly")",
                                    modelLabel: b.label, isFiveHour: b.isFiveHour,
                                    remainingRatio: b.remainingRatio, resetAt: b.resetAt))
        }
        return out
    }
}

/// 각 AI 수집기가 구현하는 프로토콜. 폴링 주기마다 `fetch()`가 호출된다.
protocol UsageProvider {
    /// Provider 식별자 (UsageSnapshot.id 와 동일).
    var id: String { get }

    /// 현재 사용량을 조회해 공통 포맷으로 반환. 실패 시 status로 표현(throw 대신).
    func fetch() async -> UsageSnapshot
}
