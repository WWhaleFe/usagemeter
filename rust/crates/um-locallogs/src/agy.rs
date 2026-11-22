//! Antigravity CLI 대화별 SQLite(`conversations/<id>.db`) 수집. 계획서 8-1, ADR 0003.
//! 쓰는 중에는 원본을 못 여므로 사본(+wal, shm)을 임시 폴더에 만들어 읽기 전용으로 연다.

use crate::ingest::{IngestOptions, IngestStats, mtime_secs, walk_files};
use crate::protobuf::{Wire, fields, find_path, string_at, varint_at};
use crate::store::{FileState, Store, save_file_state};
use crate::{Result, Roots, Source, basename};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, params};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// `gen_metadata.data` 1.4 묶음의 원시 숫자. 각 번호의 의미는 아직 검증되지 않았다.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AgyRaw {
    pub f1: Option<u64>,
    pub f2: Option<u64>,
    pub f3: Option<u64>,
    pub f5: Option<u64>,
    pub f6: Option<u64>,
    pub f9: Option<u64>,
    pub f10: Option<u64>,
}

/// 임시 총량 힌트: `.2 + .3`. **검증되지 않은 값이다.** 이 필드들이 입력·출력·캐시 중
/// 무엇인지 agy `/usage` 잔여율 하락과 맞춰 보기 전까지 토큰 총량으로 표시하면 안 된다(ADR 0003).
pub fn provisional_total(raw: &AgyRaw) -> u64 {
    raw.f2.unwrap_or(0).saturating_add(raw.f3.unwrap_or(0))
}

/// `gen_metadata` 한 행에서 읽은 값.
#[derive(Debug, Clone, PartialEq)]
pub struct GenMeta {
    /// 모델 ID(`1.19`)
    pub model: Option<String>,
    /// 사람이 읽는 표시 이름(`1.21`)
    pub label: Option<String>,
    /// 이 요청의 step 번호(`1.20` 중 키 `last_step_index`의 값)
    pub step: Option<i64>,
    pub raw: AgyRaw,
}

/// `gen_metadata` 한 행(blob)을 해석한다. 모델·표시 이름·step·숫자가 모두 없으면 `None`.
pub fn parse_gen_metadata(blob: &[u8]) -> Option<GenMeta> {
    let model = string_at(blob, &[1, 19]).filter(|m| {
        !m.is_empty() && m.len() < 128 && m.chars().all(|c| c.is_ascii_alphanumeric() || "._-:/".contains(c))
    });
    let label = string_at(blob, &[1, 21]).filter(|m| !m.trim().is_empty() && m.len() < 128 && !m.chars().any(char::is_control));
    let step = last_step_index(blob);
    let v = |n: u32| varint_at(blob, &[1, 4, n]);
    let raw = AgyRaw {
        f1: v(1),
        f2: v(2),
        f3: v(3),
        f5: v(5),
        f6: v(6),
        f9: v(9),
        f10: v(10),
    };
    if model.is_none() && label.is_none() && step.is_none() && raw == AgyRaw::default() {
        return None;
    }
    Some(GenMeta { model, label, step, raw })
}

/// `1.20` 반복 항목(키 `20.1`, 값 `20.2`)에서 `last_step_index`의 값을 찾는다.
fn last_step_index(blob: &[u8]) -> Option<i64> {
    let Some(Wire::Len(inner)) = find_path(blob, &[1]) else { return None };
    for (n, w) in fields(inner) {
        let (20, Wire::Len(entry)) = (n, w) else { continue };
        if string_at(entry, &[1]).as_deref() == Some("last_step_index") {
            return string_at(entry, &[2]).and_then(|v| v.trim().parse().ok());
        }
    }
    None
}

/// `steps.metadata`의 생성 시각(유닉스 초). `1.1` 초 + `1.2` 나노초(없을 수 있음).
fn step_time(meta: &[u8]) -> Option<f64> {
    let sec = varint_at(meta, &[1, 1])?;
    let nanos = varint_at(meta, &[1, 2]).unwrap_or(0);
    Some(sec as f64 + (nanos.min(999_999_999) as f64) / 1e9)
}

/// 사본 폴더. 버릴 때 지운다.
pub struct TempCopy {
    dir: PathBuf,
    pub db: PathBuf,
}

impl Drop for TempCopy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

static COPY_N: AtomicU32 = AtomicU32::new(0);

/// db와 `-wal`, `-shm`(있으면)을 임시 폴더로 복사한다.
pub fn copy_db(src: &Path) -> io::Result<TempCopy> {
    let dir = std::env::temp_dir().join(format!(
        "um-agy-{}-{}",
        std::process::id(),
        COPY_N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir)?;
    let copy = TempCopy { db: dir.join("copy.db"), dir };
    fs::copy(src, &copy.db)?;
    for suffix in ["-wal", "-shm"] {
        let mut s = src.as_os_str().to_os_string();
        s.push(suffix);
        let s = PathBuf::from(s);
        if s.is_file() {
            let mut d = copy.db.as_os_str().to_os_string();
            d.push(suffix);
            fs::copy(&s, PathBuf::from(d))?;
        }
    }
    Ok(copy)
}

/// 사본을 읽기 전용으로 연다.
pub fn open_copy(copy: &TempCopy) -> rusqlite::Result<Connection> {
    Connection::open_with_flags(&copy.db, OpenFlags::SQLITE_OPEN_READ_ONLY)
}

/// 대화 한 건의 요청 한 행.
#[derive(Debug, Clone, PartialEq)]
pub struct AgyRow {
    pub idx: i64,
    pub meta: GenMeta,
    /// 그 요청 step의 시각(유닉스 초). 찾지 못하면 `None`.
    pub t: Option<f64>,
}

/// 원본 db를 복사해 `gen_metadata`의 요청 행과 `steps`의 step 시각을 읽는다.
/// `steps`가 없거나 읽기에 실패하면 시각은 모두 `None`이다.
pub fn read_conversation(src: &Path) -> Result<Vec<AgyRow>> {
    let copy = copy_db(src)?;
    let conn = open_copy(&copy)?;
    let steps = read_steps(&conn).unwrap_or_else(|e| {
        tracing::warn!("agy steps 읽기 실패 {}: {e}", src.display());
        HashMap::new()
    });
    let mut stmt = conn.prepare("SELECT idx, data FROM gen_metadata")?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let idx: i64 = r.get(0)?;
        let ValueRef::Blob(b) = r.get_ref(1)? else { continue };
        if let Some(meta) = parse_gen_metadata(b) {
            let t = meta.step.and_then(|s| steps.get(&s).copied());
            out.push(AgyRow { idx, meta, t });
        }
    }
    Ok(out)
}

/// `steps`의 idx -> 시각(초).
fn read_steps(conn: &Connection) -> rusqlite::Result<HashMap<i64, f64>> {
    let mut stmt = conn.prepare("SELECT idx, metadata FROM steps")?;
    let mut rows = stmt.query([])?;
    let mut map = HashMap::new();
    while let Some(r) = rows.next()? {
        let idx: i64 = r.get(0)?;
        let ValueRef::Blob(b) = r.get_ref(1)? else { continue };
        if let Some(t) = step_time(b) {
            map.insert(idx, t);
        }
    }
    Ok(map)
}

/// `workspace_uris` 값에서 첫 `file://` 주소의 마지막 폴더 이름을 뽑는다.
fn project_from_uris(s: &str) -> Option<String> {
    let start = s.find("file://")?;
    let rest = &s[start..];
    let end = rest.find(['"', ',', ' ', ']', '\n']).unwrap_or(rest.len());
    basename(&percent_decode(&rest[..end]))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| char::from(c).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 대화 id -> 프로젝트 폴더 이름. 파일이 없거나 형식이 달라도 빈 맵으로 넘어간다.
pub fn read_summaries(src: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if !src.is_file() {
        return map;
    }
    let mut run = || -> Result<()> {
        let copy = copy_db(src)?;
        let conn = open_copy(&copy)?;
        let mut stmt =
            conn.prepare("SELECT conversation_id, workspace_uris FROM conversation_summaries")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let id = match r.get_ref(0)? {
                ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                _ => continue,
            };
            let uris = match r.get_ref(1)? {
                ValueRef::Text(t) | ValueRef::Blob(t) => String::from_utf8_lossy(t).into_owned(),
                _ => continue,
            };
            if let Some(p) = project_from_uris(&uris) {
                map.insert(id, p);
            }
        }
        Ok(())
    };
    if let Err(e) = run() {
        tracing::warn!("conversation_summaries 읽기 실패: {e}");
    }
    map
}

/// db + wal의 합산 크기와 가장 늦은 수정 시각.
fn fingerprint(db: &Path) -> io::Result<(u64, f64)> {
    let md = fs::metadata(db)?;
    let (mut size, mut mtime) = (md.len(), mtime_secs(&md));
    let mut w = db.as_os_str().to_os_string();
    w.push("-wal");
    if let Ok(wm) = fs::metadata(PathBuf::from(w)) {
        size += wm.len();
        mtime = mtime.max(mtime_secs(&wm));
    }
    Ok((size, mtime))
}

pub(crate) fn ingest(
    store: &mut Store,
    roots: &Roots,
    opts: &IngestOptions,
    stats: &mut IngestStats,
) -> Result<()> {
    let mut summaries: Option<HashMap<String, String>> = None;
    for path in walk_files(&roots.agy_conversations(), "db") {
        stats.files_seen += 1;
        let Ok((size, mtime)) = fingerprint(&path) else { continue };
        if mtime < opts.cutoff() {
            stats.files_skipped_old += 1;
            continue;
        }
        let key = path.to_string_lossy().into_owned();
        if let Some(p) = store.file_state(&key)?
            && p.size == size
            && p.mtime == mtime
        {
            continue;
        }
        let rows = match read_conversation(&path) {
            Ok(r) => r,
            Err(crate::Error::Sqlite(e)) => {
                // 형식이 바뀌었거나 잠긴 경우: 조용히 건너뛴다(ADR 0003).
                tracing::warn!("agy 대화 해석 실패 {}: {e}", path.display());
                continue;
            }
            Err(crate::Error::Io(e)) => {
                tracing::warn!("agy 대화 복사 실패 {}: {e}", path.display());
                continue;
            }
            Err(e) => return Err(e),
        };
        let conv = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let project = summaries
            .get_or_insert_with(|| read_summaries(&roots.agy_summaries()))
            .get(&conv)
            .cloned();
        let tx = store.conn.transaction()?;
        {
            // 다시 읽을 때는 같은 요청 행을 새 값으로 바꾼다(먼저 읽을 때 step 시각이 아직 없었을 수 있다).
            let mut ins = tx.prepare_cached(
                "INSERT OR REPLACE INTO agy_events
                 (conversation,idx,t,model,label,project,agy_f1,agy_f2,agy_f3,agy_f5,agy_f6,agy_f9,agy_f10)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            )?;
            for AgyRow { idx, meta, t } in rows {
                let raw = meta.raw;
                let n = ins.execute(params![
                    conv,
                    idx,
                    t.unwrap_or(mtime),
                    meta.model,
                    meta.label,
                    project,
                    raw.f1.map(|v| v as i64),
                    raw.f2.map(|v| v as i64),
                    raw.f3.map(|v| v as i64),
                    raw.f5.map(|v| v as i64),
                    raw.f6.map(|v| v as i64),
                    raw.f9.map(|v| v as i64),
                    raw.f10.map(|v| v as i64),
                ])?;
                stats.events_added += n as u64;
            }
        }
        let st = FileState { size, mtime, offset: 0, ..Default::default() };
        save_file_state(&tx, &key, Source::Agy, &st)?;
        tx.commit()?;
        stats.files_read += 1;
    }
    Ok(())
}
