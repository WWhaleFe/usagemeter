//! 문자열 표. (Swift Localization.swift)
//!
//! `strings.json` 은 `rust/tools/gen_i18n.py` 가 Swift 표에서 생성한다(직접 고치지 않는다).
//! Rust 이식에서 새로 생긴 키는 손으로 쓴 `extra.json` 에 둔다.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::types::AppLanguage;

const STRINGS_JSON: &str = include_str!("strings.json");
const EXTRA_JSON: &str = include_str!("extra.json");

type Table = HashMap<String, [String; 3]>;

fn parse(json: &str, name: &str) -> Table {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("{name} 파싱 실패: {e}"))
}

/// 처음 쓸 때 한 번만 읽는다. extra 는 기존 키를 덮어쓰지 않는다.
fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(|| {
        merge(parse(STRINGS_JSON, "strings.json"), parse(EXTRA_JSON, "extra.json"))
    })
}

/// base 에 extra 를 합친다. 겹치는 키는 base 가 이긴다.
fn merge(mut base: Table, extra: Table) -> Table {
    for (k, v) in extra {
        base.entry(k).or_insert(v);
    }
    base
}

/// 키에 해당하는 문자열. 없으면 키 자체를 돌려준다. (Swift `Loc.tr`)
pub fn tr(key: &str, lang: AppLanguage) -> String {
    match table().get(key) {
        Some(e) => match lang {
            AppLanguage::Korean => e[0].clone(),
            AppLanguage::English => e[1].clone(),
            AppLanguage::Japanese => e[2].clone(),
        },
        None => key.to_string(),
    }
}

/// `%@` 를 문자열로 치환. (Swift `tf`)
pub fn tf(key: &str, lang: AppLanguage, arg: &str) -> String {
    tr(key, lang).replace("%@", arg)
}

/// `%d` 를 숫자로 치환. (Swift `tn`)
pub fn tn(key: &str, lang: AppLanguage, n: i64) -> String {
    tr(key, lang).replace("%d", &n.to_string())
}

pub fn has(key: &str) -> bool {
    table().contains_key(key)
}

pub fn keys() -> impl Iterator<Item = &'static str> {
    table().keys().map(|k| k.as_str())
}

/// 원본 JSON(Swift 변환분). Svelte UI 가 재사용한다. extra 는 `extra_json()`.
pub fn all_json() -> &'static str {
    STRINGS_JSON
}

/// 이식에서 추가한 키의 원본 JSON.
pub fn extra_json() -> &'static str {
    EXTRA_JSON
}

#[cfg(test)]
mod tests {
    use super::*;
    use AppLanguage::*;

    /// 생성기가 출력한 Swift 키 수. 표를 다시 생성하면 함께 갱신한다.
    const SWIFT_KEYS: usize = 335;

    #[test]
    fn 사용량_표시_모드의_문구와_보조_값() {
        use crate::settings::BarFillMode::{Remaining, Used};
        for key in ["menu.line5h", "menu.lineWeekly", "info.5h", "info.weekly", "info.opus", "chart.titleFmt",
            "disp.iconDesc", "disp.percentDesc", "subItems.desc", "subItems.used"] {
            assert_eq!(Remaining.text_key(key), key);
            assert!(has(Used.text_key(key)), "{key}");
        }
        for key in ["info.countdown", "info.reset", "info.pace", "analytics.remaining"] {
            assert_eq!(Used.text_key(key), key);
        }
        assert_eq!(tr(Used.text_key("subItems.used"), Korean), "잔여");
        assert_eq!(tr(Used.text_key("subItems.used"), English), "remaining");
        assert_eq!(tr(Used.text_key("subItems.used"), Japanese), "残量");
        assert_eq!(Used.display_ratio(0.62), 0.38);
        assert_eq!(Remaining.display_ratio(0.62), 0.62);
    }

    #[test]
    fn 키_개수() {
        let base = parse(STRINGS_JSON, "strings.json");
        let extra = parse(EXTRA_JSON, "extra.json");
        assert_eq!(base.len(), SWIFT_KEYS);
        assert!(extra.keys().all(|k| !base.contains_key(k)), "extra 가 Swift 키와 겹침");
        assert_eq!(keys().count(), SWIFT_KEYS + extra.len());
    }

    #[test]
    fn 대표_값() {
        assert_eq!(tr("analytics.title", Korean), "사용량 분석");
        assert_eq!(tr("range.week", English), "Week");
        assert_eq!(tn("analytics.hourFmt", Korean, 9), "9시");
        assert_eq!(tn("analytics.hourFmt", English, 9), "9:00");
        assert_eq!(tn("analytics.hourFmt", Japanese, 9), "9時");
    }

    #[test]
    fn 없는_키는_키_자체() {
        assert_eq!(tr("no.such.key", Korean), "no.such.key");
        assert!(!has("no.such.key"));
        assert!(has("range.week"));
    }

    #[test]
    fn 치환() {
        assert_eq!(tn("preset.defaultName", Korean, 3), "프리셋 3");
        assert_eq!(tf("locallogs.lastScan", English, "10:00"), "Last scan: 10:00");
        assert_eq!(tn("menu.savePresetInfo", English, 5), "Enter a name (max 5).");
    }

    #[test]
    fn extra_는_기존_키를_덮어쓰지_않는다() {
        // 이미 있는 키를 extra 로 병합하면 원래 값이 남아야 한다.
        let base = parse(STRINGS_JSON, "strings.json");
        let before = base["range.week"].clone();
        let dup: Table = serde_json::from_str(r#"{"range.week":["x","y","z"]}"#).unwrap();
        let t = merge(base, dup);
        assert_eq!(t["range.week"], before);
        // 실제 병합 결과에도 extra 의 키가 있어야 한다.
        assert!(has("locallogs.title"));
    }

    #[test]
    fn extra_에_긴_줄표가_없다() {
        assert!(!EXTRA_JSON.contains('\u{2014}'));
        for (k, v) in parse(EXTRA_JSON, "extra.json") {
            assert!(v.iter().all(|s| !s.is_empty() && !s.contains('\u{2014}')), "{k}");
        }
    }
}
