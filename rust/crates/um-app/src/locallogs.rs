//! 로컬 사용 기록(토큰 통계) 연결(계획서 8장, 결정 D4).
//! - 기본 꺼짐. 처음 분석 창을 열 때 묻고(`asked`), 나중에 설정·분석 창에서 소스별로 바꾼다.
//! - 켜진 소스만 백그라운드에서 증분 수집하고, 새 기록은 파일 감시로 따라간다.
//! - 숫자·모델·프로젝트 폴더 이름만 저장한다(대화 내용은 읽어도 저장하지 않음, `um-locallogs`).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{Local, Offset};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Manager};
use um_locallogs::query::{Group, Range, Totals, agy_quota_groups, bundle_third_party, relabel_models};
use um_locallogs::{IngestOptions, Roots, Source, Store, Watcher, ingest_source};

pub struct LocalLogs {
    store: Mutex<Option<Store>>,
    watcher: Mutex<Option<Watcher>>,
    roots: Option<Roots>,
    scanning: AtomicBool,
}

impl LocalLogs {
    pub fn new(dir: &std::path::Path) -> Self {
        let store = Store::open(&dir.join("locallogs.db")).map_err(|e| tracing::warn!("로컬 기록 DB 열기 실패: {e}")).ok();
        LocalLogs { store: Mutex::new(store), watcher: Mutex::new(None), roots: Roots::default_roots(), scanning: AtomicBool::new(false) }
    }
}

fn meta(ll: &LocalLogs, key: &str) -> Option<String> {
    ll.store.lock().unwrap().as_ref().and_then(|s| s.get_meta(key).ok().flatten())
}

fn set_meta(ll: &LocalLogs, key: &str, v: &str) {
    if let Some(s) = ll.store.lock().unwrap().as_ref() {
        let _ = s.set_meta(key, v);
    }
}

fn enabled(ll: &LocalLogs) -> Vec<Source> {
    ll.store.lock().unwrap().as_ref().and_then(|s| s.enabled_sources().ok()).unwrap_or_default()
}

/// 앱 시작 때: 켜진 소스가 있으면 수집과 감시를 시작한다.
pub fn start(app: &AppHandle) {
    let ll = app.state::<LocalLogs>();
    if !enabled(&ll).is_empty() {
        scan(app, enabled(&ll));
        watch(app);
    }
}

/// 백그라운드 증분 수집(이미 도는 중이면 건너뛴다).
fn scan(app: &AppHandle, sources: Vec<Source>) {
    let ll = app.state::<LocalLogs>();
    if sources.is_empty() || ll.scanning.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let ll = app.state::<LocalLogs>();
        if let Some(roots) = ll.roots.clone() {
            for src in sources {
                let mut guard = ll.store.lock().unwrap();
                let Some(store) = guard.as_mut() else { break };
                match ingest_source(store, &roots, src, &IngestOptions::default()) {
                    Ok(s) => tracing::info!("로컬 기록 {}: 줄 {} · 새 이벤트 {}", src.as_str(), s.lines, s.events_added),
                    Err(e) => tracing::warn!("로컬 기록 {} 수집 실패: {e}", src.as_str()),
                }
            }
        }
        set_meta(&ll, "last_scan", &chrono::Utc::now().timestamp().to_string());
        ll.scanning.store(false, Ordering::SeqCst);
        let _ = app.emit("ui-state", ());
    });
}

/// 기록 폴더 감시(2초 디바운스). 바뀐 소스만 다시 수집한다.
fn watch(app: &AppHandle) {
    let ll = app.state::<LocalLogs>();
    let mut w = ll.watcher.lock().unwrap();
    if w.is_some() {
        return;
    }
    let Some(roots) = ll.roots.clone() else { return };
    let h = app.clone();
    *w = Watcher::start(roots, Duration::from_secs(2), move |srcs| {
        let on = enabled(&h.state::<LocalLogs>());
        scan(&h, srcs.into_iter().filter(|s| on.contains(s)).collect());
    })
    .map_err(|e| tracing::warn!("로컬 기록 감시 실패: {e}"))
    .ok();
}

/// 설정·분석 창이 그릴 로컬 기록 상태.
#[tauri::command]
pub fn ui_locallogs(app: AppHandle) -> Value {
    let ll = app.state::<LocalLogs>();
    let on = enabled(&ll);
    let exists = |src: Source| {
        ll.roots.as_ref().is_some_and(|r| match src {
            Source::Claude => r.claude_projects.is_dir(),
            Source::Codex => r.codex_sessions.is_dir(),
            Source::Agy => r.agy_dir.is_dir(),
        })
    };
    let sources: Vec<Value> =
        Source::ALL.iter().map(|s| json!({"id": s.as_str(), "enabled": on.contains(s), "found": exists(*s)})).collect();
    json!({
        "asked": meta(&ll, "asked").as_deref() == Some("1"),
        "sources": sources,
        "hideProjects": meta(&ll, "hide_projects").as_deref() == Some("1"),
        "lastScan": meta(&ll, "last_scan").and_then(|v| v.parse::<f64>().ok()),
        "scanning": ll.scanning.load(Ordering::SeqCst),
        "events": ll.store.lock().unwrap().as_ref().and_then(|s| s.count_events().ok()).unwrap_or(0),
    })
}

/// 로컬 기록 설정 바꾸기: answer{enable}, source{id, enabled}, hideProjects{on}, clear, scan.
#[tauri::command]
pub fn ui_locallogs_set(app: AppHandle, action: String, arg: Value) -> Result<(), String> {
    let ll = app.state::<LocalLogs>();
    match action.as_str() {
        "answer" => {
            set_meta(&ll, "asked", "1");
            if arg["enable"] == true {
                if let Some(s) = ll.store.lock().unwrap().as_ref() {
                    for src in Source::ALL {
                        let _ = s.set_enabled(src, true);
                    }
                }
                scan(&app, enabled(&ll));
                watch(&app);
            }
        }
        "source" => {
            let src = arg["id"].as_str().and_then(Source::parse).ok_or("id")?;
            let on = arg["enabled"] == true;
            if let Some(s) = ll.store.lock().unwrap().as_ref() {
                s.set_enabled(src, on).map_err(|e| e.to_string())?;
            }
            set_meta(&ll, "asked", "1");
            if on {
                scan(&app, vec![src]);
                watch(&app);
            }
        }
        "hideProjects" => set_meta(&ll, "hide_projects", if arg["on"] == true { "1" } else { "0" }),
        "clear" => {
            if let Some(s) = ll.store.lock().unwrap().as_mut() {
                s.clear_all().map_err(|e| e.to_string())?;
            }
        }
        "scan" => scan(&app, enabled(&ll)),
        _ => return Err(format!("unknown action {action}")),
    }
    let _ = app.emit("ui-state", ());
    Ok(())
}

/// 토큰 통계(분석 창 "토큰" 보기, 계획서 8-3). `range` = day1|day3|week|month, `source` = claude|codex|agy(없으면 전체).
#[tauri::command]
pub fn ui_tokens(app: AppHandle, range: String, source: Option<String>) -> Value {
    let ll = app.state::<LocalLogs>();
    let hide = meta(&ll, "hide_projects").as_deref() == Some("1");
    let guard = ll.store.lock().unwrap();
    let Some(store) = guard.as_ref() else { return json!({"error": "no_store"}) };
    let now = chrono::Utc::now().timestamp() as f64;
    let hours = match range.as_str() {
        "day3" => 72.0,
        "week" => 168.0,
        "month" => 720.0,
        _ => 24.0,
    };
    let r = Range { from: now - hours * 3600.0, to: now + 1.0 };
    let src = source.as_deref().and_then(Source::parse);
    let tz = Local::now().offset().fix().local_minus_utc() as i64;
    let tv = |t: &Totals| {
        json!({"input": t.input, "output": t.output, "cacheRead": t.cache_read, "cacheWrite": t.cache_write, "reasoning": t.reasoning, "turns": t.turns, "total": t.total()})
    };
    let groups = |v: Vec<Group>| -> Vec<Value> { v.iter().map(|g| json!({"key": g.key, "sessions": g.sessions, "totals": tv(&g.totals)})).collect() };
    let matrix = |m: Option<[[i64; 24]; 7]>| m.map(|m| m.iter().map(|row| row.to_vec()).collect::<Vec<_>>()).unwrap_or_default();
    let top_projects = |mut projects: Vec<Group>, by: &dyn Fn(&Group) -> i64| {
        projects.sort_by_key(|g| std::cmp::Reverse(by(g)));
        projects.truncate(10);
        if hide {
            for (i, p) in projects.iter_mut().enumerate() {
                p.key = format!("#{}", i + 1);
            }
        }
        projects
    };
    let by_total = |g: &Group| g.totals.total();
    let by_turns = |g: &Group| g.totals.turns;
    let hourly = hours <= 72.0;
    if src == Some(Source::Agy) {
        // agy는 토큰 숫자가 검증되지 않아 요청 수(행 수)로만 센다. 같은 JSON 모양에 요청 수를 담는다.
        // 요청 수를 `total`에도 넣기 위해 agy 전용 변환을 쓴다.
        let tv_req = |t: &Totals| json!({"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "reasoning": 0, "turns": t.turns, "total": t.turns});
        let groups_req = |v: Vec<Group>| -> Vec<Value> { v.into_iter().map(|g| json!({"key": g.key, "sessions": g.sessions, "totals": tv_req(&g.totals)})).collect() };
        let bundled = |v: Vec<Group>| bundle_third_party(relabel_models(v));
        let totals = store.agy_totals(r).unwrap_or_default();
        let by_model = relabel_models(store.agy_by_model(r).unwrap_or_default());
        let quota: Vec<Value> = agy_quota_groups(&by_model).into_iter().map(|q| json!({"key": q.key, "v": q.v, "models": q.models})).collect();
        let mut model_order = bundle_third_party(relabel_models(store.agy_by_model(Range { from: 0.0, to: now + 1.0 }).unwrap_or_default()));
        model_order.sort_by_key(|g| std::cmp::Reverse(g.totals.turns));
        let wh = matrix(store.agy_weekday_hour_matrix(r, tz).ok());
        return json!({
            "agy": true,
            "totals": tv_req(&totals),
            "cacheHit": Value::Null,
            "perTurn": Value::Null,
            "sessions": store.agy_sessions_count(r).unwrap_or(0),
            "byModel": groups_req(bundle_third_party(by_model)),
            "byProject": groups_req(top_projects(store.agy_by_project(r).unwrap_or_default(), &by_turns)),
            "bySource": Vec::<Value>::new(),
            "modelOrder": model_order.into_iter().map(|g| g.key).collect::<Vec<_>>(),
            "byDayModel": groups_req(bundled(store.agy_by_day_model(r, tz).unwrap_or_default())),
            "byHourModel": if hourly { groups_req(bundled(store.agy_by_hour_model(r, tz).unwrap_or_default())) } else { vec![] },
            "weekdayHour": wh,
            "weekdayHourNoCache": wh,
            "codexLimits": Vec::<Value>::new(),
            "quotaGroups": quota,
        });
    }
    let totals = store.totals(r, src).unwrap_or_default();
    let projects = top_projects(store.by_project(r, src).unwrap_or_default(), &by_total);
    // 모델 색 순서: 전체 기록·전체 소스 기준이라 기간이나 소스 칩을 바꿔도 모델 색이 바뀌지 않는다.
    let mut model_order = relabel_models(store.by_model(Range { from: 0.0, to: now + 1.0 }, None).unwrap_or_default());
    model_order.sort_by_key(|g| std::cmp::Reverse(g.totals.total()));
    let model_order: Vec<String> = model_order.into_iter().map(|g| g.key).collect();
    json!({
        "agy": false,
        "totals": tv(&totals),
        "cacheHit": totals.cache_hit_ratio(),
        "perTurn": totals.tokens_per_turn(),
        "sessions": store.sessions_count(r, src).unwrap_or(0),
        "byModel": groups(relabel_models(store.by_model(r, src).unwrap_or_default())),
        "byProject": groups(projects),
        "bySource": groups(store.by_source(r, src).unwrap_or_default()),
        "modelOrder": model_order,
        "byDayModel": groups(relabel_models(store.by_day_model(r, src, tz).unwrap_or_default())),
        // 1일·3일은 하루 막대가 1~3개뿐이라 시간 단위로도 준다(창이 3일은 3시간씩 묶는다).
        "byHourModel": if hourly { groups(relabel_models(store.by_hour_model(r, src, tz).unwrap_or_default())) } else { vec![] },
        "weekdayHour": matrix(store.weekday_hour_matrix(r, src, tz, true).ok()),
        "weekdayHourNoCache": matrix(store.weekday_hour_matrix(r, src, tz, false).ok()),
        "codexLimits": store.codex_limit_series(r).unwrap_or_default().iter().map(|p| json!({"t": p.t, "primary": p.primary_used, "secondary": p.secondary_used})).collect::<Vec<_>>(),
    })
}
