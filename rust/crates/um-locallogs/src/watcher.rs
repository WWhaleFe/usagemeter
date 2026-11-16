//! `notify`로 세 폴더를 감시하고 2초 디바운스 후 바뀐 소스만 콜백으로 알린다.
//! 수집 자체는 콜백에서 `ingest_source`를 부르면 된다(타이밍은 테스트하지 않는다).

use crate::{Result, Roots, Source};
use notify::{RecursiveMode, Watcher as _};
use std::collections::HashSet;
use std::sync::mpsc;
use std::time::Duration;

pub const DEBOUNCE: Duration = Duration::from_secs(2);

/// 감시 중인 동안만 살아 있다. 버리면 감시와 스레드가 끝난다.
pub struct Watcher {
    _watcher: notify::RecommendedWatcher,
}

/// 바뀐 경로가 어느 소스에 속하는지.
fn classify(roots: &Roots, p: &std::path::Path) -> Option<Source> {
    if p.starts_with(&roots.claude_projects) {
        Some(Source::Claude)
    } else if p.starts_with(&roots.codex_sessions) {
        Some(Source::Codex)
    } else if p.starts_with(&roots.agy_dir) {
        Some(Source::Agy)
    } else {
        None
    }
}

impl Watcher {
    /// 없는 폴더는 건너뛴다. `on_change`는 별도 스레드에서 호출된다.
    pub fn start(
        roots: Roots,
        debounce: Duration,
        on_change: impl Fn(Vec<Source>) + Send + 'static,
    ) -> Result<Watcher> {
        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
        let mut w = notify::recommended_watcher(tx)?;
        for dir in [&roots.claude_projects, &roots.codex_sessions, &roots.agy_dir] {
            if dir.is_dir()
                && let Err(e) = w.watch(dir, RecursiveMode::Recursive)
            {
                tracing::warn!("감시 시작 실패 {}: {e}", dir.display());
            }
        }
        std::thread::spawn(move || {
            // 채널이 닫히면(Watcher drop) 끝난다.
            while let Ok(first) = rx.recv() {
                let mut changed = HashSet::new();
                let mut add = |ev: notify::Result<notify::Event>| {
                    if let Ok(ev) = ev {
                        changed.extend(ev.paths.iter().filter_map(|p| classify(&roots, p)));
                    }
                };
                add(first);
                // 조용해질 때까지(마지막 이벤트 후 debounce) 모은다.
                while let Ok(ev) = rx.recv_timeout(debounce) {
                    add(ev);
                }
                if !changed.is_empty() {
                    let mut v: Vec<Source> = changed.into_iter().collect();
                    v.sort_by_key(|s| s.as_str());
                    on_change(v);
                }
            }
        });
        Ok(Watcher { _watcher: w })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn classify_by_root() {
        let r = Roots::from_home(Path::new("/h"), |_| None);
        assert_eq!(classify(&r, Path::new("/h/.claude/projects/a/x.jsonl")), Some(Source::Claude));
        assert_eq!(classify(&r, Path::new("/h/.codex/sessions/2026/a.jsonl")), Some(Source::Codex));
        assert_eq!(classify(&r, Path::new("/h/.gemini/antigravity-cli/conversations/a.db")), Some(Source::Agy));
        assert_eq!(classify(&r, Path::new("/tmp/x")), None);
    }
}
