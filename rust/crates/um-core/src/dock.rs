//! 실제 화면의 Dock 방향을 반영한 세그먼트. 메뉴바와 옆 Dock은 T자 접합부를 공유한다.
use crate::settings::Settings;
use crate::types::{BorderEdge, SegNode, SegPart};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentLine {
    pub part: SegPart,
    pub a: (f64, f64),
    pub b: (f64, f64),
}

/// 각 세그먼트를 겹치지 않는 직선 하나로 펼친다.
pub fn positioned_lines(s: &Settings, segs: &BTreeSet<SegPart>, w: f64, h: f64, side: BorderEdge) -> Vec<SegmentLine> {
    let i = s.thickness / 2.0;
    let fixed_menu = s.menu_line_enabled && s.dock_line_enabled && matches!(side, BorderEdge::Left | BorderEdge::Right);
    let mut lines: Vec<SegmentLine> = vec![];
    for &part in segs {
        if !crate::segments::seg_available(part, s.menu_line_enabled, s.dock_line_enabled) { continue; }
        let [a,b] = if fixed_menu {
            let (a,b) = crate::segments::seg_ends_on(part, true, true, side);
            [a,b].map(|n| side_node_position(s,w,h,side,n))
        } else {
            let (cw, ch) = if matches!(side, BorderEdge::Left | BorderEdge::Right) { (h, w) } else { (w, h) };
            let (a, b) = crate::segments::seg_ends(part, s.menu_line_enabled, s.dock_line_enabled);
            [a, b].map(|n| {
                let x = if n.right { cw - i } else { i };
                let y = match n.level { 0 => i, 1 => s.menu_line_height, 2 => ch - s.dock_line_height, _ => ch - i };
                match side {
                    BorderEdge::Bottom => (x, y), BorderEdge::Top => (x, h - y),
                    BorderEdge::Left => (w - y, x), BorderEdge::Right => (y, x),
                }
            })
        };
        if a != b { lines.push(SegmentLine { part, a, b }); }
    }
    lines
}

/// 고정 메뉴바와 옆 Dock의 실제 접점. 그래프·레인·캡이 같은 노드를 사용한다.
pub fn side_node_position(s: &Settings, w: f64, h: f64, side: BorderEdge, n: SegNode) -> (f64,f64) {
    let i = s.thickness/2.0;
    let m = s.menu_line_height.clamp(i,h-i);
    let d = s.dock_line_height.clamp(i,w-i);
    if n.level == 0 { return (if n.right { w-i } else { i },i); }
    let (x,y) = match (n.right,n.level) {
        (false,1) => (i,m), (true,1) => (w-i,m),
        (false,2) => (d,m), (true,2) => (d,h-i),
        (false,_) => (i,h-i), (true,_) => (w-i,h-i),
    };
    (if side == BorderEdge::Right { w-x } else { x },y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{segments::zone_perimeter_on, types::ScreenZone};

    #[test]
    fn 메뉴바와_옆_독_구역의_둘레와_합친_경계() {
        let s = Settings { thickness: 2.0, menu_line_enabled: true, menu_line_height: 25.0,
            dock_line_enabled: true, dock_line_height: 70.0, ..Default::default() };
        for side in [BorderEdge::Left, BorderEdge::Right] {
            for zone in ScreenZone::ALL {
                let segs = zone_perimeter_on(&[zone].into(), true, true, side);
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
            let all = zone_perimeter_on(&ScreenZone::ALL.into(), true, true, side);
            let lines = positioned_lines(&s, &all, 1200.0, 800.0, side);
            assert!(lines.iter().all(|e| (e.a.0 == 1.0 && e.b.0 == 1.0) || (e.a.0 == 1199.0 && e.b.0 == 1199.0)
                || (e.a.1 == 1.0 && e.b.1 == 1.0) || (e.a.1 == 799.0 && e.b.1 == 799.0)));
        }
    }
    #[test]
    fn 그림과_렌더의_공통_직선_좌표와_구역_폐쇄() {
        use crate::segments::{chain_ends_on, is_valid_segments_on, seg_adjacency_on};
        let cases: serde_json::Value = serde_json::from_str(include_str!("../tests/fixtures/dock_lines.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let side: BorderEdge = serde_json::from_value(case["side"].clone()).unwrap();
            let menu = case["menu"].as_bool().unwrap();
            let lateral = menu && matches!(side, BorderEdge::Left | BorderEdge::Right);
            let s = Settings { thickness: 0.0, menu_line_enabled: menu,
                menu_line_height: if lateral { 36.0/109.0 } else { 1.0/3.0 }, dock_line_enabled: true,
                dock_line_height: if lateral { 60.0/181.0 } else if menu { 1.0/3.0 } else { 0.5 }, ..Default::default() };
            let parts = SegPart::ALL.into_iter().filter(|&p| crate::segments::seg_available(p, menu, true)).collect();
            let lines = positioned_lines(&s, &parts, 1.0, 1.0, side);
            assert_eq!(lines.len(), parts.len());
            for line in &lines {
                assert!(line.a.0 == line.b.0 || line.a.1 == line.b.1);
                let expected = &case["lines"][line.part.raw()];
                for (got,want) in [line.a.0,line.a.1,line.b.0,line.b.1].into_iter().zip(expected.as_array().unwrap().iter().flat_map(|p| p.as_array().unwrap()).map(|v| v.as_f64().unwrap())) {
                    assert!((got-want).abs() < 1e-10, "{side:?} {menu} {:?}", line.part);
                }
            }
            for (j,a) in lines.iter().enumerate() {
                for b in &lines[j+1..] {
                    let overlap = |a:f64,b:f64,c:f64,d:f64| a.max(b).min(c.max(d))-a.min(b).max(c.min(d)) > 1e-10;
                    assert!(!(a.a.0 == a.b.0 && b.a.0 == b.b.0 && a.a.0 == b.a.0 && overlap(a.a.1,a.b.1,b.a.1,b.b.1)));
                    assert!(!(a.a.1 == a.b.1 && b.a.1 == b.b.1 && a.a.1 == b.a.1 && overlap(a.a.0,a.b.0,b.a.0,b.b.0)));
                }
            }
            // 모든 선택의 추상 노드 연결 관계를 실제 좌표의 연결 관계와 대조한다.
            for mask in 1..1024 {
                let set = SegPart::ALL.into_iter().enumerate().filter(|(j,p)| mask & (1 << j) != 0 && parts.contains(p)).map(|(_,p)| p).collect();
                let physical = positioned_lines(&s, &set, 1.0, 1.0, side);
                let mut degrees: Vec<_> = physical.iter().flat_map(|l| [l.a,l.b]).map(|p| physical.iter().filter(|l| l.a == p || l.b == p).count()).collect();
                let mut abstract_degrees: Vec<_> = seg_adjacency_on(&set, menu, true, side).values().flat_map(|v| std::iter::repeat_n(v.len(),v.len())).collect();
                degrees.sort(); abstract_degrees.sort();
                assert_eq!(degrees,abstract_degrees,"{side:?} {menu} {set:?}");
            }
            for zone in ScreenZone::ALL {
                let set = zone_perimeter_on(&[zone].into(),menu,true,side);
                assert!(is_valid_segments_on(&set,menu,true,side));
                assert!(chain_ends_on(&set,menu,true,side).is_none());
                let edges = positioned_lines(&s,&set,1.0,1.0,side);
                let min_x = edges.iter().flat_map(|l| [l.a.0,l.b.0]).fold(f64::INFINITY,f64::min);
                let max_x = edges.iter().flat_map(|l| [l.a.0,l.b.0]).fold(f64::NEG_INFINITY,f64::max);
                let min_y = edges.iter().flat_map(|l| [l.a.1,l.b.1]).fold(f64::INFINITY,f64::min);
                let max_y = edges.iter().flat_map(|l| [l.a.1,l.b.1]).fold(f64::NEG_INFINITY,f64::max);
                let length: f64 = edges.iter().map(|l| (l.a.0-l.b.0).hypot(l.a.1-l.b.1)).sum();
                assert!((length-2.0*(max_x-min_x+max_y-min_y)).abs()<1e-10);
                for p in edges.iter().flat_map(|l| [l.a,l.b]) { assert_eq!(edges.iter().filter(|l| l.a==p || l.b==p).count(),2); }
            }
        }
    }

    #[test]
    fn 예전_옆_독_선택은_실제_외곽을_유지하고_새_선택은_재변환하지_않음() {
        use crate::{hover::seg_ctx, segments::{legacy_side_segments, segs_for, zone_perimeter}};
        for side in [BorderEdge::Left, BorderEdge::Right] {
            for zone in [ScreenZone::ALL.into(), [ScreenZone::MenuBar].into(), [ScreenZone::Main].into(), [ScreenZone::Dock].into()] {
                assert_eq!(legacy_side_segments(&zone_perimeter(&zone,true,true)),zone_perimeter_on(&zone,true,true,side));
            }
            let mut s = Settings { menu_line_enabled:true,dock_line_enabled:true,..Default::default() };
            let old = [SegPart::HMenu,SegPart::LMain,SegPart::RMain].into();
            s.provider_layouts.get_mut("claude").unwrap().segments=Some(old);
            s.provider_layouts.get_mut("claude").unwrap().segment_geometry=None;
            let ctx = crate::segments::SegContext { side,..seg_ctx(&s) };
            let converted = segs_for(&ctx,"claude");
            assert_eq!(converted,[SegPart::HMenu,SegPart::LMain,SegPart::RMain].into());
            let l=s.provider_layouts.get_mut("claude").unwrap();
            let end=side==BorderEdge::Right;
            l.h_caps=Some([(crate::segments::h_cap_key(SegPart::HMenu,end),crate::types::SegCap::Up)].into());
            let migrated=crate::segments::positioned_layout(&crate::segments::SegContext { side,..seg_ctx(&s) },"claude");
            assert_eq!(migrated.h_caps.unwrap()[&crate::segments::h_cap_key(SegPart::LDock,end)],crate::types::SegCap::Up);
            let l=s.provider_layouts.get_mut("claude").unwrap();
            l.segments=Some([SegPart::LMain].into()); l.segment_geometry=Some(1);
            let saved=serde_json::to_value(s.to_snapshot()).unwrap();
            let mut loaded=Settings::default(); loaded.apply_snapshot(&serde_json::from_value(saved).unwrap());
            assert_eq!(segs_for(&crate::segments::SegContext { side,..seg_ctx(&loaded) },"claude"),[SegPart::LMain].into());
        }
    }

}
