//! Claude Code 기록(`~/.claude/projects/**/*.jsonl`) 수집.
//! 필요한 필드만 가진 구조체로 읽어 대화 내용은 문자열로도 남기지 않는다.

use crate::ingest::{IngestOptions, IngestStats, run_jsonl_file, tolerate_io, walk_files};
use crate::store::{Store, TokenEvent, insert_event};
use crate::{Result, Roots, Source, basename};
use chrono::DateTime;
use serde::Deserialize;

#[derive(Deserialize, Default)]
struct Line {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    message: Option<Message>,
    #[serde(rename = "requestId", default)]
    request_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(rename = "sessionId", default)]
    session_id: Option<String>,
    #[serde(rename = "isSidechain", default)]
    is_sidechain: Option<bool>,
}

#[derive(Deserialize, Default)]
struct Message {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens_details: Option<OutputDetails>,
}

#[derive(Deserialize, Default)]
struct OutputDetails {
    #[serde(default)]
    thinking_tokens: Option<u64>,
}

/// ISO 8601 문자열을 유닉스 초로.
pub(crate) fn parse_ts(s: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis() as f64 / 1000.0)
}

pub(crate) fn ingest(
    store: &mut Store,
    roots: &Roots,
    opts: &IngestOptions,
    stats: &mut IngestStats,
) -> Result<()> {
    for path in walk_files(&roots.claude_projects, "jsonl") {
        let fallback_session = path.file_stem().map(|s| s.to_string_lossy().into_owned());
        let key = path.to_string_lossy().into_owned();
        let r = run_jsonl_file(store, &path, Source::Claude, opts, stats, &mut |tx, _ctx, line, off| {
            // 사용량이 없는 줄(대부분)은 파싱하지 않는다.
            if !line.contains("\"usage\"") {
                return Ok(0);
            }
            let Ok(l) = serde_json::from_str::<Line>(line) else { return Ok(0) };
            if l.kind.as_deref() != Some("assistant") {
                return Ok(0);
            }
            let Some(msg) = l.message else { return Ok(0) };
            let Some(u) = msg.usage else { return Ok(0) };
            let Some(t) = l.timestamp.as_deref().and_then(parse_ts) else { return Ok(0) };
            // 한 응답이 여러 줄로 기록되므로 message.id(+requestId)로 한 번만 센다.
            let dedup_key = match &msg.id {
                Some(id) => format!("cc:{id}:{}", l.request_id.as_deref().unwrap_or("")),
                None => format!("cc:{key}:{off}"),
            };
            let ev = TokenEvent {
                t,
                source: Source::Claude.as_str(),
                model: msg.model,
                project: l.cwd.as_deref().and_then(basename),
                session: l.session_id.or_else(|| fallback_session.clone()),
                input: u.input_tokens.unwrap_or(0),
                output: u.output_tokens.unwrap_or(0),
                cache_read: u.cache_read_input_tokens.unwrap_or(0),
                cache_write: u.cache_creation_input_tokens.unwrap_or(0),
                reasoning: u.output_tokens_details.and_then(|d| d.thinking_tokens).unwrap_or(0),
                sidechain: l.is_sidechain.unwrap_or(false),
                dedup_key,
            };
            Ok(u64::from(insert_event(tx, &ev)?))
        });
        tolerate_io(r, &path)?;
    }
    Ok(())
}
