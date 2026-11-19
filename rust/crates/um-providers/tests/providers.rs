//! um-providers 통합 테스트. 픽스처는 모두 합성 데이터다(실제 토큰·이메일·ID 없음).

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use um_core::model::{UsageSnapshot, UsageStatus};
use um_providers::agy::{self, AgyParser, Os};
use um_providers::direct::{self, ClaudeOrg};
use um_providers::parse::{KeysLogOnce, parse_iso, remaining, snapshot_from_probe};
use um_providers::state::{self, KnownItem, ProviderState};

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0).unwrap()
}

fn t(s: &str) -> DateTime<Utc> {
    parse_iso(s).unwrap()
}

fn fx(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// ------------------------------------------------------------ parse

#[test]
fn remaining_clamps_and_defaults() {
    assert!(close(remaining(Some(&json!({"utilization": 25.0}))), 0.75));
    assert!(close(remaining(Some(&json!({"utilization": 150}))), 0.0));
    assert!(close(remaining(Some(&json!({"utilization": -20}))), 1.0));
    // 사용률이 없거나 숫자가 아니면 1.0, 구간 자체가 없어도 1.0
    assert!(close(remaining(Some(&json!({"resets_at": "x"}))), 1.0));
    assert!(close(remaining(Some(&json!({"utilization": "50"}))), 1.0));
    assert!(close(remaining(None), 1.0));
}

#[test]
fn reset_parsing_variants() {
    let a = t("2026-11-16T15:00:00Z");
    assert_eq!(t("2026-11-16T15:00:00.000Z"), a);
    assert_eq!(t("2026-11-17T00:00:00+09:00"), a);
    assert_eq!(t("2026-11-16T15:00:00.123456+00:00").timestamp_subsec_micros(), 123_456);
    assert!(parse_iso("2026-11-16").is_none());
    assert!(parse_iso("").is_none());
    let snap = snapshot_from_probe(
        "claude",
        &json!({"ok": true, "five_hour": {"utilization": 1, "resets_at": 12345}}),
        now(),
    );
    assert!(snap.reset_at.is_none(), "문자열이 아닌 resets_at은 None");
}

#[test]
fn failure_reason_mapping() {
    let s = snapshot_from_probe("claude", &json!({"ok": false, "reason": "not_logged_in", "status": 401}), now());
    assert_eq!(s.status, UsageStatus::AuthExpired);
    let s = snapshot_from_probe("gemini", &json!({"ok": false, "reason": "parse_failed"}), now());
    assert_eq!(s.status, UsageStatus::Unavailable("parse_failed".into()));
    let s = snapshot_from_probe("gemini", &json!({"ok": false}), now());
    assert_eq!(s.status, UsageStatus::Unavailable("unknown".into()));
    let s = snapshot_from_probe("gemini", &json!({"reason": "x"}), now());
    assert!(matches!(s.status, UsageStatus::Unavailable(_)), "ok 키가 없으면 실패");
    assert_eq!(s.remaining_ratio, 0.0);
    assert_eq!(s.last_updated, now());
}

#[test]
fn full_snapshot_and_bucket_order() {
    let raw = fx(include_str!("fixtures/claude_probe_js.json"));
    let s = snapshot_from_probe("claude", &raw, now());
    assert_eq!(s.status, UsageStatus::Ok);
    assert_eq!(s.plan.as_deref(), Some("Max (5x)"));
    assert!(close(s.remaining_ratio, 0.745));
    assert!(close(s.secondary_ratio.unwrap(), 0.40));
    assert!(close(s.opus_ratio.unwrap(), 0.90));
    assert_eq!(s.reset_at.unwrap().timestamp(), t("2026-11-16T15:00:00Z").timestamp());
    assert_eq!(s.secondary_reset_at, Some(t("2026-11-20T00:00:00Z")));
    // 서버 버킷(Fable)이 먼저, 그 뒤에 Opus(없어서 덧붙임). sonnet은 null이라 없음.
    let labels: Vec<_> = s.model_buckets.as_ref().unwrap().iter().map(|b| b.label.as_str()).collect();
    assert_eq!(labels, ["Fable", "Opus"]);
    // 표시 순서: 5h, weekly, 모델 버킷
    let ids: Vec<_> = s.sub_items().into_iter().map(|i| i.id).collect();
    assert_eq!(ids, ["5h", "weekly", "model:Fable:weekly", "model:Opus:weekly"]);
}

#[test]
fn opus_sonnet_append_rule() {
    let raw = json!({
        "ok": true,
        "five_hour": {"utilization": 10},
        "seven_day_opus": {"utilization": 20, "resets_at": "2026-11-20T00:00:00Z"},
        "seven_day_sonnet": {"utilization": 30},
        "model_buckets": [{"label": "claude OPUS 4", "utilization": 99, "resets_at": null, "five_hour": true}]
    });
    let s = snapshot_from_probe("claude", &raw, now());
    let b = s.model_buckets.unwrap();
    // limits에 대소문자 무시로 Opus가 이미 있으면 보태지 않고, Sonnet만 보탠다.
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].label, "claude OPUS 4");
    assert!(b[0].is_five_hour && close(b[0].remaining_ratio, 0.01));
    assert_eq!(b[1].label, "Sonnet");
    assert!(!b[1].is_five_hour && close(b[1].remaining_ratio, 0.70));
    // 빈 라벨·비문자열 라벨 버킷은 버린다. 버킷이 하나도 없으면 None.
    let raw = json!({"ok": true, "model_buckets": [{"label": "", "utilization": 1}, {"utilization": 1}, {"label": 3}]});
    let s = snapshot_from_probe("claude", &raw, now());
    assert!(s.model_buckets.is_none());
    assert!(close(s.remaining_ratio, 1.0), "five_hour가 없으면 대표 잔여율 1.0");
    assert!(s.secondary_ratio.is_none() && s.plan.is_none());
}

#[test]
fn empty_plan_is_none() {
    let s = snapshot_from_probe("claude", &json!({"ok": true, "plan": ""}), now());
    assert!(s.plan.is_none());
}

#[test]
fn keys_logged_once() {
    let mut k = KeysLogOnce::default();
    let raw = json!({"ok": true, "keys": ["a", "b"]});
    assert!(k.take("codex", &json!({"ok": true})).is_none(), "키 없는 응답은 소비하지 않는다");
    assert_eq!(k.take("codex", &raw).as_deref(), Some("codex usage keys=a,b"));
    assert!(k.take("codex", &raw).is_none());
}

// ------------------------------------------------------------ direct: Claude

#[test]
fn claude_plan_label_table() {
    let table = [
        ("default_claude_max_20x", Some("Max (20x)")),
        ("default_claude_max_5x", Some("Max (5x)")),
        ("default_claude_max", Some("Max")),
        ("claude_team", Some("Team")),
        ("default_claude_enterprise", Some("Enterprise")),
        ("default_claude_pro", Some("Pro")),
        ("default_claude_free", Some("Free")),
        ("default_claude_some_new_tier", Some("some new tier")),
        ("other_tier", Some("other tier")),
        ("", None),
    ];
    for (tier, want) in table {
        assert_eq!(direct::claude_plan_label(tier).as_deref(), want, "{tier}");
    }
}

#[test]
fn claude_request_sequence() {
    let r = direct::claude_orgs_request();
    assert_eq!(r.url, "https://claude.ai/api/organizations");
    assert!(r.headers.contains(&("Accept".into(), "application/json".into())));
    let orgs = fx(include_str!("fixtures/claude_orgs.json"));
    let org = direct::claude_org_from_orgs(200, Some(&orgs)).unwrap();
    assert_eq!(org.plan.as_deref(), Some("Max (5x)"));
    assert_eq!(
        direct::claude_usage_request(&org).url,
        "https://claude.ai/api/organizations/00000000-0000-0000-0000-000000000000/usage"
    );
}

#[test]
fn claude_failure_paths() {
    let e = direct::claude_org_from_orgs(403, None).unwrap_err();
    assert_eq!(e, json!({"ok": false, "reason": "not_logged_in", "status": 403}));
    assert_eq!(snapshot_from_probe("claude", &e, now()).status, UsageStatus::AuthExpired);
    for body in [json!([]), json!({}), json!([{"name": "no uuid"}])] {
        let e = direct::claude_org_from_orgs(200, Some(&body)).unwrap_err();
        assert_eq!(e["reason"], "no_org");
    }
    let org = ClaudeOrg { uuid: "x".into(), plan: None };
    let e = direct::claude_probe_from_usage(&org, 500, None);
    assert_eq!(e, json!({"ok": false, "reason": "usage_failed", "status": 500}));
    assert_eq!(snapshot_from_probe("claude", &e, now()).status, UsageStatus::Unavailable("usage_failed".into()));
}

#[test]
fn claude_direct_equals_js_path() {
    let orgs = fx(include_str!("fixtures/claude_orgs.json"));
    let usage = fx(include_str!("fixtures/claude_usage.json"));
    let org = direct::claude_org_from_orgs(200, Some(&orgs)).unwrap();
    let direct_raw = direct::claude_probe_from_usage(&org, 200, Some(&usage));
    let js_raw = fx(include_str!("fixtures/claude_probe_js.json"));
    assert_eq!(direct_raw, js_raw);
    assert_eq!(snapshot_from_probe("claude", &direct_raw, now()), snapshot_from_probe("claude", &js_raw, now()));
}

#[test]
fn claude_pick_requires_numeric_utilization_and_nulls_empty_reset() {
    let org = ClaudeOrg { uuid: "x".into(), plan: None };
    let usage = json!({"five_hour": {"utilization": "5"}, "seven_day": {"utilization": 0, "resets_at": ""}});
    let raw = direct::claude_probe_from_usage(&org, 200, Some(&usage));
    assert_eq!(raw["five_hour"], Value::Null);
    assert_eq!(raw["seven_day"], json!({"utilization": 0, "resets_at": null}));
    assert_eq!(raw["plan"], Value::Null);
    assert_eq!(raw["model_buckets"], json!([]));
}

// ------------------------------------------------------------ direct: Codex

#[test]
fn codex_plan_normalization() {
    let table = [
        ("chatgpt_plus", "Plus"),
        ("chatgpt-pro", "Pro"),
        ("chatgptteam", "Team"),
        ("team", "Team"),
        ("free", "Free"),
        ("enterprise", "Enterprise"),
        ("business_annual", "Business"),
        ("PLUS", "Plus"),
        ("weird_plan", "weird_plan"),
        ("ChatGPT_Enterprise", "ChatGPT_Enterprise"),
    ];
    for (pt, want) in table {
        assert_eq!(direct::codex_normalize_plan(pt), want, "{pt}");
    }
}

#[test]
fn codex_request_sequence() {
    let session = fx(include_str!("fixtures/codex_session.json"));
    assert_eq!(direct::codex_session_request().url, "https://chatgpt.com/api/auth/session");
    let token = direct::codex_token_from_session(200, Some(&session)).unwrap().unwrap();
    let req = direct::codex_account_request(&token);
    assert_eq!(req.url, "https://chatgpt.com/backend-api/accounts/check/v4-2023-04-27");
    assert!(req.headers.contains(&("Authorization".into(), "Bearer fake-token-not-real".into())));
    let accounts = fx(include_str!("fixtures/codex_accounts.json"));
    let (id, plan) = direct::codex_account_from_check(200, Some(&accounts));
    assert_eq!(id.as_deref(), Some("acct-fake-0002"), "is_default 계정 우선");
    assert_eq!(plan.as_deref(), Some("Plus"));
    let req = direct::codex_usage_request(Some(&token), id.as_deref());
    assert_eq!(req.url, "https://chatgpt.com/backend-api/wham/usage");
    assert!(req.headers.contains(&("ChatGPT-Account-ID".into(), "acct-fake-0002".into())));
    // 토큰·계정이 없으면 Accept만
    assert_eq!(direct::codex_usage_request(None, None).headers.len(), 1);
}

#[test]
fn codex_session_and_account_fallbacks() {
    let e = direct::codex_token_from_session(401, None).unwrap_err();
    assert_eq!(snapshot_from_probe("codex", &e, now()).status, UsageStatus::AuthExpired);
    assert_eq!(direct::codex_token_from_session(500, None).unwrap(), None);
    assert_eq!(direct::codex_token_from_session(200, Some(&json!({}))).unwrap(), None);
    assert_eq!(direct::codex_account_from_check(500, None), (None, None));
    // 기본 계정이 없으면 첫 항목, account_id 없으면 id
    let acc = json!({"accounts": {"x": {"account": {"id": "acct-fake-9", "entitlement": {"subscription_plan": "chatgpt_team"}}}}});
    assert_eq!(direct::codex_account_from_check(200, Some(&acc)), (Some("acct-fake-9".into()), Some("Team".into())));
}

#[test]
fn codex_direct_equals_js_path() {
    let usage = fx(include_str!("fixtures/codex_usage.json"));
    let raw = direct::codex_probe_from_usage(Some("Plus"), 200, Some(&usage), now());
    let js_raw = fx(include_str!("fixtures/codex_probe_js.json"));
    assert_eq!(raw, js_raw);
    assert_eq!(snapshot_from_probe("codex", &raw, now()), snapshot_from_probe("codex", &js_raw, now()));
}

#[test]
fn codex_window_classification() {
    let s = snapshot_from_probe(
        "codex",
        &direct::codex_probe_from_usage(Some("Plus"), 200, Some(&fx(include_str!("fixtures/codex_usage.json"))), now()),
        now(),
    );
    assert!(close(s.remaining_ratio, 0.70));
    assert!(close(s.secondary_ratio.unwrap(), 0.895));
    assert_eq!(s.reset_at, Some(t("2026-11-16T13:00:00Z")));
    let got: Vec<(String, bool)> = s.model_buckets.unwrap().into_iter().map(|b| (b.label, b.is_five_hour)).collect();
    assert_eq!(
        got,
        [
            ("Code Review".to_string(), true),
            ("Code Review".to_string(), false),
            ("Codex Fake Model".to_string(), true),  // 21600초 = 6시간은 5시간
            ("Codex Fake Model".to_string(), false), // 21601초는 5시간 아님
        ]
    );
    // 창 길이가 없으면 primary=5시간, secondary=주간. 길이가 길면 primary라도 주간.
    let body = json!({
        "rate_limit": {"primary": {"utilization": 1}},
        "code_review_rate_limits": {
            "primary": {"used_percent": 2},
            "secondary": {"used_percent": 3},
        },
        "additional_rate_limit": [{"limit_id": "lim_x", "primary_window": {"used_percent": 4, "limit_window_seconds": 30000}}]
    });
    let raw = direct::codex_probe_from_usage(None, 200, Some(&body), now());
    let b = raw["model_buckets"].as_array().unwrap();
    let flags: Vec<_> = b.iter().map(|x| (x["label"].as_str().unwrap(), x["five_hour"].as_bool().unwrap())).collect();
    assert_eq!(flags, [("Code Review", true), ("Code Review", false), ("lim_x", false)]);
    assert_eq!(raw["seven_day"], Value::Null);
    assert_eq!(raw["plan"], Value::Null);
}

#[test]
fn codex_reset_variants_and_failures() {
    let w = |extra: Value| {
        let mut m = json!({"used_percent": 1});
        m.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        let body = json!({"rate_limits": {"primary_window": m}});
        direct::codex_probe_from_usage(None, 200, Some(&body), now())["five_hour"]["resets_at"].clone()
    };
    assert_eq!(w(json!({"reset_after_seconds": 90})), json!("2026-11-16T12:01:30.000Z"));
    assert_eq!(w(json!({"resets_in_seconds": 5.5})), json!("2026-11-16T12:00:05.500Z"));
    assert_eq!(w(json!({"reset_at": 1795000000})), json!("2026-11-18T11:06:40.000Z"), "초 단위");
    assert_eq!(w(json!({"resets_at": 1795000000000_i64})), json!("2026-11-18T11:06:40.000Z"), "밀리초 단위");
    assert_eq!(w(json!({"reset_at": "2026-12-01T00:00:00Z"})), json!("2026-12-01T00:00:00Z"));
    assert_eq!(w(json!({"reset_at": ""})), Value::Null);
    assert_eq!(w(json!({})), Value::Null);
    // 상대 초가 절대 시각보다 우선
    assert_eq!(w(json!({"reset_after_seconds": 0, "reset_at": 1795000000})), json!("2026-11-16T12:00:00.000Z"));

    let no_data = json!({"ok": false, "reason": "no_data"});
    assert_eq!(direct::codex_probe_from_usage(None, 200, Some(&json!({})), now()), no_data);
    let nothing = json!({"rate_limit": {"primary_window": {"foo": 1}}});
    assert_eq!(direct::codex_probe_from_usage(None, 200, Some(&nothing), now()), no_data);
    assert_eq!(direct::codex_probe_from_usage(None, 401, None, now())["reason"], "not_logged_in");
    assert_eq!(direct::codex_probe_from_usage(None, 503, None, now())["reason"], "usage_failed");
}

// ------------------------------------------------------------ agy

fn agy_snap(p: &mut AgyParser, data: &str) -> Option<UsageSnapshot> {
    p.parse(data.as_bytes(), "gemini", now())
}

#[test]
fn agy_classification() {
    let mut p = AgyParser::new();
    let s = agy_snap(&mut p, include_str!("fixtures/agy_success.json")).unwrap();
    assert_eq!(s.id, "gemini");
    assert_eq!(s.plan.as_deref(), Some("Antigravity"));
    // 대표 잔여율은 Gemini 후보 중 가장 급한 값(Argon 5h 0.30), 5시간 원값은 Gemini 5h(0.60)
    assert!(close(s.remaining_ratio, 0.30));
    assert_eq!(s.reset_at, Some(t("2026-11-16T15:00:00Z")));
    assert!(close(s.five_hour_ratio.unwrap(), 0.60));
    assert!(close(s.secondary_ratio.unwrap(), 0.80));
    assert_eq!(s.secondary_reset_at, Some(t("2026-11-20T00:00:00.5Z")));
    let mb: Vec<_> = s.model_buckets.as_ref().unwrap().iter().map(|b| (b.label.as_str(), b.is_five_hour, b.remaining_ratio)).collect();
    assert_eq!(
        mb,
        [("Gemini 4 Argon", true, 0.30), ("Gemini 4 Argon", false, 0.90), ("Claude·GPT (agy)", false, 0.10)]
    );
    let raw = s.raw_buckets.unwrap();
    assert_eq!(raw.len(), 5);
    assert!(close(raw["claude-weekly"], 0.10));
}

#[test]
fn agy_monotonic_rule() {
    let mut p = AgyParser::new();
    agy_snap(&mut p, include_str!("fixtures/agy_success.json")).unwrap();
    // 같은 reset_time에서 +0.05 상승은 무시(0.60 유지), +0.15는 받아들인다.
    let s = agy_snap(&mut p, include_str!("fixtures/agy_bumped.json")).unwrap();
    assert!(close(s.five_hour_ratio.unwrap(), 0.60));
    assert!(close(s.secondary_ratio.unwrap(), 0.95));
    assert!(close(s.raw_buckets.as_ref().unwrap()["gemini-5h"], 0.60));
    // 새 reset_time이면 올라도 그대로 받아들인다.
    let s = agy_snap(&mut p, include_str!("fixtures/agy_reset_changed.json")).unwrap();
    assert!(close(s.five_hour_ratio.unwrap(), 0.62));
    assert_eq!(s.reset_at, Some(t("2026-11-16T20:00:00Z")));
    // 기록이 0.62로 갱신됐으므로 이제 0.65(원래 reset_time)는 reset_time이 달라 그대로 받아들여진다.
    let s = agy_snap(&mut p, include_str!("fixtures/agy_bumped.json")).unwrap();
    assert!(close(s.five_hour_ratio.unwrap(), 0.65));
}

#[test]
fn agy_monotonic_allows_decrease_and_big_jump() {
    let mk = |frac: f64| {
        format!(
            r#"{{"status":"SUCCESS","command":{{"data":{{"groups":[{{"name":"Gemini models","buckets":[
            {{"id":"gemini-5h","window":"5h","reset_time":"2026-11-16T15:00:00Z","remaining_fraction":{frac}}}]}}]}}}}}}"#
        )
    };
    let mut p = AgyParser::new();
    let r = |p: &mut AgyParser, f: f64| agy_snap(p, &mk(f)).unwrap().five_hour_ratio.unwrap();
    assert!(close(r(&mut p, 0.50), 0.50));
    assert!(close(r(&mut p, 0.45), 0.45), "하락은 즉시 반영");
    assert!(close(r(&mut p, 0.50), 0.45), "+0.05 무시");
    assert!(close(r(&mut p, 0.60), 0.60), "+0.12 이상은 반영");
    assert!(close(r(&mut p, 1.5), 1.0), "범위 밖은 1로 자른다");
}

#[test]
fn agy_rejects_bad_output() {
    let mut p = AgyParser::new();
    assert!(agy_snap(&mut p, include_str!("fixtures/agy_failed.json")).is_none());
    assert!(agy_snap(&mut p, "not json").is_none());
    assert!(agy_snap(&mut p, r#"{"status":"SUCCESS","command":{"data":{"groups":[]}}}"#).is_none());
    // 버킷이 있어도 remaining_fraction이 전혀 없으면 후보가 없어 None
    let none = r#"{"status":"SUCCESS","command":{"data":{"groups":[{"name":"X","buckets":[{"id":"a"}]}]}}}"#;
    assert!(agy_snap(&mut p, none).is_none());
}

#[test]
fn agy_no_gemini_candidates_falls_back_to_most_urgent_bucket() {
    let json = r#"{"status":"SUCCESS","command":{"data":{"groups":[
        {"name":"Claude and GPT models","buckets":[
          {"id":"c-5h","window":"5H","reset_time":"2026-11-16T15:00:00Z","remaining_fraction":0.7},
          {"id":"c-weekly","reset_time":"2026-11-20T00:00:00Z","remaining_fraction":0.2}]},
        {"name":"","buckets":[{"id":"z","name":"Zed","remaining_fraction":0.5}]}]}}}"#;
    let s = agy_snap(&mut AgyParser::new(), json).unwrap();
    assert!(close(s.remaining_ratio, 0.2));
    assert!(s.secondary_ratio.is_none());
    // gemini-5h가 없으면 대표 5시간은 가장 급한 값
    assert!(close(s.five_hour_ratio.unwrap(), 0.2));
    let labels: Vec<_> = s.model_buckets.unwrap().into_iter().map(|b| (b.label, b.is_five_hour)).collect();
    assert_eq!(labels, [("Claude·GPT (agy)".to_string(), true), ("Claude·GPT (agy)".to_string(), false), ("Zed".to_string(), false)]);
}

#[test]
fn agy_executable_candidates() {
    let c = agy::candidates(Os::Unix, "/Users/fake", Some("/a/bin::/b/bin"), None);
    let c: Vec<_> = c.iter().map(|p| p.to_str().unwrap()).collect();
    assert_eq!(
        c,
        [
            "/Users/fake/.local/bin/agy",
            "/Users/fake/.gemini/antigravity-cli/bin/agy",
            "/opt/homebrew/bin/agy",
            "/usr/local/bin/agy",
            "/usr/bin/agy",
            "/a/bin/agy",
            "/b/bin/agy",
        ]
    );
    let w = agy::candidates(Os::Windows, r"C:\Users\fake", Some(r#"C:\Tools\;"D:\Bin""#), Some(".EXE;.CMD"));
    let w: Vec<_> = w.iter().map(|p| p.to_str().unwrap()).collect();
    assert_eq!(
        w,
        [
            r"C:\Users\fake\.local\bin\agy.exe",
            r"C:\Users\fake\.local\bin\agy.cmd",
            r"C:\Users\fake\.gemini\antigravity-cli\bin\agy.exe",
            r"C:\Users\fake\.gemini\antigravity-cli\bin\agy.cmd",
            r"C:\Tools\agy.EXE",
            r"C:\Tools\agy.CMD",
            r"D:\Bin\agy.EXE",
            r"D:\Bin\agy.CMD",
        ]
    );
    // PATHEXT가 없으면 기본 확장자
    let d = agy::candidates(Os::Windows, "H", Some("P"), None);
    assert_eq!(d.len(), 4 + 4);
}

#[test]
fn agy_command_line() {
    assert_eq!(agy::args(Os::Unix), ["--log-file", "/dev/null", "-p", "/usage", "--output-format", "json"]);
    assert_eq!(agy::args(Os::Windows)[1], "NUL");
    assert_eq!(agy::TIMEOUT_SECS, if cfg!(windows) { 30 } else { 12 });
    assert_eq!(agy::work_dir(Os::Unix, None).as_deref(), Some("/tmp"));
    assert_eq!(agy::work_dir(Os::Windows, Some(r"C:\Temp")).as_deref(), Some(r"C:\Temp"));
    assert_eq!(
        agy::child_path_unix("/h", Some("/x")),
        "/h/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/x"
    );
    assert_eq!(agy::child_path_unix("/h", Some("")), "/h/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin");
}

// ------------------------------------------------------------ state

fn ok_snap(ratio: f64) -> UsageSnapshot {
    snapshot_from_probe("claude", &json!({"ok": true, "five_hour": {"utilization": (1.0 - ratio) * 100.0}}), now())
}

fn fail_snap(status: UsageStatus) -> UsageSnapshot {
    UsageSnapshot::failed("claude", status, now())
}

#[test]
fn apply_rules() {
    let ok = ok_snap(0.5);
    // Ok: 저장 + 로그인 + 기억
    let o = state::apply(None, ok.clone());
    assert_eq!(o.store.as_ref(), Some(&ok));
    assert_eq!(o.logged_in, Some(true));
    assert_eq!(o.remember.unwrap().len(), 1);
    // AuthExpired: 이전이 Ok여도 저장 + 로그아웃
    let auth = fail_snap(UsageStatus::AuthExpired);
    let o = state::apply(Some(&ok), auth.clone());
    assert_eq!((o.store, o.logged_in, o.remember), (Some(auth), Some(false), None));
    // Unavailable/Stale: 이전이 Ok면 무시
    for st in [UsageStatus::Unavailable("parse_failed".into()), UsageStatus::Stale] {
        let o = state::apply(Some(&ok), fail_snap(st));
        assert_eq!((o.store, o.logged_in, o.remember), (None, None, None));
    }
    // 이전이 없거나 Ok가 아니면 실패 상태를 저장(로그인 표시는 그대로)
    let un = fail_snap(UsageStatus::Unavailable("x".into()));
    let o = state::apply(None, un.clone());
    assert_eq!((o.store, o.logged_in), (Some(un.clone()), None));
    let o = state::apply(Some(&fail_snap(UsageStatus::AuthExpired)), un.clone());
    assert_eq!(o.store, Some(un));
}

#[test]
fn provider_state_known_items() {
    let mut st = ProviderState::default();
    assert!(st.apply(ok_snap(0.5)), "처음 기억은 변경");
    assert!(st.logged_in);
    assert_eq!(st.known_items, [KnownItem { id: "5h".into(), ratio: 0.5 }]);
    assert!(!st.apply(ok_snap(0.5)), "같은 값이면 변경 없음");
    // 읽기 실패: 값과 로그인 유지, 설정 탭 항목은 live
    assert!(!st.apply(fail_snap(UsageStatus::Unavailable("x".into()))));
    assert!(st.logged_in && st.snapshot.as_ref().unwrap().status == UsageStatus::Ok);
    assert!(st.sub_items().1);
    // 미로그인: 로그아웃 표시, 항목은 기억값(live=false)
    st.apply(fail_snap(UsageStatus::AuthExpired));
    assert!(!st.logged_in);
    assert!(st.active_snapshot().is_none());
    let (items, live) = st.sub_items();
    assert!(!live);
    assert_eq!(items.len(), 1);
    assert!(close(items[0].remaining_ratio, 0.5));
    // 기억한 id가 깨져 있으면 건너뛴다
    st.known_items.push(KnownItem { id: "bogus".into(), ratio: 0.1 });
    assert_eq!(st.sub_items().0.len(), 1);
    st.logout();
    assert!(st.snapshot.is_none() && !st.known_items.is_empty());
}

#[test]
fn start_retry_policy() {
    assert_eq!((state::START_RETRY_ATTEMPTS, state::START_RETRY_INTERVAL_SECS), (4, 2));
    assert!(!state::should_retry(&ok_snap(1.0)));
    assert!(!state::should_retry(&fail_snap(UsageStatus::AuthExpired)));
    assert!(state::should_retry(&fail_snap(UsageStatus::Unavailable("x".into()))));
}

#[test]
fn agy_출력_앞의_bom과_안내_줄을_건너뛴다() {
    let body = include_str!("fixtures/agy_success.json");
    let mut p = AgyParser::default();
    let plain = p.parse(body.as_bytes(), "gemini", chrono::Utc::now()).unwrap();
    for prefix in ["\u{feff}", "Checking for updates...\r\n", "\u{feff}notice\n"] {
        let mut p = AgyParser::default();
        let s = p.parse(format!("{prefix}{body}").as_bytes(), "gemini", chrono::Utc::now()).unwrap();
        assert_eq!(s.sub_items().len(), plain.sub_items().len());
    }
}
