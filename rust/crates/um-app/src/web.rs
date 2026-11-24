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
#[cfg(not(windows))]
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Safari/605.1.15";

/// WebView2와 직접 HTTP 조회는 같은 Windows Edge UA를 쓴다.
#[cfg(windows)]
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0";

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

fn login_redirect(url: &Url, spec: &ProviderSpec) -> bool {
    spec.id == "gemini" && url.host_str() == Some("accounts.google.com")
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
    // 회차마다 토큰 끝에 번호를 붙여 같은 결과 문자열이어도 제목 변경 이벤트가 다시 온다.
    let prefix = format!("UMRESULT:{token}-");
    let attempt = Arc::new(AtomicU64::new(0));
    let fired = Arc::new(AtomicBool::new(false));
    let fired_watch = fired.clone();
    // 페이지 로드가 한 번이라도 시작됐는지. 시작 전에 웹뷰를 건드리지 않는다(Windows 진단).
    let started = Arc::new(AtomicBool::new(false));
    let started_load = started.clone();
    let (token_load, attempt_load) = (token.clone(), attempt.clone());
    let delay = if spec.needs_rendered_dom {
        Duration::from_millis(1500)
    } else {
        Duration::ZERO
    };
    tracing::info!("조회 창 생성 시작");
    let auth_tx = tx.clone();
    // 진단: UM_SHOW_PROBE=1이면 조회 창을 보이게 띄운다.
    let show = std::env::var("UM_SHOW_PROBE").is_ok_and(|v| v == "1");
    // Windows는 빈 문서로 만든 뒤 navigate()하면 이동이 일어나지 않고 about:blank에 머물러 25초를 넘겼다(실기 로그).
    // 로그인 창처럼 처음부터 목적지 주소로 만든다.
    let start: Url = if cfg!(windows) { probe_start_url(spec) } else { "about:blank".parse().expect("blank") };
    // 진단: UM_PROBE_OPTS(쉼표 목록)로 조회 창 옵션을 하나씩 빼 본다. Windows에서 조회 창의 로드가 시작되지 않는 원인 찾기용.
    // focus=포커스 금지 안 함, taskbar=작업 표시줄 숨김 안 함, noscript=초기화 스크립트 없음, noua=사용자 에이전트 기본, activate=만든 뒤 활성화
    let opts = std::env::var("UM_PROBE_OPTS").unwrap_or_default();
    let opt = |k: &str| opts.split(',').any(|o| o.trim() == k);
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(start))
        .visible(show)
        .skip_taskbar(!opt("taskbar"))
        .inner_size(460.0, 720.0);
    if !opt("focus") { builder = builder.focused(false); }
    if !opt("noua") { builder = builder.user_agent(USER_AGENT); }
    if !opt("noscript") { builder = builder.initialization_script_for_all_frames(PASSKEY_OFF); }
    if !opts.is_empty() { tracing::info!("조회 창 진단 옵션: {opts}"); }
    let built = builder
        .on_page_load(move |w, p| {
            if p.event() == PageLoadEvent::Started { started_load.store(true, Ordering::SeqCst); }
            // 구글은 로그인된 상태에서도 accounts.google.com을 잠깐 거쳐 돌아올 수 있어, 그 페이지에 머물렀을 때만 판정한다.
            if p.event() == PageLoadEvent::Finished && login_redirect(p.url(), spec) {
                let _ = auth_tx.send(json!({"ok": false, "reason": "not_logged_in"}).to_string());
                return;
            }
            if p.event() == PageLoadEvent::Finished
                && host_matches(p.url(), spec)
                && !fired.swap(true, Ordering::SeqCst)
            {
                let (token, attempt) = (token_load.clone(), attempt_load.clone());
                std::thread::spawn(move || {
                    std::thread::sleep(delay);
                    eval_attempt(&w, spec, &token, &attempt);
                });
            }
        })
        .on_document_title_changed(move |_, t| {
            if let Some((n, raw)) = t.strip_prefix(&prefix).and_then(|rest| rest.split_once(':')) {
                if spec.needs_rendered_dom {
                    let reason = serde_json::from_str::<Value>(raw).ok().and_then(|v| v["reason"].as_str().map(String::from)).unwrap_or_default();
                    tracing::info!("조회 결과 {n}회차 받음 {reason}");
                }
                let _ = tx.send(raw.to_string());
            }
        })
        .build();
    let win = match built {
        Ok(w) => {
            tracing::info!("조회 창 생성 완료");
            if opt("activate") { activate(&w); }
            w
        }
        Err(_) => {
            tracing::warn!("조회 창 생성 실패");
            return json!({"ok": false, "reason": "no_webview"});
        }
    };
    // 먼저 같은 웹뷰에서 쿠키를 확인한다. 미로그인이면 네트워크 탐색도 시작하지 않고 닫는다.
    // Windows는 만든 직후(웹뷰 준비 전) 쿠키를 물으면 그 웹뷰가 빈 문서에서 멈춰 25초를 넘겼다(실기).
    // 미로그인이면 구글 로그인 페이지로 넘어가 판정되므로 Windows에서는 사전 확인을 하지 않는다.
    let failure = if cfg!(windows) { None } else { match cookies_for(&win, spec) {
        Ok(cookies) if cookies.is_empty() => Some(json!({"ok": false, "reason": "not_logged_in", "no_cookies": true})),
        Ok(_) => None,
        Err(()) => Some(json!({"ok": false, "reason": "cookie_read"})),
    } };
    if let Some(failure) = failure {
        let _ = win.destroy();
        return failure;
    }
    if cfg!(windows) {
        // 이미 목적지 주소로 만들었다.
    } else if cfg!(target_os = "macos") && !spec.needs_rendered_dom {
        load_stub(&win, spec);
    } else if win.navigate(probe_start_url(spec)).is_err() {
        let _ = win.destroy();
        return json!({"ok": false, "reason": "navigation"});
    }
    let done = Arc::new(AtomicBool::new(false));
    if spec.needs_rendered_dom {
        // Gemini는 리소스가 많아 로드 완료(Finished)가 15~25초 뒤에 오거나 안 와서 25초 제한을 넘겼다.
        // 주소가 서비스에 닿고 4초 뒤부터 직접 읽고, 화면을 못 읽었으면 8초마다 다시 읽는다.
        let (w, done, token, attempt) = (win.clone(), done.clone(), token.clone(), attempt.clone());
        std::thread::spawn(move || {
            let mut on_host = 0u32;
            let mut last = String::new();
            let mut waited = 0u32;
            while !done.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_secs(1));
                if !started.load(Ordering::SeqCst) {
                    waited += 1;
                    if waited == 10 { tracing::info!("조회 창 페이지 로드가 10초째 시작되지 않음"); }
                    continue;
                }
                let Ok(url) = w.url() else { tracing::info!("조회 창 주소 읽기 실패"); break };
                // 주소는 호스트와 경로만 남긴다(쿼리 없음).
                let at = format!("{}{}", url.host_str().unwrap_or("-"), url.path());
                if at != last { tracing::info!("조회 창 주소: {at}"); last = at; }
                on_host = if host_matches(&url, spec) { on_host + 1 } else { 0 };
                if on_host >= 4 && (on_host - 4).is_multiple_of(8) && !done.load(Ordering::SeqCst) {
                    fired_watch.store(true, Ordering::SeqCst);
                    eval_attempt(&w, spec, &token, &attempt);
                }
            }
        });
    }
    let deadline = std::time::Instant::now() + timeout;
    let timed_out = || json!({"ok": false, "reason": "timeout"});
    // 화면을 못 읽은 결과(parse_failed)는 페이지가 덜 그려졌을 수 있어 다음 회차를 기다리고, 끝까지 없으면 그것을 쓴다.
    let mut fallback: Option<Value> = None;
    let out = loop {
        let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) else {
            break fallback.unwrap_or_else(timed_out);
        };
        let Ok(s) = rx.recv_timeout(left) else { break fallback.unwrap_or_else(timed_out) };
        let v: Value = serde_json::from_str(&s).unwrap_or_else(|_| json!({"ok": false, "reason": "bad_result"}));
        if spec.needs_rendered_dom && v["reason"] == "parse_failed" {
            fallback = Some(v);
            continue;
        }
        break v;
    };
    done.store(true, Ordering::SeqCst);
    let _ = win.destroy();
    out
}

/// 사용량 JS를 새 회차 번호로 실행한다(결과 제목: `UMRESULT:<토큰>-<회차>:<JSON>`).
fn eval_attempt(w: &WebviewWindow, spec: &ProviderSpec, token: &str, attempt: &AtomicU64) {
    let n = attempt.fetch_add(1, Ordering::SeqCst);
    let ok = w.eval(wrap(spec.usage_js, &format!("{token}-{n}"))).is_ok();
    if spec.needs_rendered_dom { tracing::info!("조회 JS {n}회차 실행 ok={ok}"); }
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

/// 로그인 창을 띄우고 로그인이 끝날 때까지 기다린다(창을 닫으면 None, 끝나면 그 창에서 읽은 사용량). 로그인 판정(Swift LoginWindowController.swift):
/// Gemini는 서비스 호스트에 있고 인증 쿠키가 있으면, 나머지는 서비스 호스트에서 사용량 JS가 ok를 주면.
pub fn login(app: &AppHandle, spec: &'static ProviderSpec, title: &str) -> Option<Option<Value>> {
    let label = format!("login-{}", spec.id);
    if let Some(w) = app.get_webview_window(&label) {
        activate(&w);
        return None;
    }
    let (tx, rx) = mpsc::channel::<String>();
    let closed_tx = tx.clone();
    let app2 = app.clone();
    let popups = Arc::new(LoginPopups::default());
    let children = popups.clone();
    tracing::info!("로그인 창 생성 시작");
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
    let Ok(win) = built else { tracing::warn!("로그인 창 생성 실패"); return None };
    tracing::info!("로그인 창 생성 완료");
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
        std::thread::sleep(Duration::from_millis(600));
        if popups.closed.load(Ordering::SeqCst) { break None; }
        let Some(w) = app.get_webview_window(&label) else {
            break None;
        };
        let Ok(url) = w.url() else { continue };
        if !host_matches(&url, spec) {
            continue;
        }
        if spec.auth_cookie_names.is_empty() {
            if let Some(raw) = eval_usage(&w, spec, &rx, &popups.closed) {
                let _ = w.destroy();
                break Some(Some(raw));
            }
        } else if has_auth_cookie(&w, spec) {
            // 로그인은 끝났다. 창을 먼저 숨기고, 이미 열린 사용량 화면에서 바로 읽어 숨은 조회 창을 새로 띄우지 않는다.
            let _ = w.hide();
            let raw = eval_usage(&w, spec, &rx, &popups.closed);
            let _ = w.destroy();
            break Some(raw);
        }
    };
    close_popups(app, &popups);
    result
}

/// 로그인 창에서 사용량을 읽는다. 성공(ok=true)한 결과만 돌려준다.
fn eval_usage(w: &WebviewWindow, spec: &ProviderSpec, rx: &mpsc::Receiver<String>, closed: &AtomicBool) -> Option<Value> {
    let token = next_token();
    let prefix = format!("UMRESULT:{token}:");
    while rx.try_recv().is_ok() {}
    let _ = w.eval(wrap(spec.usage_js, &token));
    login_result(rx, &prefix, closed, Duration::from_secs(8))
}

/// 늦게 도착한 이전 회차 결과는 현재 로그인 판정에 쓰지 않는다.
fn login_result(rx: &mpsc::Receiver<String>, prefix: &str, closed: &AtomicBool, timeout: Duration) -> Option<Value> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if closed.load(Ordering::SeqCst) { return None; }
        let left = deadline.checked_duration_since(std::time::Instant::now())?;
        let title = rx.recv_timeout(left).ok()?;
        if let Some(raw) = title.strip_prefix(prefix) {
            return serde_json::from_str::<Value>(raw).ok().filter(|v| v["ok"] == true);
        }
    }
}

/// LSUIElement 앱도 다른 앱 앞으로 활성화한다(Swift WebSession.swift:151-152).
fn activate(win: &WebviewWindow) {
    // Windows는 백그라운드 앱의 새 창을 다른 창 뒤에 둘 수 있어 잠깐 맨 위로 올렸다 푼다.
    #[cfg(windows)]
    {
        let _ = win.set_always_on_top(true);
        let _ = win.set_always_on_top(false);
    }
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

#[cfg(not(windows))]
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

// Windows: 콜백 안에서 창을 만들면 교착된다. WebView2 기본 팝업에 맡기면 같은 프로필·쿠키를 쓰고 window.opener도 유지된다.
#[cfg(windows)]
fn popup(
    _: &AppHandle,
    _: Url,
    _: tauri::webview::NewWindowFeatures,
    _: Arc<LoginPopups>,
) -> tauri::webview::NewWindowResponse<tauri::Wry> {
    tracing::info!("로그인 팝업은 WebView2 기본 창으로 연다");
    tauri::webview::NewWindowResponse::Allow
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
    tracing::info!("쿠키 저장소 창 생성 시작");
    let Ok(w) = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External("about:blank".parse().expect("blank")),
    )
    .visible(false)
    .focused(false)
    .user_agent(USER_AGENT)
    .skip_taskbar(true)
    .build() else {
        tracing::warn!("쿠키 저장소 창 생성 실패");
        return;
    };
    tracing::info!("쿠키 저장소 창 생성 완료");
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

pub type Cookies = Vec<(String, String, String, String)>;

/// 서비스 도메인의 쿠키를 (이름, 값, 도메인, 경로)로 읽는다. 직접 호출 경로(`direct.rs`)용.
/// 값은 메모리에서만 쓰고 기록하지 않는다. 저장소 오류는 빈 쿠키와 구분한다.
pub fn read_cookies(
    app: &AppHandle,
    spec: &'static ProviderSpec,
) -> Result<Cookies, ()> {
    // Windows: 숨은 웹뷰 하나를 계속 살려 두고 쓴다. 매번 만들고 닫으면 마지막 웹뷰가 닫힐 때 WebView2 브라우저
    // 프로세스까지 내려가, 다음 조회·로그인·설정 창이 매번 콜드 스타트를 겪었다(Microsoft WebView2 성능 권장: 재사용).
    const STORE: &str = "cookies-store";
    let keep = cfg!(windows);
    let label = if keep { STORE.to_string() } else { format!("cookies-{}-{}", spec.id, next_token().replace('x', "-")) };
    let existing = if keep { app.get_webview_window(&label) } else { None };
    let w = match existing {
        Some(w) => w,
        None => {
            tracing::info!("쿠키 저장소 창 생성 시작");
            let built = WebviewWindowBuilder::new(
                app,
                &label,
                WebviewUrl::External("about:blank".parse().expect("blank")),
            )
            .visible(false)
            .focused(false)
            .user_agent(USER_AGENT)
            .skip_taskbar(true)
            .build();
            // 동시에 두 조회가 같은 창을 만들려 하면 늦은 쪽은 먼저 만든 창을 쓴다.
            match built.ok().or_else(|| keep.then(|| app.get_webview_window(&label)).flatten()) {
                Some(w) => { tracing::info!("쿠키 저장소 창 생성 완료"); low_memory(&w); w }
                None => { tracing::warn!("쿠키 저장소 창 생성 실패"); return Err(()); }
            }
        }
    };
    let out = cookies_for(&w, spec);
    match &out {
        Ok(cookies) => tracing::info!("쿠키 읽기 완료: {}개", cookies.len()),
        Err(()) => tracing::warn!("쿠키 읽기 실패"),
    }
    if !keep { let _ = w.destroy(); }
    out
}

/// Windows: 계속 살려 두는 숨은 웹뷰는 WebView2에 메모리를 적게 쓰라고 알린다(MemoryUsageTargetLevel = Low).
/// 보이지 않는 about:blank 창이라 동작은 그대로이고, 렌더러가 캐시·작업 메모리를 덜 잡는다.
#[cfg(windows)]
fn low_memory(w: &WebviewWindow) {
    let _ = w.with_webview(|pw| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, ICoreWebView2_19};
        use windows_core::Interface;
        let Ok(core) = pw.controller().CoreWebView2() else { return };
        match core.cast::<ICoreWebView2_19>() {
            Ok(v) => { let _ = v.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW); }
            Err(_) => tracing::info!("WebView2가 낮은 메모리 목표를 지원하지 않음(런타임이 오래됨)"),
        }
    });
}

#[cfg(not(windows))]
fn low_memory(_: &WebviewWindow) {}

fn cookies_for(w: &WebviewWindow, spec: &ProviderSpec) -> Result<Cookies, ()> {
    w
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
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 구글_인증_리다이렉트는_로그인_필요() {
        let gemini = um_providers::spec::spec("gemini").unwrap();
        let codex = um_providers::spec::spec("codex").unwrap();
        let auth = Url::parse("https://accounts.google.com/ServiceLogin").unwrap();
        assert!(login_redirect(&auth, gemini));
        assert!(!login_redirect(&auth, codex));
        assert!(!login_redirect(&Url::parse("https://accounts.google.com.example.org/").unwrap(), gemini));
        assert!(!login_redirect(&Url::parse(gemini.home_url).unwrap(), gemini));
    }

    #[test]
    fn 이전_회차_성공과_실패를_무시한다() {
        for ok in [false, true] {
            let (tx, rx) = mpsc::channel();
            tx.send(format!("UMRESULT:old:{{\"ok\":{}}}", !ok)).unwrap();
            tx.send(format!("UMRESULT:current:{{\"ok\":{ok}}}")).unwrap();
            assert_eq!(login_result(&rx, "UMRESULT:current:", &AtomicBool::new(false), Duration::from_millis(20)).is_some(), ok);
        }
    }

    #[test]
    fn 닫힌_로그인과_시간초과는_실패() {
        let (tx, rx) = mpsc::channel();
        tx.send("UMRESULT:current:{\"ok\":true}".into()).unwrap();
        assert!(login_result(&rx, "UMRESULT:current:", &AtomicBool::new(true), Duration::from_millis(20)).is_none());
        let (_tx, rx) = mpsc::channel();
        assert!(login_result(&rx, "UMRESULT:current:", &AtomicBool::new(false), Duration::from_millis(1)).is_none());
    }
}
