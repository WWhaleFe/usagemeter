//! 웹뷰 계층: 사용량 조회용 임시 숨은 웹뷰, 로그인 창, 로그아웃(계획서 5-1, 4-3 ①).
//!
//! - 조회 결과는 IPC 대신 `document.title`로 돌려받는다(`UMRESULT:<토큰>:<JSON>`).
//!   서비스 페이지에 앱 명령 권한을 열지 않기 위해서다.
//! - 조회용 웹뷰는 조회할 때만 만들고 결과를 받으면 바로 파기한다(평상시 웹뷰 0개).
//! - macOS는 서비스 오리진의 빈 문서를 네트워크 없이 띄운다(Swift `loadSimulatedRequest`, WebSession.swift:126-132).
//!   Windows는 같은 효과의 `WebResourceRequested` 처리를 아직 안 만들어, 같은 오리진의 가벼운 `robots.txt`를 띄운다(TODO).
//!
//! 이 모듈의 함수는 메인 스레드가 아닌 곳에서 부른다(결과를 기다리며 막히므로).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use serde_json::{Value, json};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use um_providers::spec::ProviderSpec;

/// Swift WebSession.swift:37-38과 같은 데스크톱 Safari UA.
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Safari/605.1.15";

/// 패스키(WebAuthn) 끄기: 내장 웹뷰는 패스키를 못 써서 비밀번호 등으로 넘어가게 한다. (WebSession.swift:42-55)
pub const PASSKEY_OFF: &str = r#"(function () {
  try { Object.defineProperty(window, 'PublicKeyCredential', { value: undefined, configurable: true }); } catch (e) {}
  try {
    if (navigator.credentials) {
      navigator.credentials.get = function () { return Promise.reject(new DOMException('disabled', 'NotAllowedError')); };
      navigator.credentials.create = function () { return Promise.reject(new DOMException('disabled', 'NotAllowedError')); };
    }
  } catch (e) {}
})();"#;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn next_token() -> String {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{n}x{t:x}")
}

/// 사용량 JS(Swift `callAsyncJavaScript` 본문)를 async 함수로 감싸 결과를 제목으로 보낸다.
fn wrap(usage_js: &str, token: &str) -> String {
    format!(
        "(async () => {{ let r; try {{ r = await (async () => {{\n{usage_js}\n}})(); }} catch (e) {{ r = {{ok:false, reason:'exception', message:String(e)}}; }}\n\
         if (r === null || typeof r !== 'object') r = {{ok:false, reason:'bad_result'}};\n\
         document.title = 'UMRESULT:{token}:' + JSON.stringify(r); }})();"
    )
}

fn host_matches(url: &Url, spec: &ProviderSpec) -> bool {
    url.host_str().is_some_and(|h| h.contains(spec.match_host))
}

/// 조회용 첫 주소. 렌더된 화면이 필요한 서비스(Gemini)는 홈, 나머지는 빈 문서(macOS) 또는 robots.txt.
fn probe_start_url(spec: &ProviderSpec) -> Url {
    let home: Url = spec.home_url.parse().expect("home url");
    if spec.needs_rendered_dom || cfg!(target_os = "macos") {
        // macOS 빈 문서는 about:blank로 만든 뒤 `load_stub`이 오리진 문서로 바꾼다.
        if spec.needs_rendered_dom {
            home
        } else {
            "about:blank".parse().expect("blank")
        }
    } else {
        let mut u = home;
        u.set_path("/robots.txt");
        u
    }
}

/// macOS: 서비스 오리진의 빈 문서를 네트워크 없이 띄운다.
#[cfg(target_os = "macos")]
fn load_stub(win: &WebviewWindow, spec: &'static ProviderSpec) {
    let home = spec.home_url.to_string();
    let _ = win.with_webview(move |pw| unsafe {
        use objc2_foundation::{NSString, NSURL, NSURLRequest};
        use objc2_web_kit::WKWebView;
        let wk = &*(pw.inner() as *const WKWebView);
        let Some(url) = NSURL::URLWithString(&NSString::from_str(&home)) else {
            return;
        };
        let req = NSURLRequest::requestWithURL(&url);
        let html =
            NSString::from_str("<!doctype html><meta charset=\"utf-8\"><title>UsageMeter</title>");
        let _ = wk.loadSimulatedRequest_responseHTMLString(&req, &html);
    });
}

#[cfg(not(target_os = "macos"))]
fn load_stub(_: &WebviewWindow, _: &'static ProviderSpec) {}

/// 숨은 임시 웹뷰로 사용량을 한 번 읽는다. 반환 = JS 결과(`{ok, …}`). 시간 초과면 `{ok:false, reason:"timeout"}`.
pub fn probe(app: &AppHandle, spec: &'static ProviderSpec, timeout: Duration) -> Value {
    let token = next_token();
    let label = format!("probe-{}-{}", spec.id, token.replace('x', "-"));
    let (tx, rx) = mpsc::channel::<String>();
    let js = wrap(spec.usage_js, &token);
    let prefix = format!("UMRESULT:{token}:");
    let fired = Arc::new(AtomicBool::new(false));
    let delay = if spec.needs_rendered_dom {
        Duration::from_millis(1500)
    } else {
        Duration::ZERO
    };
    let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(probe_start_url(spec)))
        .visible(false)
        .skip_taskbar(true)
        .focused(false)
        .inner_size(460.0, 720.0)
        .user_agent(USER_AGENT)
        .initialization_script_for_all_frames(PASSKEY_OFF)
        .on_page_load(move |w, p| {
            if p.event() == PageLoadEvent::Finished
                && host_matches(p.url(), spec)
                && !fired.swap(true, Ordering::SeqCst)
            {
                let js = js.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(delay);
                    let _ = w.eval(js);
                });
            }
        })
        .on_document_title_changed(move |_, t| {
            if let Some(rest) = t.strip_prefix(&prefix) {
                let _ = tx.send(rest.to_string());
            }
        })
        .build();
    let win = match built {
        Ok(w) => w,
        Err(e) => return json!({"ok": false, "reason": "no_webview", "message": e.to_string()}),
    };
    if !spec.needs_rendered_dom {
        load_stub(&win, spec);
    }
    let out = match rx.recv_timeout(timeout) {
        Ok(s) => serde_json::from_str(&s)
            .unwrap_or_else(|_| json!({"ok": false, "reason": "bad_result"})),
        Err(_) => json!({"ok": false, "reason": "timeout"}),
    };
    let _ = win.destroy();
    out
}

#[derive(Default)]
struct LoginPopups {
    closed: AtomicBool,
    labels: Mutex<Vec<String>>,
}

fn close_popups(app: &AppHandle, popups: &LoginPopups) {
    popups.closed.store(true, Ordering::SeqCst);
    let labels = std::mem::take(&mut *popups.labels.lock().unwrap());
    for label in labels {
        if let Some(w) = app.get_webview_window(&label) { let _ = w.destroy(); }
    }
}

/// 로그인 창을 띄우고 로그인이 끝날 때까지 기다린다(창을 닫으면 false). 로그인 판정(Swift LoginWindowController.swift):
/// Gemini는 서비스 호스트에 있고 인증 쿠키가 있으면, 나머지는 서비스 호스트에서 사용량 JS가 ok를 주면.
pub fn login(app: &AppHandle, spec: &'static ProviderSpec, title: &str) -> bool {
    let label = format!("login-{}", spec.id);
    if let Some(w) = app.get_webview_window(&label) {
        activate(&w);
        return false;
    }
    let (tx, rx) = mpsc::channel::<String>();
    let closed_tx = tx.clone();
    let app2 = app.clone();
    let popups = Arc::new(LoginPopups::default());
    let children = popups.clone();
    let built = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(spec.effective_login_url().parse().expect("login url")),
    )
    .title(title)
    .inner_size(460.0, 720.0)
    .center()
    .user_agent(USER_AGENT)
    .initialization_script_for_all_frames(PASSKEY_OFF)
    .on_document_title_changed(move |_, t| {
        if t.starts_with("UMRESULT:") {
            let _ = tx.send(t.to_string());
        }
    })
    // 중첩 팝업도 같은 로그인에 묶어 닫는다(Swift PopupWebView.swift:18-52).
    .on_new_window(move |url, features| popup(&app2, url, features, children.clone()))
    .build();
    let Ok(win) = built else { return false };
    let h = app.clone();
    let children = popups.clone();
    win.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            close_popups(&h, &children);
            let _ = closed_tx.send(String::new());
        }
    });
    activate(&win);
    let result = loop {
        std::thread::sleep(Duration::from_millis(1200));
        if popups.closed.load(Ordering::SeqCst) { break false; }
        let Some(w) = app.get_webview_window(&label) else {
            break false;
        };
        let Ok(url) = w.url() else { continue };
        if !host_matches(&url, spec) {
            continue;
        }
        let done = if spec.auth_cookie_names.is_empty() {
            let token = next_token();
            let prefix = format!("UMRESULT:{token}:");
            while rx.try_recv().is_ok() {}
            let _ = w.eval(wrap(spec.usage_js, &token));
            login_result(&rx, &prefix, &popups.closed, Duration::from_secs(8))
        } else {
            has_auth_cookie(&w, spec)
        };
        if done {
            let _ = w.destroy();
            break true;
        }
    };
    close_popups(app, &popups);
    result
}

/// 늦게 도착한 이전 회차 결과는 현재 로그인 판정에 쓰지 않는다.
fn login_result(rx: &mpsc::Receiver<String>, prefix: &str, closed: &AtomicBool, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if closed.load(Ordering::SeqCst) { return false; }
        let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) else { return false };
        let Ok(title) = rx.recv_timeout(left) else { return false };
        if let Some(raw) = title.strip_prefix(prefix) {
            return serde_json::from_str::<Value>(raw).is_ok_and(|v| v["ok"] == true);
        }
    }
}

/// LSUIElement 앱도 다른 앱 앞으로 활성화한다(Swift WebSession.swift:151-152).
fn activate(win: &WebviewWindow) {
    let _ = win.set_focus();
    #[cfg(target_os = "macos")]
    {
        let w = win.clone();
        let _ = win.app_handle().run_on_main_thread(move || {
            let Some(mtm) = objc2::MainThreadMarker::new() else { return };
            let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
            if let Ok(ptr) = w.ns_window() {
                let window = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
                window.makeKeyAndOrderFront(None);
            }
        });
    }
}

fn popup(
    app: &AppHandle,
    url: Url,
    features: tauri::webview::NewWindowFeatures,
    popups: Arc<LoginPopups>,
) -> tauri::webview::NewWindowResponse<tauri::Wry> {
    if popups.closed.load(Ordering::SeqCst) { return tauri::webview::NewWindowResponse::Deny; }
    let label = format!("popup-{}", next_token());
    let h = app.clone();
    let children = popups.clone();
    match WebviewWindowBuilder::new(app, &label, WebviewUrl::External("about:blank".parse().expect("blank")))
        .window_features(features)
        .user_agent(USER_AGENT)
        .initialization_script_for_all_frames(PASSKEY_OFF)
        .inner_size(480.0, 680.0)
        .center()
        .title(url.host_str().unwrap_or("UsageMeter"))
        .on_new_window(move |url, features| popup(&h, url, features, children.clone()))
        .build()
    {
        Ok(window) => {
            let mut labels = popups.labels.lock().unwrap();
            if popups.closed.load(Ordering::SeqCst) {
                drop(labels);
                let _ = window.destroy();
                return tauri::webview::NewWindowResponse::Deny;
            }
            labels.push(label);
            drop(labels);
            activate(&window);
            tauri::webview::NewWindowResponse::Create { window }
        }
        Err(_) => tauri::webview::NewWindowResponse::Deny,
    }
}

fn has_auth_cookie(w: &WebviewWindow, spec: &ProviderSpec) -> bool {
    w.cookies()
        .map(|cs| {
            cs.iter().any(|c| {
                spec.auth_cookie_names.contains(&c.name())
                    && c.domain()
                        .is_some_and(|d| spec.cookie_domains.iter().any(|f| d.contains(f)))
            })
        })
        .unwrap_or(false)
}

/// 로그아웃: 도메인 조각에 해당하는 쿠키를 지운다(Swift WebSession.swift:380). 쿠키 저장소에 닿으려고 임시 웹뷰를 만든다.
pub fn logout(app: &AppHandle, spec: &'static ProviderSpec) {
    let label = format!("logout-{}-{}", spec.id, next_token().replace('x', "-"));
    let Ok(w) = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External("about:blank".parse().expect("blank")),
    )
    .visible(false)
    .skip_taskbar(true)
    .build() else {
        return;
    };
    if let Ok(cs) = w.cookies() {
        for c in cs {
            if c.domain()
                .is_some_and(|d| spec.cookie_domains.iter().any(|f| d.contains(f)))
            {
                let _ = w.delete_cookie(c);
            }
        }
    }
    let _ = w.destroy();
}

/// 서비스 도메인의 쿠키를 (이름, 값, 도메인, 경로)로 읽는다. 직접 호출 경로(`direct.rs`)용.
/// 값은 메모리에서만 쓰고 기록하지 않는다.
pub fn read_cookies(
    app: &AppHandle,
    spec: &'static ProviderSpec,
) -> Vec<(String, String, String, String)> {
    let label = format!("cookies-{}-{}", spec.id, next_token().replace('x', "-"));
    let Ok(w) = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External("about:blank".parse().expect("blank")),
    )
    .visible(false)
    .skip_taskbar(true)
    .build() else {
        return vec![];
    };
    let out = w
        .cookies()
        .map(|cs| {
            cs.into_iter()
                .filter(|c| {
                    c.domain()
                        .is_some_and(|d| spec.cookie_domains.iter().any(|f| d.contains(f)))
                })
                .map(|c| {
                    (
                        c.name().to_string(),
                        c.value().to_string(),
                        c.domain().unwrap_or("").to_string(),
                        c.path().unwrap_or("/").to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let _ = w.destroy();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 이전_회차_성공과_실패를_무시한다() {
        for ok in [false, true] {
            let (tx, rx) = mpsc::channel();
            tx.send(format!("UMRESULT:old:{{\"ok\":{}}}", !ok)).unwrap();
            tx.send(format!("UMRESULT:current:{{\"ok\":{ok}}}")).unwrap();
            assert_eq!(login_result(&rx, "UMRESULT:current:", &AtomicBool::new(false), Duration::from_millis(20)), ok);
        }
    }

    #[test]
    fn 닫힌_로그인과_시간초과는_실패() {
        let (tx, rx) = mpsc::channel();
        tx.send("UMRESULT:current:{\"ok\":true}".into()).unwrap();
        assert!(!login_result(&rx, "UMRESULT:current:", &AtomicBool::new(true), Duration::from_millis(20)));
        let (_tx, rx) = mpsc::channel();
        assert!(!login_result(&rx, "UMRESULT:current:", &AtomicBool::new(false), Duration::from_millis(1)));
    }
}
