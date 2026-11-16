//! UsageMeter (Rust) 앱 진입점. 계획서 3장 `um-app`.
//! 지금 단계: 트레이 + 화면별 테두리 + Gemini(agy) 실시간 조회 + 이력의 마지막 값.

mod data;
mod overlay;
mod state;

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use um_core::i18n::tr;
use um_providers::agy::AgyParser;

use overlay::Overlay;
use state::AppState;

type Shared = Arc<Mutex<AppState>>;

thread_local! {
    /// 띠 창은 메인 스레드에서만 만지므로 여기 둔다.
    static OVERLAY: RefCell<Overlay> = RefCell::new(Overlay::default());
}

const TRAY_ID: &str = "main";

fn redraw(app: &AppHandle) {
    let st = app.state::<Shared>().inner().clone();
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let st = st.lock().unwrap();
        OVERLAY.with(|o| o.borrow_mut().redraw(&st));
        update_tray(&h, &st);
    });
}

/// 트레이 아이콘: 메뉴바 아이콘 AI가 있으면 그것, 없으면 잔여율이 가장 낮은 AI. (Swift StatusBarController.swift:847-858)
fn update_tray(app: &AppHandle, st: &AppState) {
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
    let lang = st.main.language;
    let mut tip: Vec<String> = st
        .snapshots
        .iter()
        .map(|(id, s)| {
            let mark = if st.from_history.contains(id) { " *" } else { "" };
            format!("{id} {}%{mark}", (s.remaining_ratio * 100.0).round())
        })
        .collect();
    if tip.is_empty() {
        tip.push(tr("menu.noAI", lang));
    }
    let _ = tray.set_tooltip(Some(tip.join("\n")));
}

/// Gemini(agy) 갱신. 실패하면 이전 값 유지(Swift ProviderManager.apply 규칙).
fn refresh(app: &AppHandle, parser: &Mutex<AgyParser>) {
    let snap = data::fetch_agy(&mut parser.lock().unwrap());
    if let Some(snap) = snap {
        let st = app.state::<Shared>();
        let mut st = st.lock().unwrap();
        st.from_history.retain(|x| x != "gemini");
        st.snapshots.insert("gemini".into(), snap);
        record_history(&mut st);
    }
    redraw(app);
}

fn record_history(st: &mut AppState) {
    let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
    let live: Vec<_> = st.snapshots.iter().filter(|(id, _)| !st.from_history.contains(id)).map(|(k, v)| (k.clone(), v.clone())).collect();
    let ratios = live.iter().map(|(k, v)| (k.clone(), v.remaining_ratio)).collect();
    let buckets = live.iter().filter_map(|(k, v)| v.raw_buckets.clone().map(|b| (k.clone(), b))).collect();
    let items = live.iter().map(|(k, v)| (k.clone(), v.sub_items().into_iter().map(|i| (i.id, i.remaining_ratio)).collect())).collect();
    if let Some(db) = st.history.as_mut() {
        let _ = db.record(now, &ratios, &buckets, &items);
    }
}

fn main() {
    tracing::info!("UsageMeter (Rust) 시작");
    let shared: Shared = Arc::new(Mutex::new(AppState::load()));
    {
        let mut st = shared.lock().unwrap();
        let ids = st.main.provider_order.clone();
        let seeded = st.history.as_ref().map(|db| data::seed_from_history(db, &ids, chrono::Utc::now())).unwrap_or_default();
        st.from_history = seeded.keys().cloned().collect();
        st.snapshots = seeded;
    }

    tauri::Builder::default()
        .manage(shared.clone())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let lang = shared.lock().unwrap().main.language;
            let hide = CheckMenuItem::with_id(app, "hide", tr("menu.hideBorder", lang), true, shared.lock().unwrap().hide_overlay, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "refresh", tr("menu.refresh", lang), true, Some("CmdOrCtrl+R"))?,
                    &hide,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, "quit", tr("menu.quit", lang), true, Some("CmdOrCtrl+Q"))?,
                ],
            )?;
            let parser = Arc::new(Mutex::new(AgyParser::new()));
            let p2 = parser.clone();
            TrayIconBuilder::with_id(TRAY_ID)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(move |app, e| match e.id().as_ref() {
                    "refresh" => {
                        let (app, p) = (app.clone(), p2.clone());
                        std::thread::spawn(move || refresh(&app, &p));
                    }
                    "hide" => {
                        let st = app.state::<Shared>();
                        let v = {
                            let mut st = st.lock().unwrap();
                            st.hide_overlay = !st.hide_overlay;
                            st.hide_overlay
                        };
                        let _ = hide.set_checked(v);
                        redraw(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            redraw(app.handle());

            // 갱신 주기마다 Gemini(agy) 조회. 첫 조회는 바로.
            let h = app.handle().clone();
            std::thread::spawn(move || loop {
                refresh(&h, &parser);
                let mins = h.state::<Shared>().lock().unwrap().main.effective_refresh_minutes();
                std::thread::sleep(Duration::from_secs(mins as u64 * 60));
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
