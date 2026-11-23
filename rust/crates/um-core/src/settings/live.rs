//! `Settings`: 해석이 끝난 라이브 설정(모든 필드가 값을 가짐).
//! Swift `OverlaySettings`의 영속 필드만 옮긴다(`ratio`·`hideOverlay`·훅 등 실시간 값은 제외).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::types::{
    AnchorSide, AppLanguage, BorderCorner, BorderEdge, NotchCorner, ProviderLayout, Rgba, ScreenZone, SegCap, SegPart,
};

use super::defaults::{PROVIDER_IDS, default_layouts, default_segment_for, fallback_layout};
use super::migrate::{is_valid_segments, is_valid_zones, migrate_to_segments};
use super::snapshot::SettingsSnapshot;

/// 선택 가능한 프리셋 갱신 주기(분). (Swift `refreshPresets` :445)
pub const REFRESH_PRESETS: [u32; 11] = [1, 2, 3, 4, 5, 10, 15, 20, 30, 60, 120];

/// 앱 외형. (Swift `AppAppearance` :259)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AppAppearance {
    #[default]
    System,
    Light,
    Dark,
}

/// 사용량 표시 방식. 모든 화면의 띠·수치·문구에 적용한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BarFillMode {
    #[default]
    Remaining,
    Used,
}

impl BarFillMode {
    /// 원본 잔여율은 유지하고 화면에 보여 줄 비율만 바꾼다.
    pub fn display_ratio(self, remaining: f64) -> f64 {
        match self {
            Self::Remaining => remaining,
            Self::Used => 1.0 - remaining,
        }
    }

    /// 쓴 양 모드의 표시 문구 키. 리셋·시간·분석 문구는 그대로 둔다.
    pub fn text_key(self, key: &str) -> &str {
        if self == Self::Remaining { return key; }
        match key {
            "menu.line5h" => "menu.line5hUsed",
            "menu.lineWeekly" => "menu.lineWeeklyUsed",
            "info.5h" => "info.5hUsed",
            "info.weekly" => "info.weeklyUsed",
            "info.opus" => "info.opusUsed",
            "chart.titleFmt" => "chart.usedTitleFmt",
            "disp.iconDesc" => "disp.iconUsedDesc",
            "disp.percentDesc" => "disp.percentUsedDesc",
            "subItems.desc" => "subItems.usedDesc",
            // 세부 항목 탭의 보조 값은 주 표시 비율의 반대쪽이다.
            "subItems.used" => "subItems.remaining",
            _ => key,
        }
    }

    /// 잔여율을 실제로 그릴 길이 비율로 바꾼다.
    pub fn fill_ratio(self, remaining: f64) -> f64 {
        self.display_ratio(remaining.clamp(0.0, 1.0))
    }
}

/// macOS 메뉴바 클릭 동작.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TrayClickMode {
    #[default]
    Split,
    Swapped,
    Together,
}

impl TrayClickMode {
    pub fn opens_card(self, left: bool) -> bool {
        matches!((self, left), (Self::Split, true) | (Self::Swapped, false))
    }
}

/// 앱 전체 설정. 디스플레이별 설정은 같은 구조체를 화면 수만큼 갖는다.
///
/// 필드 구분(Swift `copyBorder` :230 기준):
/// - **테두리(디스플레이별)**: thickness, color, line_opacity, extend_*_corner, start_corner_above,
///   edges, corner_radii, menu/dock_zone_radii, link_*_radii, no_overlap_lines, split_overlap_lines,
///   show_track, fade_*, anchor_*, notch_*, separate_by_ai, provider_layouts, menu/dock_line_*,
///   zones, seg_parts, seg_cap_*
/// - **전역(주 설정이 갖고 디스플레이는 따름)**: 나머지 전부(언어, AI 색, 메뉴바, 알림, 갱신 주기,
///   호버, 세부 항목, 외형, provider_order 등)
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    // ---- 테두리 ----
    pub thickness: f64,
    pub color: Rgba,
    pub line_opacity: f64,
    pub extend_start_corner: bool,
    pub extend_end_corner: bool,
    pub start_corner_above: bool,
    /// (구버전) 전역 변 선택. 저장 호환용.
    pub edges: BTreeSet<BorderEdge>,
    /// 본문 영역 곡률. 키가 빠진 모서리는 0으로 본다(`radius()`).
    pub corner_radii: BTreeMap<BorderCorner, f64>,
    pub menu_zone_radii: BTreeMap<BorderCorner, f64>,
    pub dock_zone_radii: BTreeMap<BorderCorner, f64>,
    pub link_menu_main_radii: bool,
    pub link_main_dock_radii: bool,
    pub no_overlap_lines: bool,
    pub split_overlap_lines: bool,
    pub show_track: bool,
    pub fade_enabled: bool,
    pub fade_fraction: f64,
    pub anchor_corner: BorderCorner,
    pub anchor_side: AnchorSide,
    pub anchor_clockwise: bool,
    pub notch_enabled: bool,
    pub notch_width: f64,
    pub notch_height: f64,
    pub notch_radii: HashMap<NotchCorner, f64>,
    pub menu_line_enabled: bool,
    pub menu_line_height: f64,
    pub dock_line_enabled: bool,
    pub dock_line_height: f64,
    /// 전역(겹침 모드) 영역. 스냅샷 키 `globalZones`.
    pub zones: BTreeSet<ScreenZone>,
    /// 전역 세그먼트 조합. 스냅샷 키 `globalSegments`.
    pub seg_parts: BTreeSet<SegPart>,
    pub seg_cap_start: SegCap,
    pub seg_cap_end: SegCap,
    pub separate_by_ai: bool,
    pub provider_layouts: BTreeMap<String, ProviderLayout>,

    // ---- 전역 ----
    pub hide_overlay_on_launch: bool,
    pub provider_colors: BTreeMap<String, Rgba>,
    pub keep_logged_in: bool,
    pub language: AppLanguage,
    pub refresh_preset_minutes: u32,
    pub refresh_use_custom: bool,
    pub refresh_custom_minutes: u32,
    pub bar_fill_mode: BarFillMode,
    pub tray_click_mode: TrayClickMode,
    pub border_hidden_ids: BTreeSet<String>,
    pub menu_bar_icon_id: String,
    pub menu_bar_percent_ids: BTreeSet<String>,
    pub menu_bar_percent_first_id: String,
    pub menu_show_5h: bool,
    pub menu_show_weekly: bool,
    pub menu_show_reset: bool,
    pub menu_show_updated: bool,
    pub menu_show_opus: bool,
    pub menu_show_countdown: bool,
    pub menu_show_pace: bool,
    pub menu_show_chart: bool,
    /// 미니 차트 기간(시간): 6 / 12 / 24.
    pub chart_hours: u32,
    pub appearance: AppAppearance,
    pub hover_info_enabled: bool,
    /// 호버한 선을 테두리에서 강조(선 강조). 글 강조와 따로 켜고 끈다.
    pub hover_emphasis_enabled: bool,
    /// 호버 글에서 그 선의 줄을 굵고 크게(글 강조).
    pub hover_text_emphasis: bool,
    /// 화면 분할의 메뉴바 아래 선·독 위 선에도 호버 정보를 띄울지(화면 가장자리 변 설정과 따로, 기본 꺼짐).
    pub hover_menu_line: bool,
    pub hover_dock_line: bool,
    /// 호버 영역을 배치 영역과 같게: 실제로 선이 배치된 곳에서만 정보를 띄운다(변·꼭짓점 호버 설정 대신, 2.2.1부터 기본 켜짐).
    pub hover_follow_layout: bool,
    pub chart_hover_enabled: bool,
    pub menu_bar_hover_info_enabled: bool,
    pub hover_edges: BTreeSet<BorderEdge>,
    pub hover_corners: BTreeSet<BorderCorner>,
    pub hover_corner_size: f64,
    pub provider_order: Vec<String>,
    pub notify_enabled: bool,
    pub notify_thresholds: BTreeSet<u32>,
    pub auto_check_update: bool,
    /// Store판(Windows): 새 버전을 Store에서 자동으로 설치한다. `auto_check_update`는 새 버전 알림.
    pub auto_install_update: bool,
    pub sub_item_split_ids: BTreeSet<String>,
    pub sub_item_colors: BTreeMap<String, Rgba>,
    pub sub_item_bar_hidden: BTreeSet<String>,
    pub sub_item_menu_hidden: BTreeSet<String>,
    /// 미니 차트에 선으로 그릴 세부 항목. 한 항목도 없는 AI는 대표선 하나.
    pub sub_item_chart_shown: BTreeSet<String>,
    /// 분석 창에서 숨길 세부 항목(내보내기는 전부 담는다).
    pub sub_item_analytics_hidden: BTreeSet<String>,
}

// ---- 문자열 <-> 열거형 (serde 이름 = Swift rawValue) ----

fn parse<T: DeserializeOwned>(s: &str) -> Option<T> {
    serde_json::from_value(Value::String(s.to_string())).ok()
}

fn raw<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(Value::String(s)) => s,
        _ => String::new(),
    }
}

fn parse_set<T: DeserializeOwned + Ord>(v: &[String]) -> BTreeSet<T> {
    v.iter().filter_map(|s| parse(s)).collect()
}

fn raw_list<T: Serialize>(it: impl IntoIterator<Item = T>) -> Vec<String> {
    it.into_iter().map(|v| raw(&v)).collect()
}

fn rgba(a: &[f64]) -> Option<Rgba> {
    <[f64; 4]>::try_from(a).ok()
}

/// 열거형 키 사전 복원. 모르는 키는 버린다(Swift `compactMap`).
fn dict_from<K: DeserializeOwned + std::hash::Hash + Eq>(m: &BTreeMap<String, f64>) -> HashMap<K, f64> {
    m.iter().filter_map(|(k, v)| parse::<K>(k).map(|k| (k, *v))).collect()
}

fn corner_dict(m: &BTreeMap<String, f64>) -> BTreeMap<BorderCorner, f64> {
    m.iter().filter_map(|(k, v)| parse::<BorderCorner>(k).map(|k| (k, *v))).collect()
}

fn dict_to<K: Serialize>(it: impl IntoIterator<Item = (K, f64)>) -> BTreeMap<String, f64> {
    it.into_iter().map(|(k, v)| (raw(&k), v)).collect()
}

/// 값이 NaN·무한이면 `fallback`, 아니면 범위로 제한.
fn clamp_f(v: f64, lo: f64, hi: f64, fallback: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { fallback }
}

impl Settings {
    /// 이 AI의 배치(없으면 기본 배치). (Swift `layout(for:)` :823)
    pub fn layout_for(&self, id: &str) -> ProviderLayout {
        self.provider_layouts.get(id).cloned().unwrap_or_else(|| {
            default_layouts().remove(id).unwrap_or_else(fallback_layout)
        })
    }

    /// 이 AI의 세그먼트 조합(없으면 구버전 변+영역에서 변환). (Swift `segs(for:)` :564)
    pub fn segs_for(&self, id: &str) -> BTreeSet<SegPart> {
        let l = self.layout_for(id);
        l.segments.clone().unwrap_or_else(|| {
            let edges = l.edges.clone().unwrap_or_else(|| BorderEdge::ALL.into_iter().collect());
            migrate_to_segments(&edges, &l.zone_set(), self.menu_line_enabled, self.dock_line_enabled)
        })
    }

    /// 본문 영역 모서리 곡률(없으면 0). (Swift `radius` :1117)
    pub fn radius(&self, c: BorderCorner) -> f64 {
        self.corner_radii.get(&c).copied().unwrap_or(0.0)
    }

    /// 노치 모서리 곡률(없으면 0). (Swift `notchRadius` :1120)
    pub fn notch_radius(&self, c: NotchCorner) -> f64 {
        self.notch_radii.get(&c).copied().unwrap_or(0.0)
    }

    /// 실제 적용되는 갱신 주기(분). (Swift `effectiveRefreshMinutes` :453)
    pub fn effective_refresh_minutes(&self) -> u32 {
        let m = if self.refresh_use_custom { self.refresh_custom_minutes } else { self.refresh_preset_minutes };
        m.max(1)
    }

    /// 표시 순서에서 이 AI의 위치(없으면 맨 뒤). (Swift `orderIndex` :509)
    pub fn order_index(&self, id: &str) -> usize {
        self.provider_order.iter().position(|p| p == id).unwrap_or(usize::MAX)
    }

    /// 스냅샷으로 새 설정을 만든다: 기본값 위에 `apply_snapshot`.
    pub fn from_snapshot(s: &SettingsSnapshot) -> Self {
        let mut st = Self::default();
        st.apply_snapshot(s);
        st
    }

    /// 스냅샷을 현재 설정에 적용. 값이 없거나 잘못된 항목은 기본값 또는 기존 값을 유지한다.
    /// (Swift `apply` :1198-1310, 레거시 변환 포함)
    pub fn apply_snapshot(&mut self, s: &SettingsSnapshot) {
        self.thickness = s.thickness;
        if let Some(c) = rgba(&s.color_rgba) {
            self.color = c;
        }
        let es: BTreeSet<BorderEdge> = parse_set(&s.edges);
        if !es.is_empty() {
            self.edges = es;
        }
        self.corner_radii = corner_dict(&s.corner_radii);
        self.show_track = s.show_track;
        self.notch_enabled = s.notch_enabled;
        self.notch_width = s.notch_width;
        self.notch_height = s.notch_height;
        self.notch_radii = dict_from(&s.notch_radii);
        self.anchor_corner = parse(&s.anchor_corner).unwrap_or(BorderCorner::TopLeft);
        self.anchor_side = parse(&s.anchor_side).unwrap_or(AnchorSide::Center);
        self.anchor_clockwise = s.anchor_clockwise;
        self.line_opacity = s.line_opacity.unwrap_or(1.0);
        self.extend_start_corner = s.extend_start_corner.unwrap_or(false);
        self.extend_end_corner = s.extend_end_corner.unwrap_or(false);
        self.start_corner_above = s.start_corner_above.unwrap_or(false);
        self.fade_enabled = s.fade_enabled.unwrap_or(true);
        self.fade_fraction = s.fade_fraction.unwrap_or(0.012);
        self.keep_logged_in = s.keep_logged_in.unwrap_or(true);
        self.bar_fill_mode = s.bar_fill_mode.as_deref().and_then(parse).unwrap_or_default();
        self.tray_click_mode = s.tray_click_mode.as_deref().and_then(parse).unwrap_or_default();
        self.border_hidden_ids = s.border_hidden_ids.iter().flatten().cloned().collect();
        if let Some(pc) = &s.provider_colors {
            for (id, a) in pc {
                if let Some(c) = rgba(a) {
                    self.provider_colors.insert(id.clone(), c);
                }
            }
        }
        self.separate_by_ai = s.separate_by_ai.unwrap_or(false);
        if let Some(pl) = &s.provider_layouts {
            self.provider_layouts = pl.clone();
        }
        if let Some(ids) = &s.menu_bar_percent_ids {
            self.menu_bar_percent_ids = ids.iter().cloned().collect();
        } else if s.show_percent_in_menu_bar == Some(true) {
            // 구버전: 전부 표시로 이전.
            self.menu_bar_percent_ids = PROVIDER_IDS.iter().map(|s| (*s).to_string()).collect();
        }
        if let Some(v) = &s.menu_bar_icon_id {
            self.menu_bar_icon_id = v.clone();
        }
        if let Some(v) = &s.menu_bar_percent_first_id {
            self.menu_bar_percent_first_id = v.clone();
        }
        self.menu_show_5h = s.menu_show_5h.unwrap_or(true);
        self.menu_show_weekly = s.menu_show_weekly.unwrap_or(true);
        self.menu_show_reset = s.menu_show_reset.unwrap_or(true);
        self.menu_show_updated = s.menu_show_updated.unwrap_or(false);
        if let Some(lang) = s.language.as_deref().and_then(parse::<AppLanguage>) {
            self.language = lang;
        }
        self.refresh_preset_minutes = to_u32(s.refresh_preset_minutes, 5);
        self.refresh_use_custom = s.refresh_use_custom.unwrap_or(false);
        self.refresh_custom_minutes = to_u32(s.refresh_custom_minutes, 5);
        self.menu_show_opus = s.menu_show_opus.unwrap_or(true);
        self.menu_show_countdown = s.menu_show_countdown.unwrap_or(true);
        self.menu_show_pace = s.menu_show_pace.unwrap_or(false);
        self.menu_show_chart = s.menu_show_chart.unwrap_or(false);
        self.notify_enabled = s.notify_enabled.unwrap_or(false);
        if let Some(t) = &s.notify_thresholds {
            self.notify_thresholds = t.iter().filter_map(|v| u32::try_from(*v).ok()).collect();
        }
        self.auto_check_update = s.auto_check_update.unwrap_or(true);
        self.auto_install_update = s.auto_install_update.unwrap_or(true);
        self.menu_line_enabled = s.menu_line_enabled.unwrap_or(false);
        self.menu_line_height = s.menu_line_height.unwrap_or(25.0);
        self.dock_line_enabled = s.dock_line_enabled.unwrap_or(false);
        self.dock_line_height = s.dock_line_height.unwrap_or(70.0);
        if let Some(gz) = &s.global_zones {
            let set: BTreeSet<ScreenZone> = parse_set(gz);
            if is_valid_zones(&set) {
                self.zones = set;
            }
        }
        if let Some(mr) = &s.menu_zone_radii {
            self.menu_zone_radii = corner_dict(mr);
        }
        if let Some(dr) = &s.dock_zone_radii {
            self.dock_zone_radii = corner_dict(dr);
        }
        if let Some(gs) = &s.global_segments {
            let set: BTreeSet<SegPart> = parse_set(gs);
            if is_valid_segments(&set, self.menu_line_enabled, self.dock_line_enabled) {
                self.seg_parts = set;
            }
        } else {
            // 구버전: 변+영역 설정에서 변환.
            self.seg_parts =
                migrate_to_segments(&self.edges, &self.zones, self.menu_line_enabled, self.dock_line_enabled);
        }
        self.seg_cap_start = s.seg_cap_start.as_deref().and_then(parse).unwrap_or(SegCap::None);
        self.seg_cap_end = s.seg_cap_end.as_deref().and_then(parse).unwrap_or(SegCap::None);
        self.no_overlap_lines = s.no_overlap_lines.unwrap_or(false);
        self.split_overlap_lines = s.split_overlap_lines.unwrap_or(false);
        self.chart_hours = to_u32(s.chart_hours, 24);
        self.link_menu_main_radii = s.link_menu_main_radii.unwrap_or(true);
        self.link_main_dock_radii = s.link_main_dock_radii.unwrap_or(true);
        self.appearance = s.appearance.as_deref().and_then(parse).unwrap_or_default();
        self.hover_info_enabled = s.hover_info_enabled.unwrap_or(true);
        self.hover_emphasis_enabled = s.hover_emphasis_enabled.unwrap_or(true);
        // 따로 나누기 전에는 한 옵션이 둘 다 껐으니 그 값을 따른다.
        self.hover_text_emphasis = s.hover_text_emphasis.or(s.hover_emphasis_enabled).unwrap_or(true);
        self.hover_menu_line = s.hover_menu_line.unwrap_or(false);
        self.hover_dock_line = s.hover_dock_line.unwrap_or(false);
        self.hover_follow_layout = s.hover_follow_layout.unwrap_or(true);
        self.chart_hover_enabled = s.chart_hover_enabled.unwrap_or(true);
        self.hide_overlay_on_launch = s.hide_overlay_on_launch.unwrap_or(false);
        self.menu_bar_hover_info_enabled = s.menu_bar_hover_info_enabled.unwrap_or(true);
        self.hover_edges = match &s.hover_edges {
            Some(v) => parse_set(v),
            None => BorderEdge::ALL.into_iter().collect(),
        };
        self.hover_corners = s.hover_corners.as_deref().map(parse_set).unwrap_or_default();
        self.hover_corner_size = s.hover_corner_size.unwrap_or(80.0);
        self.sub_item_split_ids = s.sub_item_split_ids.iter().flatten().cloned().collect();
        self.sub_item_colors = s
            .sub_item_colors
            .iter()
            .flatten()
            .filter_map(|(k, a)| rgba(a).map(|c| (k.clone(), c)))
            .collect();
        self.sub_item_bar_hidden = s.sub_item_bar_hidden.iter().flatten().cloned().collect();
        self.sub_item_menu_hidden = s.sub_item_menu_hidden.iter().flatten().cloned().collect();
        self.sub_item_chart_shown = s.sub_item_chart_shown.iter().flatten().cloned().collect();
        self.sub_item_analytics_hidden = s.sub_item_analytics_hidden.iter().flatten().cloned().collect();
        if let Some(po) = &s.provider_order {
            self.provider_order = normalize_provider_order(po);
        }
        // 스키마 일원화: 구버전(변+영역) 저장분은 세그먼트로 1회 변환해 고정한다. (:1301-1309)
        let (menu_on, dock_on) = (self.menu_line_enabled, self.dock_line_enabled);
        for id in PROVIDER_IDS {
            if let Some(l) = self.provider_layouts.get_mut(id)
                && l.segments.is_none()
            {
                let edges = l.edges.clone().unwrap_or_else(|| BorderEdge::ALL.into_iter().collect());
                l.segments = Some(migrate_to_segments(&edges, &l.zone_set(), menu_on, dock_on));
            }
        }
    }

    /// 현재 설정을 스냅샷으로 캡처. (Swift `snapshot()` :1125)
    pub fn to_snapshot(&self) -> SettingsSnapshot {
        let ids = |s: &BTreeSet<String>| s.iter().cloned().collect::<Vec<_>>();
        SettingsSnapshot {
            thickness: self.thickness,
            color_rgba: self.color.to_vec(),
            edges: raw_list(self.edges.iter()),
            corner_radii: dict_to(self.corner_radii.iter().map(|(k, v)| (*k, *v))),
            show_track: self.show_track,
            notch_enabled: self.notch_enabled,
            notch_width: self.notch_width,
            notch_height: self.notch_height,
            notch_radii: dict_to(self.notch_radii.iter().map(|(k, v)| (*k, *v))),
            anchor_corner: raw(&self.anchor_corner),
            anchor_side: raw(&self.anchor_side),
            anchor_clockwise: self.anchor_clockwise,
            line_opacity: Some(self.line_opacity),
            extend_start_corner: Some(self.extend_start_corner),
            extend_end_corner: Some(self.extend_end_corner),
            start_corner_above: Some(self.start_corner_above),
            fade_enabled: Some(self.fade_enabled),
            fade_fraction: Some(self.fade_fraction),
            keep_logged_in: Some(self.keep_logged_in),
            bar_fill_mode: Some(raw(&self.bar_fill_mode)),
            tray_click_mode: Some(raw(&self.tray_click_mode)),
            border_hidden_ids: Some(ids(&self.border_hidden_ids)),
            provider_colors: Some(self.provider_colors.iter().map(|(k, c)| (k.clone(), c.to_vec())).collect()),
            separate_by_ai: Some(self.separate_by_ai),
            provider_layouts: Some(self.provider_layouts.clone()),
            show_percent_in_menu_bar: None,
            menu_bar_percent_ids: Some(ids(&self.menu_bar_percent_ids)),
            menu_bar_icon_id: Some(self.menu_bar_icon_id.clone()),
            menu_bar_percent_first_id: Some(self.menu_bar_percent_first_id.clone()),
            menu_show_5h: Some(self.menu_show_5h),
            menu_show_weekly: Some(self.menu_show_weekly),
            menu_show_reset: Some(self.menu_show_reset),
            menu_show_updated: Some(self.menu_show_updated),
            language: Some(raw(&self.language)),
            refresh_preset_minutes: Some(i64::from(self.refresh_preset_minutes)),
            refresh_use_custom: Some(self.refresh_use_custom),
            refresh_custom_minutes: Some(i64::from(self.refresh_custom_minutes)),
            menu_show_opus: Some(self.menu_show_opus),
            menu_show_countdown: Some(self.menu_show_countdown),
            menu_show_pace: Some(self.menu_show_pace),
            menu_show_chart: Some(self.menu_show_chart),
            notify_enabled: Some(self.notify_enabled),
            notify_thresholds: Some(self.notify_thresholds.iter().map(|v| i64::from(*v)).collect()),
            auto_check_update: Some(self.auto_check_update),
            auto_install_update: Some(self.auto_install_update),
            menu_line_enabled: Some(self.menu_line_enabled),
            menu_line_height: Some(self.menu_line_height),
            dock_line_enabled: Some(self.dock_line_enabled),
            dock_line_height: Some(self.dock_line_height),
            global_zones: Some(raw_list(self.zones.iter())),
            menu_zone_radii: Some(dict_to(self.menu_zone_radii.iter().map(|(k, v)| (*k, *v)))),
            dock_zone_radii: Some(dict_to(self.dock_zone_radii.iter().map(|(k, v)| (*k, *v)))),
            global_segments: Some(raw_list(self.seg_parts.iter())),
            seg_cap_start: Some(raw(&self.seg_cap_start)),
            seg_cap_end: Some(raw(&self.seg_cap_end)),
            no_overlap_lines: Some(self.no_overlap_lines),
            split_overlap_lines: Some(self.split_overlap_lines),
            chart_hours: Some(i64::from(self.chart_hours)),
            link_menu_main_radii: Some(self.link_menu_main_radii),
            link_main_dock_radii: Some(self.link_main_dock_radii),
            appearance: Some(raw(&self.appearance)),
            provider_order: Some(self.provider_order.clone()),
            hover_info_enabled: Some(self.hover_info_enabled),
            hover_emphasis_enabled: Some(self.hover_emphasis_enabled),
            hover_text_emphasis: Some(self.hover_text_emphasis),
            hover_menu_line: Some(self.hover_menu_line),
            hover_dock_line: Some(self.hover_dock_line),
            hover_follow_layout: Some(self.hover_follow_layout),
            chart_hover_enabled: Some(self.chart_hover_enabled),
            hide_overlay_on_launch: Some(self.hide_overlay_on_launch),
            menu_bar_hover_info_enabled: Some(self.menu_bar_hover_info_enabled),
            hover_edges: Some(raw_list(self.hover_edges.iter())),
            hover_corners: Some(raw_list(self.hover_corners.iter())),
            hover_corner_size: Some(self.hover_corner_size),
            sub_item_split_ids: Some(ids(&self.sub_item_split_ids)),
            sub_item_colors: Some(self.sub_item_colors.iter().map(|(k, c)| (k.clone(), c.to_vec())).collect()),
            sub_item_bar_hidden: Some(ids(&self.sub_item_bar_hidden)),
            sub_item_menu_hidden: Some(ids(&self.sub_item_menu_hidden)),
            sub_item_chart_shown: Some(ids(&self.sub_item_chart_shown)),
            sub_item_analytics_hidden: Some(ids(&self.sub_item_analytics_hidden)),
        }
    }

    /// 다른 설정의 **테두리 관련 값만** 가져온다(언어·메뉴·알림 등 전역 값은 유지).
    /// Swift처럼 스냅샷으로 바꿔 `copyBorder` 후 다시 `apply`한다. (Swift :1337)
    pub fn copy_border_from(&mut self, other: &Settings) {
        let mut s = self.to_snapshot();
        s.copy_border(&other.to_snapshot());
        self.apply_snapshot(&s);
    }

    /// UI 범위로 값을 제한한다. NaN·무한은 해당 항목의 Swift 기본값으로 되돌린다.
    pub fn clamp_all(&mut self) {
        // 굵기: 1~15, 0.5 단위.
        self.thickness = if self.thickness.is_finite() {
            ((self.thickness * 2.0).round() / 2.0).clamp(1.0, 15.0)
        } else {
            2.0
        };
        self.line_opacity = clamp_f(self.line_opacity, 0.0, 1.0, 1.0);
        self.fade_fraction = clamp_f(self.fade_fraction, 0.0, 0.2, 0.012);
        for m in [&mut self.corner_radii, &mut self.menu_zone_radii, &mut self.dock_zone_radii] {
            for v in m.values_mut() {
                *v = clamp_f(*v, 0.0, 80.0, 0.0);
            }
        }
        self.notch_width = clamp_f(self.notch_width, 0.0, 500.0, 186.0);
        self.notch_height = clamp_f(self.notch_height, 0.0, 100.0, 33.0);
        for v in self.notch_radii.values_mut() {
            *v = clamp_f(*v, 0.0, 40.0, 0.0);
        }
        self.hover_corner_size = clamp_f(self.hover_corner_size, 20.0, 300.0, 80.0);
        self.menu_line_height = clamp_f(self.menu_line_height, 5.0, 200.0, 25.0);
        self.dock_line_height = clamp_f(self.dock_line_height, 5.0, 400.0, 70.0);
        self.refresh_custom_minutes = self.refresh_custom_minutes.clamp(1, 180);
        // 차트 기간은 6/12/24 중 가장 가까운 값.
        self.chart_hours = match self.chart_hours {
            0..=9 => 6,
            10..=18 => 12,
            _ => 24,
        };
    }

    /// Windows는 메뉴바선·노치만 끈다. 독 세그먼트는 작업 표시줄에 사용한다.
    pub fn apply_platform_rules(&mut self, windows: bool) {
        if !windows {
            return;
        }
        self.menu_line_enabled = false;
        self.notch_enabled = false;
        const KEEP: [SegPart; 7] = [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain, SegPart::HDock, SegPart::LDock, SegPart::RDock];
        let keep_set = |set: &BTreeSet<SegPart>| -> BTreeSet<SegPart> {
            set.iter().copied().filter(|s| KEEP.contains(s)).collect()
        };

        // 구버전 레이아웃(segments 없음)은 먼저 세그먼트로 풀어 둔다.
        for id in PROVIDER_IDS {
            if self.provider_layouts.contains_key(id) && self.provider_layouts[id].segments.is_none() {
                let segs = self.segs_for(id);
                if let Some(l) = self.provider_layouts.get_mut(id) {
                    l.segments = Some(segs);
                }
            }
        }
        for (id, l) in self.provider_layouts.iter_mut() {
            let kept = keep_set(l.segments.as_ref().unwrap_or(&BTreeSet::new()));
            l.segments = Some(if is_valid_segments(&kept, false, self.dock_line_enabled) {
                kept
            } else {
                [default_segment_for(id)].into()
            });
            if let Some(caps) = &mut l.h_caps {
                caps.retain(|k, _| {
                    let name = k.split('.').next().unwrap_or("");
                    KEEP.iter().any(|s| s.raw() == name)
                });
            }
        }
        let kept = keep_set(&self.seg_parts);
        self.seg_parts = if is_valid_segments(&kept, false, self.dock_line_enabled) {
            kept
        } else {
            crate::segments::zone_perimeter(&[ScreenZone::Main, ScreenZone::Dock].into(), false, self.dock_line_enabled)
        };
    }
}

fn to_u32(v: Option<i64>, default: u32) -> u32 {
    v.map_or(default, |v| u32::try_from(v).unwrap_or(default))
}

/// providerOrder가 모든 AI를 정확히 한 번씩 담도록 보정. (Swift :1295-1300)
fn normalize_provider_order(po: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out: Vec<String> =
        po.iter().filter(|id| PROVIDER_IDS.contains(&id.as_str()) && seen.insert((*id).clone())).cloned().collect();
    for id in PROVIDER_IDS {
        if !seen.contains(id) {
            out.push(id.to_string());
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn 기본값은_스냅샷_왕복에서_안정() {
        let d = Settings::default();
        let back = Settings::from_snapshot(&d.to_snapshot());
        assert_eq!(d, back);
    }

    #[test]
    fn 기본값_스위프트와_같다() {
        let d = Settings::default();
        assert_eq!(d.thickness, 2.0);
        assert_eq!(d.color, [0.30, 0.82, 0.45, 1.0]);
        assert_eq!(d.fade_fraction, 0.012);
        assert_eq!(d.notch_width, 186.0);
        assert_eq!(d.notch_height, 33.0);
        assert_eq!(d.notch_radius(NotchCorner::OuterLeft), 5.0);
        assert_eq!(d.notch_radius(NotchCorner::InnerRight), 12.0);
        // Windows는 처음 설치 때 곡률 없음, macOS는 Swift와 같은 22·12.
        let (top, dock) = if cfg!(windows) { (0.0, 0.0) } else { (22.0, 12.0) };
        assert_eq!((super::super::defaults::TOP_RADIUS, super::super::defaults::DOCK_RADIUS), (top, dock));
        assert_eq!(d.radius(BorderCorner::TopLeft), top);
        assert_eq!(d.radius(BorderCorner::BottomRight), 0.0);
        assert_eq!(d.dock_zone_radii[&BorderCorner::TopLeft], dock);
        assert_eq!(d.language, AppLanguage::English);
        assert_eq!(d.notify_thresholds, [75, 90, 95].into());
        assert_eq!(d.hover_corner_size, 80.0);
        assert_eq!(d.menu_line_height, 25.0);
        assert_eq!(d.dock_line_height, 70.0);
        assert!(d.menu_bar_percent_ids.is_empty());
        assert_eq!(d.segs_for("claude"), [SegPart::HTop].into());
        assert_eq!(d.segs_for("gemini"), [SegPart::HBottom].into());
        assert_eq!(d.segs_for("codex"), [SegPart::LMain].into());
        assert_eq!(d.layout_for("codex").anchor_corner, BorderCorner::BottomLeft);
        assert_eq!(d.layout_for("gemini").anchor_corner, BorderCorner::BottomLeft);
        assert_eq!(d.layout_for("claude").anchor_corner, BorderCorner::TopLeft);
        assert_eq!(d.effective_refresh_minutes(), 5);
    }

    #[test]
    fn 갱신_주기_유효값() {
        let mut s = Settings::default();
        s.refresh_use_custom = true;
        s.refresh_custom_minutes = 0;
        assert_eq!(s.effective_refresh_minutes(), 1);
        s.refresh_custom_minutes = 47;
        assert_eq!(s.effective_refresh_minutes(), 47);
        assert!(REFRESH_PRESETS.contains(&15));
    }

    #[test]
    fn 범위_제한() {
        let mut s = Settings::default();
        s.thickness = 3.26;
        s.line_opacity = 4.0;
        s.fade_fraction = -1.0;
        s.corner_radii.insert(BorderCorner::TopLeft, 200.0);
        s.notch_width = 900.0;
        s.notch_height = -3.0;
        s.notch_radii.insert(NotchCorner::InnerLeft, 99.0);
        s.hover_corner_size = 5.0;
        s.menu_line_height = 1.0;
        s.dock_line_height = 999.0;
        s.refresh_custom_minutes = 999;
        s.chart_hours = 7;
        s.clamp_all();
        assert_eq!(s.thickness, 3.5);
        assert_eq!(s.line_opacity, 1.0);
        assert_eq!(s.fade_fraction, 0.0);
        assert_eq!(s.radius(BorderCorner::TopLeft), 80.0);
        assert_eq!(s.notch_width, 500.0);
        assert_eq!(s.notch_height, 0.0);
        assert_eq!(s.notch_radius(NotchCorner::InnerLeft), 40.0);
        assert_eq!(s.hover_corner_size, 20.0);
        assert_eq!(s.menu_line_height, 5.0);
        assert_eq!(s.dock_line_height, 400.0);
        assert_eq!(s.refresh_custom_minutes, 180);
        assert_eq!(s.chart_hours, 6);
        s.thickness = f64::NAN;
        s.clamp_all();
        assert_eq!(s.thickness, 2.0);
    }

    #[test]
    fn 구버전_스냅샷_마이그레이션() {
        let json = r#"{"thickness":2,"colorRGBA":[1,0,0,1],"edges":["top","left"],
            "cornerRadii":{"topLeft":5,"bogus":3},"showTrack":true,"notchEnabled":false,
            "notchWidth":186,"notchHeight":33,"notchRadii":{"outerLeft":1},
            "anchorCorner":"x","anchorSide":"left","anchorClockwise":true,
            "showPercentInMenuBar":true,
            "providerOrder":["codex","codex","zzz"],
            "providerLayouts":{"claude":{"edges":["bottom"],"anchorCorner":"topLeft","anchorSide":"center","clockwise":true}}}"#;
        let snap: SettingsSnapshot = serde_json::from_str(json).unwrap();
        let s = Settings::from_snapshot(&snap);
        // 변 + 영역 -> 세그먼트
        assert_eq!(s.seg_parts, [SegPart::HTop, SegPart::LMain].into());
        assert_eq!(s.segs_for("claude"), [SegPart::HBottom].into());
        assert_eq!(s.provider_layouts["claude"].segments, Some([SegPart::HBottom].into()));
        // 전부 표시로 이전
        assert_eq!(s.menu_bar_percent_ids.len(), 3);
        // providerOrder 정규화
        assert_eq!(s.provider_order, vec!["codex", "claude", "gemini"]);
        // 모르는 키는 버리고, 잘못된 anchorCorner는 기본값
        assert_eq!(s.corner_radii.len(), 1);
        assert_eq!(s.anchor_corner, BorderCorner::TopLeft);
        assert_eq!(s.anchor_side, AnchorSide::Left);
        // 없는 항목은 기본값
        assert_eq!(s.fade_fraction, 0.012);
        assert!(s.hover_corners.is_empty());
        assert_eq!(s.hover_edges.len(), 4);
    }

    #[test]
    fn 테두리만_복사() {
        let mut a = Settings::default();
        let mut b = Settings::default();
        b.thickness = 7.0;
        b.language = AppLanguage::Korean;
        b.bar_fill_mode = BarFillMode::Used;
        b.notify_enabled = true;
        b.menu_line_enabled = true;
        b.seg_cap_start = SegCap::Up;
        a.copy_border_from(&b);
        assert_eq!(a.thickness, 7.0);
        assert!(a.menu_line_enabled);
        assert_eq!(a.seg_cap_start, SegCap::Up);
        assert_eq!(a.language, AppLanguage::English);
        assert_eq!(a.bar_fill_mode, BarFillMode::Remaining);
        assert!(!a.notify_enabled);
    }

    #[test]
    fn 윈도우_규칙() {
        let mut s = Settings::default();
        s.menu_line_enabled = true;
        s.dock_line_enabled = true;
        s.notch_enabled = true;
        s.provider_layouts.get_mut("claude").unwrap().segments =
            Some([SegPart::HTop, SegPart::LMenuBar, SegPart::HMenu].into());
        s.provider_layouts.get_mut("gemini").unwrap().segments = Some([SegPart::HDock, SegPart::LDock].into());
        s.seg_parts = [SegPart::HMenu].into();
        s.apply_platform_rules(true);
        assert!(!s.menu_line_enabled && s.dock_line_enabled && !s.notch_enabled);
        assert_eq!(s.segs_for("claude"), [SegPart::HTop].into());
        // 작업 표시줄 구역은 유지한다.
        assert_eq!(s.segs_for("gemini"), [SegPart::HDock, SegPart::LDock].into());
        assert_eq!(s.seg_parts.len(), 6);
        assert!(is_valid_segments(&s.seg_parts, false, true));
        // macOS에서는 아무 것도 안 한다.
        let mut m = Settings::default();
        m.notch_enabled = true;
        m.apply_platform_rules(false);
        assert!(m.notch_enabled);
    }
}
