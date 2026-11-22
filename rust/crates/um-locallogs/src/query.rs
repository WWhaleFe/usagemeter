//! 분석 화면용 조회. 시간 범위는 `[from, to)` 유닉스 초, 소스 필터는 선택.
//! 일·요일·시간 묶음은 `tz_offset_secs`(UTC 기준 초)를 더한 현지 시각으로 계산한다.
//! 프로젝트 이름 숨기기(`hide_projects`)는 표시 단계에서 처리한다(DB에는 폴더 이름이 그대로 있다).

use crate::models::display_name;
use crate::{Result, Source, Store};
use rusqlite::{Row, params};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct Range {
    pub from: f64,
    pub to: f64,
}

/// 토큰 합계. `total()`은 추론 토큰을 더하지 않는다(출력에 포함된 값으로 본다).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Totals {
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub reasoning: i64,
    /// 이벤트(응답) 수
    pub turns: i64,
}

impl Totals {
    pub fn total(&self) -> i64 {
        self.input + self.output + self.cache_read + self.cache_write
    }

    /// 캐시 적중률 = cache_read / (input + cache_read). 분모가 0이면 `None`.
    pub fn cache_hit_ratio(&self) -> Option<f64> {
        let d = self.input + self.cache_read;
        (d > 0).then(|| self.cache_read as f64 / d as f64)
    }

    /// 턴당 평균 토큰. 턴이 없으면 `None`.
    pub fn tokens_per_turn(&self) -> Option<f64> {
        (self.turns > 0).then(|| self.total() as f64 / self.turns as f64)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// 모델·날짜(`YYYY-MM-DD`)·프로젝트·소스 이름. 값이 없으면 빈 문자열.
    pub key: String,
    pub totals: Totals,
    pub sessions: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitPoint {
    pub t: f64,
    pub primary_used: Option<f64>,
    pub primary_window_min: Option<i64>,
    pub primary_resets_at: Option<i64>,
    pub secondary_used: Option<f64>,
    pub secondary_window_min: Option<i64>,
    pub secondary_resets_at: Option<i64>,
}

/// agy 모델 키: 표시 이름이 있으면 그것, 없으면 모델 ID.
const AGY_MODEL: &str = "COALESCE(NULLIF(label,''), model, '')";
const AGY_FILTER: &str = "t >= ?1 AND t < ?2";

/// agy 안에서 Claude·GPT 모델을 한 줄로 묶는 키.
pub const CLAUDE_GPT_KEY: &str = "__claude_gpt";

const SUMS: &str = "COALESCE(SUM(input),0),COALESCE(SUM(output),0),COALESCE(SUM(cache_read),0),\
COALESCE(SUM(cache_write),0),COALESCE(SUM(reasoning),0),COUNT(*)";
const FILTER: &str = "t >= ?1 AND t < ?2 AND (?3 IS NULL OR source = ?3)";

fn totals_from(r: &Row, i: usize) -> rusqlite::Result<Totals> {
    Ok(Totals {
        input: r.get(i)?,
        output: r.get(i + 1)?,
        cache_read: r.get(i + 2)?,
        cache_write: r.get(i + 3)?,
        reasoning: r.get(i + 4)?,
        turns: r.get(i + 5)?,
    })
}

impl Totals {
    fn add(&mut self, o: &Totals) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
        self.reasoning += o.reasoning;
        self.turns += o.turns;
    }
}

/// 키의 모델 부분(`when|model`이면 `|` 뒤, 아니면 전체)에 `f`를 적용하고, 같은 키끼리 합쳐 키 순으로 돌려준다.
fn remap_keys(groups: Vec<Group>, f: impl Fn(&str) -> String) -> Vec<Group> {
    let mut map: BTreeMap<String, Group> = BTreeMap::new();
    for g in groups {
        let key = match g.key.split_once('|') {
            Some((when, model)) => format!("{when}|{}", f(model)),
            None => f(&g.key),
        };
        let e = map.entry(key.clone()).or_insert_with(|| Group { key, totals: Totals::default(), sessions: 0 });
        e.totals.add(&g.totals);
        e.sessions += g.sessions;
    }
    map.into_values().collect()
}

/// 모델 이름을 표시 이름으로 바꾸고 같아진 키끼리 합친다.
pub fn relabel_models(groups: Vec<Group>) -> Vec<Group> {
    remap_keys(groups, display_name)
}

fn is_third_party(name: &str) -> bool {
    name.starts_with("Claude") || name.starts_with("GPT")
}

/// 표시 이름이 Claude·GPT로 시작하는 모델을 [`CLAUDE_GPT_KEY`] 한 키로 합친다(`relabel_models` 뒤에 쓴다).
pub fn bundle_third_party(groups: Vec<Group>) -> Vec<Group> {
    remap_keys(groups, |m| if is_third_party(m) { CLAUDE_GPT_KEY.to_string() } else { m.to_string() })
}

/// agy `/usage` 한도 묶음 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct QuotaGroup {
    /// `gemini`(공용 한도) | `argon`(별도 한도) | `thirdParty`(Claude·GPT)
    pub key: &'static str,
    pub v: i64,
    /// 묶음에 든 모델 표시 이름(요청 수 내림차순)
    pub models: Vec<String>,
}

/// 모델별 요청 수(`relabel_models`를 거친, 묶기 전 목록)를 한도 묶음으로 모은다. 요청이 0인 묶음은 뺀다.
pub fn agy_quota_groups(by_model: &[Group]) -> Vec<QuotaGroup> {
    let mut out: Vec<(QuotaGroup, Vec<(i64, &str)>)> = ["gemini", "argon", "thirdParty"]
        .into_iter()
        .map(|key| (QuotaGroup { key, v: 0, models: vec![] }, vec![]))
        .collect();
    for g in by_model {
        let i = if is_third_party(&g.key) {
            2
        } else if g.key.contains("Argon") {
            1
        } else {
            0
        };
        out[i].0.v += g.totals.turns;
        if !g.key.is_empty() {
            out[i].1.push((g.totals.turns, &g.key));
        }
    }
    out.into_iter()
        .filter(|(q, _)| q.v > 0)
        .map(|(mut q, mut ms)| {
            ms.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
            q.models = ms.into_iter().map(|(_, m)| m.to_string()).collect();
            q
        })
        .collect()
}

impl Store {
    /// 기간 전체 합계.
    pub fn totals(&self, range: Range, source: Option<Source>) -> Result<Totals> {
        let sql = format!("SELECT {SUMS} FROM token_events WHERE {FILTER}");
        Ok(self.conn.query_row(
            &sql,
            params![range.from, range.to, source.map(Source::as_str)],
            |r| totals_from(r, 0),
        )?)
    }

    /// 기간 안의 서로 다른 세션 수.
    pub fn sessions_count(&self, range: Range, source: Option<Source>) -> Result<i64> {
        let sql = format!(
            "SELECT COUNT(DISTINCT session) FROM token_events WHERE {FILTER} AND session IS NOT NULL"
        );
        Ok(self.conn.query_row(
            &sql,
            params![range.from, range.to, source.map(Source::as_str)],
            |r| r.get(0),
        )?)
    }

    fn group_by(&self, expr: &str, range: Range, source: Option<Source>, tz: i64) -> Result<Vec<Group>> {
        // expr는 이 파일 안의 상수만 들어온다.
        let sql = format!(
            "SELECT {expr} AS k, {SUMS}, COUNT(DISTINCT session) FROM token_events
             WHERE {FILTER} AND (?4 IS NULL OR 1) GROUP BY k ORDER BY k"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![range.from, range.to, source.map(Source::as_str), tz],
            |r| {
                Ok(Group {
                    key: r.get(0)?,
                    totals: totals_from(r, 1)?,
                    sessions: r.get(7)?,
                })
            },
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn by_model(&self, range: Range, source: Option<Source>) -> Result<Vec<Group>> {
        self.group_by("COALESCE(model,'')", range, source, 0)
    }

    pub fn by_project(&self, range: Range, source: Option<Source>) -> Result<Vec<Group>> {
        self.group_by("COALESCE(project,'')", range, source, 0)
    }

    pub fn by_source(&self, range: Range, source: Option<Source>) -> Result<Vec<Group>> {
        self.group_by("source", range, source, 0)
    }

    /// 현지 날짜(`YYYY-MM-DD`)별.
    pub fn by_day(&self, range: Range, source: Option<Source>, tz_offset_secs: i64) -> Result<Vec<Group>> {
        self.group_by(
            "strftime('%Y-%m-%d', t + ?4, 'unixepoch')",
            range,
            source,
            tz_offset_secs,
        )
    }

    /// 날짜 x 모델 묶음(모델별 쌓은 막대용). 키는 `YYYY-MM-DD|모델`.
    pub fn by_day_model(&self, range: Range, source: Option<Source>, tz_offset_secs: i64) -> Result<Vec<Group>> {
        self.group_by(
            "strftime('%Y-%m-%d', t + ?4, 'unixepoch') || '|' || COALESCE(model,'')",
            range,
            source,
            tz_offset_secs,
        )
    }

    /// 시간 x 모델 묶음(1일·3일 보기의 쌓은 막대용). 키는 `YYYY-MM-DD HH|모델`(현지 시각).
    pub fn by_hour_model(&self, range: Range, source: Option<Source>, tz_offset_secs: i64) -> Result<Vec<Group>> {
        self.group_by(
            "strftime('%Y-%m-%d %H', t + ?4, 'unixepoch') || '|' || COALESCE(model,'')",
            range,
            source,
            tz_offset_secs,
        )
    }

    /// 요일(월=0..일=6) x 시간(0..23) 토큰 합계 행렬. `with_cache_read`가 거짓이면 캐시 읽기를 뺀다.
    pub fn weekday_hour_matrix(
        &self,
        range: Range,
        source: Option<Source>,
        tz_offset_secs: i64,
        with_cache_read: bool,
    ) -> Result<[[i64; 24]; 7]> {
        let sum = if with_cache_read { "input + output + cache_read + cache_write" } else { "input + output + cache_write" };
        let sql = format!(
            "SELECT CAST(strftime('%w', t + ?4, 'unixepoch') AS INTEGER),
                    CAST(strftime('%H', t + ?4, 'unixepoch') AS INTEGER),
                    COALESCE(SUM({sum}),0)
             FROM token_events WHERE {FILTER} GROUP BY 1, 2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            params![range.from, range.to, source.map(Source::as_str), tz_offset_secs],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)),
        )?;
        let mut m = [[0i64; 24]; 7];
        for row in rows {
            let (w, h, v) = row?;
            // strftime %w: 일=0..토=6 -> 월=0..일=6
            let wd = ((w + 6) % 7) as usize;
            if let Some(cell) = m.get_mut(wd).and_then(|d| d.get_mut(h as usize)) {
                *cell = v;
            }
        }
        Ok(m)
    }

    /// agy 요청 수 합계(`turns`만 채운다).
    pub fn agy_totals(&self, range: Range) -> Result<Totals> {
        let sql = format!("SELECT COUNT(*) FROM agy_events WHERE {AGY_FILTER}");
        let n: i64 = self.conn.query_row(&sql, params![range.from, range.to], |r| r.get(0))?;
        Ok(Totals { turns: n, ..Totals::default() })
    }

    /// agy 기간 안의 서로 다른 대화 수.
    pub fn agy_sessions_count(&self, range: Range) -> Result<i64> {
        let sql = format!("SELECT COUNT(DISTINCT conversation) FROM agy_events WHERE {AGY_FILTER}");
        Ok(self.conn.query_row(&sql, params![range.from, range.to], |r| r.get(0))?)
    }

    /// `group_by`의 agy 판: 요청 수(`turns`)와 대화 수(`sessions`)만 채운다.
    fn agy_group_by(&self, expr: &str, range: Range, tz: i64) -> Result<Vec<Group>> {
        // expr는 이 파일 안의 상수만 들어온다.
        let sql = format!(
            "SELECT {expr} AS k, COUNT(*), COUNT(DISTINCT conversation) FROM agy_events
             WHERE {AGY_FILTER} AND (?3 IS NULL OR 1) GROUP BY k ORDER BY k"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![range.from, range.to, tz], |r| {
            Ok(Group {
                key: r.get(0)?,
                totals: Totals { turns: r.get(1)?, ..Totals::default() },
                sessions: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn agy_by_model(&self, range: Range) -> Result<Vec<Group>> {
        self.agy_group_by(AGY_MODEL, range, 0)
    }

    pub fn agy_by_project(&self, range: Range) -> Result<Vec<Group>> {
        self.agy_group_by("COALESCE(project,'')", range, 0)
    }

    /// 날짜 x 모델. 키는 `YYYY-MM-DD|모델`.
    pub fn agy_by_day_model(&self, range: Range, tz_offset_secs: i64) -> Result<Vec<Group>> {
        let expr = format!("strftime('%Y-%m-%d', t + ?3, 'unixepoch') || '|' || {AGY_MODEL}");
        self.agy_group_by(&expr, range, tz_offset_secs)
    }

    /// 시간 x 모델. 키는 `YYYY-MM-DD HH|모델`(현지 시각).
    pub fn agy_by_hour_model(&self, range: Range, tz_offset_secs: i64) -> Result<Vec<Group>> {
        let expr = format!("strftime('%Y-%m-%d %H', t + ?3, 'unixepoch') || '|' || {AGY_MODEL}");
        self.agy_group_by(&expr, range, tz_offset_secs)
    }

    /// 요일(월=0..일=6) x 시간(0..23) agy 요청 수 행렬.
    pub fn agy_weekday_hour_matrix(&self, range: Range, tz_offset_secs: i64) -> Result<[[i64; 24]; 7]> {
        let sql = format!(
            "SELECT CAST(strftime('%w', t + ?3, 'unixepoch') AS INTEGER),
                    CAST(strftime('%H', t + ?3, 'unixepoch') AS INTEGER), COUNT(*)
             FROM agy_events WHERE {AGY_FILTER} GROUP BY 1, 2"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![range.from, range.to, tz_offset_secs], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
        })?;
        let mut m = [[0i64; 24]; 7];
        for row in rows {
            let (w, h, v) = row?;
            let wd = ((w + 6) % 7) as usize;
            if let Some(cell) = m.get_mut(wd).and_then(|d| d.get_mut(h as usize)) {
                *cell = v;
            }
        }
        Ok(m)
    }

    /// Codex 한도 이력(시간순).
    pub fn codex_limit_series(&self, range: Range) -> Result<Vec<LimitPoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT t,primary_used,primary_window_min,primary_resets_at,
                    secondary_used,secondary_window_min,secondary_resets_at
             FROM codex_limits WHERE t >= ?1 AND t < ?2 ORDER BY t",
        )?;
        let rows = stmt.query_map(params![range.from, range.to], |r| {
            Ok(LimitPoint {
                t: r.get(0)?,
                primary_used: r.get(1)?,
                primary_window_min: r.get(2)?,
                primary_resets_at: r.get(3)?,
                secondary_used: r.get(4)?,
                secondary_window_min: r.get(5)?,
                secondary_resets_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(key: &str, turns: i64, sessions: i64) -> Group {
        Group { key: key.into(), totals: Totals { input: turns, turns, ..Totals::default() }, sessions }
    }

    #[test]
    fn relabel_merges_and_sorts() {
        let v = relabel_models(vec![
            g("2026-11-10|claude-opus-5-5", 2, 1),
            g("2026-11-10|Claude Opus 5.5 (High)", 3, 2),
            g("gpt-6-luna", 1, 1),
            g("claude-sonnet-5-5-medium", 4, 1),
            g("claude-sonnet-5-5", 5, 1),
            g("", 1, 0),
        ]);
        let keys: Vec<_> = v.iter().map(|x| x.key.as_str()).collect();
        assert_eq!(keys, ["", "2026-11-10|Claude Opus 5.5", "Claude Sonnet 5.5", "GPT-6 Luna"]);
        assert_eq!((v[1].totals.turns, v[1].totals.input, v[1].sessions), (5, 5, 3));
        assert_eq!(v[2].totals.turns, 9);
    }

    fn put_agy(s: &Store, conv: &str, idx: i64, t: f64, model: &str, label: Option<&str>, project: Option<&str>) {
        s.conn
            .execute(
                "INSERT INTO agy_events(conversation,idx,t,model,label,project) VALUES(?1,?2,?3,?4,?5,?6)",
                params![conv, idx, t, model, label, project],
            )
            .unwrap();
    }

    #[test]
    fn agy_queries_count_requests() {
        let s = Store::open_in_memory().unwrap();
        // 2026-11-09 월요일 10:00 UTC
        let mon = 1_794_218_400.0;
        put_agy(&s, "c1", 0, mon, "gemini-3.8-flash-n", Some("Gemini 3.8 Flash (High)"), Some("app"));
        put_agy(&s, "c1", 1, mon + 60.0, "gemini-3.8-flash-n", Some("Gemini 3.8 Flash (High)"), Some("app"));
        put_agy(&s, "c2", 0, mon + 3600.0, "gemini-4-neon", None, None);
        put_agy(&s, "c2", 1, mon + 86400.0 * 30.0, "x", None, None); // 범위 밖
        let r = Range { from: mon - 1.0, to: mon + 86400.0 };
        assert_eq!(s.agy_totals(r).unwrap().turns, 3);
        assert_eq!(s.agy_sessions_count(r).unwrap(), 2);
        let m = s.agy_by_model(r).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].key, "Gemini 3.8 Flash (High)");
        assert_eq!((m[0].totals.turns, m[0].totals.input, m[0].sessions), (2, 0, 1));
        assert_eq!(m[1].key, "gemini-4-neon");
        let p = s.agy_by_project(r).unwrap();
        assert_eq!(p.iter().map(|g| (g.key.as_str(), g.totals.turns)).collect::<Vec<_>>(), [("", 1), ("app", 2)]);
        let d = s.agy_by_day_model(r, 0).unwrap();
        assert_eq!(d[0].key, "2026-11-09|Gemini 3.8 Flash (High)");
        let h = s.agy_by_hour_model(r, 9 * 3600).unwrap();
        assert_eq!(h[0].key, "2026-11-09 19|Gemini 3.8 Flash (High)");
        let wh = s.agy_weekday_hour_matrix(r, 0).unwrap();
        assert_eq!((wh[0][10], wh[0][11]), (2, 1));
    }

    #[test]
    fn quota_groups_and_bundle() {
        let by_model = vec![
            g("Gemini 3.8 Flash", 5, 1),
            g("Gemini 4 Argon", 3, 1),
            g("Gemini 4 Neon", 7, 1),
            g("Claude Sonnet 5.5", 2, 1),
            g("GPT-OSS 120B", 4, 1),
        ];
        let q = agy_quota_groups(&by_model);
        assert_eq!(q.len(), 3);
        assert_eq!((q[0].key, q[0].v), ("gemini", 12));
        assert_eq!(q[0].models, ["Gemini 4 Neon", "Gemini 3.8 Flash"]);
        assert_eq!((q[1].key, q[1].v), ("argon", 3));
        assert_eq!((q[2].key, q[2].v), ("thirdParty", 6));
        assert_eq!(q[2].models, ["GPT-OSS 120B", "Claude Sonnet 5.5"]);
        assert!(agy_quota_groups(&by_model[..1]).iter().all(|x| x.key == "gemini"));

        let b = bundle_third_party(by_model);
        let third = b.iter().find(|x| x.key == CLAUDE_GPT_KEY).unwrap();
        assert_eq!(third.totals.turns, 6);
        assert_eq!(b.len(), 4);
        let b = bundle_third_party(vec![g("2026-11-09|Claude Opus 5", 1, 1), g("2026-11-09|GPT-6 Luna", 2, 1)]);
        assert_eq!((b.len(), b[0].key.as_str(), b[0].totals.turns), (1, "2026-11-09|__claude_gpt", 3));
    }
}
