//! 수집 통합 테스트. 모든 입력은 손으로 만든 가짜 기록이다.

use crate::agy::{self, AgyRaw};
use crate::protobuf::enc;
use crate::query::Range;
use crate::testutil::TempDir;
use crate::{IngestOptions, Roots, Source, Store, ingest_all, ingest_source};
use rusqlite::Connection;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

fn setup() -> (TempDir, Roots, Store) {
    let tmp = TempDir::new();
    let roots = Roots::from_home(&tmp.path().join("home"), |_| None);
    let store = Store::open_in_memory().unwrap();
    for s in Source::ALL {
        store.set_enabled(s, true).unwrap();
    }
    (tmp, roots, store)
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn append(path: &Path, text: &str) {
    let mut f = OpenOptions::new().append(true).open(path).unwrap();
    f.write_all(text.as_bytes()).unwrap();
}

fn cc_line(id: &str, req: &str, ts: &str, sidechain: bool, out: u64) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"{ts}","cwd":"/work/demo-proj","sessionId":"s1","isSidechain":{sidechain},"requestId":"{req}","message":{{"id":"{id}","model":"model-a","content":"SYNTHETIC","usage":{{"input_tokens":10,"output_tokens":{out},"cache_creation_input_tokens":5,"cache_read_input_tokens":40,"output_tokens_details":{{"thinking_tokens":3}}}}}}}}"#
    ) + "\n"
}

fn all_time() -> Range {
    Range { from: 0.0, to: 4e9 }
}

type Row = (i64, i64, i64, i64, i64, String, String, String, String);

fn count(store: &Store) -> i64 {
    store.count_events().unwrap()
}

const TS: &str = "2026-11-10T12:00:00.000Z";

#[test]
fn claude_dedupe_and_sidechain() {
    let (_t, roots, mut store) = setup();
    let f = roots.claude_projects.join("p/a.jsonl");
    let mut text = cc_line("m1", "r1", TS, false, 20);
    text += &cc_line("m1", "r1", TS, false, 20); // 같은 응답이 반복된 줄
    text += &cc_line("m2", "r2", TS, true, 7);
    text += r#"{"type":"user","message":{"content":"SYNTHETIC usage"}}"#;
    text += "\nnot json with \"usage\"\n";
    write(&f, &text);
    let st = ingest_source(&mut store, &roots, Source::Claude, &IngestOptions::default()).unwrap();
    assert_eq!(st.events_added, 2);
    assert_eq!(count(&store), 2);
    let t = store.totals(all_time(), None).unwrap();
    assert_eq!((t.input, t.output, t.cache_read, t.cache_write, t.reasoning), (20, 27, 80, 10, 6));
    assert_eq!(t.cache_hit_ratio(), Some(80.0 / 100.0));
    let side: i64 = store
        .conn
        .query_row("SELECT SUM(sidechain) FROM token_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(side, 1);
    let proj = store.by_project(all_time(), None).unwrap();
    assert_eq!(proj[0].key, "demo-proj");
    assert_eq!(store.sessions_count(all_time(), None).unwrap(), 1);
    // 다시 돌려도 늘지 않는다
    let st = ingest_source(&mut store, &roots, Source::Claude, &IngestOptions::default()).unwrap();
    assert_eq!(st.events_added, 0);
    assert_eq!(count(&store), 2);
}

fn codex_fixture() -> String {
    let mut s = String::new();
    s += r#"{"timestamp":"2026-11-10T11:00:00.000Z","type":"session_meta","payload":{"id":"sess-1","cwd":"/work/cx-proj","base_instructions":"SYNTHETIC"}}"#;
    s += "\n";
    s += r#"{"timestamp":"2026-11-10T11:00:01.000Z","type":"turn_context","payload":{"turn_id":"t1","model":"gpt-x","cwd":"/work/cx-proj"}}"#;
    s += "\n";
    // t1: token_count 먼저, 그 다음 token_usage_record(우선)
    s += r#"{"timestamp":"2026-11-10T11:00:02.000Z","type":"event_msg","payload":{"type":"token_count","turn_id":"t1","info":{"last_token_usage":{"input_tokens":100,"cached_input_tokens":60,"output_tokens":9,"reasoning_output_tokens":4},"total_token_usage":{"total_tokens":109}},"rate_limits":{"primary":{"used_percent":12.5,"window_minutes":300,"resets_at":1790000000},"secondary":{"used_percent":3.0,"window_minutes":10080,"resets_at":1790500000}}}}"#;
    s += "\n";
    s += r#"{"timestamp":"2026-11-10T11:00:03.000Z","type":"event_msg","payload":{"type":"token_usage_record","turn_id":"t1","response_id":"resp-1","usage":{"input_tokens":200,"cached_input_tokens":50,"cache_write_input_tokens":7,"output_tokens":11,"reasoning_output_tokens":2}}}"#;
    s += "\n";
    // t1의 늦게 온 token_count는 무시되어야 한다
    s += r#"{"timestamp":"2026-11-10T11:00:04.000Z","type":"event_msg","payload":{"type":"token_count","turn_id":"t1","info":{"last_token_usage":{"input_tokens":1,"output_tokens":1},"total_token_usage":{"total_tokens":2}}}}"#;
    s += "\n";
    // t2: token_count만, 같은 줄이 두 번 반복
    for _ in 0..2 {
        s += r#"{"timestamp":"2026-11-10T11:05:00.000Z","type":"event_msg","payload":{"type":"token_count","turn_id":"t2","info":{"last_token_usage":{"input_tokens":30,"cached_input_tokens":10,"output_tokens":5},"total_token_usage":{"total_tokens":150}},"rate_limits":{"primary":{"used_percent":14.0,"window_minutes":300,"resets_at":1790000000},"secondary":null}}}"#;
        s += "\n";
    }
    s
}

#[test]
fn codex_record_preferred_and_limits_captured() {
    let (_t, roots, mut store) = setup();
    write(&roots.codex_sessions.join("2026/11/10/rollout-a.jsonl"), &codex_fixture());
    ingest_source(&mut store, &roots, Source::Codex, &IngestOptions::default()).unwrap();
    assert_eq!(count(&store), 2);
    let mut stmt = store
        .conn
        .prepare("SELECT input,output,cache_read,cache_write,reasoning,model,project,session,dedup_key FROM token_events ORDER BY t")
        .unwrap();
    let rows: Vec<Row> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    // t1은 기록 형식(input 200-50 캐시 분리), t2는 token_count
    assert!(rows[0].8.starts_with("cx:rec:"));
    assert_eq!((rows[0].5.as_str(), rows[0].6.as_str(), rows[0].7.as_str()), ("gpt-x", "cx-proj", "sess-1"));
    assert_eq!((rows[0].0, rows[0].1, rows[0].2, rows[0].3, rows[0].4), (150, 11, 50, 7, 2));
    assert_eq!((rows[1].0, rows[1].1, rows[1].2), (20, 5, 10));
    // rate_limits: 3개 줄에서 2개 서로 다른 스냅샷(앞의 token_count 1개, t2 반복은 하나로)
    let lim = store.codex_limit_series(all_time()).unwrap();
    assert_eq!(lim.len(), 2);
    assert_eq!(lim[0].primary_used, Some(12.5));
    assert_eq!(lim[0].primary_window_min, Some(300));
    assert_eq!(lim[0].secondary_resets_at, Some(1_790_500_000));
    assert_eq!(lim[1].secondary_used, None);
}

#[test]
fn partial_last_line_waits_for_newline() {
    let (_t, roots, mut store) = setup();
    let f = roots.claude_projects.join("p/a.jsonl");
    let l2 = cc_line("m2", "r2", TS, false, 1);
    let (head, tail) = l2.split_at(l2.len() / 2);
    write(&f, &(cc_line("m1", "r1", TS, false, 1) + head));
    let o = IngestOptions::default();
    ingest_source(&mut store, &roots, Source::Claude, &o).unwrap();
    assert_eq!(count(&store), 1);
    let off = store.file_state(&f.to_string_lossy()).unwrap().unwrap().offset;
    assert_eq!(off as usize, cc_line("m1", "r1", TS, false, 1).len());
    append(&f, tail);
    ingest_source(&mut store, &roots, Source::Claude, &o).unwrap();
    assert_eq!(count(&store), 2);
}

#[test]
fn truncation_restarts_from_zero() {
    let (_t, roots, mut store) = setup();
    let f = roots.claude_projects.join("p/a.jsonl");
    write(&f, &(cc_line("m1", "r1", TS, false, 1) + &cc_line("m2", "r2", TS, false, 1)));
    let o = IngestOptions::default();
    ingest_source(&mut store, &roots, Source::Claude, &o).unwrap();
    assert_eq!(count(&store), 2);
    // 더 짧은 새 내용으로 교체
    let short = cc_line("m3", "r3", TS, false, 1);
    write(&f, &short);
    let st = ingest_source(&mut store, &roots, Source::Claude, &o).unwrap();
    assert_eq!(st.events_added, 1);
    assert_eq!(count(&store), 3);
    assert_eq!(store.file_state(&f.to_string_lossy()).unwrap().unwrap().offset as usize, short.len());
}

#[test]
fn retention_skips_old_files() {
    let (_t, roots, mut store) = setup();
    let f = roots.claude_projects.join("p/old.jsonl");
    write(&f, &cc_line("m1", "r1", TS, false, 1));
    let old = SystemTime::now() - Duration::from_secs(100 * 86_400);
    File::options().write(true).open(&f).unwrap().set_modified(old).unwrap();
    let st = ingest_source(&mut store, &roots, Source::Claude, &IngestOptions::default()).unwrap();
    assert_eq!((st.files_skipped_old, count(&store)), (1, 0));
    // 보관 기간을 늘리면 읽는다
    let o = IngestOptions { retention_days: 200, now: None };
    ingest_source(&mut store, &roots, Source::Claude, &o).unwrap();
    assert_eq!(count(&store), 1);
}

#[test]
fn per_source_enable_and_clear() {
    let tmp = TempDir::new();
    let roots = Roots::from_home(tmp.path(), |_| None);
    write(&roots.claude_projects.join("a.jsonl"), &cc_line("m1", "r1", TS, false, 1));
    write(&roots.codex_sessions.join("rollout-a.jsonl"), &codex_fixture());
    let mut store = Store::open_in_memory().unwrap();
    let o = IngestOptions::default();
    // 기본은 모두 꺼짐
    ingest_all(&mut store, &roots, &o).unwrap();
    assert_eq!(count(&store), 0);
    store.set_enabled(Source::Claude, true).unwrap();
    ingest_all(&mut store, &roots, &o).unwrap();
    assert_eq!(count(&store), 1);
    store.set_enabled(Source::Codex, true).unwrap();
    ingest_all(&mut store, &roots, &o).unwrap();
    assert_eq!(count(&store), 3);
    store.clear_source(Source::Codex).unwrap();
    assert_eq!(count(&store), 1);
    assert!(store.codex_limit_series(all_time()).unwrap().is_empty());
    assert!(store.is_enabled(Source::Codex).unwrap());
    // 지운 뒤에는 처음부터 다시 읽는다
    ingest_all(&mut store, &roots, &o).unwrap();
    assert_eq!(count(&store), 3);
    store.clear_all().unwrap();
    assert_eq!(count(&store), 0);
    assert!(store.is_enabled(Source::Claude).unwrap());
}

#[test]
fn query_aggregates() {
    let (_t, roots, mut store) = setup();
    // 2026-11-10은 화요일(월=0이면 1), 12시 UTC
    write(&roots.claude_projects.join("a.jsonl"), &cc_line("m1", "r1", TS, false, 10));
    ingest_source(&mut store, &roots, Source::Claude, &IngestOptions::default()).unwrap();
    let m = store.weekday_hour_matrix(all_time(), None, 0).unwrap();
    assert_eq!(m[1][12], 10 + 10 + 5 + 40);
    let m = store.weekday_hour_matrix(all_time(), None, 9 * 3600).unwrap();
    assert_eq!(m[1][21], 65);
    let days = store.by_day(all_time(), None, 0).unwrap();
    assert_eq!(days[0].key, "2026-11-10");
    let days = store.by_day(all_time(), None, 13 * 3600).unwrap();
    assert_eq!(days[0].key, "2026-11-11");
    let t = store.totals(all_time(), Some(Source::Codex)).unwrap();
    assert_eq!((t.turns, t.cache_hit_ratio(), t.tokens_per_turn()), (0, None, None));
    assert_eq!(store.by_model(all_time(), None).unwrap()[0].key, "model-a");
    assert_eq!(store.by_source(all_time(), None).unwrap()[0].key, "claude");
}

fn gen_blob(model: &str, f: [(u32, u64); 3]) -> Vec<u8> {
    let mut usage = Vec::new();
    for (n, v) in f {
        enc::field_varint(n, v, &mut usage);
    }
    let mut inner = Vec::new();
    enc::field_len(4, &usage, &mut inner);
    enc::field_len(19, model.as_bytes(), &mut inner);
    let mut root = Vec::new();
    enc::field_len(1, &inner, &mut root);
    root
}

fn make_agy_db(path: &Path, rows: &[(i64, Vec<u8>)]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let c = Connection::open(path).unwrap();
    c.execute_batch("CREATE TABLE gen_metadata(idx INTEGER, data BLOB)").unwrap();
    for (i, b) in rows {
        c.execute("INSERT INTO gen_metadata VALUES(?1,?2)", rusqlite::params![i, b]).unwrap();
    }
}

#[test]
fn agy_copy_then_open_and_ingest() {
    let (_t, roots, mut store) = setup();
    let db: PathBuf = roots.agy_conversations().join("conv-1.db");
    make_agy_db(
        &db,
        &[
            (0, gen_blob("gemini-x", [(1, 1318), (2, 9650), (3, 1280)])),
            (1, gen_blob("gemini-x", [(1, 1), (2, 3626), (5, 7617)])),
            (2, vec![0xff, 0xff]), // 깨진 blob은 건너뜀
        ],
    );
    let sum = roots.agy_summaries();
    let c = Connection::open(&sum).unwrap();
    c.execute_batch(
        "CREATE TABLE conversation_summaries(conversation_id TEXT, workspace_uris TEXT, last_modified_time TEXT, step_count INTEGER);
         INSERT INTO conversation_summaries VALUES('conv-1','[\"file:///Users/x/My%20App/\"]','',3);",
    )
    .unwrap();
    drop(c);

    // 복사 후 읽기 전용으로 열린다
    let copy = agy::copy_db(&db).unwrap();
    let conn = agy::open_copy(&copy).unwrap();
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM gen_metadata", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 3);
    assert!(conn.execute("DELETE FROM gen_metadata", []).is_err());
    drop(conn);
    drop(copy);

    let o = IngestOptions::default();
    let st = ingest_source(&mut store, &roots, Source::Agy, &o).unwrap();
    assert_eq!(st.events_added, 2);
    let (model, project, f1, f2, f3, f5, f9): (String, String, i64, i64, Option<i64>, Option<i64>, Option<i64>) = store
        .conn
        .query_row(
            "SELECT model,project,agy_f1,agy_f2,agy_f3,agy_f5,agy_f9 FROM agy_events WHERE idx=0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .unwrap();
    assert_eq!((model.as_str(), project.as_str(), f1, f2, f3, f5, f9), ("gemini-x", "My App", 1318, 9650, Some(1280), None, None));
    let raw = AgyRaw { f2: Some(9650), f3: Some(1280), ..Default::default() };
    assert_eq!(agy::provisional_total(&raw), 10930);
    // 변경이 없으면 다시 열지 않는다
    let st = ingest_source(&mut store, &roots, Source::Agy, &o).unwrap();
    assert_eq!((st.files_read, st.events_added), (0, 0));
}

#[test]
fn large_file_streams_with_bounded_memory() {
    const N: usize = 50_000;
    let (_t, roots, mut store) = setup();
    let f = roots.claude_projects.join("p/big.jsonl");
    fs::create_dir_all(f.parent().unwrap()).unwrap();
    {
        let mut w = BufWriter::new(File::create(&f).unwrap());
        let pad = "x".repeat(300); // 가짜 본문(저장되지 않는다)
        for i in 0..N {
            let l = cc_line(&format!("m{i}"), &format!("r{i}"), TS, i % 7 == 0, 5).replace("SYNTHETIC", &pad);
            w.write_all(l.as_bytes()).unwrap();
        }
    }
    let size = fs::metadata(&f).unwrap().len();
    let t0 = std::time::Instant::now();
    let st = ingest_source(&mut store, &roots, Source::Claude, &IngestOptions::default()).unwrap();
    let dt = t0.elapsed();
    assert_eq!(count(&store), N as i64);
    assert_eq!(st.lines, N as u64);
    // 줄 단위 스트리밍: 한 번에 올린 가장 긴 줄은 파일 전체보다 훨씬 작다
    assert!(st.peak_line_bytes < 4096, "peak line {}", st.peak_line_bytes);
    assert!((st.peak_line_bytes as u64) * 1000 < size);
    let rss = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    eprintln!(
        "50k-line ingest: file {} KiB, {:?}, peak line buf {} B, process RSS {} KiB (test binary, 포함 전체)",
        size / 1024,
        dt,
        st.peak_line_bytes,
        rss.trim()
    );
}
