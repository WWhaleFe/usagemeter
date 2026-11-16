//! 테두리 오버레이 기하·그리기. Swift `BorderView.swift`(SegmentChainShape + BorderView)의 이식.
//!
//! 흐름: `build_paths`(설정 + 잔여율 -> 화면 전체 좌표 경로·굵기 구간, 픽셀 없음) ->
//! `render`(장면을 띠 원점만큼 옮겨 tiny-skia로 그림).
//! 모양·규칙은 Swift를 그대로 따른다(계획서 5-2, 부록 B). 세그먼트 그래프·끝 보정은 `um_core::segments`.

mod draw;
mod items;
mod shape;
mod spans;

use kurbo::BezPath;
use um_core::settings::Settings;
use um_core::types::{BorderCorner, Rgba, ScreenZone, SegPart};

pub use draw::render;
pub use items::{provider_color, sub_item_bar_parts, sub_item_color};
pub use shape::{Built, ChainShape, Rec};
pub use spans::{Span, merge_spans, tapered};

/// 세부 항목 하나의 띠(잔여율 + 색). (Swift `subItemBarParts`의 원소)
#[derive(Debug, Clone, PartialEq)]
pub struct BandPart {
    pub ratio: f64,
    pub color: Rgba,
}

/// 로그인해서 그리는 AI 하나의 입력. (Swift `manager.active` 원소 + `color(forProvider:)` + `subItemBarParts`)
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderBand {
    pub id: String,
    /// 대표 잔여율 0~1
    pub ratio: f64,
    pub color: Rgba,
    /// 세부 항목 띠. 비어 있으면 `ratio`·`color` 한 줄. `sub_item_bar_parts`로 만든다.
    pub parts: Vec<BandPart>,
}

/// `build_paths` 입력. `bands`는 켜진(로그인한) AI만, 순서는 상관없다(스펙 순서로 다시 정렬).
#[derive(Debug, Clone)]
pub struct BorderInput<'a> {
    pub settings: &'a Settings,
    /// 화면 전체 크기(pt)
    pub screen_w: f64,
    pub screen_h: f64,
    pub bands: Vec<ProviderBand>,
}

/// AI 하나의 경로. 좌표: pt, 화면 왼쪽 위 원점.
#[derive(Debug, Clone)]
pub struct SceneItem {
    pub id: String,
    pub path: BezPath,
    /// 경로 길이 비율 구간별 굵기. 분할·노치가 없으면 [(0, 1, 두께, false)].
    pub spans: Vec<Span>,
    /// 트랙 색(AI 색)
    pub color: Rgba,
}

/// 그릴 띠 하나: 어느 경로(`items` 인덱스)에 어떤 잔여율·색으로.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneBand {
    pub item: usize,
    pub ratio: f64,
    pub color: Rgba,
}

/// 화면 전체 좌표의 테두리 장면(기하만).
#[derive(Debug, Clone)]
pub struct BorderScene {
    pub screen: (f64, f64),
    /// 뒤 -> 앞(잔여율 높은 AI부터)
    pub items: Vec<SceneItem>,
    /// 세부 항목까지 펼쳐 잔여율 높은 순(뒤 -> 앞). 낮은 것이 위에 그려진다.
    pub bands: Vec<SceneBand>,
    pub thickness: f64,
    pub line_opacity: f64,
    pub show_track: bool,
    pub fade_enabled: bool,
    pub fade_fraction: f64,
}

/// 설정과 잔여율로 장면(경로·굵기 구간·그리기 순서)을 만든다. (Swift `drawItems` + Canvas의 준비 단계)
pub fn build_paths(input: &BorderInput) -> BorderScene {
    let s = input.settings;
    let t = s.thickness;
    let (w, h) = (input.screen_w, input.screen_h);
    let draw_items = items::draw_items(s, &input.bands);
    let mut scene_items = vec![];
    let mut parts_of: Vec<(Vec<BandPart>, f64, Rgba)> = vec![];
    // AI별 (경로, 굵기 구간) 준비. 노치 우회가 있으면 분할이 없어도 구간을 계산해 노치를 별도 레이어로 분리한다.
    for it in &draw_items {
        let built = it.shape.build(w, h);
        let mut spans = vec![Span { from: 0.0, to: 1.0, width: t, notch: false }];
        let has_notch =
            it.shape.notch_detour && it.shape.notch_enabled && it.shape.segments.contains(&SegPart::HTop);
        if it.split || has_notch {
            let (m, total) = merge_spans(&built.rec);
            if !m.is_empty() {
                spans = tapered(&m, total, t);
            }
        }
        scene_items.push(SceneItem { id: it.id.clone(), path: built.path, spans, color: it.color });
        parts_of.push((it.parts.clone(), it.ratio, it.color));
    }
    // 세부 항목까지 펼쳐 잔여율 높은 띠부터(뒤) 그린다(안정 정렬).
    let mut bands: Vec<SceneBand> = parts_of
        .iter()
        .enumerate()
        .flat_map(|(i, (parts, ratio, color))| {
            if parts.is_empty() {
                vec![SceneBand { item: i, ratio: *ratio, color: *color }]
            } else {
                parts.iter().map(|p| SceneBand { item: i, ratio: p.ratio, color: p.color }).collect()
            }
        })
        .collect();
    bands.sort_by(|a, b| b.ratio.partial_cmp(&a.ratio).unwrap_or(std::cmp::Ordering::Equal));
    BorderScene {
        screen: (w, h),
        items: scene_items,
        bands,
        thickness: t,
        line_opacity: s.line_opacity,
        show_track: s.show_track,
        fade_enabled: s.fade_enabled,
        fade_fraction: s.fade_fraction,
    }
}

/// 띠 창이 붙는 화면 변.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripSide {
    Top,
    Bottom,
    Left,
    Right,
}

/// 띠 창 두께(pt, 화면 가장자리에서 안쪽으로). 계획서 4-3 ②:
/// inset + 선 두께 + 해당 변의 최대 곡률 + (윗변이면 노치 깊이) + 여유 4pt.
/// 곡률은 켜진 영역(본문, 메뉴바선·Dock선이 켜졌으면 그 영역)의 그 변 두 꼭짓점 중 최댓값.
/// 메뉴바선·Dock선이 띠 두께 밖에 있으면 잘리므로, 호출 쪽이 `menu_line_height`·`dock_line_height`와 비교해야 한다.
pub fn strip_extent(s: &Settings, side: StripSide) -> f64 {
    let (a, b) = match side {
        StripSide::Top => (BorderCorner::TopLeft, BorderCorner::TopRight),
        StripSide::Bottom => (BorderCorner::BottomLeft, BorderCorner::BottomRight),
        StripSide::Left => (BorderCorner::TopLeft, BorderCorner::BottomLeft),
        StripSide::Right => (BorderCorner::TopRight, BorderCorner::BottomRight),
    };
    let mut zones = vec![ScreenZone::Main];
    if s.menu_line_enabled {
        zones.push(ScreenZone::MenuBar);
    }
    if s.dock_line_enabled {
        zones.push(ScreenZone::Dock);
    }
    let radius = zones
        .iter()
        .flat_map(|z| {
            let m = match z {
                ScreenZone::MenuBar => &s.menu_zone_radii,
                ScreenZone::Main => &s.corner_radii,
                ScreenZone::Dock => &s.dock_zone_radii,
            };
            [a, b].map(|c| m.get(&c).copied().unwrap_or(0.0).max(0.0))
        })
        .fold(0.0, f64::max);
    let notch = if side == StripSide::Top && s.notch_enabled { s.notch_height.max(0.0) } else { 0.0 };
    s.thickness / 2.0 + s.thickness + radius + notch + 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_extent_default_settings() {
        let s = Settings::default(); // 두께 2, 위 곡률 22, 아래 0, 노치 켜짐(깊이 33)
        assert_eq!(strip_extent(&s, StripSide::Left), 1.0 + 2.0 + 22.0 + 4.0);
        assert_eq!(strip_extent(&s, StripSide::Right), 29.0);
        assert_eq!(strip_extent(&s, StripSide::Top), 29.0 + 33.0);
        assert_eq!(strip_extent(&s, StripSide::Bottom), 1.0 + 2.0 + 0.0 + 4.0);
    }

    #[test]
    fn strip_extent_counts_enabled_zones_only() {
        let mut s = Settings::default();
        s.dock_zone_radii.insert(BorderCorner::BottomLeft, 60.0);
        assert_eq!(strip_extent(&s, StripSide::Bottom), 7.0); // Dock선이 꺼져 있으면 무시
        s.dock_line_enabled = true;
        assert_eq!(strip_extent(&s, StripSide::Bottom), 67.0);
    }

    #[test]
    fn empty_input_gives_empty_scene() {
        let s = Settings::default();
        let scene = build_paths(&BorderInput { settings: &s, screen_w: 600.0, screen_h: 400.0, bands: vec![] });
        assert!(scene.items.is_empty() && scene.bands.is_empty());
        let pm = render(&scene, (0.0, 0.0), (10.0, 10.0), 2.0);
        assert!(pm.data().iter().all(|&b| b == 0));
    }
}
