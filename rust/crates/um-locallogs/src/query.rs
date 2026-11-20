//! 분석 화면용 조회. 시간 범위는 `[from, to)` 유닉스 초, 소스 필터는 선택.
//! 일·요일·시간 묶음은 `tz_offset_secs`(UTC 기준 초)를 더한 현지 시각으로 계산한다.
//! 프로젝트 이름 숨기기(`hide_projects`)는 표시 단계에서 처리한다(DB에는 폴더 이름이 그대로 있다).

use crate::{Result, Source, Store};
use rusqlite::{Row, params};

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
