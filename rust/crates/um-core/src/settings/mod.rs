//! 설정 스키마·기본값·마이그레이션·가져오기. (Swift OverlaySettings.swift의 영속 부분)
//!
//! - `snapshot`    `SettingsSnapshot`(Swift Codable JSON과 같은 키), `copyBorder`
//! - `live`        `Settings`(해석 완료 모델), `apply`/`snapshot`, 범위 제한, 플랫폼 규칙
//! - `defaults`    `Settings::default()`와 기본 AI 배치
//! - `migrate`     구버전(변+영역) -> 세그먼트 변환 (segments.rs와 병합 예정)
//! - `device_fit`  장치 맞춤·해상도 변경 적응(순수 함수)
//! - `presets`     저장 프리셋과 내장 표
//! - `json`        Rust 쪽 저장 형식(`settings.json`, schemaVersion 12)
//! - `import_swift` UserDefaults plist 가져오기

mod defaults;
mod device_fit;
mod import_swift;
mod json;
mod live;
mod migrate;
mod presets;
mod snapshot;

pub use defaults::{PROVIDER_IDS, default_layouts, default_provider_color};
pub use device_fit::{NotchInfo, ScreenInfo, adapt_to_screen_change, device_fit};
pub use import_swift::{
    ImportError, KnownItem, SwiftImport, default_swift_plist_path, import_from_plist, keys,
};
pub use json::{SCHEMA_VERSION, snapshot_from_json, snapshot_to_json};
pub use live::{AppAppearance, REFRESH_PRESETS, Settings, TrayClickMode};
pub use presets::{
    BRAND_COLORS, BrandColor, MAX_PRESETS, PRESET_CORNER_RADII, PRESET_NOTCH_WIDTHS,
    PRESET_THICKNESSES, SavedPreset, add_preset,
};
pub use snapshot::SettingsSnapshot;
