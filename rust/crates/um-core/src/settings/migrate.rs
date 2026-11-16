//! 구버전(변+영역) 설정 -> 세그먼트 변환.
//! TODO(merge): segments.rs로 이동 (여기 것은 설정 로드에 필요한 최소 복사본)

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{BorderEdge, ScreenZone, SegNode, SegPart};

/// 이 세그먼트가 현재 경계선 설정에서 존재하는지. (Swift `segAvailable` :686)
pub(crate) fn seg_available(s: SegPart, menu_on: bool, dock_on: bool) -> bool {
    match s {
        SegPart::HMenu | SegPart::LMenuBar | SegPart::RMenuBar => menu_on,
        SegPart::HDock | SegPart::LDock | SegPart::RDock => dock_on,
        _ => true,
    }
}

/// 세그먼트의 양 끝 노드. (Swift `segEnds` :695)
pub(crate) fn seg_ends(s: SegPart, menu_on: bool, dock_on: bool) -> (SegNode, SegNode) {
    let main_top = u8::from(menu_on);
    let main_bot = if dock_on { 2 } else { 3 };
    let n = |right: bool, level: u8| SegNode { right, level };
    match s {
        SegPart::HTop => (n(false, 0), n(true, 0)),
        SegPart::HMenu => (n(false, 1), n(true, 1)),
        SegPart::HDock => (n(false, 2), n(true, 2)),
        SegPart::HBottom => (n(false, 3), n(true, 3)),
        SegPart::LMenuBar => (n(false, 0), n(false, 1)),
        SegPart::LMain => (n(false, main_top), n(false, main_bot)),
        SegPart::LDock => (n(false, 2), n(false, 3)),
        SegPart::RMenuBar => (n(true, 0), n(true, 1)),
        SegPart::RMain => (n(true, main_top), n(true, main_bot)),
        SegPart::RDock => (n(true, 2), n(true, 3)),
    }
}

/// 선택이 '한 줄로 이어진 선(체인 또는 고리)'인지. (Swift `isValidSegments` :725)
pub(crate) fn is_valid_segments(set: &BTreeSet<SegPart>, menu_on: bool, dock_on: bool) -> bool {
    if set.is_empty() || !set.iter().all(|&s| seg_available(s, menu_on, dock_on)) {
        return false;
    }
    let mut adj: BTreeMap<SegNode, Vec<SegPart>> = BTreeMap::new();
    for &s in set {
        let (a, b) = seg_ends(s, menu_on, dock_on);
        adj.entry(a).or_default().push(s);
        adj.entry(b).or_default().push(s);
    }
    if !adj.values().all(|v| v.len() <= 2) {
        return false;
    }
    let ends = adj.values().filter(|v| v.len() == 1).count();
    if ends != 0 && ends != 2 {
        return false;
    }
    // 연결성: 아무 세그먼트에서 탐색해 전부 도달해야 한다.
    let mut visited: BTreeSet<SegPart> = BTreeSet::new();
    let mut queue = vec![*set.iter().next().expect("비어 있지 않음")];
    while let Some(s) = queue.pop() {
        if !visited.insert(s) {
            continue;
        }
        let (a, b) = seg_ends(s, menu_on, dock_on);
        for n in [a, b] {
            if let Some(v) = adj.get(&n) {
                queue.extend(v.iter().copied().filter(|x| !visited.contains(x)));
            }
        }
    }
    visited.len() == set.len()
}

/// 영역 조합이 유효한지: 비어있지 않고, 메뉴바+Dock만은 불가. (Swift `isValidZones` :670)
pub(crate) fn is_valid_zones(z: &BTreeSet<ScreenZone>) -> bool {
    if z.is_empty() {
        return false;
    }
    !(z.contains(&ScreenZone::MenuBar) && z.contains(&ScreenZone::Dock) && !z.contains(&ScreenZone::Main))
}

fn top_level(z: ScreenZone, menu_on: bool) -> u8 {
    match z {
        ScreenZone::MenuBar => 0,
        ScreenZone::Main => u8::from(menu_on),
        ScreenZone::Dock => 2,
    }
}

fn bottom_level(z: ScreenZone, dock_on: bool) -> u8 {
    match z {
        ScreenZone::MenuBar => 1,
        ScreenZone::Main => {
            if dock_on {
                2
            } else {
                3
            }
        }
        ScreenZone::Dock => 3,
    }
}

fn h_seg(level: u8) -> SegPart {
    match level {
        0 => SegPart::HTop,
        1 => SegPart::HMenu,
        2 => SegPart::HDock,
        _ => SegPart::HBottom,
    }
}

/// 영역 조합의 둘레를 이루는 세그먼트. (Swift `zonePerimeter` :755)
pub(crate) fn zone_perimeter(z: &BTreeSet<ScreenZone>, menu_on: bool, dock_on: bool) -> BTreeSet<SegPart> {
    // 꺼진 경계선의 영역은 본문으로 병합.
    let mut eff: BTreeSet<ScreenZone> = z
        .iter()
        .map(|&zone| match zone {
            ScreenZone::MenuBar if !menu_on => ScreenZone::Main,
            ScreenZone::Dock if !dock_on => ScreenZone::Main,
            other => other,
        })
        .collect();
    if eff.is_empty() {
        eff.insert(ScreenZone::Main);
    }
    let order = ScreenZone::ALL;
    let top = order.iter().copied().find(|x| eff.contains(x)).unwrap_or(ScreenZone::Main);
    let bottom = order.iter().rev().copied().find(|x| eff.contains(x)).unwrap_or(ScreenZone::Main);
    let mut out: BTreeSet<SegPart> =
        [h_seg(top_level(top, menu_on)), h_seg(bottom_level(bottom, dock_on))].into_iter().collect();
    for zone in order.iter().filter(|x| eff.contains(x)) {
        match zone {
            ScreenZone::MenuBar => out.extend([SegPart::LMenuBar, SegPart::RMenuBar]),
            ScreenZone::Main => out.extend([SegPart::LMain, SegPart::RMain]),
            ScreenZone::Dock => out.extend([SegPart::LDock, SegPart::RDock]),
        }
    }
    out
}

/// 구버전 (변 + 영역) 설정 -> 세그먼트 조합. (Swift `migrateToSegments` :787)
pub(crate) fn migrate_to_segments(
    edges: &BTreeSet<BorderEdge>,
    zones: &BTreeSet<ScreenZone>,
    menu_on: bool,
    dock_on: bool,
) -> BTreeSet<SegPart> {
    let peri = zone_perimeter(zones, menu_on, dock_on);
    // 가로선은 레벨 순(SegPart 선언 순서 = hTop, hMenu, hDock, hBottom).
    let hs: Vec<SegPart> = peri.iter().copied().filter(|s| s.is_horizontal()).collect();
    let mut out: BTreeSet<SegPart> = BTreeSet::new();
    if edges.contains(&BorderEdge::Top) {
        out.extend(hs.first().copied());
    }
    if edges.contains(&BorderEdge::Bottom) {
        out.extend(hs.last().copied());
    }
    if edges.contains(&BorderEdge::Left) {
        out.extend(peri.iter().copied().filter(|s| matches!(s, SegPart::LMenuBar | SegPart::LMain | SegPart::LDock)));
    }
    if edges.contains(&BorderEdge::Right) {
        out.extend(peri.iter().copied().filter(|s| matches!(s, SegPart::RMenuBar | SegPart::RMain | SegPart::RDock)));
    }
    if out.is_empty() {
        out.insert(SegPart::HTop);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_edges() -> BTreeSet<BorderEdge> {
        BorderEdge::ALL.into_iter().collect()
    }
    fn all_zones() -> BTreeSet<ScreenZone> {
        ScreenZone::ALL.into_iter().collect()
    }

    #[test]
    fn 경계선_꺼지면_전체_둘레() {
        let s = migrate_to_segments(&all_edges(), &all_zones(), false, false);
        let want: BTreeSet<_> = [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain].into();
        assert_eq!(s, want);
        assert!(is_valid_segments(&s, false, false));
    }

    #[test]
    fn 경계선_켜면_영역_세로선_포함() {
        let s = migrate_to_segments(&all_edges(), &all_zones(), true, true);
        assert!(s.contains(&SegPart::LMenuBar) && s.contains(&SegPart::RDock) && s.contains(&SegPart::HTop));
        assert!(!s.contains(&SegPart::HMenu));
    }

    #[test]
    fn 빈_결과는_상단_가로선() {
        let s = migrate_to_segments(&BTreeSet::new(), &all_zones(), false, false);
        assert_eq!(s, [SegPart::HTop].into());
    }

    #[test]
    fn 유효성() {
        let two: BTreeSet<_> = [SegPart::HTop, SegPart::HBottom].into();
        assert!(!is_valid_segments(&two, false, false));
        let menu: BTreeSet<_> = [SegPart::HMenu].into();
        assert!(!is_valid_segments(&menu, false, false));
        assert!(is_valid_segments(&menu, true, false));
    }
}
