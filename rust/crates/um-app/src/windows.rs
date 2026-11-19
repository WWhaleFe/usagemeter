//! 설정·분석 창(Tauri 웹뷰). 열 때 만들고 닫으면 파기한다(계획서 4-3 ③, ADR 0001).
//! macOS는 창이 열려 있는 동안만 Dock 아이콘을 보인다(Swift DockPresence).
//! 화면 내용(Svelte)은 5·6단계에서 만든다. 지금은 `ui/index.html`이 자리만 채운다.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// `page` = "settings", "settings#interval", "analytics", "support" 등. 창은 종류마다 하나.
pub fn open(app: &AppHandle, page: &str) {
    #[cfg(windows)]
    {
        let app = app.clone();
        let page = page.to_string();
        // 메인 이벤트 루프를 막으면 WebView2 생성 콜백이 돌아오지 않는다.
        std::thread::spawn(move || open_inner(&app, &page));
    }
    #[cfg(not(windows))]
    open_inner(app, page);
}

fn open_inner(app: &AppHandle, page: &str) {
    let (kind, tab) = page.split_once('#').unwrap_or((page, ""));
    // 개발용: UM_OPEN_WINDOW=linedrag로 선 위치 끌기 화면을 바로 연다.
    if kind == "linedrag" {
        return open_line_drag(app);
    }
    // 두 플랫폼이 같은 논리 크기를 사용하며 Windows DPI 배율은 Tauri가 적용한다.
    let (label, w, h) = match kind {
        "analytics" => ("analytics", 1000.0, 720.0),
        _ => ("settings", 760.0, 880.0), // 설정·후원은 같은 창(후원 탭)
    };
    let tab = if kind == "support" { "support" } else { tab };
    let hash = if tab.is_empty() { label.to_string() } else { format!("{label}/{tab}") };
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.eval(format!("location.hash = '{hash}'"));
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let lang = crate::shared(app).lock().map(|s| s.main.language).unwrap_or_default();
    let title = um_core::i18n::tr(if label == "analytics" { "analytics.title" } else { "window.settings" }, lang);
    let url = WebviewUrl::App(format!("index.html#{hash}").into());
    let builder = WebviewWindowBuilder::new(app, label, url).title(title).inner_size(w, h).min_inner_size(640.0, 520.0).center();
    #[cfg(target_os = "macos")]
    let builder = builder
        .theme(mac_theme(app))
        // Visible은 Tauri 내부에서 FullSizeContentView를 켠다. 생성 시부터 끈 뒤 불투명 제목줄을 복원한다.
        .title_bar_style(tauri::TitleBarStyle::Transparent)
        .hidden_title(false)
        .visible(false)
        .resizable(false)
        .maximizable(false);
    // Mica가 지원되는 Windows 11에서만 투명 배경을 쓴다.
    #[cfg(windows)]
    let builder = if um_platform::mica_supported() {
        builder.transparent(true)
            .effects(tauri::window::EffectsBuilder::new().effect(tauri::window::Effect::Mica).build())
    } else {
        builder.transparent(false)
    };
    tracing::info!("창 생성 시작({label})");
    match builder.build() {
        Ok(win) => {
            tracing::info!("창 생성 완료({label})");
            #[cfg(target_os = "macos")]
            center_native_title(&win, w, h);
            dock_visible(app, true);
            let h = app.clone();
            win.on_window_event(move |e| {
                if let tauri::WindowEvent::Destroyed = e {
                    #[cfg(target_os = "macos")]
                    if label == "settings" {
                        crate::native_color::close();
                    }
                    // 다른 창(설정·분석)이 남아 있지 않으면 Dock 아이콘을 다시 숨긴다.
                    let left = ["settings", "analytics"].iter().filter(|l| h.get_webview_window(l).is_some()).count();
                    if left == 0 {
                        dock_visible(&h, false);
                    }
                }
            });
            let _ = win.set_focus();
        }
        Err(e) => tracing::warn!("창 열기 실패({label}): {e}"),
    }
}

/// AppKit의 독립된 제목 줄을 사용해 창 제목을 가운데 둔다.
#[cfg(target_os = "macos")]
fn center_native_title(win: &tauri::WebviewWindow, width: f64, height: f64) {
    let handle = win.clone();
    let _ = win.run_on_main_thread(move || {
        if let Ok(ptr) = handle.ns_window() {
            // Tauri가 소유한 유효한 NSWindow 포인터를 메인 스레드에서만 빌려 쓴다.
            let native = unsafe { &*ptr.cast::<objc2_app_kit::NSWindow>() };
            // Swift의 일반 titled 창과 같은 마스크와 독립된 제목줄을 창을 보이기 전에 적용한다.
            use objc2_app_kit::{NSWindowStyleMask as Mask, NSWindowTitleVisibility};
            native.setToolbar(None);
            native.setStyleMask(Mask::Titled | Mask::Closable | Mask::Miniaturizable);
            native.setTitleVisibility(NSWindowTitleVisibility::Visible);
            native.setTitlebarAppearsTransparent(false);
            native.setToolbarStyle(objc2_app_kit::NSWindowToolbarStyle::Expanded);
            native.setContentSize(objc2_foundation::NSSize::new(width, height));
            native.center();
        }
        let _ = handle.show();
        let _ = handle.set_focus();
    });
}

/// 언어 변경 시 이미 열린 창 제목도 Swift 분석 창처럼 다시 반영한다.
#[cfg(target_os = "macos")]
pub fn refresh_titles(app: &AppHandle) {
    let lang = crate::shared(app).lock().map(|s| s.main.language).unwrap_or_default();
    for (label, key) in [
        ("settings", "window.settings"),
        ("analytics", "analytics.title"),
    ] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.set_title(&um_core::i18n::tr(key, lang));
        }
    }
}

#[cfg(target_os = "macos")]
fn mac_theme(app: &AppHandle) -> Option<tauri::Theme> {
    let appearance = crate::shared(app).lock().map(|s| s.main.appearance).unwrap_or_default();
    match appearance {
        um_core::settings::AppAppearance::System => None,
        um_core::settings::AppAppearance::Light => Some(tauri::Theme::Light),
        um_core::settings::AppAppearance::Dark => Some(tauri::Theme::Dark),
    }
}

#[cfg(target_os = "macos")]
pub fn refresh_appearance(app: &AppHandle) {
    app.set_theme(mac_theme(app));
}

fn dock_visible(app: &AppHandle, on: bool) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(if on { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory });
    #[cfg(not(target_os = "macos"))]
    let _ = (app, on);
}

/// 선 위치 끌기 화면(Swift LineDragOverlay.swift): 편집 중인 화면 전체를 덮는 투명 창, 맨 위. macOS 전용.
/// 창 좌표는 논리 pt, 왼쪽 위 원점(주 화면 왼쪽 위가 (0,0)).
#[cfg(target_os = "macos")]
pub fn open_line_drag(app: &AppHandle) {
    // 이미 열려 있으면(다른 화면에서 열었을 수 있으니) 닫고 새로 연다(Swift show()의 hide()).
    if let Some(w) = app.get_webview_window("linedrag") {
        let _ = w.destroy();
    }
    let edit = crate::shared(app).lock().ok().and_then(|st| {
        let e = app.state::<crate::ipc::Editing>().0.lock().unwrap().clone();
        e.or_else(|| st.main_display_id.clone())
    });
    let ds = um_platform::display::displays();
    let Some(d) = ds.iter().find(|d| Some(&d.id) == edit.as_ref()).or(ds.first()) else { return };
    // 전역 AppKit 좌표(왼쪽 아래 원점)의 이 화면 위쪽 가장자리를 Tauri 좌표(왼쪽 위 원점)로 바꾼다.
    let (gx, top) = d.to_global(0.0, 0.0);
    let primary_h = ds.first().map_or(d.height_pt, |p| p.height_pt);
    let builder = WebviewWindowBuilder::new(app, "linedrag", WebviewUrl::App("index.html#linedrag".into()))
        .title("UsageMeter")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .resizable(false)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .position(gx, primary_h - top)
        .inner_size(d.width_pt, d.height_pt);
    match builder.build() {
        Ok(win) => {
            let _ = win.set_focus();
        }
        Err(e) => tracing::warn!("선 끌기 창 열기 실패: {e}"),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn open_line_drag(_app: &AppHandle) {}
