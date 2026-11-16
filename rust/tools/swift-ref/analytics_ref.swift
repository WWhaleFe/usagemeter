// Swift 기준 값 생성기: 이력(history.json)에서 분석 계산 결과를 JSON으로 내보낸다.
// 사용: swiftc -O analytics_ref.swift -o /tmp/analytics_ref && /tmp/analytics_ref <history.json> <out.json>
// HistoryStore.swift / AnalyticsModel.swift / AnalyticsView.swift의 계산 코드를 그대로 복사했다.
// 바뀐 곳은 SwiftUI·ObservableObject·Color 제거, Date()를 고정 now로, Calendar.current를 서울 고정 달력으로 바꾼 것뿐이다.
import Foundation

// MARK: - 고정 환경(재현성)

let seoul = TimeZone(identifier: "Asia/Seoul")!
/// Calendar.current 대신: 서울 시간대, 주 시작 = 일요일(Calendar.firstWeekday = 1).
let calendar: Calendar = {
    var c = Calendar(identifier: .gregorian)
    c.timeZone = seoul
    c.locale = Locale(identifier: "en_US")
    c.firstWeekday = 1
    return c
}()

// MARK: - HistoryStore.swift (복사)

struct UsageSample: Codable {
    let t: Date
    let ratios: [String: Double]
    var buckets: [String: [String: Double]]? = nil
    var items: [String: [String: Double]]? = nil
}

struct PaceInfo {
    let depletion: Date
    let ratePerHour: Double
}

final class HistoryRef {
    var samples: [UsageSample] = []
    /// Date() 대신 고정 시각.
    let now: Date
    init(samples: [UsageSample], now: Date) { self.samples = samples; self.now = now }

    func series(for id: String, hours: Double = 24) -> [(Date, Double)] {
        let cutoff = now.addingTimeInterval(-hours * 3600)
        return samples.compactMap { s in
            guard s.t >= cutoff, let r = s.ratios[id] else { return nil }
            return (s.t, r)
        }
    }

    func itemSeries(for id: String, item: String, since: Date) -> [(t: Date, r: Double)] {
        samples.compactMap { s in
            guard s.t >= since, let r = s.items?[id]?[item] ?? legacyItem(s, id, item) else { return nil }
            return (s.t, r)
        }
    }

    private func legacyItem(_ s: UsageSample, _ id: String, _ item: String) -> Double? {
        guard s.items?[id] == nil, item == "5h" else { return nil }
        if let b = s.buckets?[id] { return b["gemini-5h"] }
        return s.ratios[id]
    }

    func itemIDs(for id: String, since: Date) -> [String] {
        var seen = Set<String>(), out: [String] = []
        for s in samples where s.t >= since {
            guard let m = s.items?[id] else {
                if !seen.contains("5h"), legacyItem(s, id, "5h") != nil { seen.insert("5h"); out.append("5h") }
                continue
            }
            for k in m.keys.sorted() where !seen.contains(k) { seen.insert(k); out.append(k) }
        }
        return out
    }

    var earliestItems: Date? { samples.first { $0.items != nil }?.t }
    var earliest: Date? { samples.map(\.t).min() }

    func consumptionEvents(for id: String, since: Date) -> [(t: Date, amount: Double)] {
        let s = samples
            .compactMap { smp -> (Date, Double, [String: Double]?)? in
                guard smp.t >= since, let r = smp.ratios[id] else { return nil }
                return (smp.t, r, smp.buckets?[id])
            }
            .sorted { $0.0 < $1.0 }
        guard s.count >= 2 else { return [] }
        var out: [(t: Date, amount: Double)] = []
        for i in 1..<s.count {
            var drop = max(0, s[i - 1].1 - s[i].1)
            if let prevB = s[i - 1].2, let curB = s[i].2 {
                var bucketDropMax = 0.0
                for (bk, curVal) in curB {
                    if let prevVal = prevB[bk] {
                        let d = prevVal - curVal
                        if d > bucketDropMax { bucketDropMax = d }
                    }
                }
                drop = max(drop, bucketDropMax)
            }
            if drop > 0.0005 { out.append((s[i].0, drop)) }
        }
        return out
    }

    func pace(for id: String) -> PaceInfo? {
        let s = series(for: id, hours: 3)
        guard let first = s.first, let last = s.last else { return nil }
        let dtHours = last.0.timeIntervalSince(first.0) / 3600
        guard dtHours > 0.05 else { return nil }
        let ratePerHour = (first.1 - last.1) / dtHours
        guard ratePerHour > 0.0005 else { return nil }
        let hoursToEmpty = last.1 / ratePerHour
        return PaceInfo(depletion: now.addingTimeInterval(hoursToEmpty * 3600), ratePerHour: ratePerHour)
    }
}

// MARK: - AnalyticsModel.swift (복사, Color 제거)

struct AnalyticsSeries {
    let id: String
    let providerID: String
    let itemID: String?
    let points: [(t: Date, r: Double)]
    let events: [(t: Date, amount: Double)]
    var consumed: Double { events.reduce(0) { $0 + $1.amount } }
}

struct SeriesStats {
    let current: Double?
    let consumed: Double
    let resets: Int
    let windowUse: Double?
    let limitHits: Int
    let ratePerHour: Double?
}

enum AnalyticsMath {
    static let resetJump = 0.03
    static let limitLevel = 0.02

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

// MARK: - AnalyticsView.swift (복사: 범위·집계·서식)

enum AnalyticsRange: String, CaseIterable {
    case day1, day3, week, month
    var hours: Double {
        switch self { case .day1: return 24; case .day3: return 72; case .week: return 168; case .month: return 720 }
    }
    var bucketSeconds: Double {
        switch self { case .day1: return 3600; case .day3: return 3 * 3600; case .week: return 24 * 3600; case .month: return 24 * 3600 }
    }
}

func bucketStart(_ d: Date, range: AnalyticsRange) -> Date {
    let day = calendar.startOfDay(for: d)
    let step = range.bucketSeconds
    if step >= 86400 { return day }
    return day.addingTimeInterval((d.timeIntervalSince(day) / step).rounded(.down) * step)
}

func busiestHour(_ series: [AnalyticsSeries]) -> Int? {
    var byHour = [Int: Double]()
    for s in series { for e in s.events {
        byHour[calendar.component(.hour, from: e.t), default: 0] += e.amount
    } }
    // 동률이면 Swift 사전 순서가 불안정하므로 가장 이른 시각으로 고정(Rust와 같은 규칙).
    return byHour.max(by: { $0.value < $1.value || ($0.value == $1.value && $0.key > $1.key) })?.key
}

func hourSums(_ s: AnalyticsSeries) -> [Int: Double] {
    var byHour = [Int: Double]()
    for e in s.events { byHour[calendar.component(.hour, from: e.t), default: 0] += e.amount }
    return byHour
}

func bucketSums(_ s: AnalyticsSeries, range: AnalyticsRange) -> [Date: Double] {
    var by = [Date: Double]()
    for e in s.events { by[bucketStart(e.t, range: range), default: 0] += e.amount }
    return by
}

/// 행 = 달력 첫 요일부터. 반환: 행 i의 요일 번호(0=일) 와 168칸(행 우선) 값.
func heatCells(_ series: [AnalyticsSeries]) -> (rows: [Int], cells: [Double]) {
    let syms = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
    let first = calendar.firstWeekday - 1
    let rows = (0..<7).map { syms[(first + $0) % 7] }
    var grid = [String: Double]()
    for s in series {
        for e in s.events {
            let c = calendar.dateComponents([.weekday, .hour], from: e.t)
            grid["\(syms[(c.weekday ?? 1) - 1])|\(c.hour ?? 0)", default: 0] += e.amount * 100
        }
    }
    let peak = max(grid.values.max() ?? 0, 0.0001)
    let cells = rows.enumerated().flatMap { i, d in (0..<24).map { h in
        ((grid["\(d)|\(h)"] ?? 0) / peak).squareRoot()
    } }
    return ((0..<7).map { (first + $0) % 7 }, cells)
}

func times(_ v: Double) -> String { String(format: v < 1 ? "%.2f" : "%.1f", v) }
func num(_ v: Double) -> String { v >= 10 ? String(format: "%.0f", v) : String(format: "%.1f", v) }

// MARK: - 내보내기

func unix(_ d: Date) -> Double { d.timeIntervalSince1970 }
func opt(_ v: Double?) -> Any { v.map { $0 as Any } ?? NSNull() }

func statsJSON(_ st: SeriesStats) -> [String: Any] {
    ["current": opt(st.current), "consumed": st.consumed, "resets": st.resets,
     "window_use": opt(st.windowUse), "limit_hits": st.limitHits, "rate_per_hour": opt(st.ratePerHour)]
}

func seriesJSON(_ s: AnalyticsSeries, range: AnalyticsRange, now: Date, since: Date) -> [String: Any] {
    let down = AnalyticsMath.downsample(s.points, since: since, until: now)
    let hs = hourSums(s)
    return [
        "points": s.points.count,
        "events": s.events.count,
        "consumed": s.consumed,
        "downsampled": down.count,
        "downsampled_sum_r": down.reduce(0.0) { $0 + $1.r },
        "stats": statsJSON(AnalyticsMath.stats(s, now: now)),
        "hour_sums": (0..<24).map { hs[$0] ?? 0 },
        "bucket_sums": bucketSums(s, range: range).sorted { $0.key < $1.key }.map { [unix($0.key), $0.value] },
        "busiest_hour": busiestHour([s]).map { $0 as Any } ?? NSNull(),
    ]
}

let args = CommandLine.arguments
guard args.count == 3 else { fatalError("usage: analytics_ref <history.json> <out.json>") }
let data = try Data(contentsOf: URL(fileURLWithPath: args[1]))
let all = try JSONDecoder().decode([UsageSample].self, from: data)
let now = all.map(\.t).max()!.addingTimeInterval(60)
let hist = HistoryRef(samples: all, now: now)
let providers = ["claude", "gemini", "codex"]

var out: [String: Any] = [:]
out["now"] = unix(now)
out["sample_count"] = all.count
out["earliest"] = opt(hist.earliest.map(unix))
out["earliest_items"] = opt(hist.earliestItems.map(unix))

var paceOut: [String: Any] = [:]
for p in providers {
    if let pc = hist.pace(for: p) {
        paceOut[p] = ["depletion": unix(pc.depletion), "rate_per_hour": pc.ratePerHour]
    } else { paceOut[p] = NSNull() }
}
out["pace"] = paceOut

var ranges: [String: Any] = [:]
for range in AnalyticsRange.allCases {
    let since = now.addingTimeInterval(-range.hours * 3600)
    var perProvider: [String: Any] = [:]
    var aiAll: [AnalyticsSeries] = []
    var allItems: [AnalyticsSeries] = []
    for id in providers {
        // buildModel의 aiAll: 점 2개 미만이면 계열 없음.
        let pts = hist.series(for: id, hours: range.hours).map { (t: $0.0, r: $0.1) }
        var entry: [String: Any] = ["series_points": pts.count]
        if pts.count >= 2 {
            let s = AnalyticsSeries(id: id, providerID: id, itemID: nil, points: pts,
                                    events: hist.consumptionEvents(for: id, since: since))
            aiAll.append(s)
            entry["ai"] = seriesJSON(s, range: range, now: now, since: since)
        } else {
            entry["ai"] = NSNull()
        }
        // itemSeries: 지금 스냅샷이 없으므로 순서 = 이력에 처음 나온 순서.
        let ids = hist.itemIDs(for: id, since: since)
        entry["item_ids"] = ids
        var items: [String: Any] = [:]
        for k in ids {
            let ipts = hist.itemSeries(for: id, item: k, since: since)
            guard !ipts.isEmpty else { continue }
            let s = AnalyticsSeries(id: "\(id)/\(k)", providerID: id, itemID: k, points: ipts,
                                    events: AnalyticsMath.drops(ipts))
            allItems.append(s)
            items[k] = seriesJSON(s, range: range, now: now, since: since)
        }
        entry["items"] = items
        perProvider[id] = entry
    }
    let total = aiAll.reduce(0.0) { $0 + $1.consumed }
    let days = max(1.0, range.hours / 24)
    let hits = allItems.reduce(0) { $0 + AnalyticsMath.stats($1, now: now).limitHits }
    let heat = heatCells(aiAll)
    ranges[range.rawValue] = [
        "providers": perProvider,
        "kpi": ["total": total, "per_day": total / days, "days": days, "limit_hits": hits,
                "busiest_hour": busiestHour(aiAll).map { $0 as Any } ?? NSNull(),
                "total_text": times(total), "per_day_text": times(total / days)],
        "heat": ["weekdays": heat.rows, "cells": heat.cells],
        "bucket_start_since": unix(bucketStart(since, range: range)),
        "bucket_start_now": unix(bucketStart(now, range: range)),
    ]
}
out["ranges"] = ranges

let samples: [Double] = [0, 0.004, 0.005, 0.5, 0.995, 1, 1.25, 9.95, 9.96, 10, 10.5, 11.5, 123.456, 0.125]
out["format"] = samples.map { ["v": $0, "times": times($0), "num": num($0)] }

let json = try JSONSerialization.data(withJSONObject: out, options: [.sortedKeys, .prettyPrinted])
try json.write(to: URL(fileURLWithPath: args[2]))
print("ok", all.count, "samples ->", args[2])
