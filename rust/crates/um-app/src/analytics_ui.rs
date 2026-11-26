//! 분석 창 데이터(계획서 5-6, Swift AnalyticsView.swift `buildModel`).
//! 계산은 전부 여기서(`um_core::analytics`), 창은 받은 숫자를 그리기만 한다.
//! - 전체(scope 없음): AI끼리 비교(대표 잔여율 기준, 5시간 한도 회분).
//! - AI 하나(scope): 그 AI의 세부 항목(5시간·주간·모델별) 기준, 항목 칩으로 일부만(`items`).

use chrono::Local;
use serde_json::{Value, json};
use tauri::AppHandle;
use um_core::analytics::{
    self as a, AnalyticsRange, Point, Series, busiest_hour, bucket_sums, downsample, drops, heat_cells, hour_sums, kpi_days,
    merge_item_order, stats, total_consumed, total_limit_hits,
};
use um_core::i18n::tr;
use um_core::model::UsageSubItem;
use um_providers::spec;
use um_render::border::{provider_color, sub_item_color};

use crate::state::AppState;

pub(crate) fn hex(c: [f64; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", (c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8)
}

fn range_of(s: &str) -> AnalyticsRange {
    match s {
        "day3" => AnalyticsRange::Day3,
        "week" => AnalyticsRange::Week,
        "month" => AnalyticsRange::Month,
        _ => AnalyticsRange::Day1,
    }
}

/// AI별 세부 항목 묶음(상세 표).
type Groups = Vec<(String, Vec<S>)>;

/// 차트에 그릴 계열 하나(AI 대표 또는 세부 항목).
pub(crate) struct S {
    pub(crate) provider: String,
    pub(crate) item: Option<String>,
    pub(crate) name: String,
    pub(crate) color: String,
    /// 같은 색(sRGB 0~1). 미니 차트처럼 직접 그리는 곳에서 쓴다.
    pub(crate) rgba: [f64; 4],
    pub(crate) series: Series,
    pub(crate) reset_at: Option<f64>,
    /// 주간 항목의 소진 속도(이번 창 평균). Some이면 `stats`의 최근 3시간 속도 대신 쓴다.
    pub(crate) window_rate: Option<Option<f64>>,
}

/// 주간 창 길이(초).
const WEEK_SECS: f64 = 7.0 * 24.0 * 3600.0;

impl S {
    /// 요약 통계. 주간 항목은 소진 속도를 이번 창 평균으로 바꾼다.
    pub(crate) fn stats(&self, now: f64) -> a::SeriesStats {
        let mut st = stats(&self.series, now);
        if let Some(r) = self.window_rate {
            st.rate_per_hour = r;
        }
        st
    }
}

/// 한 AI의 세부 항목 계열. 순서 = 지금 항목 → 이력에만 있는 항목(설정 탭·테두리와 같은 기본 색).
pub(crate) fn item_series(st: &AppState, id: &str, since: f64) -> Vec<S> {
    let Some(db) = st.history.as_ref() else { return vec![] };
    let lang = st.main.language;
    let (live, _) = st.providers.get(id).map(|p| p.sub_items()).unwrap_or_default();
    let live_ids: Vec<String> = live.iter().map(|i| i.id.clone()).collect();
    let order = merge_item_order(&live_ids, &db.item_ids(id, since).unwrap_or_default());
    let objs: Vec<UsageSubItem> = order
        .iter()
        .filter_map(|k| live.iter().find(|i| &i.id == k).cloned().or_else(|| UsageSubItem::from_id(k, 0.0, None)))
        .collect();
    objs.iter()
        .filter_map(|it| {
            let pts = db.item_series(id, &it.id, since).ok()?;
            if pts.is_empty() {
                return None;
            }
            // 주간 항목: 분석 기간과 상관없이 이번 창 시작(리셋 7일 전)부터의 점으로 평균 속도를 잰다.
            let window_rate = (!it.is_five_hour).then(|| {
                let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
                let start = it.reset_at.map(|d| d.timestamp() as f64 - WEEK_SECS);
                let wpts = db.item_series(id, &it.id, start.unwrap_or(now - WEEK_SECS).min(since)).unwrap_or_default();
                a::window_rate(&wpts, now, start)
            });
            let kind = tr(if it.is_five_hour { "subItems.kind5h" } else { "subItems.kindWeekly" }, lang);
            let rgba = sub_item_color(&st.main, id, &it.id, &objs);
            Some(S {
                provider: id.to_string(),
                item: Some(it.id.clone()),
                name: it.model_label.as_ref().map(|l| format!("{l} {kind}")).unwrap_or(kind),
                color: hex(rgba),
                rgba,
                series: Series { events: drops(&pts), points: pts },
                reset_at: it.reset_at.map(|d| d.timestamp() as f64),
                window_rate,
            })
        })
        .collect()
}

pub(crate) fn ai_series(st: &AppState, since: f64) -> Vec<S> {
    let Some(db) = st.history.as_ref() else { return vec![] };
    st.main
        .provider_order
        .iter()
        .filter_map(|id| {
            let pts: Vec<Point> = db.series(id, since).ok()?;
            if pts.len() < 2 {
                return None;
            }
            Some(S {
                provider: id.clone(),
                item: None,
                name: spec::spec(id).map(|s| s.name).unwrap_or("AI").to_string(),
                color: hex(provider_color(&st.main, id)),
                rgba: provider_color(&st.main, id),
                series: Series { points: pts, events: db.consumption_events(id, since).unwrap_or_default() },
                reset_at: st.snapshots.get(id).and_then(|s| s.reset_at).map(|d| d.timestamp() as f64),
                window_rate: None,
            })
        })
        .collect()
}

pub(crate) enum Eta {
    /// 바닥나기 전에 리셋된다.
    Safe,
    /// 이 시각(Unix 초)에 바닥난다.
    At(f64),
    /// 쓰는 중이 아니다.
    Idle,
}

/// 지금 속도로 쓰면 언제 바닥나는지. 리셋이 먼저 오면 "여유". (Swift etaText)
pub(crate) fn eta(stt: &a::SeriesStats, reset_at: Option<f64>, now: f64) -> Eta {
    match (stt.rate_per_hour, stt.current) {
        (Some(rate), Some(cur)) if rate > 0.0 => {
            let empty = now + cur / rate * 3600.0;
            if reset_at.is_some_and(|rt| rt < empty) { Eta::Safe } else { Eta::At(empty) }
        }
        _ => Eta::Idle,
    }
}

/// 분석 창 데이터. `range` = day1|day3|week|month, `scope` = AI id(없으면 전체), `items` = 고른 세부 항목 id(비면 전체).
#[tauri::command]
pub fn ui_analytics(app: AppHandle, range: String, scope: Option<String>, items: Vec<String>) -> Value {
    let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    let r = range_of(&range);
    let since = r.since(now);
    let tz = Local;
    let st = crate::shared(&app);
    let st = st.lock().unwrap();

    let ai_all = ai_series(&st, since);
    let has_data: Vec<String> = st
        .main
        .provider_order
        .iter()
        .filter(|id| ai_all.iter().any(|s| &s.provider == *id) || st.history.as_ref().is_some_and(|db| !db.item_ids(id, since).unwrap_or_default().is_empty()))
        .cloned()
        .collect();

    // 화면 모드별 계열 고르기(Swift buildModel).
    let ai: Vec<&S>;
    let scoped_items: Vec<Value>;
    let shown_is_items: bool;
    let mut shown: Vec<&S> = vec![];
    // 세부 항목 탭에서 "분석"을 끈 항목은 이 창에서만 뺀다(내보내기는 전부).
    let hidden = &st.main.sub_item_analytics_hidden;
    let all_groups: Groups = st
        .main
        .provider_order
        .iter()
        .map(|id| (id.clone(), item_series(&st, id, since).into_iter().filter(|s| !s.item.as_ref().is_some_and(|i| hidden.contains(&format!("{id}/{i}")))).collect()))
        .collect();
    let picked_groups: Groups;
    if let Some(id) = scope.as_ref().filter(|id| spec::spec(id).is_some()) {
        let mut g = all_groups.into_iter().find(|(gid, _)| gid == id).map(|(_, v)| v).unwrap_or_default();
        scoped_items = g.iter().map(|s| json!({"id": s.item, "name": s.name, "color": s.color})).collect();
        if !items.is_empty() {
            g.retain(|s| s.item.as_ref().is_some_and(|i| items.contains(i)));
        }
        picked_groups = vec![(id.clone(), g)];
        ai = ai_all.iter().filter(|s| &s.provider == id).collect();
        shown_is_items = true;
    } else {
        scoped_items = vec![];
        picked_groups = all_groups.into_iter().filter(|(_, v)| !v.is_empty()).collect();
        ai = ai_all.iter().collect();
        shown_is_items = false;
    }
    let groups = picked_groups;
    if shown_is_items {
        shown.extend(groups.iter().flat_map(|(_, v)| v.iter()));
    } else {
        shown.extend(ai.iter().copied());
    }

    let total = total_consumed(ai.iter().map(|s| &s.series));
    let days = kpi_days(r);
    let hits = total_limit_hits(groups.iter().flat_map(|(_, v)| v.iter().map(|s| &s.series)), now);
    let busiest = busiest_hour(&tz, ai.iter().map(|s| &s.series));
    let heat = heat_cells(&tz, 0, ai.iter().map(|s| &s.series));

    let series_v: Vec<Value> = shown
        .iter()
        .map(|s| {
            json!({
                "provider": s.provider, "item": s.item, "name": s.name, "color": s.color,
                "consumed": s.series.consumed(),
                "hours": hour_sums(&tz, &s.series.events).to_vec(),
                "remaining": downsample(&s.series.points, since, now, 240).iter().map(|p| [p.t, p.r]).collect::<Vec<_>>(),
                "buckets": bucket_sums(&tz, r, &s.series.events),
            })
        })
        .collect();

    let detail: Vec<Value> = groups
        .iter()
        .map(|(id, v)| {
            let rows: Vec<Value> = v
                .iter()
                .map(|s| {
                    let stt = s.stats(now);
                    let eta = match eta(&stt, s.reset_at, now) {
                        Eta::Safe => json!({"kind": "safe"}),
                        Eta::At(t) => json!({"kind": "at", "t": t}),
                        Eta::Idle => json!({"kind": "idle"}),
                    };
                    json!({
                        "name": s.name, "color": s.color, "current": stt.current, "consumed": stt.consumed,
                        "resets": stt.resets, "windowUse": stt.window_use, "limitHits": stt.limit_hits,
                        "ratePerHour": stt.rate_per_hour, "eta": eta,
                    })
                })
                .collect();
            json!({"id": id, "name": spec::spec(id).map(|s| s.name).unwrap_or("AI"), "color": hex(provider_color(&st.main, id)), "rows": rows})
        })
        .collect();

    json!({
        "now": now, "since": since, "bucketSeconds": r.bucket_seconds(),
        "earliest": st.history.as_ref().and_then(|db| db.earliest().ok().flatten()),
        "hasData": has_data,
        "providers": st.main.provider_order.iter().map(|id| json!({"id": id, "name": spec::spec(id).map(|s| s.name).unwrap_or("AI"), "color": hex(provider_color(&st.main, id))})).collect::<Vec<_>>(),
        "scopedItems": scoped_items,
        "itemMode": shown_is_items,
        "kpi": {
            "total": a::times(total), "dailyAvg": a::times(total / days),
            "busiest": busiest, "hits": hits,
        },
        "series": series_v,
        "heat": { "weekdays": heat.weekdays.to_vec(), "cells": heat.cells.iter().map(|c| [c.row as f64, c.hour as f64, c.amount]).collect::<Vec<_>>() },
        "detail": detail,
    })
}
