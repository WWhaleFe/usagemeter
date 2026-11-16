import SwiftUI

/// 분석 창이 그리는 계열 하나: AI 대표 잔여율, 또는 AI의 세부 항목(5시간·주간·모델별) 하나.
struct AnalyticsSeries: Identifiable {
    /// "claude"(AI 대표) 또는 "claude/5h"(세부 항목).
    let id: String
    let providerID: String
    /// 세부 항목 id(UsageSubItem.id). AI 대표 계열이면 nil.
    let itemID: String?
    let name: String
    let color: Color
    /// (시각, 잔여율 0~1) 시간순.
    let points: [(t: Date, r: Double)]
    /// 소비 이벤트(잔여율이 줄어든 만큼, 0~1).
    let events: [(t: Date, amount: Double)]
    /// 지금 한도의 리셋 시각(실시간 스냅샷에서). 소진 예상과 비교한다.
    let resetAt: Date?

    var consumed: Double { events.reduce(0) { $0 + $1.amount } }
}

/// 계열 하나의 요약 통계(세부 항목 상세 표의 한 줄).
struct SeriesStats {
    /// 마지막 잔여율.
    let current: Double?
    /// 기간 소비(0~1, 자기 한도 기준).
    let consumed: Double
    /// 리셋(잔여율이 크게 오른) 횟수.
    let resets: Int
    /// 끝난 창(리셋으로 닫힌 구간)들에서 리셋 전까지 쓴 비율의 평균. 끝난 창이 없으면 nil.
    let windowUse: Double?
    /// 잔여가 한도 도달선(2%) 이하로 떨어진 횟수.
    let limitHits: Int
    /// 최근 3시간(마지막 리셋 이후) 시간당 잔여율 하락. 쓰는 중이 아니면 nil.
    let ratePerHour: Double?
}

/// 이력 표본에서 소비·리셋·창 사용률을 계산하는 순수 함수 모음.
enum AnalyticsMath {
    /// 잔여율이 이만큼 이상 오르면 리셋으로 본다(서버 값의 미세한 흔들림은 무시).
    static let resetJump = 0.03
    /// 잔여율이 이 값 이하면 한도에 닿은 것으로 본다.
    static let limitLevel = 0.02

    /// 연속 표본의 하락분 = 소비 이벤트. 오르는 구간(리셋)은 소비로 치지 않는다.
    static func drops(_ pts: [(t: Date, r: Double)]) -> [(t: Date, amount: Double)] {
        guard pts.count >= 2 else { return [] }
        var out: [(t: Date, amount: Double)] = []
        for i in 1..<pts.count {
            let d = pts[i - 1].r - pts[i].r
            if d > 0.0005 { out.append((pts[i].t, d)) }
        }
        return out
    }

    static func stats(_ s: AnalyticsSeries, now: Date = Date()) -> SeriesStats {
        let pts = s.points
        var resets = 0, hits = 0
        var windows: [Double] = []
        var segMin = pts.first?.r ?? 1
        var lastResetAt: Date? = nil
        for i in pts.indices {
            let r = pts[i].r
            if i > 0 {
                if r - pts[i - 1].r >= resetJump {
                    // 리셋: 직전 창이 끝났다 → 그 창에서 리셋 전까지 쓴 비율을 남긴다.
                    resets += 1
                    windows.append(1 - segMin)
                    segMin = r
                    lastResetAt = pts[i].t
                } else {
                    segMin = min(segMin, r)
                }
                if r <= limitLevel && pts[i - 1].r > limitLevel { hits += 1 }
            }
        }
        // 소진 속도: 최근 3시간 중 마지막 리셋 이후 구간의 기울기.
        let from = max(now.addingTimeInterval(-3 * 3600), lastResetAt ?? .distantPast)
        let recent = pts.filter { $0.t >= from }
        var rate: Double? = nil
        if let a = recent.first, let b = recent.last {
            let dt = b.t.timeIntervalSince(a.t) / 3600
            if dt > 0.05, a.r - b.r > 0.0005 { rate = (a.r - b.r) / dt }
        }
        return SeriesStats(current: pts.last?.r, consumed: s.consumed, resets: resets,
                           windowUse: windows.isEmpty ? nil : windows.reduce(0, +) / Double(windows.count),
                           limitHits: hits, ratePerHour: rate)
    }

    /// 선 차트용 다운샘플: 기간을 `buckets`칸으로 나눠 칸마다 가장 낮은·높은 점만 남긴다
    /// (한 달 치 수만 개 점을 그대로 그리면 Charts가 느려진다. 최저·최고를 둘 다 남겨 리셋 모양은 유지).
    static func downsample(_ pts: [(t: Date, r: Double)], since: Date, until: Date,
                           buckets: Int = 240) -> [(t: Date, r: Double)] {
        guard pts.count > buckets * 2 else { return pts }
        let span = max(1, until.timeIntervalSince(since)) / Double(buckets)
        var lo: [Int: (t: Date, r: Double)] = [:], hi: [Int: (t: Date, r: Double)] = [:]
        for p in pts {
            let k = Int(p.t.timeIntervalSince(since) / span)
            if lo[k].map({ p.r < $0.r }) ?? true { lo[k] = p }
            if hi[k].map({ p.r > $0.r }) ?? true { hi[k] = p }
        }
        var out: [(t: Date, r: Double)] = []
        for k in lo.keys.sorted() {
            guard let a = lo[k], let b = hi[k] else { continue }
            if a.t == b.t { out.append(a) } else { out.append(contentsOf: a.t < b.t ? [a, b] : [b, a]) }
        }
        return out
    }
}
