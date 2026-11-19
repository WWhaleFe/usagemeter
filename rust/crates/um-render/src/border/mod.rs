//! 테두리 오버레이 기하·그리기. Swift `BorderView.swift`(SegmentChainShape + BorderView)의 이식.
//!
//! 흐름: `build_paths`(설정 + 잔여율 -> 화면 전체 좌표 경로·굵기 구간, 픽셀 없음) ->
//! `render`(장면을 띠 원점만큼 옮겨 tiny-skia로 그림).
//! 모양·규칙은 Swift를 그대로 따른다(계획서 5-2, 부록 B). 세그먼트 그래프·끝 보정은 `um_core::segments`.

mod draw;
mod hover;
mod items;
mod shape;
mod spans;

use kurbo::{BezPath, ParamCurveArclen};
use um_core::settings::Settings;
use um_core::types::{BorderCorner, Rgba, ScreenZone, SegPart};

pub use draw::{render, render_emphasized};
pub use hover::HoverGeometry;
pub use items::{provider_color, sub_item_bar_parts, sub_item_color};
pub use shape::{Built, ChainShape, Rec};
pub use spans::{Span, merge_spans, tapered};

/// 세부 항목 하나의 띠(잔여율 + 색). (Swift `subItemBarParts`의 원소)
#[derive(Debug, Clone, PartialEq)]
pub struct BandPart {
    pub id: String,
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

/// 그릴 띠 하나: 어느 경로(`items` 인덱스)에 어떤 길이 비율·색으로.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneBand {
    /// 세부 항목 id. 대표 띠는 None.
    pub part: Option<String>,
    pub item: usize,
    pub ratio: f64,
    pub color: Rgba,
}

/// 화면 전체 좌표의 테두리 장면(기하만).
#[derive(Debug, Clone)]
pub struct BorderScene {
    pub screen: (f64, f64),
    /// 경로·트랙 순서(잔여율 높은 AI부터)
    pub items: Vec<SceneItem>,
    /// 세부 항목까지 펼쳐 실제 길이가 긴 순(뒤 -> 앞). 짧은 띠가 위에 그려진다.
    pub bands: Vec<SceneBand>,
    pub thickness: f64,
    pub line_opacity: f64,
    pub show_track: bool,
    pub fade_enabled: bool,
    pub fade_fraction: f64,
}

/// 설정과 잔여율로 장면(경로·굵기 구간·그리기 순서)을 만든다. (Swift `drawItems` + Canvas의 준비 단계)
pub fn build_paths(input: &BorderInput) -> BorderScene {
    build_positioned_paths(input, StripSide::Bottom)
}

/// Dock 방향과 고정 메뉴바를 함께 반영한다. 아래 Dock은 Swift 기준 경로를 그대로 쓴다.
pub fn build_positioned_paths(input: &BorderInput, side: StripSide) -> BorderScene {
    let s = input.settings;
    let t = s.thickness;
    let fixed_menu = s.menu_line_enabled && s.dock_line_enabled && matches!(side, StripSide::Left | StripSide::Right);
    let (w, h) = if fixed_menu { (input.screen_w, input.screen_h) }
        else { side.canonical_size(input.screen_w, input.screen_h) };
    let draw_items = items::draw_items(s, &input.bands);
    let mut scene_items = vec![];
    let mut parts_of: Vec<(Vec<BandPart>, f64, Rgba)> = vec![];
    // AI별 (경로, 굵기 구간) 준비. 노치 우회가 있으면 분할이 없어도 구간을 계산해 노치를 별도 레이어로 분리한다.
    for it in &draw_items {
        let built = if fixed_menu { it.shape.build_side_dock(s, w, h, side.edge()) }
            else { it.shape.build(w, h) };
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
    // 대표·세부 항목 모두 길이 비율만 바꾼다. 경로·색·트랙은 그대로 둔다.
    let mut bands: Vec<SceneBand> = parts_of
        .iter()
        .enumerate()
        .flat_map(|(i, (parts, ratio, color))| {
            if parts.is_empty() {
                vec![SceneBand { part: None, item: i, ratio: s.bar_fill_mode.fill_ratio(*ratio), color: *color }]
            } else {
                parts.iter().map(|p| SceneBand { part: Some(p.id.clone()), item: i, ratio: s.bar_fill_mode.fill_ratio(p.ratio), color: p.color }).collect()
            }
        })
        .collect();
    // 비율이 같아도 경로 길이가 다를 수 있다. 실제 길이가 짧은 띠를 나중에 그린다(안정 정렬).
    let lengths: Vec<f64> = scene_items.iter().map(|it| {
        it.path.segments().map(|seg| seg.arclen(1e-4)).filter(|len| *len > 0.0).sum()
    }).collect();
    bands.sort_by(|a, b| {
        let (a_len, b_len) = (lengths[a.item] * a.ratio, lengths[b.item] * b.ratio);
        b_len.partial_cmp(&a_len).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut scene = BorderScene {
        screen: (w, h),
        items: scene_items,
        bands,
        thickness: t,
        line_opacity: s.line_opacity,
        show_track: s.show_track,
        fade_enabled: s.fade_enabled,
        fade_fraction: s.fade_fraction,
    };
    if !fixed_menu { side.orient_scene(&mut scene, input.screen_w, input.screen_h); }
    scene
}

/// 띠 창이 붙는 화면 변.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripSide {
    Top,
    Bottom,
    Left,
    Right,
}

impl StripSide {
    pub fn edge(self) -> um_core::types::BorderEdge {
        use um_core::types::BorderEdge;
        match self { Self::Top => BorderEdge::Top, Self::Bottom => BorderEdge::Bottom,
            Self::Left => BorderEdge::Left, Self::Right => BorderEdge::Right }
    }

    /// 작업 표시줄이 아래인 기준 좌표계의 크기.
    pub fn canonical_size(self, w: f64, h: f64) -> (f64, f64) {
        match self { Self::Left | Self::Right => (h, w), _ => (w, h) }
    }

    fn orientation(self, w: f64, h: f64) -> kurbo::Affine {
        kurbo::Affine::new(match self {
            Self::Bottom => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            Self::Top => [1.0, 0.0, 0.0, -1.0, 0.0, h],
            Self::Left => [0.0, 1.0, -1.0, 0.0, w, 0.0],
            Self::Right => [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
        })
    }

    /// 세그먼트·시작점·굵기 구간을 유지한 채 장면 전체를 실제 화면으로 옮긴다.
    pub fn orient_scene(self, scene: &mut BorderScene, w: f64, h: f64) {
        let transform = self.orientation(w, h);
        for item in &mut scene.items { item.path.apply_affine(transform); }
        scene.screen = (w, h);
    }

    /// 장면과 동일한 변환을 띠 창에도 적용해 내부 경계가 잘리지 않게 한다.
    pub fn orient_rect(self, r: [f64; 4], w: f64, h: f64) -> [f64; 4] {
        let rect = self.orientation(w, h).transform_rect_bbox(kurbo::Rect::new(r[0], r[1], r[0] + r[2], r[1] + r[3]));
        [rect.x0, rect.y0, rect.width(), rect.height()]
    }
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
    let mut radius = zones
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
    // 내부 가로선의 반대쪽 곡률도 띠 안에 포함한다(Swift OverlayFrameController.swift:95-110).
    let extra = match side {
        StripSide::Top if s.menu_line_enabled => Some((&s.menu_zone_radii, [BorderCorner::BottomLeft, BorderCorner::BottomRight])),
        StripSide::Bottom if s.dock_line_enabled => Some((&s.dock_zone_radii, [BorderCorner::TopLeft, BorderCorner::TopRight])),
        _ => None,
    };
    if let Some((radii, corners)) = extra {
        for c in corners { radius = radius.max(radii.get(&c).copied().unwrap_or(0.0)); }
    }
    let notch = if side == StripSide::Top && s.notch_enabled { s.notch_height.max(0.0) } else { 0.0 };
    s.thickness / 2.0 + s.thickness + radius + notch + 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    use um_core::settings::BarFillMode;
    use um_core::types::AnchorSide;

    const RED: Rgba = [1.0, 0.0, 0.0, 1.0];
    const BLUE: Rgba = [0.0, 0.0, 1.0, 1.0];

    fn 겹침_설정(mode: BarFillMode) -> Settings {
        let mut s = Settings {
            bar_fill_mode: mode, thickness: 4.0, show_track: false, fade_enabled: false,
            notch_enabled: false, corner_radii: Default::default(), ..Settings::default()
        };
        for id in ["claude", "codex"] {
            let l = s.provider_layouts.get_mut(id).unwrap();
            l.segments = Some([SegPart::HTop].into());
            l.anchor_corner = BorderCorner::TopLeft;
            l.anchor_side = AnchorSide::Center;
            l.clockwise = true;
        }
        s
    }

    fn 짧은_띠_픽셀_확인(scene: &BorderScene, short_color: Rgba) {
        assert_eq!(scene.bands.last().unwrap().color, short_color);
        let mut short_scene = scene.clone();
        short_scene.bands = vec![scene.bands[1].clone()];
        let mut long_scene = scene.clone();
        long_scene.bands = vec![scene.bands[0].clone()];
        let short = render(&short_scene, (0.0, 0.0), scene.screen, 1.0);
        let long = render(&long_scene, (0.0, 0.0), scene.screen, 1.0);
        let both = render(scene, (0.0, 0.0), scene.screen, 1.0);
        let mut overlap = 0;
        let mut long_only = 0;
        for ((a, b), actual) in short.pixels().iter().zip(long.pixels()).zip(both.pixels()) {
            if a.alpha() == 255 && b.alpha() == 255 {
                assert_eq!(actual, a);
                overlap += 1;
            }
            if a.alpha() == 0 && b.alpha() == 255 {
                assert_eq!(actual, b);
                long_only += 1;
            }
        }
        assert!(overlap > 100 && long_only > 100);
    }

    #[test]
    fn 옆_독_경계의_겹침_분할도_실제_세로_축을_따름() {
        use kurbo::Shape;
        let mut s = Settings { menu_line_enabled: true, dock_line_enabled: true, dock_line_height: 70.0,
            split_overlap_lines: true, notch_enabled: false, thickness: 4.0, ..Default::default() };
        for id in ["claude", "codex"] {
            s.provider_layouts.get_mut(id).unwrap().segments = Some([SegPart::HDock].into());
        }
        for side in [StripSide::Left, StripSide::Right] {
            let scene = build_positioned_paths(&BorderInput { settings: &s, screen_w: 1200.0, screen_h: 800.0,
                bands: ["claude", "codex"].iter().map(|id| ProviderBand { id: (*id).into(), ratio: 1.0,
                    color: RED, parts: vec![] }).collect() }, side);
            let a = scene.items[0].path.bounding_box();
            let b = scene.items[1].path.bounding_box();
            assert_eq!((a.x0 - b.x0).abs(), 2.0);
            assert_eq!(a.y0, b.y0);
            assert!(scene.items.iter().all(|i| i.spans.iter().all(|s| s.width <= 2.0)));
        }
    }

    #[test]
    fn 독_세_방향과_메뉴바의_실제_경로와_호버() {
        use kurbo::Shape;
        use um_core::segments::zone_perimeter;
        for menu_on in [false, true] {
            let mut s = Settings { thickness: 2.0, notch_enabled: false, menu_line_enabled: menu_on,
                menu_line_height: 25.0, dock_line_enabled: true, dock_line_height: 70.0, ..Default::default() };
            for side in [StripSide::Bottom, StripSide::Left, StripSide::Right] {
                for zone in [ScreenZone::MenuBar, ScreenZone::Dock, ScreenZone::Main] {
                    if !menu_on && zone == ScreenZone::MenuBar { continue; }
                    s.provider_layouts.get_mut("claude").unwrap().segments = Some(zone_perimeter(&[zone].into(), menu_on, true));
                    let scene = build_positioned_paths(&BorderInput { settings: &s, screen_w: 1200.0, screen_h: 800.0,
                        bands: vec![ProviderBand { id: "claude".into(), ratio: 1.0, color: RED, parts: vec![] }] }, side);
                    let b = scene.items[0].path.bounding_box();
                    assert!(!scene.items[0].path.is_empty());
                    if zone == ScreenZone::MenuBar {
                        assert!(b.y1 <= 25.0 && b.width() > 1100.0, "{side:?}: {b:?}");
                    }
                    if zone == ScreenZone::Dock {
                        let (x, y) = match side {
                            StripSide::Bottom => { assert!(b.y0 >= 730.0); (600.0, 730.0) },
                            StripSide::Left => { assert!(b.x1 <= 70.0); (70.0, 400.0) },
                            StripSide::Right => { assert!(b.x0 >= 1130.0); (1130.0, 400.0) },
                            _ => unreachable!(),
                        };
                        let geometry = HoverGeometry::new(&scene);
                        assert_eq!(geometry.candidates(&scene, x, y), vec![0]);
                        assert!(geometry.candidates(&scene, 600.0, 400.0).is_empty());
                    }
                    let image = render(&scene, (0.0, 0.0), scene.screen, 1.0);
                    let emphasized = render_emphasized(&scene, (0.0, 0.0), scene.screen, 1.0, Some(0));
                    assert!(image.pixels().iter().any(|p| p.alpha() > 0));
                    assert!(emphasized.pixels().iter().zip(image.pixels()).all(|(a, b)| a.alpha() == 0 || b.alpha() > 0));
                }
            }
        }
    }

    #[test]
    fn 작업_표시줄_구역과_경계선_네_방향() {
        use kurbo::Shape;
        let mut s = Settings { dock_line_enabled: true, dock_line_height: 40.0,
            thickness: 2.0, notch_enabled: false, ..Settings::default() };
        let l = s.provider_layouts.get_mut("claude").unwrap();
        l.segments = Some([SegPart::HDock].into());
        for side in [StripSide::Bottom, StripSide::Top, StripSide::Left, StripSide::Right] {
            let (w, h) = side.canonical_size(1200.0, 800.0);
            let mut scene = build_paths(&BorderInput { settings: &s, screen_w: w, screen_h: h,
                bands: vec![ProviderBand { id: "claude".into(), ratio: 1.0, color: RED, parts: vec![] }] });
            let original = scene.items[0].path.bounding_box();
            assert!((original.y0 - (h - 40.0)).abs() < 3.0);
            side.orient_scene(&mut scene, 1200.0, 800.0);
            let bounds = scene.items[0].path.bounding_box();
            match side {
                StripSide::Bottom => assert!((bounds.y0 - 760.0).abs() < 3.0),
                StripSide::Top => assert!((bounds.y0 - 40.0).abs() < 3.0),
                StripSide::Left => assert!((bounds.x0 - 40.0).abs() < 3.0),
                StripSide::Right => assert!((bounds.x0 - 1160.0).abs() < 3.0),
            }
            let zone = side.orient_rect([0.0, h - 40.0, w, 40.0], 1200.0, 800.0);
            let expected = match side {
                StripSide::Bottom => [0.0, 760.0, 1200.0, 40.0],
                StripSide::Top => [0.0, 0.0, 1200.0, 40.0],
                StripSide::Left => [0.0, 0.0, 40.0, 800.0],
                StripSide::Right => [1160.0, 0.0, 40.0, 800.0],
            };
            assert_eq!(zone, expected);
            // 내부 경계뿐 아니라 작업 표시줄 구역의 네 변이 모두 그 안에 놓인다.
            let mut zone_settings = s.clone();
            zone_settings.provider_layouts.get_mut("claude").unwrap().segments =
                Some([SegPart::HDock, SegPart::LDock, SegPart::HBottom, SegPart::RDock].into());
            let mut perimeter = build_paths(&BorderInput { settings: &zone_settings, screen_w: w, screen_h: h,
                bands: vec![ProviderBand { id: "claude".into(), ratio: 1.0, color: RED, parts: vec![] }] });
            side.orient_scene(&mut perimeter, 1200.0, 800.0);
            let bounds = perimeter.items[0].path.bounding_box();
            assert!(bounds.x0 >= expected[0] && bounds.y0 >= expected[1]);
            assert!(bounds.x1 <= expected[0] + expected[2] && bounds.y1 <= expected[1] + expected[3]);
            assert!((bounds.width() - expected[2]).abs() <= 2.0);
            assert!((bounds.height() - expected[3]).abs() <= 2.0);
        }
    }

    #[test]
    fn 세부_항목은_표시_방식별_짧은_색이_위() {
        for mode in [BarFillMode::Remaining, BarFillMode::Used] {
            let s = 겹침_설정(mode);
            let input = BorderInput {
                settings: &s, screen_w: 200.0, screen_h: 200.0,
                bands: vec![ProviderBand {
                    id: "claude".into(), ratio: 0.5, color: RED,
                    parts: vec![BandPart { id: "5h".into(), ratio: 0.2, color: RED }, BandPart { id: "weekly".into(), ratio: 0.8, color: BLUE }],
                }],
            };
            let original = input.bands.clone();
            let scene = build_paths(&input);
            짧은_띠_픽셀_확인(&scene, if mode == BarFillMode::Remaining { RED } else { BLUE });
            assert_eq!(input.bands, original); // 메뉴·호버 등에 쓰는 잔여율은 유지한다.
        }
    }

    #[test]
    fn 다른_ai는_비율보다_실제_길이로_정렬() {
        for mode in [BarFillMode::Remaining, BarFillMode::Used] {
            let mut s = 겹침_설정(mode);
            s.provider_layouts.get_mut("codex").unwrap().segments = Some([SegPart::HTop, SegPart::RMain].into());
            let scene = build_paths(&BorderInput {
                settings: &s, screen_w: 200.0, screen_h: 200.0,
                bands: vec![
                    ProviderBand { id: "claude".into(), ratio: if mode == BarFillMode::Remaining { 0.8 } else { 0.2 }, color: RED, parts: vec![] },
                    ProviderBand { id: "codex".into(), ratio: 0.5, color: BLUE, parts: vec![] },
                ],
            });
            assert!(scene.bands[1].ratio > scene.bands[0].ratio);
            짧은_띠_픽셀_확인(&scene, RED);
        }
    }

    #[test]
    fn 대표_띠의_표시_비율과_경로_유지() {
        for remaining in [0.0, 0.25, 1.0] {
            let mut s = 겹침_설정(BarFillMode::Remaining);
            s.show_track = true;
            s.fade_enabled = true;
            let bands = vec![ProviderBand { id: "claude".into(), ratio: remaining, color: RED, parts: vec![] }];
            let before = build_paths(&BorderInput { settings: &s, screen_w: 200.0, screen_h: 200.0, bands: bands.clone() });
            s.bar_fill_mode = BarFillMode::Used;
            let after = build_paths(&BorderInput { settings: &s, screen_w: 200.0, screen_h: 200.0, bands });
            assert_eq!(before.bands[0].ratio, remaining);
            assert_eq!(after.bands[0].ratio, 1.0 - remaining);
            assert_eq!(before.items[0].path, after.items[0].path);
            assert_eq!(before.items[0].spans, after.items[0].spans);
            assert_eq!(before.items[0].color, after.items[0].color);
            assert_eq!(before.bands[0].color, after.bands[0].color);
            assert_eq!(before.show_track, after.show_track);
            assert_eq!(before.fade_enabled, after.fade_enabled);
            assert_eq!(before.fade_fraction, after.fade_fraction);
        }
    }

    #[test]
    fn 강조한_띠는_가린_전체_길이까지_불투명하게_맨_위() {
        let mut s = 겹침_설정(BarFillMode::Remaining);
        s.line_opacity = 0.2;
        s.fade_enabled = true;
        let scene = build_paths(&BorderInput { settings: &s, screen_w: 200.0, screen_h: 200.0,
            bands: vec![ProviderBand { id: "claude".into(), ratio: 0.8, color: RED, parts: vec![] },
                ProviderBand { id: "codex".into(), ratio: 0.2, color: BLUE, parts: vec![] }] });
        let normal = render(&scene, (0.0, 0.0), scene.screen, 1.0);
        assert_eq!(normal.data(), render_emphasized(&scene, (0.0, 0.0), scene.screen, 1.0, None).data());
        let highlighted = render_emphasized(&scene, (0.0, 0.0), scene.screen, 1.0, Some(0));
        for x in [20, 100, 150] {
            let p = highlighted.pixel(x, 2).unwrap();
            assert_eq!(p.alpha(), 255);
            assert!(p.red() > p.blue());
        }
        assert_eq!(highlighted.pixel(190, 2).unwrap().alpha(), 0);
        let mut solid = scene.clone();
        solid.fade_enabled = false;
        solid.line_opacity = 1.0;
        let footprint = render(&solid, (0.0, 0.0), solid.screen, 1.0);
        assert!(highlighted.pixels().iter().enumerate().all(|(i, p)| p.alpha() == 0 || footprint.pixels()[i].alpha() > 0));
    }

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
    fn 내부_가로선_반대쪽_곡률도_포함() {
        let mut s = Settings { notch_enabled: false, menu_line_enabled: true, dock_line_enabled: true, ..Settings::default() };
        s.menu_zone_radii.insert(BorderCorner::BottomLeft, 80.0);
        s.dock_zone_radii.insert(BorderCorner::TopRight, 70.0);
        assert_eq!(strip_extent(&s, StripSide::Top), 87.0);
        assert_eq!(strip_extent(&s, StripSide::Bottom), 87.0);
    }

    #[test]
    fn 배율별_버퍼_크기() {
        let s = Settings::default();
        let scene = build_paths(&BorderInput { settings: &s, screen_w: 600.0, screen_h: 400.0, bands: vec![] });
        for scale in [1.0_f64, 1.25, 1.5, 2.0] {
            let pm = render(&scene, (0.0, 0.0), (7.0, 29.0), scale);
            assert_eq!((pm.width(), pm.height()), ((7.0 * scale).ceil() as u32, (29.0 * scale).ceil() as u32));
            if scale == 1.0 || scale == 2.0 {
                assert_eq!((pm.width(), pm.height()), ((7.0 * scale).round() as u32, (29.0 * scale).round() as u32));
            }
        }
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
