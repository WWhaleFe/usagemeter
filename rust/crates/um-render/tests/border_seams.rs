//! 창 이음매와 둥근 경로의 위치·접선 연속성 회귀 검사.

use kurbo::{ParamCurve, ParamCurveArclen, PathSeg};
use um_core::{
    settings::Settings,
    types::{BorderCorner, ScreenZone},
};
use um_render::border::{self, BandPart, BorderInput, BorderScene, ProviderBand, StripSide};

fn scene(s: &Settings, w: f64, h: f64, side: StripSide, parts: bool) -> BorderScene {
    let bands = ["claude", "codex", "gemini"]
        .into_iter()
        .enumerate()
        .map(|(i, id)| {
            let color = [
                [1.0, 0.2, 0.1, 0.75],
                [0.1, 0.8, 0.3, 1.0],
                [0.2, 0.3, 1.0, 0.6],
            ][i];
            ProviderBand {
                id: id.into(),
                ratio: 1.0 - i as f64 * 0.13,
                color,
                parts: if parts {
                    vec![
                        BandPart {
                            id: "a".into(),
                            ratio: 1.0,
                            color,
                        },
                        BandPart {
                            id: "b".into(),
                            ratio: 0.57,
                            color: [0.7, 0.3, 0.9, 0.7],
                        },
                    ]
                } else {
                    vec![]
                },
            }
        })
        .collect();
    border::build_positioned_paths(
        &BorderInput {
            settings: s,
            screen_w: w,
            screen_h: h,
            bands,
        },
        side,
    )
}

fn settings(radius: f64, notch: bool, menu: bool, dock: bool, split: bool, width: f64) -> Settings {
    let mut s = Settings {
        thickness: width,
        notch_enabled: notch,
        menu_line_enabled: menu,
        dock_line_enabled: dock,
        split_overlap_lines: split,
        line_opacity: 0.73,
        menu_line_height: 25.5,
        dock_line_height: 70.5,
        ..Default::default()
    };
    for c in BorderCorner::ALL {
        s.corner_radii.insert(c, radius);
        s.menu_zone_radii.insert(c, radius * 0.75);
        s.dock_zone_radii.insert(c, radius * 0.5);
    }
    s
}

fn tangent(s: PathSeg, end: bool) -> kurbo::Vec2 {
    match s {
        PathSeg::Line(l) => l.p1 - l.p0,
        PathSeg::Quad(q) => {
            if end {
                q.p2 - q.p1
            } else {
                q.p1 - q.p0
            }
        }
        PathSeg::Cubic(c) => {
            if end {
                c.p3 - c.p2
            } else {
                c.p1 - c.p0
            }
        }
    }
}

fn continuity(scene: &BorderScene, label: &str) {
    for item in &scene.items {
        let segs: Vec<_> = item
            .path
            .segments()
            .filter(|s| s.arclen(1e-4) > 1e-7)
            .collect();
        for seg in &segs {
            if let PathSeg::Cubic(c) = seg {
                for i in 0..=100 {
                    let t = f64::from(i) / 100.0;
                    let derivative = (c.p1 - c.p0) * (3.0 * (1.0 - t).powi(2))
                        + (c.p2 - c.p1) * (6.0 * t * (1.0 - t))
                        + (c.p3 - c.p2) * (3.0 * t * t);
                    assert!(
                        derivative.hypot() > 1e-9,
                        "연결 곡선 안에서 접선 소실 {label}"
                    );
                }
            }
        }
        let moves = item
            .path
            .elements()
            .iter()
            .filter(|el| matches!(el, kurbo::PathEl::MoveTo(_)))
            .count();
        assert!(moves <= 1, "체인이 끊김 {label} {}", item.id);
        for pair in segs.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(a.end().distance(b.start()) < 1e-7, "G0 {label} {}", item.id);
            if matches!((a, b), (PathSeg::Line(_), PathSeg::Line(_))) {
                continue;
            }
            let (u, v) = (tangent(a, true).normalize(), tangent(b, false).normalize());
            assert!(
                u.dot(v) > 1.0 - 1e-8,
                "G1 {label} {}: {a:?} -> {b:?}, dot={}",
                item.id,
                u.dot(v)
            );
        }
        if let (Some(&first), Some(&last)) = (segs.first(), segs.last())
            && first.start().distance(last.end()) < 1e-7
        {
            let (u, v) = (
                tangent(last, true).normalize(),
                tangent(first, false).normalize(),
            );
            if !matches!((last, first), (PathSeg::Line(_), PathSeg::Line(_))) {
                assert!(u.dot(v) > 1.0 - 1e-8, "고리 G1 {label} {}", item.id);
            }
        }
        for sp in item.spans.windows(2) {
            assert!((sp[0].to - sp[1].from).abs() < 1e-9, "스팬 G0 {label}");
            assert!(sp.iter().all(|s| s.width.is_finite() && s.width > 0.0));
            if sp[0].notch == sp[1].notch {
                assert!(
                    (sp[0].width - sp[1].width).abs() < scene.thickness * 0.1,
                    "굵기 단차 {label}: {sp:?}"
                );
            }
        }
    }
}

#[test]
fn 경로_모서리_연속성() {
    for radius in [0.0, 9.0, 22.0, 40.0] {
        for notch in [false, true] {
            for (menu, dock) in [(false, false), (true, false), (false, true), (true, true)] {
                for side in [
                    StripSide::Bottom,
                    StripSide::Left,
                    StripSide::Right,
                    StripSide::Top,
                ] {
                    for split in [false, true] {
                        for width in [2.0, 3.5, 6.0, 15.0] {
                            let mut s = settings(radius, notch, menu, dock, split, width);
                            for zone in ScreenZone::ALL {
                                let segments = um_core::segments::zone_perimeter_on(
                                    &[zone].into(),
                                    menu,
                                    dock,
                                    side.edge(),
                                );
                                for id in ["claude", "codex", "gemini"] {
                                    s.provider_layouts.get_mut(id).unwrap().segments =
                                        Some(segments.clone());
                                }
                                let scene = scene(&s, 1512.0, 982.0, side, true);
                                for item in &scene.items {
                                    let segs: Vec<_> = item.path.segments().collect();
                                    if let (Some(a), Some(b)) = (segs.first(), segs.last()) {
                                        assert!(
                                            a.start().distance(b.end()) < 1e-7,
                                            "구역 고리 G0 {side:?} {zone:?}"
                                        );
                                    }
                                }
                                continuity(
                                    &scene,
                                    &format!(
                                        "r={radius} n={notch} m={menu} d={dock} {side:?} split={split} t={width} {zone:?}"
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn 캡과_옆_독_접합_연속성() {
    use um_core::types::{SegCap, SegPart};
    for side in [
        StripSide::Bottom,
        StripSide::Left,
        StripSide::Right,
        StripSide::Top,
    ] {
        for radius in [0.0, 9.0, 22.0, 40.0] {
            for cap in [SegCap::None, SegCap::Up, SegCap::Down] {
                for clockwise in [false, true] {
                    for part in SegPart::ALL {
                        for next in SegPart::ALL {
                            let segments = [part, next].into();
                            if !um_core::segments::is_valid_segments_on(
                                &segments,
                                true,
                                true,
                                side.edge(),
                            ) {
                                continue;
                            }
                            let mut s = settings(radius, true, true, true, true, 6.0);
                            for id in ["claude", "codex", "gemini"] {
                                let layout = s.provider_layouts.get_mut(id).unwrap();
                                layout.clockwise = clockwise;
                                layout.segments = Some(segments.clone());
                                layout.h_caps = Some(
                                    [
                                        (um_core::segments::h_cap_key(part, false), cap),
                                        (um_core::segments::h_cap_key(part, true), cap),
                                    ]
                                    .into(),
                                );
                            }
                            continuity(
                                &scene(&s, 1512.0, 982.0, side, true),
                                &format!(
                                    "{side:?} r={radius} {cap:?} cw={clockwise} {part:?} {next:?}"
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn 스쿱_고리의_시작점도_방향이_연속() {
    use um_core::types::{AnchorSide, SegCap, SegPart};
    for side in [
        StripSide::Bottom,
        StripSide::Left,
        StripSide::Right,
        StripSide::Top,
    ] {
        for corner in BorderCorner::ALL {
            for anchor in [AnchorSide::Left, AnchorSide::Center, AnchorSide::Right] {
                for clockwise in [false, true] {
                    for cap in [SegCap::Up, SegCap::Down] {
                        let mut s = settings(22.0, true, true, true, true, 6.0);
                        for id in ["claude", "codex", "gemini"] {
                            let layout = s.provider_layouts.get_mut(id).unwrap();
                            layout.anchor_corner = corner;
                            layout.anchor_side = anchor;
                            layout.clockwise = clockwise;
                            layout.segments = Some(um_core::segments::zone_perimeter_on(
                                &[ScreenZone::Main].into(),
                                true,
                                true,
                                side.edge(),
                            ));
                            layout.h_caps = Some(
                                [SegPart::HMenu, SegPart::HDock]
                                    .into_iter()
                                    .flat_map(|p| {
                                        [false, true].map(|right| {
                                            (um_core::segments::h_cap_key(p, right), cap)
                                        })
                                    })
                                    .collect(),
                            );
                        }
                        continuity(
                            &scene(&s, 1512.0, 982.0, side, true),
                            &format!("고리 {side:?} {corner:?} {anchor:?} {clockwise} {cap:?}"),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn 분할_굵기의_평활_전환() {
    let (mut ordinary, mut notch_boundary) = (0.0_f64, 0.0_f64);
    for width in [2.0, 3.5, 6.0, 15.0] {
        let mut s = settings(40.0, true, true, true, true, width);
        for (id, zones) in [
            ("claude", vec![ScreenZone::Main]),
            ("codex", vec![ScreenZone::Main, ScreenZone::MenuBar]),
            ("gemini", vec![ScreenZone::Main, ScreenZone::Dock]),
        ] {
            s.provider_layouts.get_mut(id).unwrap().segments = Some(
                um_core::segments::zone_perimeter(&zones.into_iter().collect(), true, true),
            );
        }
        let scene = scene(&s, 1512.0, 982.0, StripSide::Bottom, true);
        for item in scene.items {
            for pair in item.spans.windows(2) {
                let delta = (pair[0].width - pair[1].width).abs() / width;
                if pair[0].notch != pair[1].notch {
                    notch_boundary = notch_boundary.max(delta);
                } else {
                    ordinary = ordinary.max(delta);
                }
            }
        }
    }
    assert!(ordinary < 0.1);
    assert!(
        notch_boundary < 0.02,
        "노치 고정 경계가 부드럽게 이어져야 한다: {notch_boundary}"
    );
    println!("굵기 인접 차/원굵기: 일반 {ordinary:.6}, 노치 고정 경계 {notch_boundary:.6}");
}

#[test]
fn 띠_합성은_전체_화면과_동일() {
    let exhaustive = std::env::var_os("CORNER_EXHAUSTIVE").is_some();
    let mut cases = 0;
    let mut checked = 0;
    for radius in [0.0, 9.0, 22.0, 40.0] {
        for notch in [false, true] {
            for (menu, dock) in [(false, false), (true, false), (false, true), (true, true)] {
                for side in [
                    StripSide::Bottom,
                    StripSide::Left,
                    StripSide::Right,
                    StripSide::Top,
                ] {
                    for split in [false, true] {
                        for parts in [false, true] {
                            for width in [2.0, 3.5, 6.0, 15.0] {
                                for scale in [1.0, 1.25, 1.5, 2.0] {
                                    for (w, h) in [
                                        (2176.0, 1360.0),
                                        (1512.0, 982.0),
                                        (1366.0, 768.0),
                                        (1367.0, 769.0),
                                    ] {
                                        let index = cases;
                                        cases += 1;
                                        // 일상 검사에서도 모든 축의 값을 포함하고 전체 조합은 별도 실행한다.
                                        if !exhaustive && index % 257 != 0 {
                                            continue;
                                        }
                                        checked += 1;
                                        let mut s =
                                            settings(radius, notch, menu, dock, split, width);
                                        for (id, zones) in [
                                            ("claude", vec![ScreenZone::Main]),
                                            ("codex", vec![ScreenZone::MenuBar, ScreenZone::Main]),
                                            ("gemini", vec![ScreenZone::Main, ScreenZone::Dock]),
                                        ] {
                                            s.provider_layouts.get_mut(id).unwrap().segments =
                                                Some(um_core::segments::zone_perimeter_on(
                                                    &zones.into_iter().collect(),
                                                    menu,
                                                    dock,
                                                    side.edge(),
                                                ));
                                        }
                                        let scene = scene(&s, w, h, side, parts);
                                        assert!(scene.items.iter().all(|i| !i.path.is_empty()));
                                        let full =
                                            border::render(&scene, (0.0, 0.0), (w, h), scale);
                                        let mut tiled =
                                            tiny_skia::Pixmap::new(full.width(), full.height())
                                                .unwrap();
                                        for r in border::strip_layout(&s, w, h, side, scale) {
                                            let pm = border::crop_strip(&full, r, scale);
                                            tiled.draw_pixmap(
                                                (r[0] * scale).round() as i32,
                                                (r[1] * scale).round() as i32,
                                                pm.as_ref(),
                                                &Default::default(),
                                                tiny_skia::Transform::identity(),
                                                None,
                                            );
                                        }
                                        if full.data() != tiled.data() {
                                            let bad = full
                                                .pixels()
                                                .iter()
                                                .zip(tiled.pixels())
                                                .position(|(a, b)| a != b)
                                                .unwrap();
                                            panic!(
                                                "이음매 ({},{}): r={radius} notch={notch} menu={menu} dock={dock} {side:?} split={split} parts={parts} t={width} scale={scale} {w}x{h}",
                                                bad as u32 % full.width(),
                                                bad as u32 / full.width()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("띠 조합 {cases}, 검사 {checked}, 전체 검사={exhaustive}, 채널 차 0");
}
