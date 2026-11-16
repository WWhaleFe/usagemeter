//! Swift 버전(UserDefaults)의 설정 가져오기. 첫 실행 때 한 번만 쓴다(계획서 7장). 원본은 건드리지 않는다.
//!
//! UserDefaults plist에서 설정 JSON은 `Data`(바이트) 값으로 들어 있다. Swift는 `try?`로 디코딩해
//! 실패하면 무시했으므로, 여기서도 항목 하나가 깨져도 나머지를 읽고 `warnings`에 남긴다.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use plist::Value;
use serde::{Deserialize, Serialize};

use super::presets::SavedPreset;
use super::snapshot::SettingsSnapshot;

/// UserDefaults 키. (OverlaySettings.swift:951-955, DisplayRegistry.swift:34-35,
/// ProviderManager.swift:25, UpdateChecker.swift:44)
pub mod keys {
    pub const CURRENT: &str = "usagemeter.currentSettings";
    /// 뒤에 디스플레이 UUID가 붙는다.
    pub const DISPLAY_PREFIX: &str = "usagemeter.display.";
    pub const SAVED_PRESETS: &str = "usagemeter.savedPresets";
    pub const DEFAULT_SETTINGS: &str = "usagemeter.defaultSettings";
    pub const SETTINGS_VERSION: &str = "usagemeter.settingsVersion";
    pub const MAIN_DISPLAY_UUID: &str = "usagemeter.mainDisplayUUID";
    pub const DISABLED_DISPLAYS: &str = "usagemeter.disabledDisplays";
    pub const KNOWN_SUB_ITEMS: &str = "knownSubItems";
    pub const LAST_NOTIFIED_VERSION: &str = "usagemeter.lastNotifiedVersion";
}

/// 기억해 둔 세부 항목. (Swift `ProviderManager.KnownItem`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownItem {
    pub id: String,
    pub ratio: f64,
}

/// 가져온 결과. 없는 키는 `None`/빈 값.
#[derive(Debug, Clone, Default)]
pub struct SwiftImport {
    /// Swift는 이 값이 11 미만이면 현재 설정을 버렸다(:1008). `usable_current`가 그 규칙을 적용한다.
    pub settings_version: Option<i64>,
    pub current: Option<SettingsSnapshot>,
    /// 키 = 디스플레이 UUID 문자열.
    pub displays: BTreeMap<String, SettingsSnapshot>,
    pub presets: Vec<SavedPreset>,
    pub default_settings: Option<SettingsSnapshot>,
    pub main_display_uuid: Option<String>,
    pub disabled_displays: Vec<String>,
    /// 키 = AI id.
    pub known_sub_items: BTreeMap<String, Vec<KnownItem>>,
    pub last_notified_version: Option<String>,
    /// 디코딩에 실패해 건너뛴 항목 설명.
    pub warnings: Vec<String>,
}

impl SwiftImport {
    /// Swift 규칙: 스키마 버전이 11 미만이면 저장된 현재 설정은 쓰지 않는다. (OverlaySettings.swift:1007-1011)
    pub fn usable_current(&self) -> Option<&SettingsSnapshot> {
        if self.settings_version.unwrap_or(0) < 11 { None } else { self.current.as_ref() }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("plist를 읽지 못했습니다: {0}")]
    Plist(#[from] plist::Error),
    #[error("plist 최상위가 사전이 아닙니다")]
    NotDictionary,
}

/// Swift 앱의 UserDefaults plist 기본 위치. macOS에서만 값이 있다.
pub fn default_swift_plist_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Library").join("Preferences").join("com.usagemeter.app.plist"))
    } else {
        None
    }
}

/// UserDefaults plist(바이너리/XML)를 읽는다.
pub fn import_from_plist(path: &Path) -> Result<SwiftImport, ImportError> {
    let root = Value::from_file(path)?;
    let dict = root.into_dictionary().ok_or(ImportError::NotDictionary)?;
    let mut out = SwiftImport::default();

    // UserDefaults는 JSON을 Data로 저장한다. 문자열로 들어온 경우도 허용한다.
    let json_bytes = |v: &Value| -> Option<Vec<u8>> {
        match v {
            Value::Data(b) => Some(b.clone()),
            Value::String(s) => Some(s.clone().into_bytes()),
            _ => None,
        }
    };
    fn decode<T: serde::de::DeserializeOwned>(
        warnings: &mut Vec<String>,
        key: &str,
        bytes: Option<Vec<u8>>,
    ) -> Option<T> {
        let Some(b) = bytes else {
            warnings.push(format!("{key}: Data가 아닌 값이라 건너뜀"));
            return None;
        };
        match serde_json::from_slice(&b) {
            Ok(v) => Some(v),
            Err(e) => {
                warnings.push(format!("{key}: JSON 디코딩 실패({e})"));
                None
            }
        }
    }

    for (key, v) in &dict {
        let bytes = || json_bytes(v);
        match key.as_str() {
            keys::CURRENT => out.current = decode(&mut out.warnings, key, bytes()),
            keys::DEFAULT_SETTINGS => out.default_settings = decode(&mut out.warnings, key, bytes()),
            keys::SAVED_PRESETS => {
                if let Some(p) = decode(&mut out.warnings, key, bytes()) {
                    out.presets = p;
                }
            }
            keys::KNOWN_SUB_ITEMS => {
                if let Some(k) = decode(&mut out.warnings, key, bytes()) {
                    out.known_sub_items = k;
                }
            }
            keys::SETTINGS_VERSION => out.settings_version = v.as_signed_integer(),
            keys::MAIN_DISPLAY_UUID => out.main_display_uuid = v.as_string().map(str::to_string),
            keys::LAST_NOTIFIED_VERSION => out.last_notified_version = v.as_string().map(str::to_string),
            keys::DISABLED_DISPLAYS => {
                if let Some(a) = v.as_array() {
                    out.disabled_displays = a.iter().filter_map(|x| x.as_string().map(str::to_string)).collect();
                }
            }
            k => {
                if let Some(uuid) = k.strip_prefix(keys::DISPLAY_PREFIX)
                    && let Some(s) = decode(&mut out.warnings, key, bytes())
                {
                    out.displays.insert(uuid.to_string(), s);
                }
            }
        }
    }
    Ok(out)
}
