//! UsageMeter (Rust) 앱 진입점. 계획서 3장 `um-app`.
//! 지금 단계: 트레이(전체 메뉴·정보 패널) + 화면별 테두리 + 호버 + 로그인·로그아웃 + 사용량 실시간 조회 + 업데이트 확인.

mod analytics_ui;
mod data;
mod direct;
mod ipc;
mod locallogs;
mod menu;
#[cfg(target_os = "macos")]
mod native_color;
mod overlay;
#[cfg(target_os = "macos")]
mod panels;
mod state;
mod update;
mod web;
mod windows;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::image::Image;
use tauri::tray::TrayIconBuilder;
#[cfg(target_os = "macos")]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
use tauri::{AppHandle, Manager};
use um_core::i18n::tr;
use um_core::model::{UsageSnapshot, UsageStatus};
use um_providers::agy::AgyParser;
use um_providers::spec;

use overlay::Overlay;
use state::AppState;

type Shared = Arc<Mutex<AppState>>;
/// agy 단조 규칙 상태(Swift AgyQuotaFetcher.monotonicBuckets).
struct Agy(Mutex<AgyParser>);
/// 직접 호출용 HTTP 클라이언트(AI별, 쿠키는 메모리에만). 실패하면 버리고 다시 만든다.
struct DirectClients(Mutex<BTreeMap<String, reqwest::blocking::Client>>);

/// 직접 호출 경로를 쓸지(기본 켬, 0단계 S3·S5 통과: rust/docs/adr/0004). `UM_DIRECT=0`이면 웹뷰만.
fn direct_enabled() -> bool {
    std::env::var("UM_DIRECT").map_or(true, |v| v != "0")
}

/// 직접 호출: 만들어 둔 클라이언트 → 실패하면 쿠키를 새로 읽어 한 번 더. 둘 다 실패하면 None(웹뷰로 넘어감).
fn fetch_direct(app: &AppHandle, sp: &'static spec::ProviderSpec) -> Option<serde_json::Value> {
    let cache = app.state::<DirectClients>();
    for attempt in 0..2 {
        let cached = cache.0.lock().unwrap().get(sp.id).cloned();
        let c = match cached {
            Some(c) if attempt == 0 => c,
            _ => {
                let c = direct::client(app, sp)?;
                cache.0.lock().unwrap().insert(sp.id.to_string(), c.clone());
                c
            }
        };
        let raw = direct::probe_with(&c, sp);
        if raw["ok"] == true {
            return Some(raw);
        }
        cache.0.lock().unwrap().remove(sp.id);
    }
    None
}

thread_local! {
    /// 띠 창은 메인 스레드에서만 만지므로 여기 둔다.
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay::default());
}

const TRAY_ID: &str = "main";
/// 조회 하나의 최대 대기(Gemini DOM 대기 최대 6초 + 로드 여유).
const PROBE_TIMEOUT: Duration = Duration::from_secs(25);

/// 개발·측정용: 테두리·트레이 없이 조회와 기록만(24시간 측정 등). `UM_HEADLESS=1`.
fn headless() -> bool {
    std::env::var("UM_HEADLESS").is_ok_and(|v| v == "1")
}

fn shared(app: &AppHandle) -> Shared {
    app.state::<Shared>().inner().clone()
}

fn redraw(app: &AppHandle) {
    if headless() {
        return;
    }
    let st = shared(app);
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let st = st.lock().unwrap();
        OVERLAY.with(|o| o.borrow_mut().redraw(&st));
        update_tray(&h, &st);
    });
}

/// 트레이 아이콘·툴팁. (Swift StatusBarController.swift:847-858, 813-842)
fn update_tray(app: &AppHandle, st: &AppState) {
    let lang = st.main.language;
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let pick = st
        .snapshots
        .get(&st.main.menu_bar_icon_id)
        .map(|s| (st.main.menu_bar_icon_id.clone(), s))
        .or_else(|| {
            st.snapshots
                .iter()
                .min_by(|a, b| a.1.remaining_ratio.total_cmp(&b.1.remaining_ratio))
                .map(|(k, v)| (k.clone(), v))
        });
    let (ratio, color) = match &pick {
        Some((id, s)) => (
            Some(s.remaining_ratio),
            um_render::border::provider_color(&st.main, id),
        ),
        None => (None, [0.55, 0.55, 0.58, 1.0]),
    };
    let pm = um_render::icon::ring(ratio, color, 2.0);
    let _ = tray.set_icon(Some(Image::new_owned(
        um_render::icon::to_straight_rgba(&pm),
        pm.width(),
        pm.height(),
    )));
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
    // macOS는 호버 패널이 같은 정보를 보여주므로 시스템 툴팁을 끈다(겹쳐 보임). Windows는 플라이아웃 전까지 툴팁.
    let _ = tray.set_tooltip(if cfg!(target_os = "macos") { None } else { Some(tip.join("\n")) });
}

/// AI 하나 조회. Gemini는 agy를 먼저, 없으면 웹. (Swift WebSession.fetchUsage)
fn fetch_one(app: &AppHandle, id: &str) -> Option<UsageSnapshot> {
    let sp = spec::spec(id)?;
    if id == "gemini"
        && let Some(s) = data::fetch_agy(&mut app.state::<Agy>().0.lock().unwrap())
    {
        return Some(s);
    }
    if direct_enabled() && !sp.needs_rendered_dom && !std::env::var("UM_S3").is_ok_and(|v| v == "1")
    {
        let t = std::time::Instant::now();
        if let Some(raw) = fetch_direct(app, sp) {
            tracing::info!("{id} 직접 조회 {:.1}초 ok=true", t.elapsed().as_secs_f64());
            return Some(um_providers::parse::snapshot_from_probe(
                id,
                &raw,
                chrono::Utc::now(),
            ));
        }
        tracing::info!("{id} 직접 조회 실패 → 웹뷰로");
    }
    let t0 = std::time::Instant::now();
    let raw = web::probe(app, sp, PROBE_TIMEOUT);
    tracing::info!(
        "{id} 웹 조회 {:.1}초 ok={}",
        t0.elapsed().as_secs_f64(),
        raw["ok"]
    );
    // 0단계 S3: UM_S3=1이면 직접 호출도 해서 결과를 비교한다(상태·잔여율만 기록, 쿠키·토큰은 기록 안 함).
    if std::env::var("UM_S3").is_ok_and(|v| v == "1") && !sp.needs_rendered_dom {
        let t1 = std::time::Instant::now();
        let dr = direct::probe(app, sp);
        let now = chrono::Utc::now();
        let (a, b) = (
            um_providers::parse::snapshot_from_probe(id, &raw, now),
            um_providers::parse::snapshot_from_probe(id, &dr, now),
        );
        tracing::info!(
            "S3 {id} 직접 {:.1}초 ok={} reason={} status={} | 웹 ok={} | 5h 같음={} 주간 같음={}",
            t1.elapsed().as_secs_f64(),
            dr["ok"],
            dr["reason"],
            dr["status"],
            raw["ok"],
            (a.remaining_ratio - b.remaining_ratio).abs() < 1e-9,
            a.secondary_ratio == b.secondary_ratio
        );
    }
    if raw["ok"] != true {
        tracing::info!("{}", um_providers::parse::failure_log_line(id, &raw));
    }
    Some(um_providers::parse::snapshot_from_probe(
        id,
        &raw,
        chrono::Utc::now(),
    ))
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
    menu::rebuild(app); // 로그인 상태가 바뀌었을 수 있다
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
                .filter(|id| {
                    st.providers
                        .get(*id)
                        .and_then(|p| p.snapshot.as_ref())
                        .is_some_and(um_providers::state::should_retry)
                })
                .cloned()
                .collect()
        };
        if retry.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_secs(
            um_providers::state::START_RETRY_INTERVAL_SECS,
        ));
        refresh_ids(app, &retry);
    }
}

fn record_history(st: &mut AppState) {
    let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    let live: Vec<(String, UsageSnapshot)> = st
        .providers
        .iter()
        .filter_map(|(k, p)| p.active_snapshot().map(|s| (k.clone(), s.clone())))
        .collect();
    let ratios = live
        .iter()
        .map(|(k, v)| (k.clone(), v.remaining_ratio))
        .collect();
    let buckets = live
        .iter()
        .filter_map(|(k, v)| v.raw_buckets.clone().map(|b| (k.clone(), b)))
        .collect();
    let items = live
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                v.sub_items()
                    .into_iter()
                    .map(|i| (i.id, i.remaining_ratio))
                    .collect(),
            )
        })
        .collect();
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
            menu::rebuild(&app);
        } else {
            // 이미 로그인돼 있으면 창 없이 끝낸다(Swift LoginWindowController.show: 먼저 조회 확인).
            if let Some(s) = fetch_one(&app, &id)
                && s.status == UsageStatus::Ok
            {
                apply(&app, &id, s);
                redraw(&app);
                return;
            }
            login_and_refresh(&app, sp, lang);
        }
    });
}

/// 로그인 창을 띄우고, 로그인이 확인되면(창은 자동으로 닫힘) 그 AI를 바로 조회한다.
fn login_and_refresh(
    app: &AppHandle,
    sp: &'static spec::ProviderSpec,
    lang: um_core::types::AppLanguage,
) -> bool {
    let t = std::time::Instant::now();
    let ok = web::login(app, sp, &format!("{}  {}", sp.name, tr("acct.login", lang)));
    tracing::info!(
        "{} 로그인 창 결과 ok={ok} {:.1}초",
        sp.id,
        t.elapsed().as_secs_f64()
    );
    if ok {
        // 쿠키가 바뀌었으니 직접 호출 클라이언트를 버린다.
        app.state::<DirectClients>().0.lock().unwrap().remove(sp.id);
        refresh_ids(app, &[sp.id.to_string()]);
    }
    ok
}

/// 트레이 아이콘 영역(Tauri: 물리 px, 왼쪽 위 원점) → 전역 AppKit 좌표(pt, 왼쪽 아래 원점).
#[cfg(target_os = "macos")]
fn tray_rect(app: &AppHandle, r: &tauri::Rect) -> panels::Rect {
    let scale = app.primary_monitor().ok().flatten().map(|m| m.scale_factor()).unwrap_or(2.0);
    let pos = r.position.to_logical::<f64>(scale);
    let size = r.size.to_logical::<f64>(scale);
    let primary_h = um_platform::display::displays().first().map(|d| d.height_pt).unwrap_or(0.0);
    panels::Rect { x: pos.x, y: primary_h - pos.y - size.height, w: size.width, h: size.height }
}

/// 패널 글꼴 로케일을 앱 언어에 맞춘다(같은 한자도 언어별 글꼴 모양).
fn apply_text_locale(lang: um_core::types::AppLanguage) {
    um_render::text::set_locale(match lang {
        um_core::types::AppLanguage::Korean => "ko-KR",
        um_core::types::AppLanguage::English => "en-US",
        um_core::types::AppLanguage::Japanese => "ja-JP",
    });
}

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
    tracing::info!("UsageMeter (Rust) 시작");
    let shared_state: Shared = Arc::new(Mutex::new(AppState::load()));
    {
        let mut st = shared_state.lock().unwrap();
        let ids = st.main.provider_order.clone();
        let seeds = st
            .history
            .as_ref()
            .map(|db| data::seed_from_history(db, &ids, chrono::Utc::now()))
            .unwrap_or_default();
        st.seeds = seeds;
        st.rebuild_snapshots();
    }

    tauri::Builder::default()
        .manage(shared_state.clone())
        .manage(Agy(Mutex::new(AgyParser::new())))
        .manage(DirectClients(Mutex::new(BTreeMap::new())))
        .manage(ipc::Editing::default())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![ipc::ui_state, ipc::ui_set, ipc::ui_action, analytics_ui::ui_analytics, locallogs::ui_locallogs, locallogs::ui_locallogs_set, locallogs::ui_tokens])
        .setup(move |app| {
            let dir = shared_state.lock().unwrap().dir.clone();
            app.manage(locallogs::LocalLogs::new(&dir));
            locallogs::start(app.handle());
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let lang = shared_state.lock().unwrap().main.language;
            apply_text_locale(lang);
            let menu = menu::build(app.handle(), &shared_state.lock().unwrap())?;
            if !headless() {
                #[cfg(all(target_os = "macos", not(feature = "store")))]
                update::watch_bundle_replacement(app.handle());
                TrayIconBuilder::with_id(TRAY_ID)
                    .menu(&menu)
                    // macOS: 왼쪽 클릭 = 정보 패널, 오른쪽 클릭 = 메뉴(결정 D1). 그 밖은 아직 왼쪽 클릭 메뉴.
                    .show_menu_on_left_click(cfg!(not(target_os = "macos")))
                    .on_tray_icon_event(|tray, e| {
                        #[cfg(target_os = "macos")]
                        {
                            let app = tray.app_handle();
                            match e {
                                TrayIconEvent::Enter { rect, .. } => panels::tray_hover(app, tray_rect(app, &rect), true),
                                TrayIconEvent::Leave { rect, .. } => panels::tray_hover(app, tray_rect(app, &rect), false),
                                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, rect, .. } => {
                                    panels::toggle_card(app, tray_rect(app, &rect))
                                }
                                TrayIconEvent::Click { button: MouseButton::Right, button_state: MouseButtonState::Down, .. } => panels::menu_opening(),
                                _ => {}
                            }
                        }
                        #[cfg(not(target_os = "macos"))]
                        let _ = (tray, e);
                    })
                    .on_menu_event(|app, e| menu::on_event(app, e.id().as_ref()))
                    .build(app)?;
                #[cfg(target_os = "macos")]
                panels::init(app.handle());
            }
            redraw(app.handle());
            // 개발용: UM_OPEN_WINDOW=settings[#탭] 또는 analytics로 시작하면 그 창을 바로 연다.
            if let Ok(page) = std::env::var("UM_OPEN_WINDOW") {
                windows::open(app.handle(), &page);
            }

            // 시작 조회(재시도 포함) 후 갱신 주기마다 전체 조회.
            let h = app.handle().clone();
            std::thread::spawn(move || {
                refresh_with_retry(&h);
                // 개발용: UM_TEST_LOGIN=<id>면 로그인 창을 강제로 띄워 자동 판정·닫힘을 확인한다.
                if let Some(sp) = std::env::var("UM_TEST_LOGIN")
                    .ok()
                    .and_then(|id| spec::spec(&id))
                {
                    let lang = shared(&h).lock().unwrap().main.language;
                    login_and_refresh(&h, sp, lang);
                }
                loop {
                    let mins = shared(&h).lock().unwrap().main.effective_refresh_minutes();
                    // 개발용: UM_REFRESH_SECS로 주기를 짧게(메모리·누수 측정).
                    let secs = std::env::var("UM_REFRESH_SECS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(mins as u64 * 60);
                    // 1초씩 기다리며 주기 설정이 바뀌면 새 주기로 다시 시작(Swift는 주기 변경 시 타이머 재설정).
                    let mut waited = 0;
                    while waited < secs {
                        std::thread::sleep(Duration::from_secs(1));
                        waited += 1;
                        if shared(&h).lock().unwrap().main.effective_refresh_minutes() != mins {
                            break;
                        }
                    }
                    if waited >= secs {
                        refresh_all(&h);
                    }
                }
            });
            // 업데이트 자동 확인: 시작 때 + 24시간마다(설정으로 끔, Store판 제외). (Swift UpdateChecker.swift:88-98)
            if !cfg!(feature = "store") && !headless() {
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    loop {
                        if shared(&h).lock().unwrap().main.auto_check_update {
                            menu::check_update(&h);
                        }
                        std::thread::sleep(Duration::from_secs(24 * 3600));
                    }
                });
            }
            // 화면 연결·해제·해상도 변경 감지(3초 간격 비교, 변화 없으면 아무것도 하지 않음).
            let h = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(3));
                    let h2 = h.clone();
                    let _ = h.run_on_main_thread(move || {
                        if OVERLAY.with(|o| o.borrow_mut().check_displays()) {
                            #[cfg(target_os = "macos")]
                            panels::refresh_displays();
                            redraw(&h2);
                        }
                    });
                }
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
