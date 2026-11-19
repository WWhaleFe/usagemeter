//! 화면 픽셀 격자에 맞춘, 겹치지 않는 띠 창 분할.

use super::StripSide;

/// 화면 안 띠 4개(왼쪽 위 원점, pt). 메뉴선·Dock선(macOS)이 켜져 있으면 위·아래 띠를 그 높이까지 넓힌다.
fn layout(s: &um_core::settings::Settings, w: f64, h: f64) -> Vec<[f64; 4]> {
    let l = super::strip_extent(s, StripSide::Left);
    let r = super::strip_extent(s, StripSide::Right);
    let mut t = super::strip_extent(s, StripSide::Top);
    let mut b = super::strip_extent(s, StripSide::Bottom);
    if s.menu_line_enabled {
        t = t.max(s.menu_line_height + super::strip_extent(s, StripSide::Top));
    }
    if s.dock_line_enabled {
        b = b.max(s.dock_line_height + super::strip_extent(s, StripSide::Bottom));
    }
    // 반투명 픽셀이 두 창에서 중복 합성되지 않게 경계를 공유한다.
    vec![
        [0.0, 0.0, l, h],
        [w - r, 0.0, r, h],
        [l, 0.0, w - l - r, t],
        [l, h - b, w - l - r, b],
    ]
}

/// 옆 Dock과 메뉴바가 함께 있으면 위 띠와 옆 띠를 각각 넓힌다.
fn positioned_layout(
    s: &um_core::settings::Settings,
    w: f64,
    h: f64,
    side: StripSide,
) -> Vec<[f64; 4]> {
    if s.menu_line_enabled
        && s.dock_line_enabled
        && matches!(side, StripSide::Left | StripSide::Right)
    {
        let extent = [
            StripSide::Top,
            StripSide::Bottom,
            StripSide::Left,
            StripSide::Right,
        ]
        .into_iter()
        .map(|edge| super::strip_extent(s, edge))
        .fold(0.0, f64::max);
        let l = extent
            + if side == StripSide::Left {
                s.dock_line_height
            } else {
                0.0
            };
        let r = extent
            + if side == StripSide::Right {
                s.dock_line_height
            } else {
                0.0
            };
        let top = extent + s.menu_line_height;
        vec![
            [0.0, 0.0, l, h],
            [w - r, 0.0, r, h],
            [l, 0.0, w - l - r, top],
            [l, h - extent, w - l - r, extent],
        ]
    } else {
        let (cw, ch) = side.canonical_size(w, h);
        layout(s, cw, ch)
            .into_iter()
            .map(|r| side.orient_rect(r, w, h))
            .collect()
    }
}

/// 변환을 마친 경계 좌표를 한 번 반올림하고 창과 픽셀 추출에 같은 사각형을 쓴다.
pub fn strip_layout(
    s: &um_core::settings::Settings,
    w: f64,
    h: f64,
    side: StripSide,
    scale: f64,
) -> Vec<[f64; 4]> {
    positioned_layout(s, w, h, side)
        .into_iter()
        .map(|r| {
            let x0 = (r[0] * scale).round();
            let y0 = (r[1] * scale).round();
            let x1 = if (r[0] + r[2] - w).abs() < 1e-7 {
                (w * scale).ceil()
            } else {
                ((r[0] + r[2]) * scale).round()
            };
            let y1 = if (r[1] + r[3] - h).abs() < 1e-7 {
                (h * scale).ceil()
            } else {
                ((r[1] + r[3]) * scale).round()
            };
            [x0 / scale, y0 / scale, (x1 - x0) / scale, (y1 - y0) / scale]
        })
        .collect()
}

/// 전체 화면을 한 번 래스터화한 결과에서 창 영역을 복사한다. 곡선의 평탄화·AA도 모든 창이 공유한다.
pub fn crop_strip(full: &tiny_skia::Pixmap, rect: [f64; 4], scale: f64) -> tiny_skia::Pixmap {
    let [x, y, w, h] = rect.map(|v| (v * scale).round() as u32);
    full.clone_rect(tiny_skia::IntRect::from_xywh(x as i32, y as i32, w, h).expect("띠 크기"))
        .expect("화면 안 띠")
}
