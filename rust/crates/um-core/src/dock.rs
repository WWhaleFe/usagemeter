//! 실제 화면의 Dock 방향을 반영한 세그먼트. 메뉴바와 옆 Dock은 T자 접합부를 공유한다.
use crate::settings::Settings;
use crate::types::{BorderEdge, SegPart};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentLine {
    pub part: SegPart,
    pub a: (f64, f64),
    pub b: (f64, f64),
}

/// 아래 기준 그래프를 실제 좌표에 펼친다. 겹쳐 되짚는 접합부는 상쇄해 합친 영역의 외곽만 남긴다.
pub fn positioned_lines(s: &Settings, segs: &BTreeSet<SegPart>, w: f64, h: f64, side: BorderEdge) -> Vec<SegmentLine> {
    use SegPart::*;
    let i = s.thickness / 2.0;
    let fixed_menu = s.menu_line_enabled && s.dock_line_enabled && matches!(side, BorderEdge::Left | BorderEdge::Right);
    let mut lines: Vec<SegmentLine> = vec![];
    for &part in segs {
        if !crate::segments::seg_available(part, s.menu_line_enabled, s.dock_line_enabled) { continue; }
        let pts = if fixed_menu {
            let (m, d) = (s.menu_line_height.clamp(i, h - i), s.dock_line_height.clamp(i, w - i));
            let points = match part {
                HTop => vec![(i, i), (w - i, i)],
                HMenu => vec![(i, m), (d, m), (w - i, m)],
                HDock => vec![(d, m), (d, h - i)],
                HBottom => vec![(i, m), (i, h - i)],
                LMenuBar => vec![(i, i), (i, m)],
                RMenuBar => vec![(w - i, i), (w - i, m)],
                LMain | LDock => vec![(i, m), (d, m)],
                RMain => vec![(w - i, m), (w - i, h - i), (d, h - i)],
                RDock => vec![(d, h - i), (i, h - i)],
            };
            // 오른쪽 Dock은 아래 본문·Dock만 좌우 반사하고 메뉴바의 좌우 이름은 유지한다.
            if side == BorderEdge::Right && !matches!(part, HTop | LMenuBar | RMenuBar) {
                points.into_iter().map(|(x, y)| (w - x, y)).collect()
            } else { points }
        } else {
            let (cw, ch) = if matches!(side, BorderEdge::Left | BorderEdge::Right) { (h, w) } else { (w, h) };
            let (a, b) = crate::segments::seg_ends(part, s.menu_line_enabled, s.dock_line_enabled);
            [a, b].into_iter().map(|n| {
                let x = if n.right { cw - i } else { i };
                let y = match n.level { 0 => i, 1 => s.menu_line_height, 2 => ch - s.dock_line_height, _ => ch - i };
                match side {
                    BorderEdge::Bottom => (x, y), BorderEdge::Top => (x, h - y),
                    BorderEdge::Left => (w - y, x), BorderEdge::Right => (y, x),
                }
            }).collect()
        };
        for pair in pts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if a == b { continue; }
            if let Some(index) = lines.iter().position(|l| (l.a == a && l.b == b) || (l.a == b && l.b == a)) {
                lines.remove(index);
            } else { lines.push(SegmentLine { part, a, b }); }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{segments::zone_perimeter, types::ScreenZone};

    #[test]
    fn 메뉴바와_옆_독_구역의_둘레와_합친_경계() {
        let s = Settings { thickness: 2.0, menu_line_enabled: true, menu_line_height: 25.0,
            dock_line_enabled: true, dock_line_height: 70.0, ..Default::default() };
        for side in [BorderEdge::Left, BorderEdge::Right] {
            for zone in ScreenZone::ALL {
                let segs = zone_perimeter(&[zone].into(), true, true);
                let lines = positioned_lines(&s, &segs, 1200.0, 800.0, side);
                let xs: Vec<_> = lines.iter().flat_map(|e| [e.a.0, e.b.0]).collect();
                let ys: Vec<_> = lines.iter().flat_map(|e| [e.a.1, e.b.1]).collect();
                let bounds = [xs.iter().copied().fold(f64::INFINITY, f64::min), ys.iter().copied().fold(f64::INFINITY, f64::min),
                    xs.iter().copied().fold(0.0, f64::max), ys.iter().copied().fold(0.0, f64::max)];
                let expected = match zone {
                    ScreenZone::MenuBar => [1.0, 1.0, 1199.0, 25.0],
                    ScreenZone::Main if side == BorderEdge::Left => [70.0, 25.0, 1199.0, 799.0],
                    ScreenZone::Main => [1.0, 25.0, 1130.0, 799.0],
                    ScreenZone::Dock if side == BorderEdge::Left => [1.0, 25.0, 70.0, 799.0],
                    ScreenZone::Dock => [1130.0, 25.0, 1199.0, 799.0],
                };
                assert_eq!(bounds, expected, "{side:?} {zone:?}");
                for p in lines.iter().flat_map(|e| [e.a, e.b]) {
                    assert_eq!(lines.iter().filter(|e| e.a == p || e.b == p).count(), 2);
                }
            }
            let all = zone_perimeter(&ScreenZone::ALL.into(), true, true);
            let lines = positioned_lines(&s, &all, 1200.0, 800.0, side);
            assert!(lines.iter().all(|e| (e.a.0 == 1.0 && e.b.0 == 1.0) || (e.a.0 == 1199.0 && e.b.0 == 1199.0)
                || (e.a.1 == 1.0 && e.b.1 == 1.0) || (e.a.1 == 799.0 && e.b.1 == 799.0)));
        }
    }
}
