//! 조회 결과(`raw`) -> `UsageSnapshot` 변환. (Swift WebSession.swift `fetchUsage` 후반부와 `remaining`/`resetDate`)
//!
//! `raw`는 usage JS가 돌려주는 모양이다. 직접 호출 경로(`direct`)도 같은 모양을 만들어 이 함수를 공유한다.
//! `{ok, plan, five_hour:{utilization,resets_at}, seven_day, seven_day_opus, seven_day_sonnet,
//!   model_buckets:[{label,utilization,resets_at,five_hour}], keys, reason, status, host, snippet}`

use chrono::{DateTime, Utc};
use serde_json::Value;
use um_core::model::{UsageModelBucket, UsageSnapshot, UsageStatus};

/// 잔여율 = 1 - 사용률/100, 0~1로 자른다. 사용률이 없으면(구간 자체가 없어도) 1.0.
/// (Swift WebSession.swift:392-395)
pub fn remaining(bucket: Option<&Value>) -> f64 {
    match bucket.and_then(|b| b.get("utilization")).and_then(Value::as_f64) {
        Some(u) => (1.0 - u / 100.0).clamp(0.0, 1.0),
        None => 1.0,
    }
}

/// ISO-8601(소수 초 있어도 없어도) -> UTC. 형식이 틀리면 None. (Swift `resetDate`)
pub fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
}

/// 구간의 `resets_at` 문자열을 시각으로. (Swift WebSession.swift:396-401)
pub fn reset_date(bucket: Option<&Value>) -> Option<DateTime<Utc>> {
    bucket.and_then(|b| b.get("resets_at")).and_then(Value::as_str).and_then(parse_iso)
}

/// 값이 JSON 객체일 때만 돌려준다(Swift `as? [String: Any]`). null·배열은 None.
fn obj<'a>(raw: &'a Value, key: &str) -> Option<&'a Value> {
    raw.get(key).filter(|v| v.is_object())
}

/// 실패 원인 문자열. 없으면 "unknown". (Swift WebSession.swift:338)
pub fn failure_reason(raw: &Value) -> String {
    raw.get("reason").and_then(Value::as_str).unwrap_or("unknown").to_string()
}

/// 실패 로그 한 줄. 사용자 제보 때 원인 파악용(Swift WebSession.swift:339-343).
pub fn failure_log_line(id: &str, raw: &Value) -> String {
    let s = |k: &str| raw.get(k).and_then(Value::as_str).unwrap_or("-").to_string();
    let status = raw.get("status").map_or("-".to_string(), |v| match v {
        Value::String(x) => x.clone(),
        other => other.to_string(),
    });
    format!(
        "{id} fetch failed reason={} host={} status={status} snippet={}",
        failure_reason(raw),
        s("host"),
        s("snippet")
    )
}

/// 응답 최상위 키 로그를 세션당 한 번만 남기기 위한 상태. (Swift `loggedKeys`, WebSession.swift:350-353)
#[derive(Debug, Default)]
pub struct KeysLogOnce {
    logged: bool,
}

impl KeysLogOnce {
    /// 처음 키 목록이 있는 응답을 만났을 때만 로그 줄을 돌려준다.
    pub fn take(&mut self, id: &str, raw: &Value) -> Option<String> {
        if self.logged {
            return None;
        }
        let keys: Vec<&str> = raw.get("keys")?.as_array()?.iter().filter_map(Value::as_str).collect();
        self.logged = true;
        Some(format!("{id} usage keys={}", keys.join(",")))
    }
}

/// `raw` -> 스냅샷. (Swift `fetchUsage`의 결과 처리 구간)
///
/// - `ok != true`: reason이 `not_logged_in`이면 AuthExpired, 그 밖은 Unavailable(reason).
/// - 성공: 대표(5시간)=five_hour, 주간=seven_day, opus=seven_day_opus, 모델 버킷, 플랜.
pub fn snapshot_from_probe(id: &str, raw: &Value, now: DateTime<Utc>) -> UsageSnapshot {
    if raw.get("ok").and_then(Value::as_bool) != Some(true) {
        let reason = failure_reason(raw);
        // 진짜 미로그인일 때만 재로그인을 유도한다(WebSession.swift:344).
        let status = if reason == "not_logged_in" { UsageStatus::AuthExpired } else { UsageStatus::Unavailable(reason) };
        return UsageSnapshot::failed(id, status, now);
    }
    let five = obj(raw, "five_hour");
    let week = obj(raw, "seven_day");
    let opus = obj(raw, "seven_day_opus");
    let plan = raw.get("plan").and_then(Value::as_str).filter(|p| !p.is_empty()).map(str::to_string);

    let mut buckets: Vec<UsageModelBucket> = raw
        .get("model_buckets")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|b| {
                    let label = b.get("label").and_then(Value::as_str).filter(|l| !l.is_empty())?;
                    Some(UsageModelBucket {
                        label: label.to_string(),
                        remaining_ratio: remaining(Some(b)),
                        reset_at: reset_date(Some(b)),
                        is_five_hour: b.get("five_hour").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Opus·Sonnet 주간 필드는 limits(weekly_scoped)에 같은 모델이 없을 때만 세부 항목으로 보탠다(WebSession.swift:364-370).
    for (key, label) in [("seven_day_opus", "Opus"), ("seven_day_sonnet", "Sonnet")] {
        let Some(b) = obj(raw, key) else { continue };
        let needle = label.to_lowercase();
        if buckets.iter().any(|x| x.label.to_lowercase().contains(&needle)) {
            continue;
        }
        buckets.push(UsageModelBucket {
            label: label.to_string(),
            remaining_ratio: remaining(Some(b)),
            reset_at: reset_date(Some(b)),
            is_five_hour: false,
        });
    }

    UsageSnapshot {
        id: id.to_string(),
        remaining_ratio: remaining(five),
        secondary_ratio: week.map(|w| remaining(Some(w))),
        opus_ratio: opus.map(|o| remaining(Some(o))),
        reset_at: reset_date(five),
        secondary_reset_at: reset_date(week),
        opus_reset_at: reset_date(opus),
        status: UsageStatus::Ok,
        last_updated: now,
        plan,
        model_buckets: if buckets.is_empty() { None } else { Some(buckets) },
        five_hour_ratio: None,
        five_hour_reset_at: None,
        raw_buckets: None,
    }
}
