//! 사용량 알림 판정. (Swift NotificationManager.swift)
//! 사용률 = round((1 - 잔여율) × 100). 임계값을 오름차순으로 보며 넘은 것마다 한 번씩 보낸다.
//! 중복 방지 키 = "id|임계값|리셋시각(epoch 초, 없으면 na)". 리셋 시각이 바뀌면 다시 보낼 수 있다.
//! Swift는 키를 메모리에만 둬서 재시작하면 다시 보냈다. Rust는 호출자가 키 집합을 저장한다(부록 C).

use chrono::{DateTime, Utc};
use std::collections::BTreeSet;

/// 새로 보내야 할 (임계값, 키) 목록. `sent`에 없는 것만 돌려준다.
pub fn due_alerts(
    provider_id: &str,
    remaining_ratio: f64,
    reset_at: Option<DateTime<Utc>>,
    thresholds: &BTreeSet<i32>,
    sent: &BTreeSet<String>,
) -> Vec<(i32, String)> {
    let used = ((1.0 - remaining_ratio) * 100.0).round() as i32;
    let reset = reset_at.map(|d| d.timestamp().to_string()).unwrap_or_else(|| "na".into());
    thresholds
        .iter()
        .filter(|&&t| used >= t)
        .map(|&t| (t, format!("{provider_id}|{t}|{reset}")))
        .filter(|(_, k)| !sent.contains(k))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fires_once_per_threshold() {
        let th: BTreeSet<i32> = [75, 90, 95].into();
        let mut sent = BTreeSet::new();
        let a = due_alerts("claude", 0.08, None, &th, &sent);
        assert_eq!(a.iter().map(|x| x.0).collect::<Vec<_>>(), vec![75, 90]);
        sent.extend(a.into_iter().map(|x| x.1));
        assert!(due_alerts("claude", 0.08, None, &th, &sent).is_empty());
    }
}
