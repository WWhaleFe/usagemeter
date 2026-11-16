//! Rust 쪽 저장 형식. `settings.json` = 스냅샷 JSON + `schemaVersion`.
//! (계획서 7장: Swift의 settingsVersion 11 다음인 12부터 시작. 필드명은 Swift와 같다.)
//! 파일 입출력은 앱이 맡는다.

use serde_json::Value;

use super::live::Settings;
use super::snapshot::SettingsSnapshot;

/// 현재 설정 스키마 버전.
pub const SCHEMA_VERSION: u32 = 12;

/// 스냅샷을 `schemaVersion`을 붙여 보기 좋은 JSON으로.
pub fn snapshot_to_json(s: &SettingsSnapshot) -> Result<String, serde_json::Error> {
    let mut v = serde_json::to_value(s)?;
    if let Value::Object(m) = &mut v {
        m.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
    }
    serde_json::to_string_pretty(&v)
}

/// JSON에서 스냅샷을 읽는다. `schemaVersion`을 비롯한 모르는 키는 무시한다.
pub fn snapshot_from_json(text: &str) -> Result<SettingsSnapshot, serde_json::Error> {
    serde_json::from_str(text)
}

impl Settings {
    /// `settings.json` 내용.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        snapshot_to_json(&self.to_snapshot())
    }

    /// `settings.json` 읽기(없는 항목은 기본값, 구버전 항목은 변환).
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        Ok(Self::from_snapshot(&snapshot_from_json(text)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 스키마_버전_포함_왕복() {
        let s = Settings::default();
        let text = s.to_json().unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["schemaVersion"], 12);
        assert_eq!(v["colorRGBA"].as_array().unwrap().len(), 4);
        assert_eq!(Settings::from_json(&text).unwrap(), s);
    }
}
