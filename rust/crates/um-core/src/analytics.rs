//! 분석 계산. (Swift AnalyticsModel.swift 의 `AnalyticsMath` + AnalyticsView.swift 의 집계·서식)
//!
//! 전부 순수 함수다. 시각은 Unix 초(`f64`)이고, "로컬 시각"이 필요한 계산(시간대별·요일×시간대·
//! 기간 버킷)은 `chrono::TimeZone`을 인자로 받는다(Swift `Calendar.current`에 해당).
//! 앱은 `chrono::Local`, 테스트는 `FixedOffset`을 넘기면 결과가 재현된다.

use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike};

/// 잔여율이 이만큼 이상 오르면 리셋으로 본다. (Swift `AnalyticsMath.resetJump`)
pub const RESET_JUMP: f64 = 0.03;
/// 잔여율이 이 값 이하면 한도에 닿은 것으로 본다. (Swift `AnalyticsMath.limitLevel`)
pub const LIMIT_LEVEL: f64 = 0.02;
/// 소비로 치는 최소 하락분. (Swift `drops`, `consumptionEvents`)
pub const MIN_DROP: f64 = 0.0005;
/// 선 차트 다운샘플 칸 수. (Swift `downsample(buckets:)` 기본값)
pub const DOWNSAMPLE_BUCKETS: usize = 240;

/// (시각, 잔여율 0~1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub t: f64,
    pub r: f64,
}

/// 소비 이벤트: 잔여율이 줄어든 만큼(0~1)을 나중 시각에 기록.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    pub t: f64,
    pub amount: f64,
}

/// 계열 하나(AI 대표 또는 세부 항목)의 점과 소비 이벤트. (Swift `AnalyticsSeries`에서 표시 정보 제외)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Series {
    pub points: Vec<Point>,
    pub events: Vec<Event>,
}

impl Series {
    /// 기간 소비 = 이벤트 합. (Swift `consumed`, 앞에서부터 순서대로 더한다)
    pub fn consumed(&self) -> f64 {
        self.events.iter().fold(0.0, |a, e| a + e.amount)
    }
}

/// 계열 하나의 요약 통계. (Swift `SeriesStats`)
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesStats {
    pub current: Option<f64>,
    pub consumed: f64,
    pub resets: u32,
    /// 끝난 창들의 리셋 전 사용 비율 평균. 끝난 창이 없으면 None.
    pub window_use: Option<f64>,
    pub limit_hits: u32,
    /// 최근 3시간(마지막 리셋 이후) 시간당 잔여율 하락. 쓰는 중이 아니면 None.
    pub rate_per_hour: Option<f64>,
}

/// 연속 점의 하락분 = 소비 이벤트. 오르는 구간(리셋)은 소비로 치지 않는다. (Swift `drops`)
pub fn drops(pts: &[Point]) -> Vec<Event> {
    pts.windows(2)
        .filter_map(|w| {
            let d = w[0].r - w[1].r;
            (d > MIN_DROP).then_some(Event {
                t: w[1].t,
                amount: d,
            })
        })
        .collect()
}

/// 요약 통계. `now`는 소진 속도의 "최근 3시간" 기준. (Swift `AnalyticsMath.stats`)
pub fn stats(s: &Series, now: f64) -> SeriesStats {
    let pts = &s.points;
    let (mut resets, mut hits) = (0u32, 0u32);
    let mut windows: Vec<f64> = Vec::new();
    let mut seg_min = pts.first().map_or(1.0, |p| p.r);
    for i in 1..pts.len() {
        let r = pts[i].r;
        if r - pts[i - 1].r >= RESET_JUMP {
            // 리셋: 직전 창이 끝났다. 그 창에서 리셋 전까지 쓴 비율을 남긴다.
            resets += 1;
            windows.push(1.0 - seg_min);
            seg_min = r;
        } else {
            seg_min = seg_min.min(r);
        }
        if r <= LIMIT_LEVEL && pts[i - 1].r > LIMIT_LEVEL {
            hits += 1;
        }
    }
    let rate = recent_rate(pts, now);
    SeriesStats {
        current: pts.last().map(|p| p.r),
        consumed: s.consumed(),
        resets,
        window_use: (!windows.is_empty())
            .then(|| windows.iter().fold(0.0, |a, w| a + w) / windows.len() as f64),
        limit_hits: hits,
        rate_per_hour: rate,
    }
}

/// 마지막 리셋(잔여율이 `RESET_JUMP` 이상 오른 점) 시각. 없으면 None.
fn last_reset(pts: &[Point]) -> Option<f64> {
    pts.windows(2).rev().find(|w| w[1].r - w[0].r >= RESET_JUMP).map(|w| w[1].t)
}

/// `from` 이후 첫 점과 마지막 점의 기울기(시간당 하락). 구간이 3분 이하거나 줄지 않았으면 None.
fn slope_from(pts: &[Point], from: f64) -> Option<f64> {
    let a = pts.iter().find(|p| p.t >= from)?;
    let b = pts.last()?;
    let dt = (b.t - a.t) / 3600.0;
    (dt > 0.05 && a.r - b.r > MIN_DROP).then(|| (a.r - b.r) / dt)
}

/// 최근 소진 속도: 최근 3시간 중 마지막 리셋 이후 구간의 기울기. 5시간 창처럼 짧은 창에 쓴다.
/// `now` 이후 점은 없다고 본다.
pub fn recent_rate(pts: &[Point], now: f64) -> Option<f64> {
    let from = (now - 3.0 * 3600.0).max(last_reset(pts).unwrap_or(f64::NEG_INFINITY));
    slope_from(pts, from)
}

/// 긴 창(주간)에서 평균 속도를 잴 최소 구간(시간). 이보다 짧으면 최근 속도로 잰다.
pub const LONG_RATE_MIN_HOURS: f64 = 6.0;

/// 긴 창(주간)의 소진 속도: 이번 창이 시작된 뒤(`window_start` 또는 마지막 리셋 중 늦은 쪽)의 평균 기울기.
/// 3시간 기울기는 주간 창에서 몇 시간 몰아 쓴 것을 일주일 내내 그렇게 쓴다고 늘려 잡아 크게 흔들린다.
/// 창 시작 뒤 지난 시간이 `LONG_RATE_MIN_HOURS`보다 짧으면 `recent_rate`로 잰다.
pub fn window_rate(pts: &[Point], now: f64, window_start: Option<f64>) -> Option<f64> {
    let start = match (window_start, last_reset(pts)) {
        (Some(a), Some(b)) => a.max(b),
        (a, b) => a.or(b).or_else(|| pts.first().map(|p| p.t))?,
    };
    let first = pts.iter().find(|p| p.t >= start)?;
    let last = pts.last()?;
    if (last.t - first.t) / 3600.0 < LONG_RATE_MIN_HOURS {
        return recent_rate(pts, now);
    }
    slope_from(pts, start)
}

/// 선 차트용 다운샘플: 기간을 `buckets`칸으로 나눠 칸마다 가장 낮은·높은 점만 남긴다.
/// 최저·최고를 둘 다 남겨 리셋 모양을 유지하고, 칸 안에서는 시간순으로 내보낸다.
/// 점이 `buckets * 2` 이하면 그대로 돌려준다. (Swift `downsample`)
pub fn downsample(pts: &[Point], since: f64, until: f64, buckets: usize) -> Vec<Point> {
    if pts.len() <= buckets * 2 {
        return pts.to_vec();
    }
    let span = (until - since).max(1.0) / buckets as f64;
    // 칸 번호 -> (최저, 최고). Swift는 사전, 여기서는 정렬 맵(번호 순 출력).
    let mut grid: std::collections::BTreeMap<i64, (Point, Point)> = Default::default();
    for p in pts {
        // Swift `Int(x)`는 0 방향 절삭. `as i64`도 같다.
        let k = ((p.t - since) / span) as i64;
        grid.entry(k)
            .and_modify(|(lo, hi)| {
                if p.r < lo.r {
                    *lo = *p;
                }
                if p.r > hi.r {
                    *hi = *p;
                }
            })
            .or_insert((*p, *p));
    }
    let mut out = Vec::with_capacity(grid.len() * 2);
    for (a, b) in grid.into_values() {
        if a.t == b.t {
            out.push(a);
        } else if a.t < b.t {
            out.extend([a, b]);
        } else {
            out.extend([b, a]);
        }
    }
    out
}

/// 분석 대시보드 기간. (Swift `AnalyticsRange`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsRange {
    Day1,
    Day3,
    Week,
    Month,
}

impl AnalyticsRange {
    pub const ALL: [AnalyticsRange; 4] = [Self::Day1, Self::Day3, Self::Week, Self::Month];

    /// 조회 범위(시간).
    pub fn hours(self) -> f64 {
        match self {
            Self::Day1 => 24.0,
            Self::Day3 => 72.0,
            Self::Week => 168.0,
            Self::Month => 720.0,
        }
    }

    /// 시계열 버킷 크기(초).
    pub fn bucket_seconds(self) -> f64 {
        match self {
            Self::Day1 => 3600.0,
            Self::Day3 => 3.0 * 3600.0,
            Self::Week | Self::Month => 24.0 * 3600.0,
        }
    }

    /// 문자열 키. (Swift `labelKey`)
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Day1 => "range.day1",
            Self::Day3 => "range.day3",
            Self::Week => "range.week",
            Self::Month => "range.month",
        }
    }

    /// 이 기간의 조회 시작 시각. (Swift `since`)
    pub fn since(self, now: f64) -> f64 {
        now - self.hours() * 3600.0
    }
}

// MARK: 로컬 시각 계산

fn local<Tz: TimeZone>(tz: &Tz, t: f64) -> DateTime<Tz> {
    let secs = t.floor();
    DateTime::from_timestamp(secs as i64, 0)
        .unwrap_or_default()
        .with_timezone(tz)
}

/// 로컬 시(0~23). (Swift `Calendar.component(.hour)`)
pub fn local_hour<Tz: TimeZone>(tz: &Tz, t: f64) -> u32 {
    local(tz, t).hour()
}

/// 로컬 요일, 일요일 = 0. (Swift `weekday - 1`)
pub fn local_weekday<Tz: TimeZone>(tz: &Tz, t: f64) -> u32 {
    local(tz, t).weekday().num_days_from_sunday()
}

/// 그 시각이 속한 로컬 날의 시작(자정). (Swift `Calendar.startOfDay`)
/// 자정이 없는 날(일광절약 건너뜀)은 그날 처음 존재하는 시각으로 잡는다.
pub fn start_of_day<Tz: TimeZone>(tz: &Tz, t: f64) -> f64 {
    let date: NaiveDate = local(tz, t).date_naive();
    for h in 0..24 {
        if let Some(naive) = date.and_hms_opt(h, 0, 0)
            && let Some(dt) = tz.from_local_datetime(&naive).earliest()
        {
            return dt.timestamp() as f64;
        }
    }
    local(tz, t).timestamp() as f64
}

/// 이벤트 시각의 버킷 시작(1일 = 1시간, 3일 = 3시간, 주간·한달 = 하루). 로컬 자정 기준.
/// (Swift AnalyticsView `bucketStart`)
pub fn bucket_start<Tz: TimeZone>(tz: &Tz, range: AnalyticsRange, t: f64) -> f64 {
    let day = start_of_day(tz, t);
    let step = range.bucket_seconds();
    if step >= 86400.0 {
        return day;
    }
    day + ((t - day) / step).floor() * step
}

/// 이벤트를 로컬 시(0~23)별로 합산. (Swift `hourChart` 의 byHour)
pub fn hour_sums<Tz: TimeZone>(tz: &Tz, events: &[Event]) -> [f64; 24] {
    let mut out = [0.0; 24];
    for e in events {
        out[local_hour(tz, e.t) as usize] += e.amount;
    }
    out
}

/// 가장 많이 쓴 시각(KPI "가장 바쁜 시간"). 이벤트가 없으면 None.
/// 동률이면 이른 시각(Swift는 사전 순서에 따라 불안정해서 고정 규칙을 둔다). (Swift `busiestHourText`)
pub fn busiest_hour<'a, Tz: TimeZone>(
    tz: &Tz,
    series: impl IntoIterator<Item = &'a Series>,
) -> Option<u32> {
    let mut by = [0.0f64; 24];
    let mut seen = [false; 24];
    for s in series {
        for e in &s.events {
            let h = local_hour(tz, e.t) as usize;
            by[h] += e.amount;
            seen[h] = true;
        }
    }
    let mut best: Option<usize> = None;
    for h in 0..24 {
        if seen[h] && best.is_none_or(|b| by[h] > by[b]) {
            best = Some(h);
        }
    }
    best.map(|h| h as u32)
}

/// 기간 버킷별 소비 합(시작 시각 순). (Swift `overTimeChart` 의 by)
pub fn bucket_sums<Tz: TimeZone>(
    tz: &Tz,
    range: AnalyticsRange,
    events: &[Event],
) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = Vec::new();
    for e in events {
        let k = bucket_start(tz, range, e.t);
        // 이벤트는 시간순이라 같은 버킷이 이어지지만, 일반 입력도 맞게 찾는다.
        match out.iter_mut().find(|(t, _)| *t == k) {
            Some(slot) => slot.1 += e.amount,
            None => out.push((k, e.amount)),
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// 히트맵 칸 하나. `amount`는 0~1(최댓값 대비 제곱근).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatCell {
    pub hour: u32,
    pub row: u32,
    pub amount: f64,
}

/// 요일 × 시간대 히트맵. `weekdays[row]` = 그 행의 요일(일요일 0). 칸은 행 우선 168개.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatMap {
    pub weekdays: [u32; 7],
    pub cells: Vec<HeatCell>,
}

/// 요일 × 시각 칸마다 소비(%p) 합계를 구해 색 농도(0~1)로 바꾼다.
/// 행 순서 = 달력의 첫 요일부터(`first_weekday`: 일요일 0). (Swift `heatCells`)
/// 농도 = 최댓값 대비 제곱근(몇몇 큰 칸에 묻혀 나머지가 다 어둡게 보이지 않도록).
pub fn heat_cells<'a, Tz: TimeZone>(
    tz: &Tz,
    first_weekday: u32,
    series: impl IntoIterator<Item = &'a Series>,
) -> HeatMap {
    let mut grid = [[0.0f64; 24]; 7];
    for s in series {
        for e in &s.events {
            let dt = local(tz, e.t);
            grid[dt.weekday().num_days_from_sunday() as usize][dt.hour() as usize] +=
                e.amount * 100.0;
        }
    }
    // 피크 하한 0.0001(전부 0일 때 0으로 나누지 않도록).
    let peak = grid
        .iter()
        .flatten()
        .fold(0.0f64, |a, &v| a.max(v))
        .max(0.0001);
    let mut weekdays = [0u32; 7];
    let mut cells = Vec::with_capacity(7 * 24);
    for (row, wd) in weekdays.iter_mut().enumerate() {
        *wd = (first_weekday + row as u32) % 7;
        for hour in 0..24u32 {
            let v = grid[*wd as usize][hour as usize];
            cells.push(HeatCell {
                hour,
                row: row as u32,
                amount: (v / peak).sqrt(),
            });
        }
    }
    HeatMap { weekdays, cells }
}

/// 계열들의 소비 합(KPI "총 소비"). (Swift `kpiRow` total)
pub fn total_consumed<'a>(series: impl IntoIterator<Item = &'a Series>) -> f64 {
    series.into_iter().fold(0.0, |a, s| a + s.consumed())
}

/// KPI "하루 평균"의 일수 = max(1, 시간/24). (Swift `kpiRow` days)
pub fn kpi_days(range: AnalyticsRange) -> f64 {
    (range.hours() / 24.0).max(1.0)
}

/// KPI "한도 도달" = 세부 항목 계열들의 limitHits 합. (Swift `kpiRow` hits)
pub fn total_limit_hits<'a>(items: impl IntoIterator<Item = &'a Series>, now: f64) -> u32 {
    items.into_iter().map(|s| stats(s, now).limit_hits).sum()
}

/// 세부 항목 순서 = 지금 스냅샷 순서 -> 이력에만 있는 항목. (Swift AnalyticsView `itemSeries`)
pub fn merge_item_order(live: &[String], history: &[String]) -> Vec<String> {
    let mut order = live.to_vec();
    for k in history {
        if !order.contains(k) {
            order.push(k.clone());
        }
    }
    order
}

/// 회분 표기: 1 미만은 소수 둘째 자리까지(조금 쓴 날이 0.0으로 보이지 않게). (Swift `times`)
pub fn times(v: f64) -> String {
    if v < 1.0 {
        format!("{v:.2}")
    } else {
        format!("{v:.1}")
    }
}

/// 10 이상은 정수, 그 아래는 소수 한 자리. (Swift `num`)
pub fn num(v: f64) -> String {
    if v >= 10.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

/// 두 계열이 사실상 같은 선인지. 같은 시각(초 단위)에 찍힌 점의 잔여율이 모두 0.5%p 안이고,
/// 짧은 쪽 점의 95% 이상이 서로 맞물리면 같다고 본다. (미니 차트에서 기본 선과 겹치는 세부 항목 선 빼기)
pub fn same_series(a: &[Point], b: &[Point]) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let by_t: std::collections::HashMap<i64, f64> = b.iter().map(|p| (p.t.round() as i64, p.r)).collect();
    let mut matched = 0usize;
    for p in a {
        match by_t.get(&(p.t.round() as i64)) {
            Some(r) if (r - p.r).abs() <= 0.005 => matched += 1,
            Some(_) => return false,
            None => {}
        }
    }
    matched * 100 >= a.len().min(b.len()) * 95
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn p(t: f64, r: f64) -> Point {
        Point { t, r }
    }

    #[test]
    fn drops_ignore_rises_and_noise() {
        let d = drops(&[
            p(0.0, 1.0),
            p(1.0, 0.9),
            p(2.0, 0.9004),
            p(3.0, 1.0),
            p(4.0, 0.5),
        ]);
        assert_eq!(d.len(), 2);
        assert!((d[0].amount - 0.1).abs() < 1e-12 && d[0].t == 1.0);
        assert_eq!(d[1].t, 4.0);
    }

    #[test]
    fn stats_reset_window_hit_rate() {
        // 1.0 -> 0.4 (창 사용 0.6) -> 리셋 1.0 -> 0.02 (도달) -> 0.01
        let pts = vec![
            p(0.0, 1.0),
            p(100.0, 0.4),
            p(200.0, 1.0),
            p(300.0, 0.02),
            p(400.0, 0.01),
        ];
        let s = Series {
            events: drops(&pts),
            points: pts,
        };
        let st = stats(&s, 400.0);
        assert_eq!((st.resets, st.limit_hits), (1, 1));
        assert!((st.window_use.unwrap() - 0.6).abs() < 1e-12);
        assert_eq!(st.current, Some(0.01));
        // 마지막 리셋(200) 이후 구간 길이 200초 = 0.0556h > 0.05h
        let want = (1.0 - 0.01) / (200.0 / 3600.0);
        assert!((st.rate_per_hour.unwrap() - want).abs() < 1e-9);
    }

    #[test]
    fn window_rate_averages_since_window_start() {
        let h = 3600.0;
        // 창 시작(0)부터 48시간에 걸쳐 0.96 -> 0.48: 평균 시간당 0.01.
        // 마지막 3시간에 몰아 쓴 0.06(시간당 0.02)은 평균에 묻혀야 한다.
        let pts = vec![p(0.0, 1.0), p(1.0 * h, 0.96), p(45.0 * h, 0.54), p(48.0 * h, 0.48)];
        let now = 48.0 * h;
        assert!((recent_rate(&pts, now).unwrap() - 0.02).abs() < 1e-12);
        let w = window_rate(&pts, now, Some(1.0 * h)).unwrap();
        assert!((w - 0.48 / 47.0).abs() < 1e-12);
        // 창 시작을 모르면 마지막 리셋, 그것도 없으면 첫 점부터.
        assert!((window_rate(&pts, now, None).unwrap() - 0.52 / 48.0).abs() < 1e-12);
        // 리셋이 있으면 그 뒤만 본다.
        let pts2 = vec![p(0.0, 0.2), p(1.0 * h, 1.0), p(11.0 * h, 0.9)];
        assert!((window_rate(&pts2, 11.0 * h, None).unwrap() - 0.01).abs() < 1e-12);
        // 창 시작 뒤 6시간이 안 되면 최근 속도로.
        let pts3 = vec![p(0.0, 1.0), p(2.0 * h, 0.9)];
        assert!((window_rate(&pts3, 2.0 * h, Some(0.0)).unwrap() - 0.05).abs() < 1e-12);
        // 줄지 않았으면 None.
        assert_eq!(window_rate(&[p(0.0, 0.5), p(10.0 * h, 0.5)], 10.0 * h, Some(0.0)), None);
    }

    #[test]
    fn downsample_keeps_lo_hi_in_time_order() {
        let pts: Vec<Point> = (0..1000)
            .map(|i| p(i as f64, if i % 2 == 0 { 0.2 } else { 0.8 }))
            .collect();
        let d = downsample(&pts, 0.0, 1000.0, 10);
        assert_eq!(d.len(), 20);
        assert!(d.windows(2).all(|w| w[0].t < w[1].t));
        assert_eq!(downsample(&pts[..20], 0.0, 20.0, 10).len(), 20);
    }

    #[test]
    fn formats() {
        assert_eq!(times(0.5), "0.50");
        assert_eq!(times(1.25), "1.2");
        assert_eq!(num(9.95), "9.9");
        assert_eq!(num(10.5), "10");
        assert_eq!(num(11.5), "12");
    }

    #[test]
    fn buckets_and_busiest() {
        let tz = FixedOffset::east_opt(9 * 3600).unwrap();
        // 1767193200 = 2025-12-31 15:00 UTC = 2026-01-01 00:00 KST(로컬 자정).
        let midnight = 1_767_193_200.0;
        let day = start_of_day(&tz, midnight + 5.0 * 3600.0);
        assert_eq!(day, midnight);
        let t = day + 5.0 * 3600.0 + 1234.0;
        assert_eq!(
            bucket_start(&tz, AnalyticsRange::Day1, t),
            day + 5.0 * 3600.0
        );
        assert_eq!(
            bucket_start(&tz, AnalyticsRange::Day3, t),
            day + 3.0 * 3600.0
        );
        assert_eq!(bucket_start(&tz, AnalyticsRange::Week, t), day);
        let s = Series {
            points: vec![],
            events: vec![
                Event { t, amount: 0.1 },
                Event {
                    t: t + 86400.0,
                    amount: 0.1,
                },
            ],
        };
        assert_eq!(busiest_hour(&tz, [&s]), Some(local_hour(&tz, t)));
        assert_eq!(busiest_hour(&tz, [&Series::default()]), None);
        let h = heat_cells(&tz, 0, [&s]);
        assert_eq!(h.cells.len(), 168);
        assert!(h.cells.iter().all(|c| (0.0..=1.0).contains(&c.amount)));
    }

    #[test]
    fn 같은_선_판정() {
        let p = |t: f64, r: f64| Point { t, r };
        let base = [p(0.0, 1.0), p(300.0, 0.9), p(600.0, 0.8)];
        assert!(same_series(&[p(0.0, 1.0), p(300.0, 0.9), p(600.0, 0.801)], &base));
        // 한 점이라도 크게 다르면 다른 선
        assert!(!same_series(&[p(0.0, 1.0), p(300.0, 0.7), p(600.0, 0.8)], &base));
        // 시각이 거의 안 겹치면 같다고 하지 않는다
        assert!(!same_series(&[p(10.0, 1.0), p(310.0, 0.9), p(600.0, 0.8)], &base));
        assert!(!same_series(&[], &base));
    }
}
