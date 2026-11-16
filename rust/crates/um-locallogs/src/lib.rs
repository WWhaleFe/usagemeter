//! um-locallogs: 로컬 AI 도구 기록(Claude Code·Codex·Antigravity)을 SQLite로 증분 수집한다.
//! 계획서 8장. 개인정보: 숫자·모델 이름·프로젝트 폴더 이름·세션 id·시각만 저장한다.

pub mod agy;
mod claude;
mod codex;
mod ingest;
pub mod paths;
pub mod protobuf;
pub mod query;
mod store;
#[cfg(test)]
mod testutil;
#[cfg(test)]
mod tests_ingest;
pub mod watcher;

pub use ingest::{IngestOptions, IngestStats, ingest_all, ingest_source};
pub use paths::Roots;
pub use store::Store;
pub use watcher::Watcher;

/// 기록 소스. DB에는 `as_str()` 값이 들어간다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    Claude,
    Codex,
    Agy,
}

impl Source {
    pub const ALL: [Source; 3] = [Source::Claude, Source::Codex, Source::Agy];

    pub fn as_str(self) -> &'static str {
        match self {
            Source::Claude => "claude",
            Source::Codex => "codex",
            Source::Agy => "agy",
        }
    }

    pub fn parse(s: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|x| x.as_str() == s)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("watch: {0}")]
    Watch(#[from] notify::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 경로의 마지막 폴더 이름. `/`와 `\` 모두 구분자로 본다.
pub fn basename(path: &str) -> Option<String> {
    let name = path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next()?;
    if name.is_empty() { None } else { Some(name.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basename_handles_both_separators() {
        assert_eq!(basename("/a/b/proj/").as_deref(), Some("proj"));
        assert_eq!(basename("C:\\work\\app").as_deref(), Some("app"));
        assert_eq!(basename(""), None);
    }
}
