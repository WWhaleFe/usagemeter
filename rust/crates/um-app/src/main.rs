//! UsageMeter (Rust) 앱 진입점. 계획서 3장 `um-app`.
//! 지금 단계: 트레이 + 화면별 테두리 + 로그인·로그아웃 + 사용량 실시간 조회(Claude·Codex 웹뷰, Gemini agy 우선).

mod data;
mod overlay;
mod state;
mod web;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};
use um_core::i18n::tr;
use um_core::model::{UsageSnapshot, UsageStatus};
use um_providers::agy::AgyParser;
use um_providers::spec;

use overlay::Overlay;
use state::AppState;

type Shared = Arc<Mutex<AppState>>;
/// AI별 "로그인…/로그아웃" 메뉴 항목(상태가 바뀌면 글자를 바꾼다).
struct AcctMenu(Mutex<BTreeMap<String, MenuItem<Wry>>>);
/// agy 단조 규칙 상태(Swift AgyQuotaFetcher.monotonicBuckets).
struct Agy(Mutex<AgyParser>);

thread_local! {
    /// 띠 창은 메인 스레드에서만 만지므로 여기 둔다.
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay::default());
}

const TRAY_ID: &str = "main";
/// 조회 하나의 최대 대기(Gemini DOM 대기 최대 6초 + 로드 여유).
const PROBE_TIMEOUT: Duration = Duration::from_secs(25);

fn shared(app: &AppHandle) -> Shared {
    app.state::<Shared>().inner().clone()
}

fn redraw(app: &AppHandle) {
    let st = shared(app);
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let st = st.lock().unwrap();
        OVERLAY.with(|o| o.borrow_mut().redraw(&st));
        update_tray(&h, &st);
    });
}

/// 트레이 아이콘·툴팁·로그인 메뉴 글자. (Swift StatusBarController.swift:847-858, 813-842)
fn update_tray(app: &AppHandle, st: &AppState) {
    let lang = st.main.language;
    if let Some(menu) = app.try_state::<AcctMenu>() {
        for (id, item) in menu.0.lock().unwrap().iter() {
            let name = spec::spec(id).map(|s| s.name).unwrap_or(id);
            let key = if st.is_logged_in(id) { "acct.logout" } else { "acct.login" };
            let _ = item.set_text(format!("{name}  {}", tr(key, lang)));
        }
    }
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let pick = st
        .snapshots
        .get(&st.main.menu_bar_icon_id)
        .map(|s| (st.main.menu_bar_icon_id.clone(), s))
        .or_else(|| st.snapshots.iter().min_by(|a, b| a.1.remaining_ratio.total_cmp(&b.1.remaining_ratio)).map(|(k, v)| (k.clone(), v)));
    let (ratio, color) = match &pick {
        Some((id, s)) => (Some(s.remaining_ratio), um_render::border::provider_color(&st.main, id)),
        None => (None, [0.55, 0.55, 0.58, 1.0]),
    };
    let pm = um_render::icon::ring(ratio, color, 2.0);
    let _ = tray.set_icon(Some(Image::new_owned(um_render::icon::to_straight_rgba(&pm), pm.width(), pm.height())));
    let mut tip: Vec<String> = st
        .snapshots
        .iter()
        .map(|(id, s)| {
            let name = spec::spec(id).map(|s| s.name).unwrap_or(id);
            let mark = if st.seeds.contains_key(id) { " *" } else { "" };
            format!("{name} {}%{mark}", (s.remaining_ratio * 100.0).round())
        })
        .collect();
    if tip.is_empty() {
        tip.push(tr("menu.noAI", lang));
    }
    let _ = tray.set_tooltip(Some(tip.join("\n")));
}

/// AI 하나 조회. Gemini는 agy를 먼저, 없으면 웹. (Swift WebSession.fetchUsage)
fn fetch_one(app: &AppHandle, id: &str) -> Option<UsageSnapshot> {
    let sp = spec::spec(id)?;
    if id == "gemini"
        && let Some(s) = data::fetch_agy(&mut app.state::<Agy>().0.lock().unwrap())
    {
        return Some(s);
    }
    let t0 = std::time::Instant::now();
    let raw = web::probe(app, sp, PROBE_TIMEOUT);
    tracing::info!("{id} 웹 조회 {:.1}초 ok={}", t0.elapsed().as_secs_f64(), raw["ok"]);
    if raw["ok"] != true {
        tracing::info!("{}", um_providers::parse::failure_log_line(id, &raw));
    }
    Some(um_providers::parse::snapshot_from_probe(id, &raw, chrono::Utc::now()))
}

/// 결과 반영(Swift ProviderManager.apply). 성공·로그아웃 판정이 나면 그 AI의 이력 값은 버린다.
fn apply(app: &AppHandle, id: &str, snap: UsageSnapshot) {
    let st = shared(app);
    let mut st = st.lock().unwrap();
    if matches!(snap.status, UsageStatus::Ok | UsageStatus::AuthExpired) {
        st.seeds.remove(id);
    }
    st.providers.entry(id.to_string()).or_default().apply(snap);
    st.rebuild_snapshots();
}

fn refresh_ids(app: &AppHandle, ids: &[String]) {
    for id in ids {
        if let Some(s) = fetch_one(app, id) {
            apply(app, id, s);
        }
    }
    record_history(&mut shared(app).lock().unwrap());
    redraw(app);
}

fn refresh_all(app: &AppHandle) {
    let ids = shared(app).lock().unwrap().main.provider_order.clone();
    refresh_ids(app, &ids);
}

/// 시작 시 조회. 읽기 실패(로그아웃 아님)한 AI만 2초 간격으로 최대 3번 더 시도. (Swift refreshWithRetry, 4회·2초)
fn refresh_with_retry(app: &AppHandle) {
    refresh_all(app);
    for _ in 0..(um_providers::state::START_RETRY_ATTEMPTS - 1) {
        let retry: Vec<String> = {
            let st = shared(app);
            let st = st.lock().unwrap();
            st.main
                .provider_order
                .iter()
                .filter(|id| st.providers.get(*id).and_then(|p| p.snapshot.as_ref()).is_some_and(um_providers::state::should_retry))
                .cloned()
                .collect()
        };
        if retry.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_secs(um_providers::state::START_RETRY_INTERVAL_SECS));
        refresh_ids(app, &retry);
    }
}

fn record_history(st: &mut AppState) {
    let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    let live: Vec<(String, UsageSnapshot)> =
        st.providers.iter().filter_map(|(k, p)| p.active_snapshot().map(|s| (k.clone(), s.clone()))).collect();
    let ratios = live.iter().map(|(k, v)| (k.clone(), v.remaining_ratio)).collect();
    let buckets = live.iter().filter_map(|(k, v)| v.raw_buckets.clone().map(|b| (k.clone(), b))).collect();
    let items = live.iter().map(|(k, v)| (k.clone(), v.sub_items().into_iter().map(|i| (i.id, i.remaining_ratio)).collect())).collect();
    if let Some(db) = st.history.as_mut() {
        let _ = db.record(now, &ratios, &buckets, &items);
    }
}

/// 로그인/로그아웃 메뉴 클릭.
fn toggle_account(app: &AppHandle, id: String) {
    let Some(sp) = spec::spec(&id) else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        let (logged_in, lang) = {
            let st = shared(&app);
            let st = st.lock().unwrap();
            (st.is_logged_in(&id), st.main.language)
        };
        if logged_in {
            web::logout(&app, sp);
            let st = shared(&app);
            let mut st = st.lock().unwrap();
            if let Some(p) = st.providers.get_mut(&id) {
                p.logout();
            }
            st.seeds.remove(&id);
            st.rebuild_snapshots();
            drop(st);
            redraw(&app);
        } else if web::login(&app, sp, &format!("{}  {}", sp.name, tr("acct.login", lang))) {
            refresh_ids(&app, std::slice::from_ref(&id));
        }
    });
}

fn main() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_target(false).with_writer(std::io::stderr).init();
    tracing::info!("UsageMeter (Rust) 시작");
    let shared_state: Shared = Arc::new(Mutex::new(AppState::load()));
    {
        let mut st = shared_state.lock().unwrap();
        let ids = st.main.provider_order.clone();
        let seeds = st.history.as_ref().map(|db| data::seed_from_history(db, &ids, chrono::Utc::now())).unwrap_or_default();
        st.seeds = seeds;
        st.rebuild_snapshots();
    }

    tauri::Builder::default()
        .manage(shared_state.clone())
        .manage(Agy(Mutex::new(AgyParser::new())))
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let (lang, hidden, order) = {
                let st = shared_state.lock().unwrap();
                (st.main.language, st.hide_overlay, st.main.provider_order.clone())
            };
            let hide = CheckMenuItem::with_id(app, "hide", tr("menu.hideBorder", lang), true, hidden, None::<&str>)?;
            let acct = Submenu::with_id(app, "acct", tr("menu.aiLoginout", lang), true)?;
            let mut items = BTreeMap::new();
            for id in &order {
                let item = MenuItem::with_id(app, format!("acct:{id}"), id, true, None::<&str>)?;
                acct.append(&item)?;
                items.insert(id.clone(), item);
            }
            app.manage(AcctMenu(Mutex::new(items)));
            let menu = Menu::with_items(
                app,
                &[
                    &acct,
                    &MenuItem::with_id(app, "refresh", tr("menu.refresh", lang), true, Some("CmdOrCtrl+R"))?,
                    &hide,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, "quit", tr("menu.quit", lang), true, Some("CmdOrCtrl+Q"))?,
                ],
            )?;
            TrayIconBuilder::with_id(TRAY_ID)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, e| match e.id().as_ref() {
                    "refresh" => {
                        let app = app.clone();
                        std::thread::spawn(move || refresh_all(&app));
                    }
                    "hide" => {
                        let v = {
                            let st = shared(app);
                            let mut st = st.lock().unwrap();
                            st.hide_overlay = !st.hide_overlay;
                            st.hide_overlay
                        };
                        let _ = hide.set_checked(v);
                        redraw(app);
                    }
                    "quit" => app.exit(0),
                    other => {
                        if let Some(id) = other.strip_prefix("acct:") {
                            toggle_account(app, id.to_string());
                        }
                    }
                })
                .build(app)?;
            redraw(app.handle());

            // 시작 조회(재시도 포함) 후 갱신 주기마다 전체 조회.
            let h = app.handle().clone();
            std::thread::spawn(move || {
                refresh_with_retry(&h);
                loop {
                    let mins = shared(&h).lock().unwrap().main.effective_refresh_minutes();
                    std::thread::sleep(Duration::from_secs(mins as u64 * 60));
                    refresh_all(&h);
                }
            });
            // 화면 연결·해제·해상도 변경 감지(3초 간격 비교, 변화 없으면 아무것도 하지 않음).
            let h = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(3));
                let h2 = h.clone();
                let _ = h.run_on_main_thread(move || {
                    if OVERLAY.with(|o| o.borrow_mut().check_displays()) {
                        redraw(&h2);
                    }
                });
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("UsageMeter 시작 실패")
        .run(|_, e| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = e {
                // 창을 모두 닫아도 트레이 앱은 계속 돈다. 명시적 종료(code 있음)만 허용.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
