//! 웹뷰 없는 직접 호출 경로의 순수 부분. (계획서 4-3 ①, Swift Providers.swift의 usageJS를 Rust로 옮김)
//!
//! 네트워크는 하지 않는다. 호출자가 `*_request`가 기술한 요청을 보내고, 받은 상태 코드와 JSON 본문을
//! 아래 해석 함수에 넘기면 JS가 돌려줄 `raw` 값과 같은 모양을 만든다. 그래서 웹뷰 경로와
//! `parse::snapshot_from_probe`를 공유한다. JS의 `try/catch` 예외 경로(`exception`)는 호출자 몫이다.

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{Map, Value, json};

/// 보낼 HTTP 요청 기술. 쿠키(`credentials:'include'`에 해당)는 호출자가 붙인다.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
}

fn get(url: impl Into<String>, mut headers: Vec<(String, String)>) -> HttpRequest {
    headers.insert(0, ("Accept".into(), "application/json".into()));
    HttpRequest { method: "GET", url: url.into(), headers }
}

fn fail(reason: &str) -> Value {
    json!({"ok": false, "reason": reason})
}

fn fail_status(reason: &str, status: u16) -> Value {
    json!({"ok": false, "reason": reason, "status": status})
}

/// JS 진리값: null·false·0·"" 는 거짓.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// JS `a || b`.
fn or<'a>(a: Option<&'a Value>, b: Option<&'a Value>) -> Option<&'a Value> {
    match a {
        Some(v) if truthy(v) => Some(v),
        _ => b.filter(|v| truthy(v)),
    }
}

fn str_of(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// 유한한 숫자만(JS `num`).
fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|f| f.is_finite())
}

// ---------------------------------------------------------------- Claude

/// Claude 1단계 `GET /api/organizations`.
pub fn claude_orgs_request() -> HttpRequest {
    get("https://claude.ai/api/organizations", vec![])
}

/// 조직 정보(첫 번째 조직).
#[derive(Debug, Clone, PartialEq)]
pub struct ClaudeOrg {
    pub uuid: String,
    /// 사람이 읽는 플랜 라벨. rate_limit_tier가 비면 None.
    pub plan: Option<String>,
}

/// `rate_limit_tier` -> 플랜 라벨. (Providers.swift planLabel 함수, 순서 중요)
pub fn claude_plan_label(tier: &str) -> Option<String> {
    if tier.is_empty() {
        return None;
    }
    let label = if tier.contains("max_20x") {
        "Max (20x)".to_string()
    } else if tier.contains("max_5x") {
        "Max (5x)".to_string()
    } else if tier.contains("max") {
        "Max".into()
    } else if tier.contains("team") {
        "Team".into()
    } else if tier.contains("enterprise") {
        "Enterprise".into()
    } else if tier.contains("pro") {
        "Pro".into()
    } else if tier.contains("free") {
        "Free".into()
    } else {
        tier.strip_prefix("default_claude_").unwrap_or(tier).replace('_', " ")
    };
    Some(label)
}

/// 1단계 응답 해석. 실패면 JS가 돌려줄 실패 `raw`를 Err로.
/// 200이 아니면 not_logged_in, 배열이 아니거나 비면 no_org.
pub fn claude_org_from_orgs(status: u16, body: Option<&Value>) -> Result<ClaudeOrg, Value> {
    if status != 200 {
        return Err(fail_status("not_logged_in", status));
    }
    let org = body.and_then(Value::as_array).and_then(|a| a.first()).ok_or_else(|| fail("no_org"))?;
    // uuid가 없으면 JS는 ".../undefined/usage"로 호출하게 되는데, 의미 없는 호출이라 no_org로 본다(보수적 해석).
    let uuid = str_of(org.get("uuid")).ok_or_else(|| fail("no_org"))?.to_string();
    let tier = org.get("rate_limit_tier").and_then(Value::as_str).unwrap_or("");
    Ok(ClaudeOrg { uuid, plan: claude_plan_label(tier) })
}

/// Claude 2단계 `GET /api/organizations/{uuid}/usage`.
pub fn claude_usage_request(org: &ClaudeOrg) -> HttpRequest {
    get(format!("https://claude.ai/api/organizations/{}/usage", org.uuid), vec![])
}

/// JS `pick`: utilization이 숫자인 객체만 `{utilization, resets_at||null}`.
fn pick_window(x: Option<&Value>) -> Value {
    let Some(x) = x else { return Value::Null };
    match x.get("utilization").filter(|u| u.is_number()) {
        Some(u) if x.is_object() => {
            let resets = or(x.get("resets_at"), None).cloned().unwrap_or(Value::Null);
            json!({"utilization": u, "resets_at": resets})
        }
        _ => Value::Null,
    }
}

/// 2단계 응답 -> `raw`. limits 중 kind=weekly_scoped(모델 이름·percent 있는 것)를 model_buckets로.
pub fn claude_probe_from_usage(org: &ClaudeOrg, status: u16, body: Option<&Value>) -> Value {
    if status != 200 {
        return fail_status("usage_failed", status);
    }
    let null = Value::Null;
    let u = body.unwrap_or(&null);
    let buckets: Vec<Value> = u
        .get("limits")
        .and_then(Value::as_array)
        .map(|ls| {
            ls.iter()
                .filter(|l| l.get("kind").and_then(Value::as_str) == Some("weekly_scoped"))
                .filter_map(|l| {
                    let name = l.pointer("/scope/model/display_name").filter(|v| truthy(v))?;
                    let pct = l.get("percent").filter(|p| p.is_number())?;
                    let resets = or(l.get("resets_at"), None).cloned().unwrap_or(Value::Null);
                    Some(json!({"label": name, "utilization": pct, "resets_at": resets}))
                })
                .collect()
        })
        .unwrap_or_default();
    json!({
        "ok": true,
        "plan": org.plan,
        "five_hour": pick_window(u.get("five_hour")),
        "seven_day": pick_window(u.get("seven_day")),
        "seven_day_opus": pick_window(u.get("seven_day_opus")),
        "seven_day_sonnet": pick_window(u.get("seven_day_sonnet")),
        "model_buckets": buckets,
    })
}

// ---------------------------------------------------------------- Codex

/// Codex 1단계 `GET /api/auth/session`.
pub fn codex_session_request() -> HttpRequest {
    get("https://chatgpt.com/api/auth/session", vec![])
}

/// 1단계 결과: 401/403이면 미로그인 `raw`, 그 밖에는 토큰(없을 수 있음, 쿠키만으로 계속 시도).
pub fn codex_token_from_session(status: u16, body: Option<&Value>) -> Result<Option<String>, Value> {
    if status == 401 || status == 403 {
        return Err(fail_status("not_logged_in", status));
    }
    if status != 200 {
        return Ok(None);
    }
    Ok(str_of(body.and_then(|b| b.get("accessToken"))).map(str::to_string))
}

/// Codex 2단계 `GET /backend-api/accounts/check/v4-2023-04-27` (토큰이 있을 때만).
pub fn codex_account_request(token: &str) -> HttpRequest {
    get(
        "https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27",
        vec![("Authorization".into(), format!("Bearer {token}"))],
    )
}

/// 플랜 문자열 정규화. `chatgpt` 접두어와 첫 `-`/`_` 뒤를 버리고 소문자로 표에서 찾는다.
pub fn codex_normalize_plan(pt: &str) -> String {
    let rest = pt.strip_prefix("chatgpt").map_or(pt, |r| r.strip_prefix(['-', '_']).unwrap_or(r));
    let key = rest.split(['-', '_']).next().unwrap_or("").to_lowercase();
    match key.as_str() {
        "plus" => "Plus",
        "pro" => "Pro",
        "team" => "Team",
        "free" => "Free",
        "enterprise" => "Enterprise",
        "business" => "Business",
        _ => return pt.to_string(),
    }
    .to_string()
}

/// 2단계 응답 -> (계정 ID, 플랜). 실패해도 진행하므로 항상 값을 돌려준다.
/// 기본 계정(`is_default`)을 우선하고 없으면 첫 항목(serde_json `preserve_order`로 응답 순서 유지, JS `Object.values`와 같음).
pub fn codex_account_from_check(status: u16, body: Option<&Value>) -> (Option<String>, Option<String>) {
    if status != 200 {
        return (None, None);
    }
    let entries: Vec<&Value> = body
        .and_then(|b| b.get("accounts"))
        .and_then(Value::as_object)
        .map(|m| m.values().collect())
        .unwrap_or_default();
    let is_default = |a: &&&Value| a.pointer("/account/is_default").is_some_and(truthy);
    let Some(pick) = entries.iter().find(is_default).or(entries.first()) else { return (None, None) };
    let Some(acc) = pick.get("account") else { return (None, None) };
    let account_id = or(acc.get("account_id"), acc.get("id")).and_then(Value::as_str).map(str::to_string);
    let pt = or(or(acc.get("plan_type"), acc.get("plan")), acc.pointer("/entitlement/subscription_plan"));
    let plan = str_of(pt).map(codex_normalize_plan);
    (account_id, plan)
}

/// Codex 3단계 `GET /backend-api/wham/usage`. 토큰·계정 ID는 있을 때만 헤더에 붙인다.
pub fn codex_usage_request(token: Option<&str>, account_id: Option<&str>) -> HttpRequest {
    let mut headers = vec![];
    if let Some(t) = token {
        headers.push(("Authorization".to_string(), format!("Bearer {t}")));
    }
    if let Some(a) = account_id {
        headers.push(("ChatGPT-Account-ID".to_string(), a.to_string()));
    }
    get("https://chatgpt.com/backend-api/wham/usage", headers)
}

/// JS `new Date(ms).toISOString()`.
fn iso_millis(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn from_millis(ms: f64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(ms as i64).map(iso_millis)
}

/// 리셋 시각 변형 흡수(JS `iso`). 상대 초(reset_after_seconds|resets_in_seconds)가 우선,
/// 다음 reset_at|resets_at(숫자는 1e12 초과면 ms, 아니면 초; 문자열은 그대로).
fn reset_iso(w: &Value, now: DateTime<Utc>) -> Value {
    if let Some(after) = num(w.get("reset_after_seconds")).or(num(w.get("resets_in_seconds"))) {
        let t = now + Duration::milliseconds((after * 1000.0) as i64);
        return Value::String(iso_millis(t));
    }
    let at = match w.get("reset_at") {
        Some(v) if !v.is_null() => Some(v),
        _ => w.get("resets_at"),
    };
    match at {
        Some(Value::Number(n)) => match n.as_f64().filter(|f| f.is_finite()) {
            Some(f) => from_millis(if f > 1e12 { f } else { f * 1000.0 }).map_or(Value::Null, Value::String),
            None => Value::Null,
        },
        Some(Value::String(s)) if !s.is_empty() => Value::String(s.clone()),
        _ => Value::Null,
    }
}

/// 유한한 숫자 값(원래 표기 그대로 복사해 JS의 `5`가 `5.0`으로 바뀌지 않게 한다).
fn num_value(v: Option<&Value>) -> Option<&Value> {
    v.filter(|x| x.as_f64().is_some_and(f64::is_finite))
}

/// 한 창 -> `{utilization, resets_at}`. 사용률(used_percent|usage_percent|utilization)이 없으면 None.
fn window(w: Option<&Value>, now: DateTime<Utc>) -> Option<(Value, Value)> {
    let w = w.filter(|v| truthy(v))?;
    let used = num_value(w.get("used_percent")).or(num_value(w.get("usage_percent"))).or(num_value(w.get("utilization")))?;
    Some((used.clone(), reset_iso(w, now)))
}

fn window_json(v: &Option<(Value, Value)>) -> Value {
    match v {
        Some((u, r)) => json!({"utilization": u, "resets_at": r}),
        None => Value::Null,
    }
}

/// 창 길이(초). limit_window_seconds 우선, 없으면 window_minutes*60.
fn window_secs(w: &Value) -> Option<f64> {
    num(w.get("limit_window_seconds")).or(num(w.get("window_minutes")).map(|m| m * 60.0))
}

/// 5시간 창 판정 기준(초). 창 길이가 이 이하면 5시간 항목. (Providers.swift `secs <= 6 * 3600`)
pub const FIVE_HOUR_MAX_SECS: f64 = 6.0 * 3600.0;

/// JS `pretty`: `_`/`-` 연속을 공백 하나로, 각 단어 첫 글자를 대문자로(ASCII 단어 기준).
fn pretty(x: &str) -> String {
    let mut out = String::new();
    let mut in_sep = false;
    let mut prev_word = false;
    for c in x.chars() {
        if c == '_' || c == '-' {
            if !in_sep {
                out.push(' ');
            }
            in_sep = true;
            prev_word = false;
            continue;
        }
        in_sep = false;
        let is_word = c.is_ascii_alphanumeric();
        out.push(if is_word && !prev_word { c.to_ascii_uppercase() } else { c });
        prev_word = is_word;
    }
    out
}

/// 세부 한도 하나(코드 리뷰·추가 한도)를 버킷으로 보탠다. primary=기본 5시간, secondary=기본 주간,
/// 창 길이가 있으면 `FIVE_HOUR_MAX_SECS` 이하일 때 5시간.
fn add_limit(out: &mut Vec<Value>, label: Option<&str>, r: Option<&Value>, now: DateTime<Utc>) {
    let (Some(label), Some(r)) = (label, r.filter(|v| truthy(v))) else { return };
    let pairs = [(or(r.get("primary_window"), r.get("primary")), true), (or(r.get("secondary_window"), r.get("secondary")), false)];
    for (w, dflt_5h) in pairs {
        let Some((used, resets)) = window(w, now) else { continue };
        let five = window_secs(w.expect("window이 Some이면 w도 Some")).map_or(dflt_5h, |s| s <= FIVE_HOUR_MAX_SECS);
        out.push(json!({"label": label, "utilization": used, "resets_at": resets, "five_hour": five}));
    }
}

/// 3단계 응답 -> `raw`. `now`는 상대 초 리셋의 기준(JS `Date.now()`).
pub fn codex_probe_from_usage(plan: Option<&str>, status: u16, body: Option<&Value>, now: DateTime<Utc>) -> Value {
    if status == 401 || status == 403 {
        return fail_status("not_logged_in", status);
    }
    if status != 200 {
        return fail_status("usage_failed", status);
    }
    let empty = Value::Null;
    let u = body.unwrap_or(&empty);
    let Some(rl) = or(u.get("rate_limit"), u.get("rate_limits")) else { return fail("no_data") };
    let primary = window(or(rl.get("primary_window"), rl.get("primary")), now);
    let secondary = window(or(rl.get("secondary_window"), rl.get("secondary")), now);
    if primary.is_none() && secondary.is_none() {
        return fail("no_data");
    }
    let mut buckets = Vec::new();
    add_limit(&mut buckets, Some("Code Review"), or(u.get("code_review_rate_limit"), u.get("code_review_rate_limits")), now);
    let extra = or(u.get("additional_rate_limits"), u.get("additional_rate_limit"));
    for a in extra.and_then(Value::as_array).into_iter().flatten().filter(|a| truthy(a)) {
        let pretty_feature = str_of(a.get("metered_feature")).map(pretty);
        let name = str_of(a.get("limit_name")).map(str::to_string).or(pretty_feature).or(str_of(a.get("limit_id")).map(str::to_string));
        let r = or(or(a.get("rate_limit"), a.get("rate_limits")), Some(a));
        add_limit(&mut buckets, name.as_deref(), r, now);
    }
    // JS의 Object.keys(u). serde_json 기본은 키 순서를 보존하지 않아 사전순이다.
    let keys: Vec<&String> = u.as_object().map(|m: &Map<String, Value>| m.keys().collect()).unwrap_or_default();
    json!({
        "ok": true,
        "plan": plan,
        "five_hour": window_json(&primary),
        "seven_day": window_json(&secondary),
        "model_buckets": buckets,
        "keys": keys,
    })
}
