//! agy 활동 감시와 보조 조회(Swift AgyIntegration.swift:239-360, RefreshScheduler.swift:26-38).

use std::time::{Duration, Instant};
use notify::{RecursiveMode, Watcher};
use tauri::AppHandle;

fn activity(path: &std::path::Path) -> bool {
    let p = path.to_string_lossy();
    (p.contains("/conversations/") && (p.ends_with(".db") || p.ends_with(".db-wal")))
        || p.ends_with("/transcript.jsonl") || p.ends_with("/transcript_full.jsonl") || p.ends_with("/history.jsonl")
}

pub fn start(app: &AppHandle) {
    let Some(home) = directories::BaseDirs::new() else { return };
    let base = home.home_dir().join(".gemini").join("antigravity-cli");
    let h = app.clone();
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(mut watcher) = notify::recommended_watcher(tx) else { return };
        // 시작 때 폴더가 없어도 나중에 CLI를 설치하면 감시를 시작한다.
        while !base.is_dir() { std::thread::sleep(Duration::from_secs(3)); }
        if watcher.watch(&base, RecursiveMode::Recursive).is_err() { return; }
        let mut last_activity = None::<Instant>;
        let mut last_trigger = None::<Instant>;
        let mut last_fast = Instant::now();
        let mut leading = None::<Instant>;
        let mut trailing = None::<Instant>;
        let mut follow_up = None::<Instant>;
        loop {
            if let Ok(Ok(event)) = rx.recv_timeout(Duration::from_millis(100))
                && !matches!(event.kind, notify::EventKind::Access(_))
                && event.paths.iter().any(|p| activity(p))
            {
                let now = Instant::now();
                last_activity = Some(now);
                if leading.is_none() && last_trigger.is_none_or(|t| t.elapsed() >= Duration::from_secs(7)) {
                    leading = Some(now + Duration::from_millis(1200));
                }
                trailing = Some(now + Duration::from_millis(2500));
            }
            let now = Instant::now();
            let mut refresh = false;
            if leading.is_some_and(|t| now >= t) { leading = None; refresh = true; }
            if trailing.is_some_and(|t| now >= t) {
                trailing = None;
                follow_up = Some(now + Duration::from_millis(6500));
                refresh = true;
            }
            if follow_up.is_some_and(|t| now >= t) { follow_up = None; refresh = true; }
            if last_activity.is_some_and(|t| t.elapsed() < Duration::from_secs(600)) && last_fast.elapsed() >= Duration::from_secs(25) {
                last_fast = now;
                refresh = true;
            }
            if refresh {
                last_trigger = Some(now);
                let h = h.clone();
                std::thread::spawn(move || crate::refresh_ids(&h, &["gemini".into()]));
            }
        }
    });
}
