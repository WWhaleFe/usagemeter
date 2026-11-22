//! SQLite 저장소. 스키마와 소스별 설정·삭제.

use crate::{Result, Source};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::path::Path;

const SCHEMA_VERSION: &str = "2";

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE IF NOT EXISTS settings(source TEXT PRIMARY KEY, enabled INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS ingest_files(
    path TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    size INTEGER NOT NULL,
    mtime REAL NOT NULL,
    "offset" INTEGER NOT NULL,
    ctx_model TEXT,
    ctx_project TEXT,
    ctx_session TEXT
);
CREATE TABLE IF NOT EXISTS token_events(
    t REAL NOT NULL,
    source TEXT NOT NULL,
    model TEXT,
    project TEXT,
    session TEXT,
    input INTEGER NOT NULL,
    output INTEGER NOT NULL,
    cache_read INTEGER NOT NULL,
    cache_write INTEGER NOT NULL,
    reasoning INTEGER NOT NULL,
    sidechain INTEGER NOT NULL DEFAULT 0,
    dedup_key TEXT NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS token_events_t ON token_events(t);
CREATE TABLE IF NOT EXISTS codex_limits(
    t REAL NOT NULL,
    primary_used REAL,
    primary_window_min INTEGER,
    primary_resets_at INTEGER,
    secondary_used REAL,
    secondary_window_min INTEGER,
    secondary_resets_at INTEGER
);
-- NULL도 같은 값으로 보아 반복된 줄을 한 번만 저장한다.
CREATE UNIQUE INDEX IF NOT EXISTS codex_limits_uniq
    ON codex_limits(t, COALESCE(primary_used,-1), COALESCE(secondary_used,-1));
CREATE INDEX IF NOT EXISTS codex_limits_t ON codex_limits(t);
CREATE TABLE IF NOT EXISTS agy_events(
    conversation TEXT NOT NULL,
    idx INTEGER NOT NULL,
    t REAL NOT NULL,
    model TEXT,
    label TEXT,
    project TEXT,
    agy_f1 INTEGER, agy_f2 INTEGER, agy_f3 INTEGER, agy_f5 INTEGER,
    agy_f6 INTEGER, agy_f9 INTEGER, agy_f10 INTEGER,
    PRIMARY KEY(conversation, idx)
);
CREATE INDEX IF NOT EXISTS agy_events_t ON agy_events(t);
"#;

/// 버전 1 -> 2: `agy_events.label` 열을 더하고, 요청 시각을 다시 읽도록 agy 기록을 비운다.
fn migrate_v1(conn: &Connection) -> Result<()> {
    // 열이 이미 있으면 오류가 나므로 무시한다.
    let _ = conn.execute("ALTER TABLE agy_events ADD COLUMN label TEXT", []);
    conn.execute("DELETE FROM agy_events", [])?;
    conn.execute("DELETE FROM ingest_files WHERE source='agy'", [])?;
    conn.execute("UPDATE meta SET value=?1 WHERE key='schema_version'", [SCHEMA_VERSION])?;
    Ok(())
}

/// 파일별 읽은 위치. `ctx_*`는 Codex처럼 앞줄에서 정해지는 문맥을 이어받기 위한 값(개인정보 아님).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileState {
    pub size: u64,
    pub mtime: f64,
    pub offset: u64,
    pub ctx_model: Option<String>,
    pub ctx_project: Option<String>,
    pub ctx_session: Option<String>,
}

/// `token_events` 한 행.
#[derive(Debug, Clone, Default)]
pub struct TokenEvent {
    pub t: f64,
    pub source: &'static str,
    pub model: Option<String>,
    pub project: Option<String>,
    pub session: Option<String>,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub reasoning: u64,
    pub sidechain: bool,
    pub dedup_key: String,
}

pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        Store::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Store> {
        Store::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .or_else(|_| conn.execute_batch("PRAGMA synchronous=NORMAL;"))?;
        // 새 DB인지는 스키마를 만들기 전에 본다(새 DB는 이전하지 않는다).
        let existed: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='meta')",
            [],
            |r| r.get(0),
        )?;
        conn.execute_batch(SCHEMA)?;
        conn.execute(
            "INSERT OR IGNORE INTO meta(key,value) VALUES('schema_version',?1)",
            [SCHEMA_VERSION],
        )?;
        let version: Option<String> =
            conn.query_row("SELECT value FROM meta WHERE key='schema_version'", [], |r| r.get(0)).optional()?;
        if existed && version.as_deref() == Some("1") {
            migrate_v1(&conn)?;
        }
        // 소스별 설정은 기본 꺼짐(결정 D4)
        for s in Source::ALL {
            conn.execute(
                "INSERT OR IGNORE INTO settings(source,enabled) VALUES(?1,0)",
                [s.as_str()],
            )?;
        }
        Ok(Store { conn })
    }

    pub fn set_enabled(&self, source: Source, enabled: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings(source,enabled) VALUES(?1,?2)
             ON CONFLICT(source) DO UPDATE SET enabled=excluded.enabled",
            params![source.as_str(), enabled],
        )?;
        Ok(())
    }

    pub fn is_enabled(&self, source: Source) -> Result<bool> {
        let v: Option<bool> = self
            .conn
            .query_row(
                "SELECT enabled FROM settings WHERE source=?1",
                [source.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v.unwrap_or(false))
    }

    pub fn enabled_sources(&self) -> Result<Vec<Source>> {
        let mut out = Vec::new();
        for s in Source::ALL {
            if self.is_enabled(s)? {
                out.push(s);
            }
        }
        Ok(out)
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key,value) VALUES(?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// 한 소스의 수집 기록과 읽은 위치를 지운다(설정은 유지). "수집한 기록 지우기".
    pub fn clear_source(&mut self, source: Source) -> Result<()> {
        let tx = self.conn.transaction()?;
        let s = source.as_str();
        tx.execute("DELETE FROM token_events WHERE source=?1", [s])?;
        tx.execute("DELETE FROM ingest_files WHERE source=?1", [s])?;
        match source {
            Source::Codex => {
                tx.execute("DELETE FROM codex_limits", [])?;
            }
            Source::Agy => {
                tx.execute("DELETE FROM agy_events", [])?;
            }
            Source::Claude => {}
        }
        tx.commit()?;
        Ok(())
    }

    /// 모든 수집 기록을 지운다(설정·메타는 유지).
    pub fn clear_all(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        for t in ["token_events", "ingest_files", "codex_limits", "agy_events"] {
            tx.execute(&format!("DELETE FROM {t}"), [])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn file_state(&self, path: &str) -> Result<Option<FileState>> {
        Ok(self
            .conn
            .query_row(
                r#"SELECT size,mtime,"offset",ctx_model,ctx_project,ctx_session
                   FROM ingest_files WHERE path=?1"#,
                [path],
                |r| {
                    Ok(FileState {
                        size: r.get::<_, i64>(0)? as u64,
                        mtime: r.get(1)?,
                        offset: r.get::<_, i64>(2)? as u64,
                        ctx_model: r.get(3)?,
                        ctx_project: r.get(4)?,
                        ctx_session: r.get(5)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn count_events(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM token_events", [], |r| r.get(0))?)
    }
}

pub(crate) fn save_file_state(
    tx: &Transaction,
    path: &str,
    source: Source,
    st: &FileState,
) -> Result<()> {
    tx.prepare_cached(
        r#"INSERT INTO ingest_files(path,source,size,mtime,"offset",ctx_model,ctx_project,ctx_session)
           VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
           ON CONFLICT(path) DO UPDATE SET size=excluded.size, mtime=excluded.mtime,
             "offset"=excluded."offset", ctx_model=excluded.ctx_model,
             ctx_project=excluded.ctx_project, ctx_session=excluded.ctx_session"#,
    )?
    .execute(params![
        path,
        source.as_str(),
        st.size as i64,
        st.mtime,
        st.offset as i64,
        st.ctx_model,
        st.ctx_project,
        st.ctx_session
    ])?;
    Ok(())
}

/// 중복(`dedup_key`)이면 무시하고 새로 들어갔는지를 돌려준다. 토큰이 모두 0이면 저장하지 않는다.
pub(crate) fn insert_event(tx: &Transaction, e: &TokenEvent) -> Result<bool> {
    if e.input + e.output + e.cache_read + e.cache_write + e.reasoning == 0 {
        return Ok(false);
    }
    let n = tx
        .prepare_cached(
            "INSERT OR IGNORE INTO token_events
             (t,source,model,project,session,input,output,cache_read,cache_write,reasoning,sidechain,dedup_key)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        )?
        .execute(params![
            e.t,
            e.source,
            e.model,
            e.project,
            e.session,
            e.input as i64,
            e.output as i64,
            e.cache_read as i64,
            e.cache_write as i64,
            e.reasoning as i64,
            e.sidechain,
            e.dedup_key
        ])?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_off_and_toggle() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.enabled_sources().unwrap().is_empty());
        s.set_enabled(Source::Codex, true).unwrap();
        assert_eq!(s.enabled_sources().unwrap(), vec![Source::Codex]);
        s.set_enabled(Source::Codex, false).unwrap();
        assert!(!s.is_enabled(Source::Codex).unwrap());
        assert_eq!(s.get_meta("schema_version").unwrap().as_deref(), Some("2"));
    }

    #[test]
    fn migrates_v1_agy_events() {
        let dir = crate::testutil::TempDir::new();
        let path = dir.path().join("old.db");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(
                "CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT);
                 INSERT INTO meta VALUES('schema_version','1');
                 CREATE TABLE ingest_files(path TEXT PRIMARY KEY, source TEXT NOT NULL, size INTEGER NOT NULL,
                   mtime REAL NOT NULL, \"offset\" INTEGER NOT NULL, ctx_model TEXT, ctx_project TEXT, ctx_session TEXT);
                 INSERT INTO ingest_files VALUES('a','agy',1,1,0,NULL,NULL,NULL);
                 INSERT INTO ingest_files VALUES('b','claude',1,1,0,NULL,NULL,NULL);
                 CREATE TABLE agy_events(conversation TEXT NOT NULL, idx INTEGER NOT NULL, t REAL NOT NULL,
                   model TEXT, project TEXT, agy_f1 INTEGER, agy_f2 INTEGER, agy_f3 INTEGER, agy_f5 INTEGER,
                   agy_f6 INTEGER, agy_f9 INTEGER, agy_f10 INTEGER, PRIMARY KEY(conversation, idx));
                 INSERT INTO agy_events(conversation,idx,t) VALUES('c',0,1);",
            )
            .unwrap();
        }
        let s = Store::open(&path).unwrap();
        let n: i64 = s.conn.query_row("SELECT COUNT(*) FROM agy_events", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
        s.conn.execute("INSERT INTO agy_events(conversation,idx,t,label) VALUES('c',0,1,'x')", []).unwrap();
        let files: i64 = s.conn.query_row("SELECT COUNT(*) FROM ingest_files", [], |r| r.get(0)).unwrap();
        assert_eq!(files, 1);
        assert_eq!(s.get_meta("schema_version").unwrap().as_deref(), Some("2"));
    }
}
