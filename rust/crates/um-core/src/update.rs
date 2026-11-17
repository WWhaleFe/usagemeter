//! 버전 비교. (Swift UpdateChecker.swift:208-224)
//! 숫자 버전은 0으로 채워 비교하고, 같은 번호의 프리릴리스는 정식보다 낮게 비교한다.

use std::cmp::Ordering;

fn parts(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    v.split(['-', '+']).next().unwrap_or(v).split('.')
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
    let pre = |v: &str| v.split('+').next().unwrap_or(v).split_once('-').map(|(_, p)| p.to_string());
    match (pre(a), pre(b)) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            let (a, b): (Vec<_>, Vec<_>) = (a.split('.').collect(), b.split('.').collect());
            for (x, y) in a.iter().zip(&b) {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    _ => x.cmp(y),
                };
                if order != Ordering::Equal { return order; }
            }
            a.len().cmp(&b.len())
        }
    }
}

/// `latest`가 `current`보다 새 버전인지.
pub fn is_newer(latest: &str, current: &str) -> bool {
    compare_versions(latest, current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 프리릴리스_순서() {
        assert!(is_newer("2.0.0", "2.0.0-beta.1"));
        assert!(!is_newer("2.0.0-beta.1", "2.0.0"));
        assert!(is_newer("2.0.0-beta.10", "2.0.0-beta.2"));
        assert!(is_newer("2.0.0-rc.1", "2.0.0-beta.10"));
        assert_eq!(compare_versions("2.0.0+build.2", "2.0.0"), Ordering::Equal);
    }
    #[test]
    fn compares() {
        assert!(is_newer("v1.5.0", "1.4.2"));
        assert!(!is_newer("1.5", "1.5.0"));
        assert!(is_newer("2.0.0-beta1", "1.9.9"));
        assert_eq!(compare_versions(" V1.10 ", "1.9"), Ordering::Greater);
    }
}
