//! 사용량 데이터: Gemini는 agy CLI로 실시간 조회, 아직 조회 방법이 없는 AI는 이력의 마지막 값을 보여준다.
//! (Claude·Codex 실시간 조회는 로그인·웹뷰 계층을 만든 뒤 붙인다: 계획서 5-1, STATUS 다음 할 일 2)

use std::collections::BTreeMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use um_core::history::HistoryDb;
use um_core::model::UsageSubItem;
use um_core::model::{UsageModelBucket, UsageSnapshot, UsageStatus};
use um_providers::agy::{self, AgyParser, Os};

/// 이력에서 AI별 마지막 값을 스냅샷으로 되살린다(35일 이내). 세부 항목 id로 5시간·주간·모델 버킷을 복원한다.
pub fn seed_from_history(
    db: &HistoryDb,
    ids: &[String],
    now: DateTime<Utc>,
) -> BTreeMap<String, UsageSnapshot> {
    let since = now.timestamp() as f64 - 35.0 * 86400.0;
    let mut out = BTreeMap::new();
    for id in ids {
        let Ok(series) = db.series(id, since) else {
            continue;
        };
        let Some(last) = series.last() else { continue };
        let mut snap = UsageSnapshot::failed(
            id,
            UsageStatus::Ok,
            DateTime::from_timestamp(last.t as i64, 0).unwrap_or(now),
        );
        snap.remaining_ratio = last.r;
        let mut buckets = vec![];
        for item in db.item_ids(id, since).unwrap_or_default() {
            let Some(p) = db
                .item_series(id, &item, since)
                .ok()
                .and_then(|v| v.last().copied())
            else {
                continue;
            };
            match item.as_str() {
                "5h" => snap.five_hour_ratio = Some(p.r),
                "weekly" => snap.secondary_ratio = Some(p.r),
                _ => {
                    if let Some(si) = UsageSubItem::from_id(&item, p.r, None) {
                        buckets.push(UsageModelBucket {
                            label: si.model_label.unwrap_or_default(),
                            remaining_ratio: p.r,
                            reset_at: None,
                            is_five_hour: si.is_five_hour,
                        });
                    }
                }
            }
        }
        if !buckets.is_empty() {
            snap.model_buckets = Some(buckets);
        }
        out.insert(id.clone(), snap);
    }
    out
}

/// agy를 한 번 실행해 Gemini 스냅샷을 얻는다(12초 제한). agy가 없거나 실패하면 None.
pub fn fetch_agy(parser: &std::sync::Mutex<AgyParser>) -> Option<UsageSnapshot> {
    let os = Os::current();
    let home = directories::BaseDirs::new()?
        .home_dir()
        .to_string_lossy()
        .to_string();
    let Some(exe) = agy::find_executable(os, &home).or_else(where_agy) else {
        tracing::info!("agy 실행 파일 없음: Gemini는 웹 조회로(세부 항목이 적다)");
        return None;
    };
    let mut cmd = Command::new(exe);
    cmd.args(agy::args(os))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let temp = std::env::var_os("TEMP").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    if let Some(wd) = agy::work_dir(os, temp.to_str()) {
        cmd.current_dir(wd);
    }
    #[cfg(unix)]
    cmd.env("HOME", &home).env(
        "PATH",
        agy::child_path_unix(&home, std::env::var("PATH").ok().as_deref()),
    );
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("agy 실행 실패: {e}");
            return None;
        }
    };
    // 출력이 파이프 버퍼보다 커도 막히지 않게 따로 읽는다.
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) if st.success() => break,
            Ok(Some(st)) => {
                tracing::warn!("agy 비정상 종료: {st}");
                return None;
            }
            Ok(None) if start.elapsed() > Duration::from_secs(agy::TIMEOUT_SECS) => {
                tracing::warn!("agy 시간 초과({}초)", agy::TIMEOUT_SECS);
                let _ = child.kill();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => return None,
        }
    }
    let out = reader.join().ok()?;
    let snap = parser.lock().unwrap().parse(&out, "gemini", Utc::now());
    match &snap {
        // 출력 내용(계정·대화 정보가 섞일 수 있음)은 남기지 않고 크기·개수만.
        Some(s) => tracing::info!("agy 조회 {:.1}초, 세부 항목 {}개", start.elapsed().as_secs_f64(), s.sub_items().len()),
        None => tracing::warn!("agy 출력 해석 실패({}바이트)", out.len()),
    }
    snap
}

/// PATH가 앱 시작 뒤에 바뀌었거나 사용자 PATH에만 있을 때를 위해 Windows `where`로 한 번 더 찾는다.
#[cfg(windows)]
fn where_agy() -> Option<std::path::PathBuf> {
    use std::os::windows::process::CommandExt;
    let out = Command::new("where.exe").arg("agy").stdin(Stdio::null()).stderr(Stdio::null())
        .creation_flags(0x0800_0000).output().ok()?;
    if !out.status.success() { return None; }
    String::from_utf8_lossy(&out.stdout).lines().map(str::trim).filter(|l| !l.is_empty())
        .map(std::path::PathBuf::from).find(|p| agy::is_executable(p))
}

#[cfg(not(windows))]
fn where_agy() -> Option<std::path::PathBuf> {
    None
}
