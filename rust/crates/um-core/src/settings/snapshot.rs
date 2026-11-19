//! `SettingsSnapshot`: Swift가 UserDefaults에 JSON으로 저장한 구조체와 같은 키·형태.
//! (Swift OverlaySettings.swift:151-225)
//!
//! 인코딩 메모: Swift 합성 Codable은 `nil` 옵셔널 키를 생략하고, `Set`은 배열(순서 불정),
//! `[String: X]`는 JSON 객체로 쓴다. 열거형 키 사전은 이미 rawValue 문자열로 바꿔 저장한다
//! (`snapshot()`의 `$0.key.rawValue`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::ProviderLayout;

/// 저장 프리셋·UserDefaults에 담기는 설정 스냅샷(잔여율 등 실시간 값 제외).
/// 앞 12개는 Swift에서 옵셔널이 아니므로 없으면 디코딩 실패(Swift `try?`와 같은 결과).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSnapshot {
    pub thickness: f64,
    #[serde(rename = "colorRGBA")]
    pub color_rgba: Vec<f64>,
    pub edges: Vec<String>,
    pub corner_radii: BTreeMap<String, f64>,
    pub show_track: bool,
    pub notch_enabled: bool,
    pub notch_width: f64,
    pub notch_height: f64,
    pub notch_radii: BTreeMap<String, f64>,
    pub anchor_corner: String,
    pub anchor_side: String,
    pub anchor_clockwise: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_opacity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend_start_corner: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend_end_corner: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_corner_above: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_fraction: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_logged_in: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tray_click_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bar_fill_mode: Option<String>,
    #[serde(default, rename = "borderHiddenIDs", skip_serializing_if = "Option::is_none")]
    pub border_hidden_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_colors: Option<BTreeMap<String, Vec<f64>>>,
    #[serde(default, rename = "separateByAI", skip_serializing_if = "Option::is_none")]
    pub separate_by_ai: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_layouts: Option<BTreeMap<String, ProviderLayout>>,
    /// 구버전 호환(단일 토글). 새로 저장할 때는 항상 None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_percent_in_menu_bar: Option<bool>,
    #[serde(default, rename = "menuBarPercentIDs", skip_serializing_if = "Option::is_none")]
    pub menu_bar_percent_ids: Option<Vec<String>>,
    #[serde(default, rename = "menuBarIconID", skip_serializing_if = "Option::is_none")]
    pub menu_bar_icon_id: Option<String>,
    #[serde(default, rename = "menuBarPercentFirstID", skip_serializing_if = "Option::is_none")]
    pub menu_bar_percent_first_id: Option<String>,
    #[serde(default, rename = "menuShow5h", skip_serializing_if = "Option::is_none")]
    pub menu_show_5h: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_weekly: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_reset: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_updated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_preset_minutes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_use_custom: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_custom_minutes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_opus: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_countdown: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_pace: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_show_chart: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify_thresholds: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_check_update: Option<bool>,
    // 화면 분할(파티션) - 가로 경계선.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_line_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_line_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock_line_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock_line_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_zones: Option<Vec<String>>,
    // 영역별 모서리 곡률(본문은 cornerRadii 재사용).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_zone_radii: Option<BTreeMap<String, f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dock_zone_radii: Option<BTreeMap<String, f64>>,
    // 세그먼트 신모델(전역).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_segments: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seg_cap_start: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seg_cap_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_overlap_lines: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split_overlap_lines: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart_hours: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_menu_main_radii: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_main_dock_radii: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_order: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_info_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_emphasis_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_menu_line: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_dock_line: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chart_hover_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_overlay_on_launch: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu_bar_hover_info_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_edges: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_corners: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_corner_size: Option<f64>,
    // 세부 항목(5시간·주간·모델별) 색상·표시. 키 = "provider/item".
    #[serde(default, rename = "subItemSplitIDs", skip_serializing_if = "Option::is_none")]
    pub sub_item_split_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_item_colors: Option<BTreeMap<String, Vec<f64>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_item_bar_hidden: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_item_menu_hidden: Option<Vec<String>>,
}

impl SettingsSnapshot {
    /// 테두리 모양에 해당하는 값만 `o`에서 가져온다. 나머지(언어·메뉴바·알림·갱신 주기·AI 색 등)는
    /// 앱 전역 값으로 그대로 둔다. (Swift `copyBorder` :230-248)
    pub fn copy_border(&mut self, o: &SettingsSnapshot) {
        self.thickness = o.thickness;
        self.color_rgba = o.color_rgba.clone();
        self.edges = o.edges.clone();
        self.corner_radii = o.corner_radii.clone();
        self.show_track = o.show_track;
        self.notch_enabled = o.notch_enabled;
        self.notch_width = o.notch_width;
        self.notch_height = o.notch_height;
        self.notch_radii = o.notch_radii.clone();
        self.anchor_corner = o.anchor_corner.clone();
        self.anchor_side = o.anchor_side.clone();
        self.anchor_clockwise = o.anchor_clockwise;
        self.line_opacity = o.line_opacity;
        self.extend_start_corner = o.extend_start_corner;
        self.extend_end_corner = o.extend_end_corner;
        self.start_corner_above = o.start_corner_above;
        self.fade_enabled = o.fade_enabled;
        self.fade_fraction = o.fade_fraction;
        self.separate_by_ai = o.separate_by_ai;
        self.provider_layouts = o.provider_layouts.clone();
        self.menu_line_enabled = o.menu_line_enabled;
        self.menu_line_height = o.menu_line_height;
        self.dock_line_enabled = o.dock_line_enabled;
        self.dock_line_height = o.dock_line_height;
        self.global_zones = o.global_zones.clone();
        self.menu_zone_radii = o.menu_zone_radii.clone();
        self.dock_zone_radii = o.dock_zone_radii.clone();
        self.global_segments = o.global_segments.clone();
        self.seg_cap_start = o.seg_cap_start.clone();
        self.seg_cap_end = o.seg_cap_end.clone();
        self.no_overlap_lines = o.no_overlap_lines;
        self.split_overlap_lines = o.split_overlap_lines;
        self.link_menu_main_radii = o.link_menu_main_radii;
        self.link_main_dock_radii = o.link_main_dock_radii;
    }
}
