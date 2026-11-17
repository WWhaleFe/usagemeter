//! 직접 호출 경로(계획서 4-3 ①, 0단계 S3): 웹뷰 없이 reqwest로 같은 API를 부른다.
//! 쿠키는 로그인 웹뷰 저장소에서 읽어 메모리 쿠키 통에만 넣는다(저장·기록하지 않음).
//! 응답 해석은 웹뷰 경로와 같은 `um_providers::direct` 함수를 쓰므로 결과 모양이 같다.

use std::sync::Arc;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::cookie::Jar;
use serde_json::{Value, json};
use tauri::{AppHandle, Url};
use um_providers::direct::{self as d, HttpRequest};
use um_providers::spec::ProviderSpec;

use crate::web;

/// 로그인 웹뷰 저장소의 쿠키로 클라이언트를 만든다(이때만 잠깐 숨은 웹뷰가 생긴다).
pub fn client(app: &AppHandle, spec: &'static ProviderSpec) -> Option<Client> {
    client_with_cookies(spec, web::read_cookies(app, spec), Duration::from_secs(15))
}

pub fn client_with_cookies(
    spec: &ProviderSpec,
    cookies: Vec<(String, String, String, String)>,
    timeout: Duration,
) -> Option<Client> {
    let jar = Arc::new(Jar::default());
    let home: Url = spec.home_url.parse().ok()?;
    for (name, value, domain, path) in cookies {
        let dom = domain.trim_start_matches('.');
        let base: Url = format!("https://{dom}/")
            .parse()
            .unwrap_or_else(|_| home.clone());
        jar.add_cookie_str(
            &format!("{name}={value}; Domain={domain}; Path={path}"),
            &base,
        );
    }
    Client::builder()
        .cookie_provider(jar)
        .user_agent(web::USER_AGENT)
        .timeout(timeout)
        .build()
        .ok()
}

/// (HTTP 상태, JSON 본문). 네트워크 오류면 상태 0.
fn send(c: &Client, r: &HttpRequest) -> (u16, Option<Value>) {
    let mut b = c.get(&r.url).header("Accept", "application/json");
    for (k, v) in &r.headers {
        b = b.header(k, v);
    }
    match b.send() {
        Ok(resp) => {
            let st = resp.status().as_u16();
            (st, resp.json::<Value>().ok())
        }
        Err(_) => (0, None),
    }
}

/// 웹뷰 없이 사용량을 읽는다(쿠키를 새로 읽음). S3 비교용.
pub fn probe(app: &AppHandle, spec: &'static ProviderSpec) -> Value {
    match client(app, spec) {
        Some(c) => probe_with(&c, spec),
        None => json!({"ok": false, "reason": "no_client"}),
    }
}

/// 만들어 둔 클라이언트로 사용량을 읽는다. 반환 모양은 사용량 JS 결과와 같다(`snapshot_from_probe` 입력).
pub fn probe_with(c: &Client, spec: &ProviderSpec) -> Value {
    match spec.id {
        "claude" => {
            let (st, body) = send(c, &d::claude_orgs_request());
            let org = match d::claude_org_from_orgs(st, body.as_ref()) {
                Ok(o) => o,
                Err(v) => return v,
            };
            let (st, body) = send(c, &d::claude_usage_request(&org));
            d::claude_probe_from_usage(&org, st, body.as_ref())
        }
        "codex" => {
            let (st, body) = send(c, &d::codex_session_request());
            let token = match d::codex_token_from_session(st, body.as_ref()) {
                Ok(t) => t,
                Err(v) => return v,
            };
            // 반환 순서는 (계정 ID, 플랜)이다.
            let (account, plan) = match token.as_deref() {
                Some(t) => {
                    let (st, body) = send(c, &d::codex_account_request(t));
                    d::codex_account_from_check(st, body.as_ref())
                }
                None => (None, None),
            };
            let (st, body) = send(
                c,
                &d::codex_usage_request(token.as_deref(), account.as_deref()),
            );
            d::codex_probe_from_usage(plan.as_deref(), st, body.as_ref(), chrono::Utc::now())
        }
        _ => json!({"ok": false, "reason": "unsupported"}),
    }
}
