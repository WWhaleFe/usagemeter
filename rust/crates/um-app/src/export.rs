//! 분석 내용 내보내기(CSV·JSON)와 주기 저장.
//! - 분석 창과 설정의 "내보내기" 탭이 같은 명령(`ui_export`)을 쓴다.
//! - 설정은 데이터 폴더의 `export.json`에 따로 둔다(테두리 설정·프리셋과 섞지 않는다).
//! - 한 번 내보낼 때마다 고른 폴더 안에 `UsageMeter_날짜_시각` 폴더를 만들고 그 안에 파일을 쓴다.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};
use um_core::analytics::{self as a, Point};
use um_core::i18n::{tf, tr};
use um_providers::spec;

use crate::analytics_ui::{Eta, S, ai_series, eta, item_series};

const FILE: &str = "export.json";
/// 설정이 바뀌거나 내보내기가 끝나면 두 창에 알린다.
const EVENT: &str = "export-state";
const RANGES: [&str; 5] = ["day1", "day3", "week", "month", "all"];
const FORMATS: [&str; 3] = ["both", "csv", "json"];
const PERIODS: [&str; 3] = ["daily", "weekly", "monthly"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportSettings {
    /// 지금 내보내기 기간: day1|day3|week|month|all
    pub range: String,
    /// both|csv|json
    pub format: String,
    /// 지금 내보내기에서 마지막으로 고른 폴더(다음 선택 창의 시작 위치).
    pub last_folder: Option<String>,
    pub auto: bool,
    pub auto_folder: Option<String>,
    /// daily|weekly|monthly. 저장 기간은 주기를 따른다(1일·7일·30일).
    pub period: String,
    pub hour: u32,
    pub minute: u32,
    /// 1=월요일 … 7=일요일
    pub weekday: u32,
    /// 1~28(모든 달에 있는 날만)
    pub monthday: u32,
    /// 마지막 자동 저장(또는 자동 저장을 켜거나 일정을 바꾼) 시각. 이보다 늦은 예정 시각만 실행한다.
    pub last_auto: Option<f64>,
    /// 직접 내보낸 마지막 결과.
    pub last_result: Option<ExportResult>,
    /// 자동 저장의 마지막 결과(직접 내보낸 결과를 가리지 않게 따로 둔다).
    pub last_auto_result: Option<ExportResult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub ok: bool,
    pub auto: bool,
    pub t: f64,
    pub path: Option<String>,
    pub error: Option<String>,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            range: "week".into(),
            format: "both".into(),
            last_folder: None,
            auto: false,
            auto_folder: None,
            period: "daily".into(),
            hour: 9,
            minute: 0,
            weekday: 1,
            monthday: 1,
            last_auto: None,
            last_result: None,
            last_auto_result: None,
        }
    }
}

impl ExportSettings {
    fn normalize(&mut self) {
        let d = Self::default();
        if !RANGES.contains(&self.range.as_str()) { self.range = d.range; }
        if !FORMATS.contains(&self.format.as_str()) { self.format = d.format; }
        if !PERIODS.contains(&self.period.as_str()) { self.period = d.period; }
        self.hour = self.hour.min(23);
        self.minute = self.minute.min(59);
        self.weekday = self.weekday.clamp(1, 7);
        self.monthday = self.monthday.clamp(1, 28);
    }

    /// 자동 저장이 담는 기간. 주기와 같은 길이라 저장본끼리 겹치지 않는다.
    fn auto_range(&self) -> &'static str {
        match self.period.as_str() {
            "weekly" => "week",
            "monthly" => "month",
            _ => "day1",
        }
    }

    fn matches(&self, d: NaiveDate) -> bool {
        match self.period.as_str() {
            "weekly" => d.weekday().number_from_monday() == self.weekday,
            "monthly" => d.day() == self.monthday,
            _ => true,
        }
    }
}

/// 그 날의 예정 시각. 서머타임으로 없는 시각이면 한 시간 뒤로.
fn at<Tz: TimeZone>(tz: &Tz, d: NaiveDate, h: u32, m: u32) -> Option<DateTime<Tz>> {
    let ndt = d.and_hms_opt(h, m, 0)?;
    tz.from_local_datetime(&ndt).earliest().or_else(|| tz.from_local_datetime(&(ndt + chrono::Duration::hours(1))).earliest())
}

/// `now` 이전(같음 포함) 가장 최근 예정 시각.
fn prev_due<Tz: TimeZone>(c: &ExportSettings, tz: &Tz, now: &DateTime<Tz>) -> Option<DateTime<Tz>> {
    let today = now.date_naive();
    (0..62).filter_map(|i| today.checked_sub_days(chrono::Days::new(i))).filter(|d| c.matches(*d)).find_map(|d| at(tz, d, c.hour, c.minute).filter(|t| t <= now))
}

/// `now` 이후 다음 예정 시각.
fn next_due<Tz: TimeZone>(c: &ExportSettings, tz: &Tz, now: &DateTime<Tz>) -> Option<DateTime<Tz>> {
    let today = now.date_naive();
    (0..62).filter_map(|i| today.checked_add_days(chrono::Days::new(i))).filter(|d| c.matches(*d)).find_map(|d| at(tz, d, c.hour, c.minute).filter(|t| t > now))
}

/// 지금 자동 저장할 차례인가. 앱이 꺼져 있던 동안 지난 예정 시각은 다음 확인 때 한 번만 따라잡는다.
fn is_due<Tz: TimeZone>(c: &ExportSettings, tz: &Tz, now: &DateTime<Tz>) -> bool {
    if !c.auto || c.auto_folder.is_none() { return false; }
    let Some(last) = c.last_auto else { return false };
    prev_due(c, tz, now).is_some_and(|t| t.timestamp() as f64 > last)
}

// ---------- 설정 파일 ----------

static SETTINGS: Mutex<Option<ExportSettings>> = Mutex::new(None);

fn path() -> PathBuf {
    crate::state::data_dir().join(FILE)
}

fn with<R>(f: impl FnOnce(&mut ExportSettings) -> R) -> R {
    let mut g = SETTINGS.lock().unwrap();
    let c = g.get_or_insert_with(|| {
        let mut c: ExportSettings = std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        c.normalize();
        c
    });
    f(c)
}

fn save(c: &ExportSettings) {
    let p = path();
    if let Some(dir) = p.parent() { let _ = std::fs::create_dir_all(dir); }
    if let Ok(s) = serde_json::to_string_pretty(c)
        && let Err(e) = std::fs::write(&p, s)
    {
        tracing::warn!("내보내기 설정 저장 실패: {e}");
    }
}

fn now_secs() -> f64 {
    chrono::Utc::now().timestamp_millis() as f64 / 1000.0
}

fn state_value(c: &ExportSettings) -> Value {
    let now = Local::now();
    json!({
        "settings": c,
        "nextDue": (c.auto && c.auto_folder.is_some()).then(|| next_due(c, &Local, &now).map(|t| t.timestamp())).flatten(),
        "autoRange": c.auto_range(),
    })
}

// ---------- 데이터 모으기 ----------

/// 계열 하나: AI 대표(item 없음) 또는 세부 항목.
struct Row {
    provider: String,
    provider_name: String,
    s: S,
}

fn collect(app: &AppHandle, since: f64) -> Vec<Row> {
    let st = crate::shared(app);
    let st = st.lock().unwrap();
    let ai = ai_series(&st, since);
    let mut out = vec![];
    for id in &st.main.provider_order {
        let name = spec::spec(id).map(|s| s.name).unwrap_or("AI").to_string();
        let items = item_series(&st, id, since);
        if let Some(s) = ai.iter().find(|s| &s.provider == id) {
            out.push(Row { provider: id.clone(), provider_name: name.clone(), s: S { provider: s.provider.clone(), item: None, name: s.name.clone(), color: s.color.clone(), rgba: s.rgba, series: s.series.clone(), reset_at: s.reset_at } });
        }
        out.extend(items.into_iter().map(|s| Row { provider: id.clone(), provider_name: name.clone(), s }));
    }
    out
}

fn since_of(range: &str, now: f64) -> f64 {
    match range {
        "day1" => a::AnalyticsRange::Day1.since(now),
        "day3" => a::AnalyticsRange::Day3.since(now),
        "month" => a::AnalyticsRange::Month.since(now),
        "all" => 0.0,
        _ => a::AnalyticsRange::Week.since(now),
    }
}

// ---------- 파일 만들기 ----------

fn cell(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

/// UTF-8 BOM을 붙여 Excel에서도 한글·일본어가 깨지지 않게 한다.
fn csv(header: &[&str], rows: impl IntoIterator<Item = Vec<String>>) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str(&header.join(","));
    out.push_str("\r\n");
    for r in rows {
        out.push_str(&r.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

fn pct(v: f64) -> String {
    format!("{:.2}", v * 100.0)
}

fn opt(v: Option<String>) -> String {
    v.unwrap_or_default()
}

fn time<Tz: TimeZone>(tz: &Tz, t: f64) -> String
where
    Tz::Offset: std::fmt::Display,
{
    tz.timestamp_opt(t as i64, 0).single().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default()
}

/// 계열을 가리키는 공통 열: ai, ai_name, series(ai|item), item_id, name
fn ident(r: &Row) -> Vec<String> {
    vec![r.provider.clone(), r.provider_name.clone(), if r.s.item.is_some() { "item" } else { "ai" }.into(), opt(r.s.item.clone()), r.s.name.clone()]
}
const IDENT: [&str; 5] = ["ai", "ai_name", "series", "item_id", "name"];

fn with_ident<'a>(rest: &[&'a str]) -> Vec<&'a str> {
    IDENT.iter().copied().chain(rest.iter().copied()).collect()
}

/// (파일 이름, 내용) 목록. 시간대를 인자로 받아 테스트에서 결과를 고정한다.
fn render<Tz: TimeZone>(tz: &Tz, rows: &[Row], range: &str, since: f64, now: f64, format: &str) -> Vec<(&'static str, String)>
where
    Tz::Offset: std::fmt::Display,
{
    let stats: Vec<a::SeriesStats> = rows.iter().map(|r| a::stats(&r.s.series, now)).collect();
    let etas: Vec<(String, String)> = rows
        .iter()
        .zip(&stats)
        .map(|(r, st)| match eta(st, r.s.reset_at, now) {
            Eta::Safe => ("safe".into(), String::new()),
            Eta::At(t) => ("runs_out".into(), time(tz, t)),
            Eta::Idle => ("idle".into(), String::new()),
        })
        .collect();
    let mut files = vec![];
    if format != "json" {
        files.push((
            "summary.csv",
            csv(
                &with_ident(&["remaining_pct", "used_pct", "used_limits", "resets", "avg_window_use_pct", "limit_hits", "burn_pct_per_hour", "status", "runs_out_at", "reset_at", "period_start", "period_end"]),
                rows.iter().zip(&stats).zip(&etas).map(|((r, st), (status, at))| {
                    let mut v = ident(r);
                    v.extend([
                        opt(st.current.map(pct)),
                        pct(st.consumed),
                        format!("{:.3}", st.consumed),
                        st.resets.to_string(),
                        opt(st.window_use.map(pct)),
                        st.limit_hits.to_string(),
                        opt(st.rate_per_hour.map(pct)),
                        status.clone(),
                        at.clone(),
                        opt(r.s.reset_at.map(|t| time(tz, t))),
                        time(tz, since.max(r.s.series.points.first().map_or(since, |p| p.t))),
                        time(tz, now),
                    ]);
                    v
                }),
            ),
        ));
        files.push((
            "remaining.csv",
            csv(
                &["time", "unix_time", "ai", "ai_name", "series", "item_id", "name", "remaining_pct"],
                rows.iter().flat_map(|r| {
                    r.s.series.points.iter().map(move |p| {
                        let mut v = vec![time(tz, p.t), (p.t as i64).to_string()];
                        v.extend(ident(r));
                        v.push(pct(p.r));
                        v
                    })
                }),
            ),
        ));
        files.push((
            "usage_events.csv",
            csv(
                &["time", "unix_time", "ai", "ai_name", "series", "item_id", "name", "used_pct"],
                rows.iter().flat_map(|r| {
                    r.s.series.events.iter().map(move |e| {
                        let mut v = vec![time(tz, e.t), (e.t as i64).to_string()];
                        v.extend(ident(r));
                        v.push(pct(e.amount));
                        v
                    })
                }),
            ),
        ));
        files.push((
            "daily.csv",
            csv(
                &["date", "ai", "ai_name", "series", "item_id", "name", "used_pct"],
                rows.iter().flat_map(|r| {
                    let mut days: std::collections::BTreeMap<NaiveDate, f64> = Default::default();
                    for e in &r.s.series.events {
                        if let Some(d) = tz.timestamp_opt(e.t as i64, 0).single() {
                            *days.entry(d.date_naive()).or_default() += e.amount;
                        }
                    }
                    days.into_iter().map(move |(d, amt)| {
                        let mut v = vec![d.format("%Y-%m-%d").to_string()];
                        v.extend(ident(r));
                        v.push(pct(amt));
                        v
                    })
                }),
            ),
        ));
        files.push((
            "hourly.csv",
            csv(
                &["hour", "ai", "ai_name", "series", "item_id", "name", "used_pct"],
                rows.iter().flat_map(|r| {
                    let sums = a::hour_sums(tz, &r.s.series.events);
                    (0..24usize).map(move |h| {
                        let mut v = vec![h.to_string()];
                        v.extend(ident(r));
                        v.push(pct(sums[h]));
                        v
                    })
                }),
            ),
        ));
    }
    if format != "csv" {
        let pts = |p: &[Point]| p.iter().map(|p| json!([p.t as i64, (p.r * 10000.0).round() / 100.0])).collect::<Vec<_>>();
        let series: Vec<Value> = rows
            .iter()
            .zip(&stats)
            .zip(&etas)
            .map(|((r, st), (status, at))| {
                json!({
                    "ai": r.provider, "aiName": r.provider_name, "series": if r.s.item.is_some() { "item" } else { "ai" },
                    "itemId": r.s.item, "name": r.s.name, "color": r.s.color,
                    "summary": {
                        "remainingPct": st.current.map(|v| v * 100.0), "usedPct": st.consumed * 100.0, "usedLimits": st.consumed,
                        "resets": st.resets, "avgWindowUsePct": st.window_use.map(|v| v * 100.0), "limitHits": st.limit_hits,
                        "burnPctPerHour": st.rate_per_hour.map(|v| v * 100.0), "status": status, "runsOutAt": (!at.is_empty()).then_some(at),
                        "resetAt": r.s.reset_at.map(|t| time(tz, t)),
                    },
                    "hourlyUsedPct": a::hour_sums(tz, &r.s.series.events).iter().map(|v| (v * 10000.0).round() / 100.0).collect::<Vec<_>>(),
                    "remaining": pts(&r.s.series.points),
                    "usageEvents": r.s.series.events.iter().map(|e| json!([e.t as i64, (e.amount * 10000.0).round() / 100.0])).collect::<Vec<_>>(),
                })
            })
            .collect();
        let doc = json!({
            "app": "UsageMeter",
            "version": env!("CARGO_PKG_VERSION"),
            "exportedAt": time(tz, now),
            "range": {"id": range, "start": time(tz, since.max(rows.iter().filter_map(|r| r.s.series.points.first()).map(|p| p.t).fold(f64::INFINITY, f64::min).min(now))), "end": time(tz, now)},
            "units": {
                "pct": "percent of the limit (0-100)",
                "usedLimits": "1.0 = one full limit",
                "remaining": "[unix_time, remaining_pct]",
                "usageEvents": "[unix_time, used_pct]",
                "hourlyUsedPct": "local hour 0-23",
            },
            "series": series,
        });
        files.push(("analytics.json", serde_json::to_string_pretty(&doc).unwrap_or_default()));
    }
    files
}

fn write(app: &AppHandle, folder: &Path, range: &str, format: &str) -> Result<PathBuf, String> {
    if !folder.is_dir() {
        let lang = crate::shared(app).lock().unwrap().main.language;
        return Err(tf("export.noDir", lang, &folder.display().to_string()));
    }
    let now = now_secs();
    let since = since_of(range, now);
    let rows = collect(app, since);
    let stamp = Local::now().format("%Y-%m-%d_%H%M%S");
    let mut dir = folder.join(format!("UsageMeter_{stamp}"));
    let mut n = 2;
    while dir.exists() {
        dir = folder.join(format!("UsageMeter_{stamp}_{n}"));
        n += 1;
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for (name, body) in render(&Local, &rows, range, since, now, format) {
        std::fs::write(dir.join(name), body).map_err(|e| e.to_string())?;
    }
    tracing::info!("분석 내보내기({range}, {format}): {}", dir.display());
    Ok(dir)
}

fn finish(app: &AppHandle, res: &Result<PathBuf, String>, auto: bool) -> ExportResult {
    let r = ExportResult {
        ok: res.is_ok(),
        auto,
        t: now_secs(),
        path: res.as_ref().ok().map(|p| p.display().to_string()),
        error: res.as_ref().err().cloned(),
    };
    with(|c| {
        if auto { c.last_auto_result = Some(r.clone()) } else { c.last_result = Some(r.clone()) }
        save(c);
    });
    let _ = app.emit(EVENT, ());
    r
}

// ---------- 자동 저장 ----------

/// 30초마다 예정 시각이 지났는지 본다. 잠자기에서 깨어나도 다음 확인 때 따라잡는다.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(30));
            tick(&app);
        }
    });
}

fn tick(app: &AppHandle) {
    let job = with(|c| {
        if c.auto && c.last_auto.is_none() {
            // 손으로 고친 파일 등: 지금부터 센다.
            c.last_auto = Some(now_secs());
            save(c);
        }
        if !is_due(c, &Local, &Local::now()) { return None; }
        // 실패해도 같은 예정 시각을 30초마다 다시 시도하지 않게 먼저 적는다.
        c.last_auto = Some(now_secs());
        save(c);
        Some((PathBuf::from(c.auto_folder.clone()?), c.auto_range(), c.format.clone()))
    });
    let Some((folder, range, format)) = job else { return };
    let res = write(app, &folder, range, &format);
    if let Err(e) = &res {
        tracing::warn!("자동 저장 실패: {e}");
        let lang = crate::shared(app).lock().unwrap().main.language;
        crate::notifications::export_failed(app, &tr("export.autoFailed", lang), e);
    }
    finish(app, &res, true);
}

// ---------- 창 명령 ----------

async fn pick(window: &tauri::WebviewWindow, title: String, start: Option<String>) -> Option<PathBuf> {
    let mut d = rfd::AsyncFileDialog::new().set_title(title).set_parent(window);
    let start = start.map(PathBuf::from).filter(|p| p.is_dir()).or_else(|| directories::UserDirs::new().and_then(|u| u.document_dir().map(Path::to_path_buf)));
    if let Some(s) = start { d = d.set_directory(s); }
    d.pick_folder().await.map(|h| h.path().to_path_buf())
}

/// 분석 창·설정 탭 공용. action = state|set|pickAutoFolder|exportNow|reveal
#[tauri::command]
pub async fn ui_export(app: AppHandle, window: tauri::WebviewWindow, action: String, arg: Value) -> Result<Value, String> {
    let lang = crate::shared(&app).lock().unwrap().main.language;
    match action.as_str() {
        "state" => Ok(with(|c| state_value(c))),
        "set" => {
            let v = with(|c| {
                let before = c.clone();
                let s = |k: &str| arg.get(k).and_then(Value::as_str).map(str::to_string);
                let n = |k: &str| arg.get(k).and_then(Value::as_u64).map(|v| v as u32);
                if let Some(v) = s("range") { c.range = v; }
                if let Some(v) = s("format") { c.format = v; }
                if let Some(v) = s("period") { c.period = v; }
                if let Some(v) = n("hour") { c.hour = v; }
                if let Some(v) = n("minute") { c.minute = v; }
                if let Some(v) = n("weekday") { c.weekday = v; }
                if let Some(v) = n("monthday") { c.monthday = v; }
                if let Some(v) = arg.get("auto").and_then(Value::as_bool) { c.auto = v; }
                c.normalize();
                // 켜거나 일정을 바꾸면 그 시점부터 센다(바꾸자마자 지난 시각으로 저장되지 않게).
                if (c.auto && !before.auto) || (&c.period, c.hour, c.minute, c.weekday, c.monthday) != (&before.period, before.hour, before.minute, before.weekday, before.monthday) {
                    c.last_auto = Some(now_secs());
                }
                save(c);
                state_value(c)
            });
            let _ = app.emit(EVENT, ());
            Ok(v)
        }
        "pickAutoFolder" => {
            let start = with(|c| c.auto_folder.clone().or_else(|| c.last_folder.clone()));
            if let Some(p) = pick(&window, tr("export.pickAutoTitle", lang), start).await {
                with(|c| {
                    c.auto_folder = Some(p.display().to_string());
                    c.last_auto = Some(now_secs());
                    save(c);
                });
                let _ = app.emit(EVENT, ());
            }
            Ok(with(|c| state_value(c)))
        }
        "exportNow" => {
            let (start, range, format) = with(|c| (c.last_folder.clone(), c.range.clone(), c.format.clone()));
            let Some(folder) = pick(&window, tr("export.pickTitle", lang), start).await else {
                return Ok(json!({"cancelled": true}));
            };
            with(|c| {
                c.last_folder = Some(folder.display().to_string());
                save(c);
            });
            let h = app.clone();
            let res = tauri::async_runtime::spawn_blocking(move || write(&h, &folder, &range, &format)).await.map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(finish(&app, &res, false)).unwrap_or_default())
        }
        "reveal" => {
            let p = arg.get("path").and_then(Value::as_str).ok_or("path")?;
            open::that_detached(p).map_err(|e| e.to_string())?;
            Ok(Value::Null)
        }
        _ => Err(format!("알 수 없는 동작: {action}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    use um_core::analytics::Series;

    fn kst() -> FixedOffset {
        FixedOffset::east_opt(9 * 3600).unwrap()
    }

    fn cfg(period: &str) -> ExportSettings {
        ExportSettings { auto: true, auto_folder: Some("/tmp".into()), period: period.into(), hour: 9, minute: 30, weekday: 3, monthday: 15, ..Default::default() }
    }

    #[test]
    fn 매일_예정_시각() {
        let tz = kst();
        let c = cfg("daily");
        let now = tz.with_ymd_and_hms(2026, 11, 20, 8, 0, 0).unwrap();
        assert_eq!(prev_due(&c, &tz, &now).unwrap(), tz.with_ymd_and_hms(2026, 11, 19, 9, 30, 0).unwrap());
        assert_eq!(next_due(&c, &tz, &now).unwrap(), tz.with_ymd_and_hms(2026, 11, 20, 9, 30, 0).unwrap());
        let now = tz.with_ymd_and_hms(2026, 11, 20, 9, 30, 0).unwrap();
        assert_eq!(prev_due(&c, &tz, &now).unwrap(), now);
    }

    #[test]
    fn 매주_매월_예정_시각() {
        let tz = kst();
        // 2026-11-20은 금요일. 수요일(3) 09:30
        let now = tz.with_ymd_and_hms(2026, 11, 20, 12, 0, 0).unwrap();
        let c = cfg("weekly");
        assert_eq!(prev_due(&c, &tz, &now).unwrap(), tz.with_ymd_and_hms(2026, 11, 18, 9, 30, 0).unwrap());
        assert_eq!(next_due(&c, &tz, &now).unwrap(), tz.with_ymd_and_hms(2026, 11, 25, 9, 30, 0).unwrap());
        let c = cfg("monthly");
        assert_eq!(prev_due(&c, &tz, &now).unwrap(), tz.with_ymd_and_hms(2026, 11, 15, 9, 30, 0).unwrap());
        assert_eq!(next_due(&c, &tz, &now).unwrap(), tz.with_ymd_and_hms(2026, 12, 15, 9, 30, 0).unwrap());
    }

    #[test]
    fn 지난_예정은_한_번만_따라잡는다() {
        let tz = kst();
        let mut c = cfg("daily");
        let now = tz.with_ymd_and_hms(2026, 11, 20, 12, 0, 0).unwrap();
        // 어제 저장 후 앱이 꺼져 있었다: 오늘 09:30을 놓쳤으니 지금 저장.
        c.last_auto = Some(tz.with_ymd_and_hms(2026, 11, 19, 9, 30, 5).unwrap().timestamp() as f64);
        assert!(is_due(&c, &tz, &now));
        // 저장한 뒤에는 다음 날까지 아니다.
        c.last_auto = Some(now.timestamp() as f64);
        assert!(!is_due(&c, &tz, &now));
        // 꺼져 있거나 폴더가 없으면 아니다.
        c.last_auto = Some(0.0);
        c.auto_folder = None;
        assert!(!is_due(&c, &tz, &now));
        c.auto_folder = Some("/tmp".into());
        c.auto = false;
        assert!(!is_due(&c, &tz, &now));
        // 처음 켤 때(기준 시각 없음)는 바로 저장하지 않는다.
        c.auto = true;
        c.last_auto = None;
        assert!(!is_due(&c, &tz, &now));
    }

    #[test]
    fn 잘못된_값은_바로잡는다() {
        let mut c = ExportSettings { range: "x".into(), format: "xlsx".into(), period: "yearly".into(), hour: 30, minute: 99, weekday: 0, monthday: 31, ..Default::default() };
        c.normalize();
        assert_eq!((c.range.as_str(), c.format.as_str(), c.period.as_str()), ("week", "both", "daily"));
        assert_eq!((c.hour, c.minute, c.weekday, c.monthday), (23, 59, 1, 28));
        // 예전 파일에 없는 필드는 기본값.
        let c: ExportSettings = serde_json::from_str(r#"{"auto":true}"#).unwrap();
        assert_eq!(c.period, "daily");
        assert!(c.auto);
    }

    #[test]
    fn csv_칸_감싸기() {
        assert_eq!(cell("a,b"), "\"a,b\"");
        assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(cell("Opus 주간"), "Opus 주간");
        let s = csv(&["a", "b"], [vec!["1".into(), "x,y".into()]]);
        assert_eq!(s, "\u{feff}a,b\r\n1,\"x,y\"\r\n");
    }

    #[test]
    fn 파일_내용() {
        let tz = kst();
        let t0 = tz.with_ymd_and_hms(2026, 11, 20, 9, 0, 0).unwrap().timestamp() as f64;
        let pts = vec![Point { t: t0, r: 0.9 }, Point { t: t0 + 3600.0, r: 0.7 }, Point { t: t0 + 7200.0, r: 0.6 }];
        let series = Series { events: a::drops(&pts), points: pts };
        let rows = vec![Row {
            provider: "claude".into(),
            provider_name: "Claude".into(),
            s: S { provider: "claude".into(), item: Some("5h".into()), name: "5시간".into(), color: "#d97757".into(), rgba: [0.85, 0.47, 0.34, 1.0], series, reset_at: None },
        }];
        let now = t0 + 7200.0;
        let files = render(&tz, &rows, "day1", t0 - 3600.0, now, "both");
        let names: Vec<&str> = files.iter().map(|f| f.0).collect();
        assert_eq!(names, ["summary.csv", "remaining.csv", "usage_events.csv", "daily.csv", "hourly.csv", "analytics.json"]);
        let get = |n: &str| files.iter().find(|f| f.0 == n).unwrap().1.clone();
        let summary = get("summary.csv");
        assert!(summary.lines().nth(1).unwrap().starts_with("claude,Claude,item,5h,5시간,60.00,30.00,0.300,0,"), "{summary}");
        let remaining = get("remaining.csv");
        assert_eq!(remaining.lines().count(), 4);
        assert!(remaining.lines().nth(1).unwrap().starts_with("2026-11-20 09:00:00,"));
        assert!(get("daily.csv").contains("2026-11-20,claude,Claude,item,5h,5시간,30.00"));
        let hourly = get("hourly.csv");
        assert!(hourly.contains("\r\n10,claude,Claude,item,5h,5시간,20.00\r\n"), "{hourly}");
        let j: Value = serde_json::from_str(&get("analytics.json")).unwrap();
        assert_eq!(j["series"][0]["summary"]["usedPct"].as_f64().unwrap().round(), 30.0);
        assert_eq!(j["series"][0]["remaining"][2][1], json!(60.0));
        // 형식 고르기
        assert_eq!(render(&tz, &rows, "day1", t0, now, "json").len(), 1);
        assert_eq!(render(&tz, &rows, "day1", t0, now, "csv").len(), 5);
    }
}
