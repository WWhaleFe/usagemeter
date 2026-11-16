//! Codex CLI 기록(`~/.codex/sessions/**/rollout-*.jsonl`) 수집과 `rate_limits` 저장.

use crate::claude::parse_ts;
use crate::ingest::{FileCtx, IngestOptions, IngestStats, run_jsonl_file, tolerate_io, walk_files};
use crate::store::{Store, TokenEvent, insert_event};
use crate::{Result, Roots, Source, basename};
use rusqlite::{Transaction, params};
use serde::Deserialize;

#[derive(Deserialize, Default)]
struct Line {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    payload: Option<Payload>,
}

#[derive(Deserialize, Default)]
struct Payload {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    turn_id: Option<String>,
    #[serde(default)]
    response_id: Option<String>,
    #[serde(default)]
    info: Option<Info>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize, Default)]
struct Info {
    #[serde(default)]
    last_token_usage: Option<Usage>,
    #[serde(default)]
    total_token_usage: Option<TotalUsage>,
}

#[derive(Deserialize, Default)]
struct TotalUsage {
    #[serde(default)]
    total_tokens: Option<u64>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    cached_input_tokens: Option<u64>,
    #[serde(default)]
    cache_write_input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    reasoning_output_tokens: Option<u64>,
}

#[derive(Deserialize, Default)]
struct RateLimits {
    #[serde(default)]
    primary: Option<Window>,
    #[serde(default)]
    secondary: Option<Window>,
}

#[derive(Deserialize, Default)]
struct Window {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    window_minutes: Option<i64>,
    #[serde(default)]
    resets_at: Option<i64>,
}

/// Codex의 `input_tokens`는 캐시된 입력을 포함하므로 뺀 값을 `input`으로 저장한다
/// (캐시 적중률 = cache_read / (input + cache_read) 식이 맞도록).
fn to_event(u: &Usage, t: f64, ctx: &FileCtx, key: String) -> TokenEvent {
    let cached = u.cached_input_tokens.unwrap_or(0);
    TokenEvent {
        t,
        source: Source::Codex.as_str(),
        model: ctx.model.clone(),
        project: ctx.project.clone(),
        session: ctx.session.clone(),
        input: u.input_tokens.unwrap_or(0).saturating_sub(cached),
        output: u.output_tokens.unwrap_or(0),
        cache_read: cached,
        cache_write: u.cache_write_input_tokens.unwrap_or(0),
        reasoning: u.reasoning_output_tokens.unwrap_or(0),
        sidechain: false,
        dedup_key: key,
    }
}

/// `prefix`(끝이 ':')로 시작하는 dedup_key 범위: [prefix, prefix의 마지막 ':'을 ';'로 바꾼 값).
fn prefix_range(prefix: &str) -> (String, String) {
    let mut hi = prefix.to_string();
    hi.pop();
    hi.push(';');
    (prefix.to_string(), hi)
}

fn has_prefix(tx: &Transaction, prefix: &str) -> Result<bool> {
    let (lo, hi) = prefix_range(prefix);
    Ok(tx
        .prepare_cached("SELECT 1 FROM token_events WHERE dedup_key >= ?1 AND dedup_key < ?2 LIMIT 1")?
        .exists(params![lo, hi])?)
}

fn insert_limits(tx: &Transaction, t: f64, rl: &RateLimits) -> Result<()> {
    let (p, s) = (rl.primary.as_ref(), rl.secondary.as_ref());
    if p.is_none() && s.is_none() {
        return Ok(());
    }
    tx.prepare_cached(
        "INSERT OR IGNORE INTO codex_limits
         (t,primary_used,primary_window_min,primary_resets_at,secondary_used,secondary_window_min,secondary_resets_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7)",
    )?
    .execute(params![
        t,
        p.and_then(|w| w.used_percent),
        p.and_then(|w| w.window_minutes),
        p.and_then(|w| w.resets_at),
        s.and_then(|w| w.used_percent),
        s.and_then(|w| w.window_minutes),
        s.and_then(|w| w.resets_at),
    ])?;
    Ok(())
}

pub(crate) fn ingest(
    store: &mut Store,
    roots: &Roots,
    opts: &IngestOptions,
    stats: &mut IngestStats,
) -> Result<()> {
    for path in walk_files(&roots.codex_sessions, "jsonl") {
        let file_key = path.to_string_lossy().into_owned();
        let fallback_session = path.file_stem().map(|s| s.to_string_lossy().into_owned());
        let r = run_jsonl_file(store, &path, Source::Codex, opts, stats, &mut |tx, ctx, line, off| {
            // 관심 있는 줄만 파싱한다.
            let interesting = line.contains("token_count")
                || line.contains("token_usage_record")
                || line.contains("session_meta")
                || line.contains("turn_context");
            if !interesting {
                return Ok(0);
            }
            let Ok(l) = serde_json::from_str::<Line>(line) else { return Ok(0) };
            let Some(p) = l.payload else { return Ok(0) };
            let kind = p.kind.as_deref().or(l.kind.as_deref()).unwrap_or("");
            let t = l.timestamp.as_deref().and_then(parse_ts);
            match kind {
                "session_meta" => {
                    if let Some(s) = p.session_id.or(p.id) {
                        ctx.session = Some(s);
                    }
                    if let Some(c) = p.cwd.as_deref().and_then(basename) {
                        ctx.project = Some(c);
                    }
                    Ok(0)
                }
                "turn_context" => {
                    if p.model.is_some() {
                        ctx.model = p.model;
                    }
                    if let Some(c) = p.cwd.as_deref().and_then(basename) {
                        ctx.project = Some(c);
                    }
                    Ok(0)
                }
                "token_usage_record" => {
                    let (Some(t), Some(u)) = (t, p.usage.as_ref()) else { return Ok(0) };
                    if ctx.session.is_none() {
                        ctx.session = fallback_session.clone();
                    }
                    let turn = p.turn_id.as_deref().unwrap_or("-");
                    let resp = p.response_id.clone().unwrap_or_else(|| off.to_string());
                    // 같은 턴의 token_count가 이미 들어갔다면 기록 형식을 우선해 지운다.
                    if p.turn_id.is_some() {
                        let (lo, hi) = prefix_range(&format!("cx:tc:{file_key}:{turn}:"));
                        tx.prepare_cached(
                            "DELETE FROM token_events WHERE dedup_key >= ?1 AND dedup_key < ?2",
                        )?
                        .execute(params![lo, hi])?;
                    }
                    let ev = to_event(u, t, ctx, format!("cx:rec:{file_key}:{turn}:{resp}"));
                    Ok(u64::from(insert_event(tx, &ev)?))
                }
                "token_count" => {
                    let Some(t) = t else { return Ok(0) };
                    if let Some(rl) = &p.rate_limits {
                        insert_limits(tx, t, rl)?;
                    }
                    let Some(info) = p.info.as_ref() else { return Ok(0) };
                    let Some(u) = info.last_token_usage.as_ref() else { return Ok(0) };
                    if ctx.session.is_none() {
                        ctx.session = fallback_session.clone();
                    }
                    let turn = p.turn_id.as_deref().unwrap_or("-");
                    if p.turn_id.is_some() && has_prefix(tx, &format!("cx:rec:{file_key}:{turn}:"))? {
                        return Ok(0);
                    }
                    // 같은 줄이 반복되면 누적 합계가 같으므로 (턴, 누적 합계)로 중복을 거른다.
                    let seq = info
                        .total_token_usage
                        .as_ref()
                        .and_then(|t| t.total_tokens)
                        .unwrap_or(off);
                    let ev = to_event(u, t, ctx, format!("cx:tc:{file_key}:{turn}:{seq}"));
                    Ok(u64::from(insert_event(tx, &ev)?))
                }
                _ => Ok(0),
            }
        });
        tolerate_io(r, &path)?;
    }
    Ok(())
}
