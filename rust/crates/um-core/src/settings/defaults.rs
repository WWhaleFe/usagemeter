//! 기본값. (Swift OverlaySettings.swift:281-560, 809-821)

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::types::{
    AnchorSide, AppLanguage, BorderCorner, BorderEdge, NotchCorner, ProviderLayout, Rgba, ScreenZone, SegCap, SegPart,
};

use super::live::{AppAppearance, Settings};

/// `ProviderSpec.all` 순서(Providers.swift:38). 실제 AI 목록은 um-providers가 갖게 되면 거기서 가져온다.
pub const PROVIDER_IDS: [&str; 3] = ["claude", "gemini", "codex"];

/// AI 기본 띠 색. (Providers.swift:79, 196, 296)
pub fn default_provider_color(id: &str) -> Option<Rgba> {
    match id {
        "claude" => Some([0.85, 0.47, 0.34, 1.0]),
        "gemini" => Some([0.26, 0.52, 0.96, 1.0]),
        "codex" => Some([0.06, 0.64, 0.50, 1.0]),
        _ => None,
    }
}

/// 기본 AI별 배치: 첫 AI=상단, 둘째=하단, 셋째=좌·본문, 넷째=우·본문. (Swift `defaultLayouts` :809)
pub fn default_layouts() -> BTreeMap<String, ProviderLayout> {
    PROVIDER_IDS
        .iter()
        .enumerate()
        .map(|(i, id)| ((*id).to_string(), default_layout_at(i)))
        .collect()
}

/// 목록 위치 `i`의 기본 배치. 범위를 벗어나면 `i % 4`.
pub(crate) fn default_layout_at(i: usize) -> ProviderLayout {
    let (seg, corner) = match i % 4 {
        0 => (SegPart::HTop, BorderCorner::TopLeft),
        1 => (SegPart::HBottom, BorderCorner::BottomLeft),
        2 => (SegPart::LMain, BorderCorner::BottomLeft),
        _ => (SegPart::RMain, BorderCorner::TopRight),
    };
    layout_with(corner, seg)
}

/// `layout(for:)`의 최종 폴백. (Swift :825)
pub(crate) fn fallback_layout() -> ProviderLayout {
    layout_with(BorderCorner::TopLeft, SegPart::HTop)
}

fn layout_with(anchor_corner: BorderCorner, seg: SegPart) -> ProviderLayout {
    ProviderLayout {
        edges: None,
        anchor_corner,
        anchor_side: AnchorSide::Center,
        clockwise: true,
        extend_start: None,
        extend_end: None,
        no_curve_extend: None,
        zones: None,
        segments: Some([seg].into()),
        cap_start: None,
        cap_end: None,
        h_caps: None,
    }
}

/// 이 AI의 기본 세그먼트(목록 위치 기준). 알 수 없는 id는 `hTop`.
pub(crate) fn default_segment_for(id: &str) -> SegPart {
    match PROVIDER_IDS.iter().position(|p| *p == id) {
        Some(i) => [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain][i % 4],
        None => SegPart::HTop,
    }
}

fn corner_map(tl: f64, tr: f64, br: f64, bl: f64) -> BTreeMap<BorderCorner, f64> {
    [
        (BorderCorner::TopLeft, tl),
        (BorderCorner::TopRight, tr),
        (BorderCorner::BottomRight, br),
        (BorderCorner::BottomLeft, bl),
    ]
    .into()
}

impl Default for Settings {
    fn default() -> Self {
        let first = PROVIDER_IDS[0].to_string();
        Self {
            thickness: 2.0,
            color: [0.30, 0.82, 0.45, 1.0],
            line_opacity: 1.0,
            extend_start_corner: false,
            extend_end_corner: false,
            start_corner_above: false,
            edges: BorderEdge::ALL.into_iter().collect(),
            corner_radii: corner_map(22.0, 22.0, 0.0, 0.0),
            menu_zone_radii: corner_map(22.0, 22.0, 0.0, 0.0),
            dock_zone_radii: corner_map(12.0, 12.0, 0.0, 0.0),
            link_menu_main_radii: true,
            link_main_dock_radii: true,
            no_overlap_lines: false,
            split_overlap_lines: false,
            show_track: true,
            hide_overlay_on_launch: false,
            fade_enabled: true,
            fade_fraction: 0.012,
            anchor_corner: BorderCorner::TopLeft,
            anchor_side: AnchorSide::Center,
            anchor_clockwise: true,
            notch_enabled: true,
            notch_width: 186.0,
            notch_height: 33.0,
            notch_radii: HashMap::from([
                (NotchCorner::OuterLeft, 5.0),
                (NotchCorner::OuterRight, 5.0),
                (NotchCorner::InnerLeft, 12.0),
                (NotchCorner::InnerRight, 12.0),
            ]),
            provider_colors: PROVIDER_IDS
                .iter()
                .filter_map(|id| default_provider_color(id).map(|c| ((*id).to_string(), c)))
                .collect(),
            keep_logged_in: true,
            language: AppLanguage::English, // 처음 설치 시 영문 기본
            refresh_preset_minutes: 5,
            refresh_use_custom: false,
            refresh_custom_minutes: 5,
            menu_bar_icon_id: first.clone(),
            menu_bar_percent_ids: BTreeSet::new(),
            menu_bar_percent_first_id: first,
            menu_show_5h: true,
            menu_show_weekly: true,
            menu_show_reset: true,
            menu_show_updated: false,
            menu_show_opus: true,
            menu_show_countdown: true,
            menu_show_pace: false,
            menu_show_chart: false,
            chart_hours: 24,
            appearance: AppAppearance::System,
            hover_info_enabled: true,
            chart_hover_enabled: true,
            menu_bar_hover_info_enabled: true,
            hover_edges: BorderEdge::ALL.into_iter().collect(),
            hover_corners: BTreeSet::new(),
            hover_corner_size: 80.0,
            provider_order: PROVIDER_IDS.iter().map(|s| (*s).to_string()).collect(),
            notify_enabled: false,
            notify_thresholds: [75, 90, 95].into(),
            auto_check_update: true,
            menu_line_enabled: false,
            menu_line_height: 25.0,
            dock_line_enabled: false,
            dock_line_height: 70.0,
            zones: ScreenZone::ALL.into_iter().collect(),
            seg_parts: [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain].into(),
            seg_cap_start: SegCap::None,
            seg_cap_end: SegCap::None,
            separate_by_ai: false,
            provider_layouts: default_layouts(),
            sub_item_split_ids: BTreeSet::new(),
            sub_item_colors: BTreeMap::new(),
            sub_item_bar_hidden: BTreeSet::new(),
            sub_item_menu_hidden: BTreeSet::new(),
        }
    }
}
