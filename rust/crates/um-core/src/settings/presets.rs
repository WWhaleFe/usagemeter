//! 저장 프리셋과 내장 표. (Swift OverlaySettings.swift:252, 950, 1312-1389)

use serde::{Deserialize, Serialize};

use super::snapshot::SettingsSnapshot;

/// 저장 프리셋 최대 개수. (Swift `maxPresets` :950; 주석의 5는 오기, 실제 10)
pub const MAX_PRESETS: usize = 10;

/// 이름이 붙은 저장 프리셋. `id`는 UUID 문자열(Swift `UUID` Codable = 대문자 문자열).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedPreset {
    pub id: String,
    pub name: String,
    pub snapshot: SettingsSnapshot,
}

/// 프리셋을 목록 끝에 추가한다. 가득 찼으면 `false`(목록 불변).
/// 이름은 앞뒤 공백을 자르고, 비면 `default_name(N)`(N = 현재 개수 + 1)을 쓴다.
/// Swift는 "프리셋 N"으로 고정이었으나 호출자가 현지화한 이름을 준다. (Swift `saveCurrent` :1314)
pub fn add_preset(
    list: &mut Vec<SavedPreset>,
    id: String,
    name: &str,
    snapshot: SettingsSnapshot,
    default_name: impl FnOnce(usize) -> String,
) -> bool {
    if list.len() >= MAX_PRESETS {
        return false;
    }
    let trimmed = name.trim();
    let name = if trimmed.is_empty() { default_name(list.len() + 1) } else { trimmed.to_string() };
    list.push(SavedPreset { id, name, snapshot });
    true
}

/// 곡률 프리셋(이름은 Swift 원문 한국어; 화면 문구는 i18n으로 대체 가능). (Swift :1362)
pub const PRESET_CORNER_RADII: [(&str, f64); 5] =
    [("직각 (0)", 0.0), ("살짝 (6)", 6.0), ("보통 (12)", 12.0), ("둥글게 (20)", 20.0), ("많이 (32)", 32.0)];

/// 굵기 프리셋. (Swift :1378)
pub const PRESET_THICKNESSES: [(&str, f64); 5] = [
    ("머리카락 (1)", 1.0),
    ("아주 얇게 (1.5)", 1.5),
    ("얇게 (2)", 2.0),
    ("보통 (3)", 3.0),
    ("두껍게 (5)", 5.0),
];

/// 노치 폭 프리셋. (Swift :1386)
pub const PRESET_NOTCH_WIDTHS: [(&str, f64); 4] =
    [("좁게 (160)", 160.0), ("보통 (200)", 200.0), ("넓게 (230)", 230.0), ("아주 넓게 (260)", 260.0)];

/// 브랜드 색 프리셋 한 항목.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrandColor {
    pub name: &'static str,
    /// sRGB 0~1 (Swift 소스의 값 그대로).
    pub rgb: [f64; 3],
    /// Swift 주석에 적힌 16진 값.
    pub hex: &'static str,
}

/// 대표 AI 브랜드 로고 주요 색. (Swift `presetColors` :1368)
pub const BRAND_COLORS: [BrandColor; 7] = [
    BrandColor { name: "Claude", rgb: [0.85, 0.47, 0.34], hex: "#D97757" },
    BrandColor { name: "ChatGPT", rgb: [0.06, 0.64, 0.50], hex: "#10A37F" },
    BrandColor { name: "Gemini", rgb: [0.26, 0.52, 0.96], hex: "#4285F4" },
    BrandColor { name: "Copilot", rgb: [0.00, 0.47, 0.83], hex: "#0078D4" },
    BrandColor { name: "Perplexity", rgb: [0.13, 0.50, 0.55], hex: "#20808D" },
    BrandColor { name: "Mistral", rgb: [0.98, 0.31, 0.06], hex: "#FA5010" },
    BrandColor { name: "Grok", rgb: [0.10, 0.10, 0.11], hex: "#1A1A1C" },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn 저장_규칙() {
        let snap = Settings::default().to_snapshot();
        let mut list = Vec::new();
        assert!(add_preset(&mut list, "A".into(), "  내 설정  ", snap.clone(), |n| format!("P{n}")));
        assert_eq!(list[0].name, "내 설정");
        assert!(add_preset(&mut list, "B".into(), "   ", snap.clone(), |n| format!("P{n}")));
        assert_eq!(list[1].name, "P2");
        for i in 0..8 {
            assert!(add_preset(&mut list, format!("{i}"), "x", snap.clone(), |n| n.to_string()));
        }
        assert_eq!(list.len(), MAX_PRESETS);
        assert!(!add_preset(&mut list, "Z".into(), "x", snap, |n| n.to_string()));
        assert_eq!(list.len(), MAX_PRESETS);
    }

    #[test]
    fn 내장_표_개수와_16진() {
        assert_eq!(BRAND_COLORS.len(), 7);
        for b in BRAND_COLORS {
            let v = u32::from_str_radix(&b.hex[1..], 16).unwrap();
            let want = [(v >> 16) & 255, (v >> 8) & 255, v & 255];
            for (c, w) in b.rgb.iter().zip(want) {
                assert!((c * 255.0 - f64::from(w)).abs() < 1.5, "{} {}", b.name, b.hex);
            }
        }
        assert_eq!(PRESET_CORNER_RADII.map(|p| p.1), [0.0, 6.0, 12.0, 20.0, 32.0]);
        assert_eq!(PRESET_THICKNESSES.map(|p| p.1), [1.0, 1.5, 2.0, 3.0, 5.0]);
        assert_eq!(PRESET_NOTCH_WIDTHS.map(|p| p.1), [160.0, 200.0, 230.0, 260.0]);
    }
}
