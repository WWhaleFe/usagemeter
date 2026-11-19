//! 트레이 전체 메뉴(Swift StatusBarController.swift:87-207 `populateMenu`, 계획서 부록 A-3).
//! 상태(언어·로그인·프리셋·업데이트)가 바뀌면 메뉴를 통째로 다시 만든다(`rebuild`).
//! macOS는 클릭 설정에 따라 열며 함께 표시 모드에서는 카드·차트를 메뉴 위에 넣는다.

use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};
use um_core::i18n::{tf, tn, tr};
use um_core::settings::{MAX_PRESETS, REFRESH_PRESETS, ScreenInfo, Settings, device_fit};
use um_core::types::AppLanguage;
use um_providers::spec;

use crate::state::AppState;
use crate::update::{DownloadState, UpdateState};

/// 개발자 메일(Swift SupportView.swift).
pub const REPORT_EMAIL: &str = "feedback.wwhale@gmail.com";

/// 표시 정보 체크 8개: (메뉴 id, 문자열 키). 순서는 Swift와 같다.
const INFO_ITEMS: [(&str, &str); 8] = [
    ("info:5h", "info.5h"),
    ("info:weekly", "info.weekly"),
    ("info:opus", "info.opus"),
    ("info:reset", "info.reset"),
    ("info:countdown", "info.countdown"),
    ("info:pace", "info.pace"),
    ("info:chart", "info.chart"),
    ("info:updated", "info.updated"),
];

fn info_flag<'a>(s: &'a mut Settings, id: &str) -> Option<&'a mut bool> {
    Some(match id {
        "info:5h" => &mut s.menu_show_5h,
        "info:weekly" => &mut s.menu_show_weekly,
        "info:opus" => &mut s.menu_show_opus,
        "info:reset" => &mut s.menu_show_reset,
        "info:countdown" => &mut s.menu_show_countdown,
        "info:pace" => &mut s.menu_show_pace,
        "info:chart" => &mut s.menu_show_chart,
        "info:updated" => &mut s.menu_show_updated,
        _ => return None,
    })
}

type R<T> = tauri::Result<T>;

fn item(app: &AppHandle, id: &str, text: impl AsRef<str>, enabled: bool, accel: Option<&str>) -> R<MenuItem<Wry>> {
    MenuItem::with_id(app, id, text, enabled, accel)
}

fn check(app: &AppHandle, id: &str, text: impl AsRef<str>, on: bool) -> R<CheckMenuItem<Wry>> {
    CheckMenuItem::with_id(app, id, text, true, on, None::<&str>)
}

fn sep(app: &AppHandle) -> R<PredefinedMenuItem<Wry>> {
    PredefinedMenuItem::separator(app)
}

/// 메뉴 전체를 만든다.
pub fn build(app: &AppHandle, st: &AppState) -> R<Menu<Wry>> {
    let s = &st.main;
    let lang = s.language;
    let menu = Menu::new(app)?;

    #[cfg(target_os = "macos")]
    if s.tray_click_mode == um_core::settings::TrayClickMode::Together {
        menu.append(&item(app, "card", crate::panels::MENU_CARD_TITLE, false, None)?)?;
        menu.append(&sep(app)?)?;
    }
    menu.append(&item(app, "analytics", tr("menu.analytics", lang), true, None)?)?;

    // 표시 정보(체크해도 메뉴가 닫히는 것은 네이티브 메뉴의 한계, Swift는 열린 채 유지).
    let info = Submenu::with_id(app, "info", tr("menu.showInfo", lang), true)?;
    for (id, key) in INFO_ITEMS {
        let mut tmp = s.clone();
        let on = info_flag(&mut tmp, id).map(|b| *b).unwrap_or(false);
        info.append(&check(app, id, tr(s.bar_fill_mode.text_key(key), lang), on)?)?;
    }
    menu.append(&info)?;

    // 자동 갱신 주기: 프리셋(사용자 설정이 꺼져 있을 때 체크) + 사용자 설정.
    let interval = Submenu::with_id(app, "interval", tr("menu.interval", lang), true)?;
    for m in REFRESH_PRESETS {
        let on = !s.refresh_use_custom && s.refresh_preset_minutes == m;
        interval.append(&check(app, &format!("interval:{m}"), tn("interval.minutesFmt", lang, m as i64), on)?)?;
    }
    interval.append(&sep(app)?)?;
    interval.append(&check(app, "interval:custom", tr("menu.intervalCustom", lang), s.refresh_use_custom)?)?;
    menu.append(&interval)?;

    let langs = Submenu::with_id(app, "lang", tr("lang.title", lang), true)?;
    for l in [AppLanguage::Korean, AppLanguage::English, AppLanguage::Japanese] {
        langs.append(&check(app, &format!("lang:{}", l.code()), l.display_name(), l == lang)?)?;
    }
    menu.append(&langs)?;
    menu.append(&sep(app)?)?;

    // AI 로그인 / 로그아웃: "● 이름"(로그인) / "○ 이름" 머리말 + 동작 항목. (Swift :813-842)
    let acct = Submenu::with_id(app, "acct", tr("menu.aiLoginout", lang), true)?;
    for sp in spec::ALL {
        let id = sp.id;
        let on = st.is_logged_in(id);
        acct.append(&item(app, &format!("acct-h:{id}"), format!("{} {}", if on { "●" } else { "○" }, sp.name), false, None)?)?;
        acct.append(&item(app, &format!("acct:{id}"), tr(if on { "menu.logout" } else { "menu.login" }, lang), true, None)?)?;
    }
    menu.append(&acct)?;
    menu.append(&item(app, "refresh", tr("menu.refresh", lang), true, Some("CmdOrCtrl+R"))?)?;
    menu.append(&sep(app)?)?;

    menu.append(&item(app, "settings", tr("menu.settings", lang), true, Some("CmdOrCtrl+,"))?)?;

    // 프리셋: 불러오기·삭제·현재 저장. (Swift :753-808)
    let presets = Submenu::with_id(app, "presets", tr("menu.presets", lang), true)?;
    if st.presets.is_empty() {
        presets.append(&item(app, "preset-none", tr("menu.noPreset", lang), false, None)?)?;
    } else {
        for p in &st.presets {
            presets.append(&item(app, &format!("preset:load:{}", p.id), format!("{}{}", tr("menu.loadPrefix", lang), p.name), true, None)?)?;
        }
        presets.append(&sep(app)?)?;
        for p in &st.presets {
            presets.append(&item(app, &format!("preset:del:{}", p.id), format!("{}{}", tr("menu.deletePrefix", lang), p.name), true, None)?)?;
        }
    }
    presets.append(&sep(app)?)?;
    presets.append(&item(app, "preset:save", tr("menu.saveCurrent", lang), st.presets.len() < MAX_PRESETS, None)?)?;
    menu.append(&presets)?;

    let borders = Submenu::with_id(app, "borders", tr("menu.borderLines", lang), true)?;
    for id in &s.provider_order {
        if let Some(sp) = spec::spec(id) {
            borders.append(&check(app, &format!("border:{id}"), sp.name, !s.border_hidden_ids.contains(id))?)?;
        }
    }
    menu.append(&borders)?;
    menu.append(&check(app, "hide", tr("menu.hideBorder", lang), st.hide_overlay)?)?;
    menu.append(&item(app, "fit", tr("menu.deviceFit", lang), true, None)?)?;
    menu.append(&sep(app)?)?;

    if !cfg!(feature = "store") {
        let report = Submenu::with_id(app, "report", tr("menu.reportSubmenu", lang), true)?;
        report.append(&item(app, "report-h", format!("{}  {REPORT_EMAIL}", tr("menu.reportTitle", lang)), false, None)?)?;
        report.append(&sep(app)?)?;
        report.append(&item(app, "report:mail", tr("menu.reportMail", lang), true, None)?)?;
        report.append(&item(app, "report:copy", tr("menu.reportCopy", lang), true, None)?)?;
        menu.append(&report)?;
        menu.append(&sep(app)?)?;
        menu.append(&item(app, "support", format!("☕ {}", tr("menu.support", lang)), true, None)?)?;
        menu.append(&sep(app)?)?;
    }

    menu.append(&item(app, "version", format!("UsageMeter {}", tf("update.version", lang, crate::update::current_version())), false, None)?)?;
    if !cfg!(feature = "store") {
        for it in update_items(app, st, lang)? {
            menu.append(it.as_ref())?;
        }
    }
    menu.append(&sep(app)?)?;
    menu.append(&item(app, "quit", tr("menu.quit", lang), true, Some("CmdOrCtrl+Q"))?)?;
    Ok(menu)
}

/// 업데이트 상태 줄 + 확인 + 내려받기 + 릴리스 페이지. (Swift :180-206)
fn update_items(app: &AppHandle, st: &AppState, lang: AppLanguage) -> R<Vec<Box<dyn IsMenuItem<Wry>>>> {
    let mut v: Vec<Box<dyn IsMenuItem<Wry>>> = vec![];
    match &st.update {
        UpdateState::Idle => {}
        UpdateState::Checking => v.push(Box::new(item(app, "upd-s", tr("update.checking", lang), false, None)?)),
        UpdateState::UpToDate => v.push(Box::new(item(app, "upd-s", format!("✓ {}", tr("update.upToDate", lang)), false, None)?)),
        UpdateState::Available { version, .. } => {
            v.push(Box::new(item(app, "upd-s", format!("🔵 {}", tf("update.available", lang, &format!("v{version}"))), false, None)?))
        }
        UpdateState::Failed => v.push(Box::new(item(app, "update:check", format!("⚠️ {}", tr("update.failed", lang)), true, None)?)),
    }
    v.push(Box::new(item(app, "update:check", tr("update.check", lang), st.update != UpdateState::Checking, None)?));
    let dl = match &st.download {
        DownloadState::Idle => tr("update.download", lang),
        DownloadState::Downloading => tr("update.downloading", lang),
        DownloadState::Done(_) => tr("update.downloaded", lang),
        DownloadState::Failed => tr("update.downloadFail", lang),
    };
    v.push(Box::new(item(app, "update:download", dl, st.download != DownloadState::Downloading, None)?));
    v.push(Box::new(item(app, "update:page", tr("update.openPage", lang), true, None)?));
    Ok(v)
}

/// 메뉴를 다시 만들어 트레이에 단다(메인 스레드 밖에서 불러도 된다).
pub fn rebuild(app: &AppHandle) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        #[cfg(windows)]
        if crate::panels::menu_tracking() { return; }
        let st = crate::shared(&h);
        let Ok(st) = st.lock() else { return };
        if let (Some(tray), Ok(menu)) = (h.tray_by_id(crate::TRAY_ID), build(&h, &st)) {
            let _ = tray.set_menu(Some(menu));
        }
    });
}

/// 설정을 바꾸고 저장한 뒤 메뉴·테두리를 갱신한다.
fn change(app: &AppHandle, f: impl FnOnce(&mut AppState)) {
    let auto_update_enabled;
    {
        let st = crate::shared(app);
        let mut st = st.lock().unwrap();
        let prev_auto = st.main.auto_check_update;
        f(&mut st);
        auto_update_enabled = !prev_auto && st.main.auto_check_update;
        st.rebuild_snapshots();
        st.save();
    }
    crate::ipc::changed(app);
    if auto_update_enabled && !cfg!(feature = "store") { check_update(app); }
}

/// 메뉴 클릭 처리.
pub fn on_event(app: &AppHandle, id: &str) {
    if cfg!(feature = "store") && (id.starts_with("update:") || id.starts_with("report:") || id == "support") { return; }
    tracing::info!("메뉴 선택: {id}");
    match id {
        "refresh" => {
            let app = app.clone();
            std::thread::spawn(move || crate::refresh_all(&app));
        }
        "hide" => change(app, |st| st.hide_overlay = !st.hide_overlay),
        "quit" => app.exit(0),
        "analytics" => crate::windows::open(app, "analytics"),
        "settings" => crate::windows::open(app, "settings"),
        "support" => crate::windows::open(app, "settings#support"),
        "fit" => fit_all(app),
        "report:mail" => {
            let lang = crate::shared(app).lock().unwrap().main.language;
            let subject = tr("report.mailSubject", lang).replace(' ', "%20");
            let _ = open::that(format!("mailto:{REPORT_EMAIL}?subject={subject}"));
        }
        "report:copy" => {
            if let Ok(mut c) = arboard::Clipboard::new() {
                let _ = c.set_text(REPORT_EMAIL);
            }
        }
        "update:check" => check_update(app),
        "update:page" => {
            let page = match &crate::shared(app).lock().unwrap().update {
                UpdateState::Available { page, .. } => page.clone(),
                _ => crate::update::PAGE.to_string(),
            };
            let _ = open::that(page);
        }
        "update:download" => download(app),
        "interval:custom" => {
            change(app, |st| st.main.refresh_use_custom = true);
            crate::windows::open(app, "settings#interval");
        }
        "preset:save" => crate::windows::open(app, "settings#presets"),
        _ => {
            if let Some(pid) = id.strip_prefix("border:").filter(|id| spec::spec(id).is_some()) {
                change(app, |st| {
                    if !st.main.border_hidden_ids.remove(pid) { st.main.border_hidden_ids.insert(pid.to_string()); }
                });
            } else if let Some(rest) = id.strip_prefix("acct:") {
                crate::toggle_account(app, rest.to_string());
            } else if let Some(m) = id.strip_prefix("interval:").and_then(|m| m.parse::<u32>().ok()) {
                change(app, |st| {
                    st.main.refresh_use_custom = false;
                    st.main.refresh_preset_minutes = m;
                });
            } else if let Some(code) = id.strip_prefix("lang:") {
                let l = match code {
                    "ko" => AppLanguage::Korean,
                    "ja" => AppLanguage::Japanese,
                    _ => AppLanguage::English,
                };
                crate::apply_text_locale(l);
                change(app, |st| st.main.language = l);
            } else if id.starts_with("info:") {
                change(app, |st| {
                    if let Some(b) = info_flag(&mut st.main, id) {
                        *b = !*b;
                    }
                });
            } else if let Some(pid) = id.strip_prefix("preset:load:") {
                change(app, |st| {
                    if let Some(p) = st.presets.iter().find(|p| p.id == pid).cloned() {
                        st.main.apply_snapshot(&p.snapshot);
                        st.main.apply_platform_rules(cfg!(windows));
                    }
                });
            } else if let Some(pid) = id.strip_prefix("preset:del:") {
                change(app, |st| st.presets.retain(|p| p.id != pid));
            }
        }
    }
}

/// 프리셋 id(UUID 형태 문자열, 의존성 없이 시각+난수로).
pub fn new_id() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let h = t ^ (std::process::id() as u128) << 64;
    format!("{:08X}-{:04X}-4{:03X}-8{:03X}-{:012X}", (h >> 96) as u32, (h >> 80) as u16, (h >> 68) as u16 & 0xFFF, (h >> 56) as u16 & 0xFFF, h as u64 & 0xFFFF_FFFF_FFFF)
}

/// 이 기기에 맞춤: 화면마다 노치·폭에 맞게 곡률·노치를 다시 맞춘다. (Swift applyDeviceFitAll)
fn fit_all(app: &AppHandle) {
    let h = app.clone();
    let _ = app.run_on_main_thread(move || {
        let ds = um_platform::display::displays();
        change(&h, |st| {
            for d in &ds {
                let info = ScreenInfo { width_pt: d.width_pt, notch: d.notch.map(|n| um_core::settings::NotchInfo { width: n.width, height: n.height }) };
                if st.main_display_id.as_deref() == Some(d.id.as_str()) || st.main_display_id.is_none() {
                    device_fit(&mut st.main, &info);
                } else {
                    let s = st.displays.entry(d.id.clone()).or_insert_with(|| st.main.clone());
                    device_fit(s, &info);
                }
            }
            st.main.apply_platform_rules(cfg!(windows));
        });
    });
}

/// 업데이트 확인(별도 스레드). 끝나면 메뉴를 다시 만든다.
pub fn check_update(app: &AppHandle) {
    if cfg!(feature = "store") { return; }
    crate::shared(app).lock().unwrap().update = UpdateState::Checking;
    rebuild(app);
    let app = app.clone();
    std::thread::spawn(move || {
        let r = crate::update::check();
        if let UpdateState::Available { version, .. } = &r { crate::notifications::update(&app, version); }
        crate::shared(&app).lock().unwrap().update = r;
        crate::ipc::changed(&app);
    });
}

fn download(app: &AppHandle) {
    if cfg!(feature = "store") { return; }
    let (dl, upd) = {
        let st = crate::shared(app);
        let st = st.lock().unwrap();
        (st.download.clone(), st.update.clone())
    };
    match (dl, upd) {
        (DownloadState::Done(p), _) => crate::update::reveal(&p),
        (DownloadState::Failed, _) => {
            let _ = open::that(crate::update::PAGE);
        }
        (_, UpdateState::Available { asset: Some(url), .. }) => {
            crate::shared(app).lock().unwrap().download = DownloadState::Downloading;
            rebuild(app);
            let app = app.clone();
            std::thread::spawn(move || {
                let r = crate::update::download(&url);
                if let DownloadState::Done(p) = &r {
                    crate::update::reveal(p);
                }
                crate::shared(&app).lock().unwrap().download = r;
                rebuild(&app);
            });
        }
        // 새 버전 정보가 없으면 릴리스 페이지로.
        _ => {
            let _ = open::that(crate::update::PAGE);
        }
    }
}
