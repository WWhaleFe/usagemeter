//! 업데이트 확인·내려받기(Swift UpdateChecker.swift, 계획서 5-7).
//! GitHub 최신 릴리스를 확인하고(시작 시 + 24시간마다, 설정으로 끔), 새 버전이면 메뉴에 표시한다.
//! 내려받기는 다운로드 폴더에 저장하고 위치만 보여준다(설치는 사용자가). Store판(`store` 기능)은 이 모듈을 쓰지 않는다.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

// 저장소가 플랫폼별로 나뉘었다: macOS는 usagemeter, Windows(GitHub판)는 usagemeter-windows.
#[cfg(not(windows))]
const API: &str = "https://api.github.com/repos/WWhaleFe/usagemeter/releases/latest";
#[cfg(not(windows))]
pub const PAGE: &str = "https://github.com/WWhaleFe/usagemeter/releases/latest";
#[cfg(windows)]
const API: &str = "https://api.github.com/repos/WWhaleFe/usagemeter-windows/releases/latest";
#[cfg(windows)]
pub const PAGE: &str = "https://github.com/WWhaleFe/usagemeter-windows/releases/latest";

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

/// Homebrew 등으로 번들이 교체되면 종료 후 다시 연다(Swift UpdateChecker.swift:62-85).
#[cfg(all(target_os = "macos", not(feature = "store")))]
pub fn watch_bundle_replacement(app: &tauri::AppHandle) {
    let Ok(exe) = std::env::current_exe() else { return };
    // 개발용 실행 파일은 제외하고 .app/Contents/MacOS 안에서 실행 중일 때만 감시한다.
    let Some(macos) = exe.parent().filter(|p| p.file_name().is_some_and(|n| n == "MacOS")) else { return };
    let Some(contents) = macos.parent().filter(|p| p.file_name().is_some_and(|n| n == "Contents")) else { return };
    let Some(bundle) = contents.parent().filter(|p| p.extension().is_some_and(|e| e == "app")) else { return };
    let plist = contents.join("Info.plist");
    let Some(running) = bundle_build(&plist) else { return };
    let bundle = bundle.to_path_buf();
    let app = app.clone();
    std::thread::spawn(move || {
        let mut last = None;
        loop {
            std::thread::sleep(Duration::from_secs(10));
            let on_disk = bundle_build(&plist);
            // 복사 중인 번들·읽기 실패는 제외한다. 바뀐 번호가 두 번 연속 같아야 한다.
            let replaced = replacement_ready(&running, last.as_deref(), on_disk.as_deref());
            last = on_disk;
            if !replaced {
                continue;
            }
            // 현재 프로세스가 끝난 뒤 보통 방식으로 연다. 새 인스턴스 옵션은 쓰지 않는다.
            match std::process::Command::new("/bin/sh")
                .args(["-c", "while kill -0 \"$0\" 2>/dev/null; do sleep 0.2; done; exec /usr/bin/open -g \"$1\""])
                .arg(std::process::id().to_string())
                .arg(&bundle)
                .spawn()
            {
                Ok(_) => {
                    app.exit(0);
                    return;
                }
                Err(e) => tracing::warn!("번들 교체 후 재실행 준비 실패: {e}"),
            }
        }
    });
}

#[cfg(all(target_os = "macos", not(feature = "store")))]
fn bundle_build(path: &std::path::Path) -> Option<String> {
    let plist = plist::Value::from_file(path).ok()?;
    plist.as_dictionary()?.get("CFBundleVersion")?.as_string().map(String::from)
}

#[cfg(all(target_os = "macos", not(feature = "store")))]
fn replacement_ready(running: &str, last: Option<&str>, on_disk: Option<&str>) -> bool {
    on_disk.is_some_and(|build| build != running && Some(build) == last)
}

#[cfg(all(test, target_os = "macos", not(feature = "store")))]
mod bundle_tests {
    use super::replacement_ready;

    #[test]
    fn waits_for_two_matching_new_builds() {
        assert!(!replacement_ready("1", None, Some("2")));
        assert!(!replacement_ready("1", Some("2"), Some("3")));
        assert!(replacement_ready("1", Some("2"), Some("2")));
        assert!(!replacement_ready("1", Some("1"), Some("1")));
        assert!(!replacement_ready("1", Some("2"), None));
        assert!(!replacement_ready("1", None, Some("2")));
    }
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
