//! 업데이트 확인·내려받기(Swift UpdateChecker.swift, 계획서 5-7).
//! GitHub 최신 릴리스를 확인하고(시작 시 + 24시간마다, 설정으로 끔), 새 버전이면 메뉴에 표시한다.
//! 내려받기는 다운로드 폴더에 저장하고 위치만 보여준다(설치는 사용자가). Store판(`store` 기능)은 이 모듈을 쓰지 않는다.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

const API: &str = "https://api.github.com/repos/WWhaleFe/usagemeter/releases/latest";
pub const PAGE: &str = "https://github.com/WWhaleFe/usagemeter/releases/latest";

#[derive(Debug, Clone, PartialEq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available { version: String, page: String, asset: Option<String> },
    Failed,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum DownloadState {
    #[default]
    Idle,
    Downloading,
    Done(PathBuf),
    Failed,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// 이 OS용 내려받기 파일: macOS는 .zip, Windows는 설치 파일(-setup.exe).
fn pick_asset(assets: &[Value]) -> Option<String> {
    let want: &[&str] = if cfg!(windows) { &["-setup.exe", ".msi"] } else { &[".zip"] };
    assets.iter().filter_map(|a| a["browser_download_url"].as_str()).find(|u| want.iter().any(|w| u.ends_with(w))).map(String::from)
}

/// GitHub 최신 릴리스 확인. (Swift UpdateChecker.swift:100-150, 15초 제한)
pub fn check() -> UpdateState {
    let client = match reqwest::blocking::Client::builder().timeout(Duration::from_secs(15)).user_agent(format!("UsageMeter/{}", current_version())).build() {
        Ok(c) => c,
        Err(_) => return UpdateState::Failed,
    };
    let Ok(resp) = client.get(API).header("Accept", "application/vnd.github+json").send() else { return UpdateState::Failed };
    if !resp.status().is_success() {
        return UpdateState::Failed;
    }
    let Ok(v) = resp.json::<Value>() else { return UpdateState::Failed };
    let Some(tag) = v["tag_name"].as_str() else { return UpdateState::Failed };
    if um_core::update::is_newer(tag, current_version()) {
        UpdateState::Available {
            version: tag.trim_start_matches(['v', 'V']).to_string(),
            page: v["html_url"].as_str().unwrap_or(PAGE).to_string(),
            asset: v["assets"].as_array().and_then(|a| pick_asset(a)),
        }
    } else {
        UpdateState::UpToDate
    }
}

/// 다운로드 폴더에 저장한다(이름이 겹치면 -1, -2 …). (Swift UpdateChecker.swift:160-200)
pub fn download(url: &str) -> DownloadState {
    let Some(dir) = directories::UserDirs::new().and_then(|u| u.download_dir().map(PathBuf::from)) else { return DownloadState::Failed };
    let name = url.rsplit('/').next().unwrap_or("UsageMeter.zip").to_string();
    let mut path = dir.join(&name);
    let (stem, ext) = name.rsplit_once('.').map(|(a, b)| (a.to_string(), format!(".{b}"))).unwrap_or((name.clone(), String::new()));
    let mut n = 1;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}{ext}"));
        n += 1;
    }
    let client = match reqwest::blocking::Client::builder().timeout(Duration::from_secs(300)).user_agent(format!("UsageMeter/{}", current_version())).build() {
        Ok(c) => c,
        Err(_) => return DownloadState::Failed,
    };
    let Ok(resp) = client.get(url).send() else { return DownloadState::Failed };
    if !resp.status().is_success() {
        return DownloadState::Failed;
    }
    let Ok(bytes) = resp.bytes() else { return DownloadState::Failed };
    if std::fs::write(&path, &bytes).is_err() {
        return DownloadState::Failed;
    }
    DownloadState::Done(path)
}

/// 파일 위치를 탐색기에서 보여준다(macOS Finder, Windows 탐색기).
pub fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(format!("/select,{}", path.display())).spawn();
}
