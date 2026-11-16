//! 사용 이력(SQLite). (Swift HistoryStore.swift)
//!
//! Swift는 35일 치 표본 전체(약 1.2만 개)를 메모리에 올리고 저장할 때마다 JSON을 통째로 다시 쓴다.
//! 여기서는 값 하나를 한 행으로 두고, 조회는 SQL로 기간·AI·항목을 걸러서 필요한 만큼만 읽는다.
//!
//! 스키마 선택: 값 하나당 한 행의 평평한 표 하나.
//! `samples(provider, kind, key, t, value)`, 기본키 `(provider, kind, key, t)`, `WITHOUT ROWID`.
//! - `kind`: 0 = 대표 잔여율(key ''), 1 = 세부 버킷(`rawBuckets`), 2 = 세부 항목(`subItems`).
//! - 기본키가 곧 "AI·종류·키별 시계열" 인덱스라 시계열 조회는 기간 범위 스캔 한 번이고, 따로 인덱스를 두지 않는다.
//!   같은 키가 같은 시각에 두 번 들어갈 수 없어 가져오기(import)가 `INSERT OR IGNORE`만으로 멱등이 된다.
//! - 보조 인덱스 `(t, provider, kind)`: 보관 기간 삭제, 최초 시각, "이 시각에 이 AI의 항목 기록이 있었나"(구버전 표본 판정).
//! - 표본을 따로 두지 않으므로(같은 `t`를 공유하는 행이 한 표본) 행당 중복은 `provider` 문자열뿐이다.
//!   표본 11.8k개 기준 수만 행이라 정규화(표본 표 + 값 표)의 조인 비용보다 단순함이 낫다고 봤다.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, SeqAccess, Visitor};

use crate::analytics::{Event, MIN_DROP, Point};

/// 35일 보관(분석 대시보드 한 달 뷰용). (Swift `maxAge`)
pub const MAX_AGE_SECS: f64 = 35.0 * 24.0 * 3600.0;
/// 표본 간 최소 간격(초). (Swift `minGap`)
pub const MIN_GAP_SECS: f64 = 45.0;
/// Swift 기준 시각(2001-01-01)과 Unix 시각의 차(초).
pub const SWIFT_REFERENCE_OFFSET: f64 = 978_307_200.0;
/// 보관 기간 삭제를 돌리는 최소 간격(초). 삭제는 인덱스 범위 삭제라 싸지만 기록마다 돌릴 필요는 없다.
const PRUNE_EVERY_SECS: f64 = 3600.0;

const KIND_RATIO: i64 = 0;
const KIND_BUCKET: i64 = 1;
const KIND_ITEM: i64 = 2;

/// 이력 오류.
#[derive(Debug)]
pub enum HistoryError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    Io(std::io::Error),
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "이력 DB 오류: {e}"),
            Self::Json(e) => write!(f, "이력 JSON 오류: {e}"),
            Self::Io(e) => write!(f, "이력 파일 오류: {e}"),
        }
    }
}

impl std::error::Error for HistoryError {}

impl From<rusqlite::Error> for HistoryError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
impl From<serde_json::Error> for HistoryError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<std::io::Error> for HistoryError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, HistoryError>;

/// AI id -> 값. (Swift `[String: Double]`)
pub type Ratios = BTreeMap<String, f64>;
/// AI id -> 키 -> 값. (Swift `[String: [String: Double]]`)
pub type Nested = BTreeMap<String, BTreeMap<String, f64>>;

/// 소진 예측 결과. (Swift `PaceInfo`)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaceInfo {
    /// 이 속도면 잔여 0이 되는 예상 시각(Unix 초).
    pub depletion: f64,
    /// 시간당 잔여율 하락(양수 = 소진 중).
    pub rate_per_hour: f64,
}

/// 가져오기 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportStats {
    /// 파일에 있던 표본 수.
    pub samples: usize,
    /// 새로 들어간 행 수(다시 가져오면 0).
    pub rows_inserted: usize,
}

/// 직전에 기록한 표본(변화 판정용). 전체 이력은 메모리에 두지 않고 마지막 한 표본만 둔다.
#[derive(Debug, Default)]
struct LastSample {
    ratios: Ratios,
    buckets: Nested,
    items: Nested,
}

pub struct HistoryDb {
    conn: Connection,
    last: Option<LastSample>,
    /// 마지막 기록 시각. 프로세스 시작 직후에는 None(Swift `lastRecord = .distantPast`).
    last_record: Option<f64>,
    last_prune: Option<f64>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS samples (
    provider TEXT NOT NULL,
    kind     INTEGER NOT NULL,
    key      TEXT NOT NULL,
    t        REAL NOT NULL,
    value    REAL NOT NULL,
    PRIMARY KEY (provider, kind, key, t)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS samples_t ON samples (t, provider, kind);
";

/// 구버전 표본(세부 항목 기록 전)에서 되살린 "5h" 값: (t, 값) 행. 값이 NULL이면 Swift에서 nil.
/// 그 시각에 그 AI의 항목 행이 하나도 없는 표본만 해당. 버킷이 있으면 gemini-5h, 없으면 대표 잔여율.
/// (Swift `legacyItem`) 바인딩: ?1 = provider, ?2 = since.
const LEGACY_5H: &str = "
SELECT r.t AS t,
       CASE WHEN EXISTS (SELECT 1 FROM samples b WHERE b.t = r.t AND b.provider = r.provider AND b.kind = 1)
            THEN (SELECT b.value FROM samples b
                  WHERE b.provider = r.provider AND b.kind = 1 AND b.key = 'gemini-5h' AND b.t = r.t)
            ELSE r.value END AS v
FROM samples r
WHERE r.provider = ?1 AND r.kind = 0 AND r.t >= ?2
  AND NOT EXISTS (SELECT 1 FROM samples i WHERE i.t = r.t AND i.provider = r.provider AND i.kind = 2)";

impl HistoryDb {
    /// 파일 DB를 연다(없으면 만든다). WAL 모드.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    /// 메모리 DB(테스트용).
    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        // 메모리 목표: 페이지 캐시를 1MB로 제한(기본은 2MB).
        conn.pragma_update(None, "cache_size", -1024)?;
        conn.execute_batch(SCHEMA)?;
        let mut db = Self {
            conn,
            last: None,
            last_record: None,
            last_prune: None,
        };
        db.reload_last()?;
        Ok(db)
    }

    /// DB의 가장 최근 표본을 `last`로 읽는다.
    fn reload_last(&mut self) -> Result<()> {
        let Some(t) = self.conn.query_row("SELECT MAX(t) FROM samples", [], |r| {
            r.get::<_, Option<f64>>(0)
        })?
        else {
            self.last = None;
            return Ok(());
        };
        let mut last = LastSample::default();
        let mut stmt = self
            .conn
            .prepare_cached("SELECT provider, kind, key, value FROM samples WHERE t = ?1")?;
        let rows = stmt.query_map([t], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, f64>(3)?,
            ))
        })?;
        for row in rows {
            let (p, kind, key, v) = row?;
            match kind {
                KIND_RATIO => {
                    last.ratios.insert(p, v);
                }
                KIND_BUCKET => {
                    last.buckets.entry(p).or_default().insert(key, v);
                }
                _ => {
                    last.items.entry(p).or_default().insert(key, v);
                }
            }
        }
        drop(stmt);
        self.last = Some(last);
        Ok(())
    }

    /// 표본 기록. 기록했으면 true. (Swift `record`)
    /// - 활성 AI가 없으면(`ratios` 비어 있음) 건너뛴다.
    /// - 직전 표본 대비 변화(> 0.0005)나 새 키가 있으면 즉시, 아니면 마지막 기록 후 45초 이상일 때만.
    ///
    /// 세부 항목이 빈 AI는 행이 없으므로 기록·비교에서 뺀다(Swift는 항상 1개 이상이라 같은 결과).
    pub fn record(
        &mut self,
        now: f64,
        ratios: &Ratios,
        buckets: &Nested,
        items: &Nested,
    ) -> Result<bool> {
        if ratios.is_empty() {
            return Ok(false);
        }
        let buckets = non_empty(buckets);
        let items = non_empty(items);
        let changed = self.changed(ratios, &buckets, &items);
        if !changed && self.last_record.is_some_and(|lr| now - lr < MIN_GAP_SECS) {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        insert_sample(&tx, now, ratios, &buckets, &items)?;
        tx.commit()?;
        self.last_record = Some(now);
        self.last = Some(LastSample {
            ratios: ratios.clone(),
            buckets: buckets
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            items: items
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        });
        self.prune(now)?;
        Ok(true)
    }

    /// 직전 표본 대비 실제 변화가 있는지. (Swift `record` 의 changed)
    fn changed(
        &self,
        ratios: &Ratios,
        buckets: &[(&String, &BTreeMap<String, f64>)],
        items: &[(&String, &BTreeMap<String, f64>)],
    ) -> bool {
        let Some(prev) = &self.last else { return true };
        for (k, v) in ratios {
            match prev.ratios.get(k) {
                None => return true,
                Some(pv) if (pv - v).abs() > MIN_DROP => return true,
                _ => {}
            }
        }
        for (k, bmap) in buckets {
            let Some(pb) = prev.buckets.get(*k) else {
                return true;
            };
            // 버킷 키는 새로 생겨도 변화로 치지 않는다(Swift 그대로).
            for (bk, bv) in *bmap {
                if pb.get(bk).is_some_and(|pbv| (pbv - bv).abs() > MIN_DROP) {
                    return true;
                }
            }
        }
        for (k, imap) in items {
            let Some(pi) = prev.items.get(*k) else {
                return true;
            };
            for (ik, iv) in *imap {
                match pi.get(ik) {
                    None => return true,
                    Some(piv) if (piv - iv).abs() > MIN_DROP => return true,
                    _ => {}
                }
            }
        }
        false
    }

    /// 보관 기간(35일) 밖 행 삭제. 매 기록마다 하지 않고 1시간에 한 번.
    fn prune(&mut self, now: f64) -> Result<()> {
        if self.last_prune.is_some_and(|p| now - p < PRUNE_EVERY_SECS) {
            return Ok(());
        }
        self.conn
            .execute("DELETE FROM samples WHERE t < ?1", [now - MAX_AGE_SECS])?;
        self.last_prune = Some(now);
        Ok(())
    }

    /// `since` 이후 특정 AI의 (시각, 잔여율) 시계열(시간순). (Swift `series(for:hours:)`, 기준 시각은 호출자가 계산)
    pub fn series(&self, provider: &str, since: f64) -> Result<Vec<Point>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT t, value FROM samples WHERE provider = ?1 AND kind = 0 AND key = '' AND t >= ?2 ORDER BY t",
        )?;
        let rows = stmt.query_map(params![provider, since], |r| {
            Ok(Point {
                t: r.get(0)?,
                r: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// 특정 AI 세부 항목의 시계열(`since` 이후, 시간순). 항목 기록 전 표본은 "5h"만 되살린다.
    /// (Swift `itemSeries`)
    pub fn item_series(&self, provider: &str, item: &str, since: f64) -> Result<Vec<Point>> {
        let map = |r: &rusqlite::Row| {
            Ok(Point {
                t: r.get(0)?,
                r: r.get(1)?,
            })
        };
        if item == "5h" {
            let mut stmt = self.conn.prepare_cached(&format!(
                "SELECT t, v FROM (
                    SELECT t, value AS v FROM samples WHERE provider = ?1 AND kind = 2 AND key = '5h' AND t >= ?2
                    UNION ALL {LEGACY_5H}
                 ) WHERE v IS NOT NULL ORDER BY t"
            ))?;
            let rows = stmt.query_map(params![provider, since], map)?;
            return Ok(rows.collect::<rusqlite::Result<_>>()?);
        }
        let mut stmt = self.conn.prepare_cached(
            "SELECT t, value FROM samples WHERE provider = ?1 AND kind = 2 AND key = ?3 AND t >= ?2 ORDER BY t",
        )?;
        let rows = stmt.query_map(params![provider, since, item], map)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// `since` 이후 이력에 나타난 세부 항목 id들(처음 나온 순서, 같은 표본 안에서는 이름순).
    /// 구버전 표본이 있으면 "5h"가 그 시각 위치에 들어간다. (Swift `itemIDs`)
    pub fn item_ids(&self, provider: &str, since: f64) -> Result<Vec<String>> {
        let mut legacy_at: Option<f64> = self
            .conn
            .prepare_cached(&format!(
                "SELECT t FROM ({LEGACY_5H}) WHERE v IS NOT NULL ORDER BY t LIMIT 1"
            ))?
            .query_row(params![provider, since], |r| r.get(0))
            .optional()?;
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<String> = Vec::new();
        let mut stmt = self.conn.prepare_cached(
            "SELECT t, key FROM samples WHERE provider = ?1 AND kind = 2 AND t >= ?2 ORDER BY t, key",
        )?;
        let rows = stmt.query_map(params![provider, since], |r| {
            Ok((r.get::<_, f64>(0)?, r.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (t, key) = row?;
            if legacy_at.is_some_and(|lt| lt < t) {
                legacy_at = None;
                if seen.insert("5h".to_string()) {
                    out.push("5h".to_string());
                }
            }
            if seen.insert(key.clone()) {
                out.push(key);
            }
        }
        if legacy_at.is_some() && seen.insert("5h".to_string()) {
            out.push("5h".to_string());
        }
        Ok(out)
    }

    /// 세부 항목 기록이 시작된 첫 시각(1.5.0 이전 이력에는 없다). (Swift `earliestItems`)
    pub fn earliest_items(&self) -> Result<Option<f64>> {
        Ok(self
            .conn
            .query_row("SELECT MIN(t) FROM samples WHERE kind = 2", [], |r| {
                r.get(0)
            })?)
    }

    /// 가장 오래된 표본 시각(데이터 수집 시작일 안내용). (Swift `earliest`)
    pub fn earliest(&self) -> Result<Option<f64>> {
        Ok(self
            .conn
            .query_row("SELECT MIN(t) FROM samples", [], |r| r.get(0))?)
    }

    /// 표본 수(서로 다른 시각 수). 진단·테스트용.
    pub fn sample_count(&self) -> Result<usize> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(DISTINCT t) FROM samples", [], |r| {
                r.get::<_, i64>(0)
            })? as usize)
    }

    /// 소비 이벤트: 연속 표본에서 잔여율이 줄어든 만큼(양수)을 나중 시각에 기록.
    /// 세부 버킷이 있으면 대표 비율 하락과 공통 버킷의 최대 하락 중 큰 값을 쓴다. (Swift `consumptionEvents`)
    pub fn consumption_events(&self, provider: &str, since: f64) -> Result<Vec<Event>> {
        let mut ratio_stmt = self.conn.prepare_cached(
            "SELECT t, value FROM samples WHERE provider = ?1 AND kind = 0 AND key = '' AND t >= ?2 ORDER BY t",
        )?;
        let mut bucket_stmt = self.conn.prepare_cached(
            "SELECT t, key, value FROM samples WHERE provider = ?1 AND kind = 1 AND t >= ?2 ORDER BY t",
        )?;
        let ratios = ratio_stmt.query_map(params![provider, since], |r| {
            Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?))
        })?;
        let mut buckets = bucket_stmt
            .query_map(params![provider, since], |r| {
                Ok((
                    r.get::<_, f64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, f64>(2)?,
                ))
            })?
            .peekable();

        let mut out = Vec::new();
        // (t, 잔여율, 그 표본의 버킷 맵(없으면 None))
        let mut prev: Option<(f64, Option<BTreeMap<String, f64>>)> = None;
        let mut prev_r = 0.0;
        for row in ratios {
            let (t, r) = row?;
            // 이 표본의 버킷 행만 모은다(비율은 있는데 버킷이 없는 표본은 건너뛴 행으로 소비).
            let mut cur: Option<BTreeMap<String, f64>> = None;
            while let Some(peek) = buckets.peek() {
                let bt = match peek {
                    Ok((bt, _, _)) => *bt,
                    Err(_) => break,
                };
                if bt > t {
                    break;
                }
                let (bt, k, v) = buckets.next().expect("peeked")?;
                if bt == t {
                    cur.get_or_insert_with(BTreeMap::new).insert(k, v);
                }
            }
            if let Some((_, prev_b)) = &prev {
                let mut drop = (prev_r - r).max(0.0);
                if let (Some(pb), Some(cb)) = (prev_b, &cur) {
                    let mut max_drop = 0.0;
                    for (bk, cv) in cb {
                        if let Some(pv) = pb.get(bk) {
                            let d = pv - cv;
                            if d > max_drop {
                                max_drop = d;
                            }
                        }
                    }
                    drop = drop.max(max_drop);
                }
                if drop > MIN_DROP {
                    out.push(Event { t, amount: drop });
                }
            }
            prev = Some((t, cur));
            prev_r = r;
        }
        // 버킷 스트림에 남은 오류가 있으면 알린다.
        for rest in buckets {
            rest?;
        }
        Ok(out)
    }

    /// 최근 추세로 소진 예측. 잔여가 감소 중일 때만 값을 준다. (Swift `pace`)
    pub fn pace(&self, provider: &str, now: f64) -> Result<Option<PaceInfo>> {
        let s = self.series(provider, now - 3.0 * 3600.0)?; // 최근 3시간 추세
        let (Some(first), Some(last)) = (s.first(), s.last()) else {
            return Ok(None);
        };
        let dt_hours = (last.t - first.t) / 3600.0;
        if dt_hours <= 0.05 {
            return Ok(None);
        }
        let rate = (first.r - last.r) / dt_hours; // 양수면 소진 중
        if rate <= MIN_DROP {
            return Ok(None);
        }
        let hours_to_empty = last.r / rate;
        Ok(Some(PaceInfo {
            depletion: now + hours_to_empty * 3600.0,
            rate_per_hour: rate,
        }))
    }

    /// Swift `history.json`(`[UsageSample]`, `t` = 2001-01-01 기준 초)을 가져온다.
    /// 이미 있는 행은 건너뛰어 다시 가져와도 중복되지 않는다. 파일은 표본 단위로 읽어 한꺼번에 올리지 않는다.
    pub fn import_swift_history_json(&mut self, path: impl AsRef<Path>) -> Result<ImportStats> {
        let file = BufReader::new(File::open(path)?);
        let tx = self.conn.transaction()?;
        let sink = RefCell::new(ImportSink {
            tx: &tx,
            stats: ImportStats {
                samples: 0,
                rows_inserted: 0,
            },
            err: None,
        });
        let mut de = serde_json::Deserializer::from_reader(file);
        let res = de.deserialize_seq(SamplesVisitor { sink: &sink });
        let ImportSink { stats, err, .. } = sink.into_inner();
        if let Some(e) = err {
            return Err(e.into());
        }
        res?;
        de.end()?;
        tx.commit()?;
        self.reload_last()?;
        Ok(stats)
    }
}

/// 비어 있는 AI 항목을 뺀 목록.
fn non_empty(m: &Nested) -> Vec<(&String, &BTreeMap<String, f64>)> {
    m.iter().filter(|(_, v)| !v.is_empty()).collect()
}

/// 한 표본의 모든 값을 행으로 넣는다. 이미 있는 행은 무시. 새로 들어간 행 수를 돌려준다.
fn insert_sample(
    tx: &Transaction,
    t: f64,
    ratios: &Ratios,
    buckets: &[(&String, &BTreeMap<String, f64>)],
    items: &[(&String, &BTreeMap<String, f64>)],
) -> rusqlite::Result<usize> {
    let mut stmt = tx.prepare_cached(
        "INSERT OR IGNORE INTO samples (provider, kind, key, t, value) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut n = 0;
    for (p, v) in ratios {
        n += stmt.execute(params![p, KIND_RATIO, "", t, v])?;
    }
    for (p, m) in buckets {
        for (k, v) in *m {
            n += stmt.execute(params![p, KIND_BUCKET, k, t, v])?;
        }
    }
    for (p, m) in items {
        for (k, v) in *m {
            n += stmt.execute(params![p, KIND_ITEM, k, t, v])?;
        }
    }
    Ok(n)
}

// MARK: Swift JSON 가져오기(표본 단위 스트리밍)

#[derive(Deserialize)]
struct SwiftSample {
    t: f64,
    ratios: Ratios,
    #[serde(default)]
    buckets: Option<Nested>,
    #[serde(default)]
    items: Option<Nested>,
}

struct ImportSink<'a> {
    tx: &'a Transaction<'a>,
    stats: ImportStats,
    err: Option<rusqlite::Error>,
}

struct SamplesVisitor<'a, 'b> {
    sink: &'a RefCell<ImportSink<'b>>,
}

impl<'de> Visitor<'de> for SamplesVisitor<'_, '_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("UsageSample 배열")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<(), A::Error> {
        while seq
            .next_element_seed(SampleSeed { sink: self.sink })?
            .is_some()
        {}
        Ok(())
    }
}

struct SampleSeed<'a, 'b> {
    sink: &'a RefCell<ImportSink<'b>>,
}

impl<'de> DeserializeSeed<'de> for SampleSeed<'_, '_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        let s = SwiftSample::deserialize(d)?;
        let mut sink = self.sink.borrow_mut();
        let t = s.t + SWIFT_REFERENCE_OFFSET;
        let empty = Nested::new();
        let buckets = non_empty(s.buckets.as_ref().unwrap_or(&empty));
        let items = non_empty(s.items.as_ref().unwrap_or(&empty));
        match insert_sample(sink.tx, t, &s.ratios, &buckets, &items) {
            Ok(n) => {
                sink.stats.samples += 1;
                sink.stats.rows_inserted += n;
                Ok(())
            }
            Err(e) => {
                sink.err = Some(e);
                Err(de::Error::custom("sqlite insert failed"))
            }
        }
    }
}
