//! 설정·분석 창(Tauri 웹뷰). 열 때 만들고 닫으면 파기한다(계획서 4-3 ③, ADR 0001).
//! macOS는 창이 열려 있는 동안만 Dock 아이콘을 보인다(Swift DockPresence).
//! 화면 내용(Svelte)은 5·6단계에서 만든다. 지금은 `ui/index.html`이 자리만 채운다.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// `page` = "settings", "settings#interval", "analytics", "support" 등. 창은 종류마다 하나.
pub fn open(app: &AppHandle, page: &str) {
    let (kind, tab) = page.split_once('#').unwrap_or((page, ""));
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
    let title = um_core::i18n::tr(if label == "analytics" { "analytics.title" } else { "menu.settings" }, lang);
    let url = WebviewUrl::App(format!("index.html#{hash}").into());
    match WebviewWindowBuilder::new(app, label, url).title(title).inner_size(w, h).min_inner_size(640.0, 520.0).center().build() {
        Ok(win) => {
            dock_visible(app, true);
            let h = app.clone();
            win.on_window_event(move |e| {
                if let tauri::WindowEvent::Destroyed = e {
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

fn dock_visible(app: &AppHandle, on: bool) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(if on { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory });
    #[cfg(not(target_os = "macos"))]
    let _ = (app, on);
}
