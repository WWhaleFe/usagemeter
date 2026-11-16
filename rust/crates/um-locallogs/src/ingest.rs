//! 증분 수집 공통부: 파일 순회, 줄 단위 스트리밍 읽기, 오프셋 관리.

use crate::store::{FileState, Store, save_file_state};
use crate::{Error, Result, Roots, Source};
use rusqlite::Transaction;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct IngestOptions {
    /// 이 일수보다 오래전에 수정된 파일은 건너뛴다(기본 35일).
    pub retention_days: u32,
    /// 현재 시각(초). 테스트에서 고정할 때만 쓴다.
    pub now: Option<f64>,
}

impl Default for IngestOptions {
    fn default() -> Self {
        IngestOptions { retention_days: 35, now: None }
    }
}

impl IngestOptions {
    pub(crate) fn cutoff(&self) -> f64 {
        self.now.unwrap_or_else(now_secs) - f64::from(self.retention_days) * 86_400.0
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IngestStats {
    pub files_seen: u32,
    pub files_read: u32,
    pub files_skipped_old: u32,
    pub lines: u64,
    pub events_added: u64,
    pub bytes_read: u64,
    /// 한 번에 메모리에 올린 가장 긴 줄의 바이트 수(스트리밍 검증용).
    pub peak_line_bytes: usize,
}

impl IngestStats {
    pub(crate) fn merge(&mut self, o: &IngestStats) {
        self.files_seen += o.files_seen;
        self.files_read += o.files_read;
        self.files_skipped_old += o.files_skipped_old;
        self.lines += o.lines;
        self.events_added += o.events_added;
        self.bytes_read += o.bytes_read;
        self.peak_line_bytes = self.peak_line_bytes.max(o.peak_line_bytes);
    }
}

/// 파일을 읽는 동안 이어지는 문맥(Codex의 모델·프로젝트·세션).
#[derive(Debug, Clone, Default)]
pub(crate) struct FileCtx {
    pub model: Option<String>,
    pub project: Option<String>,
    pub session: Option<String>,
}

/// 줄 처리기: (트랜잭션, 문맥, 줄, 줄 시작 바이트 위치) -> 새로 넣은 이벤트 수.
pub(crate) type LineHandler<'a> =
    dyn FnMut(&Transaction, &mut FileCtx, &str, u64) -> Result<u64> + 'a;

pub(crate) fn now_secs() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

pub(crate) fn mtime_secs(md: &fs::Metadata) -> f64 {
    md.modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map_or(0.0, |d| d.as_secs_f64())
}

/// 폴더 아래 `ext` 확장자 파일을 재귀로 모은다(심볼릭 링크는 따라가지 않는다).
pub(crate) fn walk_files(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for ent in rd.flatten() {
            let Ok(ft) = ent.file_type() else { continue };
            let p = ent.path();
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() && p.extension().is_some_and(|e| e == ext) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// jsonl 파일 하나를 저장된 오프셋부터 끝까지 줄 단위로 읽는다.
/// 개행으로 끝나지 않은 마지막 줄은 다음 번으로 미룬다.
pub(crate) fn run_jsonl_file(
    store: &mut Store,
    path: &Path,
    source: Source,
    opts: &IngestOptions,
    stats: &mut IngestStats,
    handler: &mut LineHandler,
) -> Result<()> {
    stats.files_seen += 1;
    let md = fs::metadata(path)?;
    let (size, mtime) = (md.len(), mtime_secs(&md));
    if mtime < opts.cutoff() {
        stats.files_skipped_old += 1;
        return Ok(());
    }
    let key = path.to_string_lossy().into_owned();
    let prev = store.file_state(&key)?;
    // 크기가 줄었거나 mtime이 과거로 갔으면 교체된 파일로 보고 처음부터 읽는다.
    let (start, mut ctx) = match &prev {
        Some(p) if size >= p.offset && mtime >= p.mtime => (
            p.offset,
            FileCtx {
                model: p.ctx_model.clone(),
                project: p.ctx_project.clone(),
                session: p.ctx_session.clone(),
            },
        ),
        _ => (0, FileCtx::default()),
    };
    if prev.is_some() && start >= size {
        return Ok(());
    }

    let tx = store.conn.transaction()?;
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(start))?;
    let mut rd = BufReader::with_capacity(64 * 1024, f);
    let mut buf: Vec<u8> = Vec::new();
    let mut pos = start;
    loop {
        buf.clear();
        let n = rd.read_until(b'\n', &mut buf)?;
        if n == 0 || buf.last() != Some(&b'\n') {
            break; // EOF 또는 미완성 마지막 줄
        }
        stats.peak_line_bytes = stats.peak_line_bytes.max(buf.capacity());
        if let Ok(line) = std::str::from_utf8(&buf) {
            let line = line.trim();
            if !line.is_empty() {
                stats.lines += 1;
                stats.events_added += handler(&tx, &mut ctx, line, pos)?;
            }
        }
        pos += n as u64;
    }
    stats.bytes_read += pos - start;
    stats.files_read += 1;
    let st = FileState {
        size,
        mtime,
        offset: pos,
        ctx_model: ctx.model,
        ctx_project: ctx.project,
        ctx_session: ctx.session,
    };
    save_file_state(&tx, &key, source, &st)?;
    tx.commit()?;
    Ok(())
}

/// 한 소스를 수집한다. 꺼져 있으면 아무것도 하지 않는다.
pub fn ingest_source(
    store: &mut Store,
    roots: &Roots,
    source: Source,
    opts: &IngestOptions,
) -> Result<IngestStats> {
    let mut stats = IngestStats::default();
    if !store.is_enabled(source)? {
        return Ok(stats);
    }
    match source {
        Source::Claude => crate::claude::ingest(store, roots, opts, &mut stats)?,
        Source::Codex => crate::codex::ingest(store, roots, opts, &mut stats)?,
        Source::Agy => crate::agy::ingest(store, roots, opts, &mut stats)?,
    }
    Ok(stats)
}

/// 켜진 소스를 모두 수집한다.
pub fn ingest_all(store: &mut Store, roots: &Roots, opts: &IngestOptions) -> Result<IngestStats> {
    let mut total = IngestStats::default();
    for s in Source::ALL {
        total.merge(&ingest_source(store, roots, s, opts)?);
    }
    Ok(total)
}

/// 파일 단위 I/O 오류는 경고만 남기고 넘어간다(파일이 사라지는 경우 등). DB 오류는 전파한다.
pub(crate) fn tolerate_io(r: Result<()>, path: &Path) -> Result<()> {
    match r {
        Err(Error::Io(e)) => {
            tracing::warn!("로컬 기록 읽기 실패 {}: {e}", path.display());
            Ok(())
        }
        other => other,
    }
}
