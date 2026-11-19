//! 메뉴바·트레이 링 아이콘. (Swift StatusBarController.swift:904-932 `ringImage`)
//! 18×18pt, inset 2.5, 선 2.4, 반지름 6.5. 트랙 = AI 색 알파 0.28(흐림 0.25),
//! 호 = 12시에서 시계 방향으로 잔여율 × 360°, 둥근 끝.

use kurbo::{Arc, Point, Shape, Vec2};
use tiny_skia::{LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};
use um_core::types::Rgba;

const SIZE: f64 = 18.0;
const LINE: f64 = 2.4;
const RADIUS: f64 = 6.5;

/// 링 아이콘. `ratio`가 None이면 흐린 트랙만(로그인된 AI 없음). 반환은 미리곱한 RGBA.
pub fn ring(ratio: Option<f64>, color: Rgba, scale: f64) -> Pixmap {
    let px = (SIZE * scale).round() as u32;
    let mut pm = Pixmap::new(px, px).expect("icon size");
    let c = SIZE * scale / 2.0;
    let r = RADIUS * scale;
    let stroke = Stroke { width: (LINE * scale) as f32, line_cap: LineCap::Round, ..Default::default() };
    let rgba8 = |a: f64| {
        let mut p = Paint { anti_alias: true, ..Default::default() };
        p.set_color_rgba8(
            (color[0] * 255.0).round() as u8,
            (color[1] * 255.0).round() as u8,
            (color[2] * 255.0).round() as u8,
            (a * 255.0).round() as u8,
        );
        p
    };
    let track_alpha = if ratio.is_some() { 0.28 } else { 0.25 };
    #[cfg(windows)]
    let track_alpha = if ratio.is_none() { 0.85 } else { track_alpha };
    let mut pb = PathBuilder::new();
    pb.push_circle(c as f32, c as f32, r as f32);
    if let Some(p) = pb.finish() {
        pm.stroke_path(&p, &rgba8(color[3] * track_alpha), &stroke, Transform::identity(), None);
    }
    if let Some(ratio) = ratio {
        let sweep = ratio.clamp(0.0, 1.0) * std::f64::consts::TAU;
        if sweep > 0.0 {
            // 화면 좌표(y 아래)에서 12시 = -90°, 시계 방향 = 양의 각.
            let arc = Arc::new(Point::new(c, c), Vec2::new(r, r), -std::f64::consts::FRAC_PI_2, sweep, 0.0);
            let mut b = PathBuilder::new();
            for el in arc.path_elements(0.1) {
                match el {
                    kurbo::PathEl::MoveTo(p) => b.move_to(p.x as f32, p.y as f32),
                    kurbo::PathEl::LineTo(p) => b.line_to(p.x as f32, p.y as f32),
                    kurbo::PathEl::QuadTo(a, p) => b.quad_to(a.x as f32, a.y as f32, p.x as f32, p.y as f32),
                    kurbo::PathEl::CurveTo(a, d, p) => b.cubic_to(a.x as f32, a.y as f32, d.x as f32, d.y as f32, p.x as f32, p.y as f32),
                    kurbo::PathEl::ClosePath => b.close(),
                }
            }
            if let Some(p) = b.finish() {
                pm.stroke_path(&p, &rgba8(color[3]), &stroke, Transform::identity(), None);
            }
        }
    }
    pm
}

/// 미리곱한 픽셀을 일반 RGBA로(트레이 API용).
pub fn to_straight_rgba(pm: &Pixmap) -> Vec<u8> {
    pm.pixels().iter().flat_map(|p| { let c = p.demultiply(); [c.red(), c.green(), c.blue(), c.alpha()] }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 링_크기와_채움() {
        let full = ring(Some(1.0), [0.85, 0.47, 0.34, 1.0], 2.0);
        assert_eq!((full.width(), full.height()), (36, 36));
        let ink = |pm: &Pixmap| pm.pixels().iter().filter(|p| p.alpha() > 200).count();
        let empty = ring(Some(0.0), [0.85, 0.47, 0.34, 1.0], 2.0);
        assert!(ink(&full) > ink(&empty) + 100, "가득 찬 링이 더 진해야 함");
        assert_eq!(to_straight_rgba(&full).len(), 36 * 36 * 4);
    }
}
