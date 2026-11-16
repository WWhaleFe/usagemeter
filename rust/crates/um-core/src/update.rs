//! 버전 비교. (Swift UpdateChecker.swift:208-224)
//! 앞의 v/V·공백 제거 → 점으로 나눔 → 각 조각의 앞쪽 숫자만(없으면 0) → 왼쪽부터 숫자 비교, 짧은 쪽은 0으로 채움.

use std::cmp::Ordering;

fn parts(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    v.split('.')
        .map(|p| {
            let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

/// a와 b를 비교한다.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        match x.cmp(&y) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    Ordering::Equal
}

/// `latest`가 `current`보다 새 버전인지.
pub fn is_newer(latest: &str, current: &str) -> bool {
    compare_versions(latest, current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compares() {
        assert!(is_newer("v1.5.0", "1.4.2"));
        assert!(!is_newer("1.5", "1.5.0"));
        assert!(is_newer("2.0.0-beta1", "1.9.9"));
        assert_eq!(compare_versions(" V1.10 ", "1.9"), Ordering::Greater);
    }
}
