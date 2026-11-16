//! Antigravity CLI(`agy`) 쿼터 조회의 순수 부분. (Swift AgyIntegration.swift `AgyQuotaFetcher`)
//!
//! 프로세스 실행·12초 제한·동시 호출 합치기는 호출자(앱) 몫이다. 여기서는 실행 파일 후보,
//! 명령줄, 출력 파서만 둔다.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;
use um_core::model::{UsageModelBucket, UsageSnapshot, UsageStatus};

use crate::parse::parse_iso;

/// 실행 제한 시간(초). (AgyIntegration.swift `runProcess` timeoutSeconds)
pub const TIMEOUT_SECS: u64 = 12;
/// 같은 리셋 주기 안에서 이만큼 미만의 상승은 레플리카 차이로 보고 무시한다.
pub const MONOTONIC_EPSILON: f64 = 0.12;
/// Gemini 대표 항목의 플랜 라벨.
pub const PLAN_LABEL: &str = "Antigravity";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Unix,
    Windows,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(windows) { Os::Windows } else { Os::Unix }
    }
}

/// 후보 경로 목록(우선순위 순). 존재 여부는 보지 않는다.
///
/// - Unix: `~/.local/bin/agy`, `~/.gemini/antigravity-cli/bin/agy`, `/opt/homebrew/bin`, `/usr/local/bin`,
///   `/usr/bin`, 그 뒤 PATH 각 디렉터리의 `agy`. (AgyIntegration.swift `findAgyExecutable`)
/// - Windows: `%USERPROFILE%\.local\bin\agy.exe`, `%USERPROFILE%\.gemini\antigravity-cli\bin\agy.exe|agy.cmd`,
///   그 뒤 PATH 각 디렉터리 x PATHEXT. (계획서 6장)
pub fn candidates(os: Os, home: &str, path_env: Option<&str>, pathext: Option<&str>) -> Vec<PathBuf> {
    let mut out: Vec<String> = Vec::new();
    match os {
        Os::Unix => {
            out.push(format!("{home}/.local/bin/agy"));
            out.push(format!("{home}/.gemini/antigravity-cli/bin/agy"));
            for d in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
                out.push(format!("{d}/agy"));
            }
            for dir in path_env.unwrap_or("").split(':').filter(|d| !d.is_empty()) {
                out.push(format!("{dir}/agy"));
            }
        }
        Os::Windows => {
            out.push(format!("{home}\\.local\\bin\\agy.exe"));
            out.push(format!("{home}\\.gemini\\antigravity-cli\\bin\\agy.exe"));
            out.push(format!("{home}\\.gemini\\antigravity-cli\\bin\\agy.cmd"));
            let exts: Vec<&str> = pathext.filter(|p| !p.is_empty()).unwrap_or(".COM;.EXE;.BAT;.CMD").split(';').filter(|e| !e.is_empty()).collect();
            for dir in path_env.unwrap_or("").split(';').map(|d| d.trim_matches('"')).filter(|d| !d.is_empty()) {
                let dir = dir.trim_end_matches('\\');
                for ext in &exts {
                    out.push(format!("{dir}\\agy{ext}"));
                }
            }
        }
    }
    out.into_iter().map(PathBuf::from).collect()
}

/// 실행 가능한 파일인가(Unix는 실행 비트, Windows는 일반 파일).
pub fn is_executable(p: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(p) else { return false };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

/// 현재 환경에서 첫 번째 실행 가능한 `agy`를 찾는다.
pub fn find_executable(os: Os, home: &str) -> Option<PathBuf> {
    let path_env = std::env::var("PATH").ok();
    let pathext = std::env::var("PATHEXT").ok();
    candidates(os, home, path_env.as_deref(), pathext.as_deref()).into_iter().find(|p| is_executable(p))
}

/// 명령줄 인자. `--log-file`로 로그 파일이 남지 않게 한다(NUL/`/dev/null`).
pub fn args(os: Os) -> Vec<String> {
    let null = if os == Os::Windows { "NUL" } else { "/dev/null" };
    ["--log-file", null, "-p", "/usage", "--output-format", "json"].map(String::from).to_vec()
}

/// 작업 폴더. Unix `/tmp`, Windows는 `%TEMP%`(없으면 None이라 호출자가 시스템 임시 폴더를 쓴다).
pub fn work_dir(os: Os, temp_env: Option<&str>) -> Option<String> {
    match os {
        Os::Unix => Some("/tmp".into()),
        Os::Windows => temp_env.map(String::from),
    }
}

/// 자식 프로세스 PATH. GUI 앱은 PATH가 좁아 Unix에서는 주요 경로를 앞에 붙인다. (AgyIntegration.swift `runProcess`)
pub fn child_path_unix(home: &str, existing: Option<&str>) -> String {
    let extra = format!("{home}/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin");
    match existing.filter(|e| !e.is_empty()) {
        Some(e) => format!("{extra}:{e}"),
        None => extra,
    }
}

#[derive(Debug)]
struct Latest {
    ratio: f64,
    reset_time: String,
}

/// `agy -p /usage --output-format json` 출력 파서. 버킷별 최소 잔여율을 기억해 단조 규칙을 적용한다.
///
/// 같은 `reset_time` 안에서 직전보다 0.12 미만으로 오른 값은 직전 값을 유지한다(백엔드 레플리카 간 캐시 차이로
/// 잠깐 튀는 현상 방지). `reset_time`이 바뀌었거나 0.12 이상 오르면 새 값을 받는다. (AgyIntegration.swift `parseUsageJSON`)
#[derive(Debug, Default)]
pub struct AgyParser {
    monotonic: HashMap<String, Latest>,
}

/// 대표 잔여율 후보(잔여율, 리셋 시각).
type Cand = (f64, Option<DateTime<Utc>>);

impl AgyParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// 출력 바이트 -> 스냅샷. 상태가 SUCCESS가 아니거나 그룹이 없으면 None(웹 조회로 폴백).
    pub fn parse(&mut self, data: &[u8], provider_id: &str, now: DateTime<Utc>) -> Option<UsageSnapshot> {
        let root: Value = serde_json::from_slice(data).ok()?;
        if root.get("status").and_then(Value::as_str) != Some("SUCCESS") {
            return None;
        }
        let groups = root.pointer("/command/data/groups")?.as_array().filter(|g| !g.is_empty())?;

        let mut gemini_5h: Option<Cand> = None;
        let mut gemini_weekly: Option<Cand> = None;
        let mut model_buckets: Vec<UsageModelBucket> = Vec::new();
        let mut raw_buckets: BTreeMap<String, f64> = BTreeMap::new();
        let mut urgent: Vec<Cand> = Vec::new();

        for g in groups {
            let group_name = g.get("name").and_then(Value::as_str).unwrap_or("");
            let group_lc = group_name.to_lowercase();
            let Some(buckets) = g.get("buckets").and_then(Value::as_array) else { continue };
            let is_gemini_core = group_lc.contains("gemini models");

            for b in buckets {
                let Some(rem) = b.get("remaining_fraction").and_then(Value::as_f64) else { continue };
                let raw_ratio = rem.clamp(0.0, 1.0);
                let bid = b.get("id").and_then(Value::as_str).unwrap_or("");
                let window = b.get("window").and_then(Value::as_str).unwrap_or("").to_lowercase();
                let reset_str = b.get("reset_time").and_then(Value::as_str).unwrap_or("");
                let reset = if reset_str.is_empty() { None } else { parse_iso(reset_str) };
                let is_5h = window == "5h" || bid.ends_with("-5h");

                let ratio = match self.monotonic.get(bid) {
                    Some(prev) if !bid.is_empty() && prev.reset_time == reset_str && raw_ratio > prev.ratio && raw_ratio - prev.ratio < MONOTONIC_EPSILON => {
                        prev.ratio
                    }
                    _ => {
                        if !bid.is_empty() {
                            self.monotonic.insert(bid.to_string(), Latest { ratio: raw_ratio, reset_time: reset_str.to_string() });
                        }
                        raw_ratio
                    }
                };
                if !bid.is_empty() {
                    raw_buckets.insert(bid.to_string(), ratio);
                }

                if bid == "gemini-5h" || (is_gemini_core && is_5h) {
                    gemini_5h = Some((ratio, reset));
                    urgent.push((ratio, reset));
                } else if bid == "gemini-weekly" || (is_gemini_core && !is_5h) {
                    gemini_weekly = Some((ratio, reset));
                    urgent.push((ratio, reset));
                } else {
                    // 별도 모델 그룹(예: "Gemini 4 Argon", "Claude and GPT models")
                    let label = if group_lc.contains("claude") {
                        "Claude·GPT (agy)".to_string()
                    } else if !group_name.is_empty() {
                        group_name.to_string()
                    } else {
                        b.get("name").and_then(Value::as_str).unwrap_or(bid).to_string()
                    };
                    model_buckets.push(UsageModelBucket { label, remaining_ratio: ratio, reset_at: reset, is_five_hour: is_5h });
                    // Gemini 계열 버킷은 Gemini 대표 잔여율(가장 급한 비율) 후보에 포함.
                    if bid.to_lowercase().contains("gemini") || group_lc.contains("gemini") {
                        urgent.push((ratio, reset));
                    }
                }
            }
        }

        // Gemini 후보가 없으면 전체 버킷 중 가장 급한 것을 쓴다.
        if urgent.is_empty() {
            urgent.extend(model_buckets.iter().map(|b| (b.remaining_ratio, b.reset_at)));
        }
        // 동률이면 먼저 나온 것(Swift `min(by:)`와 같다).
        let most_urgent = urgent.iter().copied().min_by(|a, b| a.0.total_cmp(&b.0))?;
        let primary_5h = gemini_5h.unwrap_or(most_urgent);

        Some(UsageSnapshot {
            id: provider_id.to_string(),
            remaining_ratio: most_urgent.0,
            secondary_ratio: gemini_weekly.map(|w| w.0),
            opus_ratio: None,
            reset_at: most_urgent.1.or(primary_5h.1),
            secondary_reset_at: gemini_weekly.and_then(|w| w.1),
            opus_reset_at: None,
            status: UsageStatus::Ok,
            last_updated: now,
            plan: Some(PLAN_LABEL.to_string()),
            model_buckets: if model_buckets.is_empty() { None } else { Some(model_buckets) },
            five_hour_ratio: Some(primary_5h.0),
            five_hour_reset_at: primary_5h.1,
            raw_buckets: if raw_buckets.is_empty() { None } else { Some(raw_buckets) },
        })
    }
}
