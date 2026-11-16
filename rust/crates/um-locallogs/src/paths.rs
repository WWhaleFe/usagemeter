//! 기록 폴더 위치. 테스트에서 바꿀 수 있도록 값 객체로 둔다.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Roots {
    /// `~/.claude/projects`
    pub claude_projects: PathBuf,
    /// `~/.codex/sessions`
    pub codex_sessions: PathBuf,
    /// `~/.gemini/antigravity-cli`
    pub agy_dir: PathBuf,
}

impl Roots {
    /// 홈 폴더와 환경변수 조회 함수로 계산한다(macOS·Windows 공통, Windows 홈은 `%USERPROFILE%`).
    pub fn from_home(home: &Path, env: impl Fn(&str) -> Option<String>) -> Roots {
        let nonempty = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        let claude = nonempty("CLAUDE_CONFIG_DIR").unwrap_or_else(|| home.join(".claude"));
        let codex = nonempty("CODEX_HOME").unwrap_or_else(|| home.join(".codex"));
        Roots {
            claude_projects: claude.join("projects"),
            codex_sessions: codex.join("sessions"),
            agy_dir: home.join(".gemini").join("antigravity-cli"),
        }
    }

    /// 실제 환경 기준 기본 위치. 홈을 못 찾으면 `None`.
    pub fn default_roots() -> Option<Roots> {
        let home = directories::BaseDirs::new()?.home_dir().to_path_buf();
        Some(Roots::from_home(&home, |k| std::env::var(k).ok()))
    }

    pub fn agy_conversations(&self) -> PathBuf {
        self.agy_dir.join("conversations")
    }

    pub fn agy_summaries(&self) -> PathBuf {
        self.agy_dir.join("conversation_summaries.db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_overrides() {
        let home = Path::new("/h");
        let r = Roots::from_home(home, |_| None);
        assert_eq!(r.claude_projects, Path::new("/h/.claude/projects"));
        assert_eq!(r.codex_sessions, Path::new("/h/.codex/sessions"));
        let r = Roots::from_home(home, |k| match k {
            "CLAUDE_CONFIG_DIR" => Some("/c".into()),
            "CODEX_HOME" => Some("/x".into()),
            _ => None,
        });
        assert_eq!(r.claude_projects, Path::new("/c/projects"));
        assert_eq!(r.codex_sessions, Path::new("/x/sessions"));
    }
}
