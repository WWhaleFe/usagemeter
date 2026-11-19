//! 세그먼트 그래프. Swift `OverlaySettings`의 세그먼트 구간(:560-880)과 `enforceNoOverlap`(:364)을
//! Settings 구조체 없이 쓸 수 있게 옮긴 순수 함수 모음.
//!
//! 설정 값과 화면 방향은 `SegContext`로 받는다. 아래 Dock 동작은 Swift와 같고,
//! `tests/segments.rs`가 Swift 기준값(`tests/fixtures/segments_ref.jsonl`)과 전수 비교한다.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{AnchorSide, BorderCorner, BorderEdge, ProviderLayout, ScreenZone, SegCap, SegNode, SegPart};

/// `ProviderSpec.all` 순서. (Swift Providers.swift:38)
pub const PROVIDER_IDS: [&str; 3] = ["claude", "gemini", "codex"];

/// 세그먼트 조합. Swift `Set<SegPart>`.
pub type SegSet = BTreeSet<SegPart>;

/// 세그먼트 판정에 필요한 설정 값 묶음. Settings 타입에 의존하지 않는다.
#[derive(Debug, Clone, Copy)]
pub struct SegContext<'a> {
    /// 현재 화면의 Dock 방향. 저장 형식에는 포함하지 않는다.
    pub side: BorderEdge,
    /// `menuLineEnabled`
    pub menu_on: bool,
    /// `dockLineEnabled`
    pub dock_on: bool,
    /// `noOverlapLines`
    pub no_overlap: bool,
    /// AI id -> 레이아웃 (`providerLayouts`)
    pub layouts: &'a BTreeMap<String, ProviderLayout>,
    /// AI 순서 (`ProviderSpec.all`), 보통 `PROVIDER_IDS`
    pub provider_ids: &'a [&'a str],
    /// 본문 영역 곡률 (`cornerRadii`)
    pub main_radii: &'a BTreeMap<BorderCorner, f64>,
    /// 메뉴바 영역 곡률 (`menuZoneRadii`)
    pub menu_radii: &'a BTreeMap<BorderCorner, f64>,
    /// Dock 영역 곡률 (`dockZoneRadii`)
    pub dock_radii: &'a BTreeMap<BorderCorner, f64>,
}

/// 열린 체인의 한쪽 끝: 노드 + 그 끝에 붙은 세그먼트.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainEnd {
    pub node: SegNode,
    pub seg: SegPart,
}

/// 열린 체인의 두 끝. 정렬 순서 고정(위 -> 아래, 좌 -> 우).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainEnds {
    pub start: ChainEnd,
    pub end: ChainEnd,
}

const LEFT_SEGS: [SegPart; 3] = [SegPart::LMenuBar, SegPart::LMain, SegPart::LDock];
const RIGHT_SEGS: [SegPart; 3] = [SegPart::RMenuBar, SegPart::RMain, SegPart::RDock];

/// 메뉴바가 화면 위에 고정되는 옆 Dock 모드.
pub fn lateral_menu(menu: bool, dock: bool, side: BorderEdge) -> bool {
    menu && dock && matches!(side, BorderEdge::Left | BorderEdge::Right)
}

fn node(right: bool, level: u8) -> SegNode {
    SegNode { right, level }
}

/// 옆 메뉴 모델 노드의 상대 좌표. 내부 접점도 독립 노드로 구분한다.
pub fn side_node_point(n: SegNode, side: BorderEdge) -> (f64, f64) {
    let (x, y) = match (n.right, n.level) {
        (false, 0) => return (0.0, 0.0), (true, 0) => return (1.0, 0.0),
        (false, 1) => (0.0, 0.25), (true, 1) => (1.0, 0.25),
        (false, 2) => (0.25, 0.25), (true, 2) => (0.25, 1.0),
        (false, _) => (0.0, 1.0), (true, _) => (1.0, 1.0),
    };
    (if side == BorderEdge::Right { 1.0 - x } else { x }, y)
}

fn side_capable(seg: SegPart) -> bool {
    seg.is_horizontal() || seg == SegPart::LDock
}

fn side_cap_vector(ctx: &SegContext, seg: SegPart, cap: SegCap) -> (f64, f64) {
    let sign = if cap == SegCap::Up { -1.0 } else { 1.0 };
    if matches!(seg, SegPart::HDock | SegPart::HBottom) {
        (if ctx.side == BorderEdge::Left { -sign } else { sign }, 0.0)
    } else { (0.0, sign) }
}

fn side_corner_node(ctx: &SegContext, zone: ScreenZone, corner: BorderCorner) -> SegNode {
    let perimeter = zone_perimeter_on(&[zone].into(), ctx.menu_on, ctx.dock_on, ctx.side);
    let nodes: Vec<_> = seg_adjacency_on(&perimeter, ctx.menu_on, ctx.dock_on, ctx.side).keys().copied().collect();
    let right = matches!(corner, BorderCorner::TopRight | BorderCorner::BottomRight);
    let bottom = matches!(corner, BorderCorner::BottomLeft | BorderCorner::BottomRight);
    *nodes.iter().max_by(|&&a, &&b| {
        let score = |n| { let (x,y) = side_node_point(n, ctx.side); x * if right { 1.0 } else { -1.0 } + y * if bottom { 1.0 } else { -1.0 } };
        score(a).total_cmp(&score(b))
    }).unwrap()
}

// ---- 기본 레이아웃 (Swift :809-821) ----

/// 기본 분배: i번째 AI = [상단, 하단, 좌·본문, 우·본문][i % 4]. 앵커는 세그먼트별 고정. (Swift defaultLayouts)
pub fn default_layout(provider_index: usize) -> ProviderLayout {
    let (seg, anchor) = match provider_index % 4 {
        0 => (SegPart::HTop, BorderCorner::TopLeft),
        1 => (SegPart::HBottom, BorderCorner::BottomLeft),
        2 => (SegPart::LMain, BorderCorner::BottomLeft),
        _ => (SegPart::RMain, BorderCorner::TopRight),
    };
    ProviderLayout {
        segment_geometry: Some(1),
        edges: None,
        anchor_corner: anchor,
        anchor_side: AnchorSide::Center,
        clockwise: true,
        extend_start: None,
        extend_end: None,
        no_curve_extend: None,
        zones: None,
        segments: Some([seg].into_iter().collect()),
        cap_start: None,
        cap_end: None,
        h_caps: None,
    }
}

/// `PROVIDER_IDS` 전체의 기본 레이아웃. (Swift `defaultLayouts()`)
pub fn default_layouts() -> BTreeMap<String, ProviderLayout> {
    PROVIDER_IDS.iter().enumerate().map(|(i, id)| (id.to_string(), default_layout(i))).collect()
}

/// 저장된 레이아웃이 없으면 기본값, 기본값에도 없으면 상단 한 줄. (Swift `layout(for:)`)
pub fn layout_for(ctx: &SegContext, id: &str) -> ProviderLayout {
    if let Some(l) = ctx.layouts.get(id) {
        return l.clone();
    }
    if let Some(i) = PROVIDER_IDS.iter().position(|p| *p == id) {
        return default_layout(i);
    }
    ProviderLayout { segments: Some([SegPart::HTop].into_iter().collect()), ..default_layout(0) }
}

/// 이 AI의 세그먼트 조합. 없으면 구버전 변+영역 설정에서 변환. (Swift `segs(for:)`)
pub fn segs_for(ctx: &SegContext, id: &str) -> SegSet {
    let l = layout_for(ctx, id);
    if let Some(s) = l.segments.clone() {
        if lateral_menu(ctx.menu_on, ctx.dock_on, ctx.side) && l.segment_geometry.is_none() {
            return legacy_side_segments(&s);
        }
        return s;
    }
    let edges = l.edges.clone().unwrap_or_else(|| BorderEdge::ALL.into_iter().collect());
    let set = migrate_to_segments(&edges, &l.zone_set(), ctx.menu_on, ctx.dock_on);
    if lateral_menu(ctx.menu_on, ctx.dock_on, ctx.side) { legacy_side_segments(&set) } else { set }
}

/// 옛 접합부의 되짚는 선을 새 직선 선택으로 환산한다. 저장 원본은 건드리지 않는다.
pub fn legacy_side_segments(set: &SegSet) -> SegSet {
    use SegPart::*;
    let mut out = SegSet::new();
    for &part in set {
        let parts: &[SegPart] = match part {
            HMenu => &[HMenu, LDock], LMain | LDock => &[LDock],
            RMain => &[LMain, RMain], _ => std::slice::from_ref(&part),
        };
        for &p in parts { if !out.insert(p) { out.remove(&p); } }
    }
    out
}

/// 화면 방향에 맞춘 편집·렌더용 배치. 옛 전체 메뉴선의 Dock 쪽 캡도 함께 옮긴다.
pub fn positioned_layout(ctx: &SegContext, id: &str) -> ProviderLayout {
    let mut layout = layout_for(ctx, id);
    let legacy = lateral_menu(ctx.menu_on, ctx.dock_on, ctx.side) && layout.segment_geometry.is_none();
    layout.segments = Some(segs_for(ctx, id));
    if legacy {
        if let Some(caps) = &mut layout.h_caps {
            let end = ctx.side == BorderEdge::Right;
            if let Some(cap) = caps.remove(&h_cap_key(SegPart::HMenu, end)) {
                caps.insert(h_cap_key(SegPart::LDock, end), cap);
            }
        }
        layout.segment_geometry = Some(1);
    }
    layout
}

// ---- 가로선 캡 (Swift :572-578) ----

/// 가로선 캡 딕셔너리 키: "hMenu.L" / "hMenu.R". (Swift `hCapKey`)
pub fn h_cap_key(s: SegPart, right: bool) -> String {
    format!("{}.{}", s.raw(), if right { "R" } else { "L" })
}

/// 이 AI의 특정 가로선 좌/우 끝 캡. 없으면 None. (Swift `hCap`)
pub fn h_cap(ctx: &SegContext, id: &str, s: SegPart, right: bool) -> SegCap {
    positioned_layout(ctx, id)
        .h_caps
        .and_then(|m| m.get(&h_cap_key(s, right)).copied())
        .unwrap_or(SegCap::None)
}

// ---- 그래프 기본 (Swift :686-741) ----

/// 이 세그먼트가 현재 경계선 설정에서 존재하는지. menu 계열은 menuOn, dock 계열은 dockOn 필요.
pub fn seg_available(s: SegPart, menu_on: bool, dock_on: bool) -> bool {
    match s {
        SegPart::HMenu | SegPart::LMenuBar | SegPart::RMenuBar => menu_on,
        SegPart::HDock | SegPart::LDock | SegPart::RDock => dock_on,
        _ => true,
    }
}

/// 세그먼트의 양 끝 노드. 본문 세로선은 경계선 유무에 따라 늘어난다(menuOn ? 1 : 0 ~ dockOn ? 2 : 3).
pub fn seg_ends(s: SegPart, menu_on: bool, dock_on: bool) -> (SegNode, SegNode) {
    seg_ends_on(s, menu_on, dock_on, BorderEdge::Bottom)
}

/// 현재 Dock 방향의 그래프를 사용한다.
pub fn seg_ends_on(s: SegPart, menu_on: bool, dock_on: bool, side: BorderEdge) -> (SegNode, SegNode) {
    if lateral_menu(menu_on, dock_on, side) {
        use SegPart::*;
        return match s {
            HTop => (node(false, 0), node(true, 0)),
            LMenuBar => (node(false, 0), node(side == BorderEdge::Right, 1)),
            RMenuBar => (node(true, 0), node(side == BorderEdge::Left, 1)),
            HMenu => (node(false, 2), node(true, 1)),
            LDock => (node(false, 1), node(false, 2)),
            LMain => (node(true, 1), node(true, 3)),
            HDock => (node(false, 2), node(true, 2)),
            HBottom => (node(false, 1), node(false, 3)),
            RMain => (node(true, 3), node(true, 2)),
            RDock => (node(true, 2), node(false, 3)),
        };
    }
    let main_top = if menu_on { 1 } else { 0 };
    let main_bot = if dock_on { 2 } else { 3 };
    match s {
        SegPart::HTop => (node(false, 0), node(true, 0)),
        SegPart::HMenu => (node(false, 1), node(true, 1)),
        SegPart::HDock => (node(false, 2), node(true, 2)),
        SegPart::HBottom => (node(false, 3), node(true, 3)),
        SegPart::LMenuBar => (node(false, 0), node(false, 1)),
        SegPart::LMain => (node(false, main_top), node(false, main_bot)),
        SegPart::LDock => (node(false, 2), node(false, 3)),
        SegPart::RMenuBar => (node(true, 0), node(true, 1)),
        SegPart::RMain => (node(true, main_top), node(true, main_bot)),
        SegPart::RDock => (node(true, 2), node(true, 3)),
    }
}

/// 노드 -> 인접 세그먼트 목록.
pub fn seg_adjacency(set: &SegSet, menu_on: bool, dock_on: bool) -> BTreeMap<SegNode, Vec<SegPart>> {
    seg_adjacency_on(set, menu_on, dock_on, BorderEdge::Bottom)
}

/// 현재 Dock 방향의 그래프를 사용한다.
pub fn seg_adjacency_on(set: &SegSet, menu_on: bool, dock_on: bool, side: BorderEdge) -> BTreeMap<SegNode, Vec<SegPart>> {
    let mut adj: BTreeMap<SegNode, Vec<SegPart>> = BTreeMap::new();
    for &s in set {
        let (a, b) = seg_ends_on(s, menu_on, dock_on, side);
        adj.entry(a).or_default().push(s);
        adj.entry(b).or_default().push(s);
    }
    adj
}

/// 선택이 '한 줄로 이어진 선(체인 또는 고리)'인지.
/// 조건: 비어 있지 않음, 전부 존재하는 세그먼트, 노드 차수 <= 2, 전체 연결, 끝점 0개(고리) 또는 2개(체인).
pub fn is_valid_segments(set: &SegSet, menu_on: bool, dock_on: bool) -> bool {
    is_valid_segments_on(set, menu_on, dock_on, BorderEdge::Bottom)
}

/// 현재 Dock 방향의 그래프를 사용한다.
pub fn is_valid_segments_on(set: &SegSet, menu_on: bool, dock_on: bool, side: BorderEdge) -> bool {
    if set.is_empty() || !set.iter().all(|&s| seg_available(s, menu_on, dock_on)) {
        return false;
    }
    let adj = seg_adjacency_on(set, menu_on, dock_on, side);
    if !adj.values().all(|v| v.len() <= 2) {
        return false;
    }
    let ends = adj.values().filter(|v| v.len() == 1).count();
    if ends != 0 && ends != 2 {
        return false;
    }
    // 연결성: 아무 세그먼트에서 탐색해 전부 도달해야 한다.
    let mut visited: SegSet = BTreeSet::new();
    let mut stack = vec![*set.iter().next().unwrap()];
    while let Some(s) = stack.pop() {
        if !visited.insert(s) {
            continue;
        }
        let (a, b) = seg_ends_on(s, menu_on, dock_on, side);
        for n in [a, b] {
            if let Some(v) = adj.get(&n) {
                stack.extend(v.iter().copied().filter(|x| !visited.contains(x)));
            }
        }
    }
    visited.len() == set.len()
}

/// 열린 체인의 두 끝(정렬 순서 고정). 고리이거나 유효하지 않으면 None. (Swift :745)
pub fn chain_ends(set: &SegSet, menu_on: bool, dock_on: bool) -> Option<ChainEnds> {
    chain_ends_on(set, menu_on, dock_on, BorderEdge::Bottom)
}

/// 현재 Dock 방향의 그래프를 사용한다.
pub fn chain_ends_on(set: &SegSet, menu_on: bool, dock_on: bool, side: BorderEdge) -> Option<ChainEnds> {
    if !is_valid_segments_on(set, menu_on, dock_on, side) {
        return None;
    }
    let adj = seg_adjacency_on(set, menu_on, dock_on, side);
    // BTreeMap 순회 = SegNode 정렬 순서 (Swift `keys.sorted()`).
    let mut eps: Vec<(SegNode, SegPart)> =
        adj.iter().filter(|(_, v)| v.len() == 1).map(|(n, v)| (*n, v[0])).collect();
    if lateral_menu(menu_on, dock_on, side) {
        eps.sort_by(|a,b| {
            let (ax,ay) = side_node_point(a.0, side);
            let (bx,by) = side_node_point(b.0, side);
            ay.total_cmp(&by).then(ax.total_cmp(&bx))
        });
    }
    if eps.len() != 2 {
        return None;
    }
    Some(ChainEnds {
        start: ChainEnd { node: eps[0].0, seg: eps[0].1 },
        end: ChainEnd { node: eps[1].0, seg: eps[1].1 },
    })
}

// ---- 영역 둘레·구버전 변환 (Swift :755-800) ----

fn zone_top_level(z: ScreenZone, menu_on: bool) -> u8 {
    match z {
        ScreenZone::MenuBar => 0,
        ScreenZone::Main => u8::from(menu_on),
        ScreenZone::Dock => 2,
    }
}

fn zone_bottom_level(z: ScreenZone, dock_on: bool) -> u8 {
    match z {
        ScreenZone::MenuBar => 1,
        ScreenZone::Main => if dock_on { 2 } else { 3 },
        ScreenZone::Dock => 3,
    }
}

fn h_seg_of_level(level: u8) -> SegPart {
    match level {
        0 => SegPart::HTop,
        1 => SegPart::HMenu,
        2 => SegPart::HDock,
        _ => SegPart::HBottom,
    }
}

/// 영역 조합의 둘레를 이루는 세그먼트. 꺼진 경계선의 영역은 본문으로 병합, 빈 집합은 본문.
pub fn zone_perimeter(z: &BTreeSet<ScreenZone>, menu_on: bool, dock_on: bool) -> SegSet {
    zone_perimeter_on(z, menu_on, dock_on, BorderEdge::Bottom)
}

/// 현재 Dock 방향의 그래프를 사용한다.
pub fn zone_perimeter_on(z: &BTreeSet<ScreenZone>, menu_on: bool, dock_on: bool, side: BorderEdge) -> SegSet {
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
    if lateral_menu(menu_on, dock_on, side) {
        use SegPart::*;
        let mut out = SegSet::new();
        for zone in eff {
            let perimeter: &[SegPart] = match zone {
                ScreenZone::MenuBar => &[HTop, LMenuBar, RMenuBar, HMenu, LDock],
                ScreenZone::Main => &[HMenu, LMain, HDock, RMain],
                ScreenZone::Dock => &[LDock, HDock, HBottom, RDock],
            };
            for &part in perimeter {
                if !out.insert(part) { out.remove(&part); }
            }
        }
        return out;
    }
    let order = ScreenZone::ALL;
    let top = order.iter().copied().find(|x| eff.contains(x)).unwrap_or(ScreenZone::Main);
    let bottom = order.iter().rev().copied().find(|x| eff.contains(x)).unwrap_or(ScreenZone::Main);
    let mut out: SegSet = [
        h_seg_of_level(zone_top_level(top, menu_on)),
        h_seg_of_level(zone_bottom_level(bottom, dock_on)),
    ]
    .into_iter()
    .collect();
    for zone in order {
        if !eff.contains(&zone) {
            continue;
        }
        match zone {
            ScreenZone::MenuBar => out.extend([SegPart::LMenuBar, SegPart::RMenuBar]),
            ScreenZone::Main => out.extend([SegPart::LMain, SegPart::RMain]),
            ScreenZone::Dock => out.extend([SegPart::LDock, SegPart::RDock]),
        }
    }
    out
}

/// 구버전 (변 + 영역) 설정 -> 세그먼트 조합. 결과가 비면 {hTop}.
pub fn migrate_to_segments(
    edges: &BTreeSet<BorderEdge>,
    zones: &BTreeSet<ScreenZone>,
    menu_on: bool,
    dock_on: bool,
) -> SegSet {
    let peri = zone_perimeter(zones, menu_on, dock_on);
    // 둘레의 가로선을 위->아래 순으로 정렬: 맨 위가 top, 맨 아래가 bottom.
    let lvl = |s: &SegPart| match s {
        SegPart::HTop => 0,
        SegPart::HMenu => 1,
        SegPart::HDock => 2,
        _ => 3,
    };
    let mut hs: Vec<SegPart> = peri.iter().copied().filter(|s| s.is_horizontal()).collect();
    hs.sort_by_key(lvl);
    let mut out: SegSet = BTreeSet::new();
    if edges.contains(&BorderEdge::Top)
        && let Some(&t) = hs.first()
    {
        out.insert(t);
    }
    if edges.contains(&BorderEdge::Bottom)
        && let Some(&b) = hs.last()
    {
        out.insert(b);
    }
    if edges.contains(&BorderEdge::Left) {
        out.extend(peri.iter().copied().filter(|s| LEFT_SEGS.contains(s)));
    }
    if edges.contains(&BorderEdge::Right) {
        out.extend(peri.iter().copied().filter(|s| RIGHT_SEGS.contains(s)));
    }
    if out.is_empty() {
        out.insert(SegPart::HTop);
    }
    out
}

// ---- AI 간 보정·겹침 (Swift :583-668, :364) ----

/// 체인 끝(노드+세그먼트)의 교차 AI 보정. 반환 (half, auto_arc). (Swift `endOverride`)
/// - 내 가로선 끝에 up/down 캡이 있고 다른 AI의 세로선이 그 방향으로 노드에 붙어 있으면 half.
/// - 내 세로선 끝에서 다른 AI 가로선의 캡이 내 쪽을 향하면 (true, true).
pub fn end_override(ctx: &SegContext, id: &str, node: SegNode, seg: SegPart) -> (bool, bool) {
    let side = ctx.side;
    let (menu_on, dock_on) = (ctx.menu_on, ctx.dock_on);
    if lateral_menu(menu_on, dock_on, side) {
        let p = side_node_point(node, side);
        for &other in ctx.provider_ids.iter().filter(|&&o| o != id) {
            for part in segs_for(ctx, other) {
                let (a,b) = seg_ends_on(part, menu_on, dock_on, side);
                if a != node && b != node { continue; }
                let cap_seg = if side_capable(seg) { seg } else { part };
                if !side_capable(cap_seg) { continue; }
                let owner = if side_capable(seg) { id } else { other };
                let (ca,cb) = seg_ends_on(cap_seg, menu_on, dock_on, side);
                let pa = side_node_point(ca, side);
                let pb = side_node_point(cb, side);
                let end = if pa.1 == pb.1 { p.0 > pa.0.min(pb.0) } else { p.1 > pa.1.min(pb.1) };
                let cap = h_cap(ctx, owner, cap_seg, end);
                if !matches!(cap, SegCap::Up | SegCap::Down) { continue; }
                let v = side_cap_vector(ctx, cap_seg, cap);
                let line = if side_capable(seg) { (a,b) } else { seg_ends_on(seg, menu_on, dock_on, side) };
                let q = side_node_point(if line.0 == node { line.1 } else { line.0 }, side);
                if (q.0-p.0)*v.0 + (q.1-p.1)*v.1 > 0.0 { return (true, !side_capable(seg)); }
            }
        }
        return (false, false);
    }
    if seg.is_horizontal() {
        let cap = h_cap(ctx, id, seg, node.right);
        if cap != SegCap::Up && cap != SegCap::Down {
            return (false, false);
        }
        for &other in ctx.provider_ids.iter().filter(|&&o| o != id) {
            for s in segs_for(ctx, other).into_iter().filter(|s| !s.is_horizontal()) {
                let (a, b) = seg_ends_on(s, menu_on, dock_on, side);
                if a != node && b != node {
                    continue;
                }
                let other_level = if a == node { b.level } else { a.level };
                if (cap == SegCap::Down && other_level > node.level)
                    || (cap == SegCap::Up && other_level < node.level)
                {
                    return (true, false);
                }
            }
        }
        (false, false)
    } else {
        let (a, b) = seg_ends_on(seg, menu_on, dock_on, side);
        if a != node && b != node {
            return (false, false);
        }
        let other_end = if a == node { b } else { a };
        let my_down = other_end.level > node.level; // 내 세로선이 이 노드에서 아래로 뻗는가
        for &other in ctx.provider_ids.iter().filter(|&&o| o != id) {
            for s in segs_for(ctx, other).into_iter().filter(|s| s.is_horizontal()) {
                let (ha, hb) = seg_ends_on(s, menu_on, dock_on, side);
                if ha != node && hb != node {
                    continue;
                }
                let cap = h_cap(ctx, other, s, node.right);
                if (cap == SegCap::Down && my_down) || (cap == SegCap::Up && !my_down) {
                    return (true, true);
                }
            }
        }
        (false, false)
    }
}

/// 이 AI가 세그먼트 조합을 쓸 수 있는지: 한 줄로 이어져야 하고, '겹치는 선 없음'이 켜져 있으면
/// 다른 AI가 쓰는 세그먼트와 겹칠 수 없다. (Swift `canUseSegments`)
pub fn can_use_segments(ctx: &SegContext, id: &str, set: &SegSet) -> bool {
    if !is_valid_segments_on(set, ctx.menu_on, ctx.dock_on, ctx.side) {
        return false;
    }
    if ctx.no_overlap {
        for &other in ctx.provider_ids.iter().filter(|&&o| o != id) {
            if !segs_for(ctx, other).is_disjoint(set) {
                return false;
            }
        }
    }
    true
}

/// '겹치는 선 없음'을 켤 때 기존 겹침을 정리한다. 앞선 AI가 세그먼트를 갖고 뒤 AI에서 겹침을 뺀다.
/// 비거나 한 줄이 아니게 되면 비어 있는 첫 세그먼트(`SegPart::ALL` 순서) 하나를 준다.
/// 반환: `provider_ids`별 갱신된 레이아웃(호출 쪽이 `providerLayouts`에 덮어쓴다). (Swift :364-381)
pub fn enforce_no_overlap(ctx: &SegContext) -> BTreeMap<String, ProviderLayout> {
    let side = ctx.side;
    let (menu_on, dock_on) = (ctx.menu_on, ctx.dock_on);
    let mut used: SegSet = BTreeSet::new();
    let mut out = BTreeMap::new();
    for &id in ctx.provider_ids {
        let mut mine: SegSet = segs_for(ctx, id).difference(&used).copied().collect();
        if mine.is_empty() || !is_valid_segments_on(&mine, menu_on, dock_on, side) {
            let free = SegPart::ALL
                .into_iter()
                .find(|&s| seg_available(s, menu_on, dock_on) && !used.contains(&s));
            mine = free.into_iter().collect();
        }
        let mut l = positioned_layout(ctx, id);
        l.segments = Some(mine.clone());
        if lateral_menu(menu_on, dock_on, side) { l.segment_geometry = Some(1); }
        out.insert(id.to_string(), l);
        used.extend(mine);
    }
    out
}

/// 이 영역 꼭짓점 곡률이 현재 어느 선택에서든 실제로 쓰이는지. (Swift `cornerUsed`)
/// 쓰임 = 그 꼭짓점에서 가로+세로 세그먼트가 꺾이거나, 체인 끝 캡이 그 꼭짓점을 향함.
pub fn corner_used(ctx: &SegContext, zone: ScreenZone, corner: BorderCorner) -> bool {
    let (menu_on, dock_on) = (ctx.menu_on, ctx.dock_on);
    if lateral_menu(menu_on, dock_on, ctx.side) {
        let n = side_corner_node(ctx, zone, corner);
        let perimeter = zone_perimeter_on(&[zone].into(), menu_on, dock_on, ctx.side);
        let incident = seg_adjacency_on(&perimeter, menu_on, dock_on, ctx.side)[&n].clone();
        return ctx.provider_ids.iter().any(|&id| {
            let set = segs_for(ctx, id);
            incident.iter().all(|p| set.contains(p)) || incident.iter().any(|&part| {
                if !side_capable(part) || !set.contains(&part) { return false; }
                let (a,b) = seg_ends_on(part, menu_on, dock_on, ctx.side);
                let p = side_node_point(n, ctx.side);
                let pa = side_node_point(a, ctx.side); let pb = side_node_point(b, ctx.side);
                let end = if pa.1 == pb.1 { p.0 > pa.0.min(pb.0) } else { p.1 > pa.1.min(pb.1) };
                let cap = h_cap(ctx, id, part, end);
                matches!(cap, SegCap::Up | SegCap::Down) && {
                    let v = side_cap_vector(ctx, part, cap);
                    let other = incident.iter().copied().find(|&s| s != part).unwrap();
                    let (a,b) = seg_ends_on(other, menu_on, dock_on, ctx.side);
                    let q = side_node_point(if a == n { b } else { a }, ctx.side);
                    (q.0-p.0)*v.0 + (q.1-p.1)*v.1 > 0.0
                }
            })
        });
    }
    // 병합돼 없는 영역(경계선 꺼짐)의 곡률은 안 쓰인다.
    if zone == ScreenZone::MenuBar && !menu_on {
        return false;
    }
    if zone == ScreenZone::Dock && !dock_on {
        return false;
    }
    let is_top = matches!(corner, BorderCorner::TopLeft | BorderCorner::TopRight);
    let level = if is_top { zone_top_level(zone, menu_on) } else { zone_bottom_level(zone, dock_on) };
    let right = matches!(corner, BorderCorner::TopRight | BorderCorner::BottomRight);
    let v_seg = match (zone, right) {
        (ScreenZone::MenuBar, true) => SegPart::RMenuBar,
        (ScreenZone::MenuBar, false) => SegPart::LMenuBar,
        (ScreenZone::Main, true) => SegPart::RMain,
        (ScreenZone::Main, false) => SegPart::LMain,
        (ScreenZone::Dock, true) => SegPart::RDock,
        (ScreenZone::Dock, false) => SegPart::LDock,
    };
    let h_seg = h_seg_of_level(level);
    for &id in ctx.provider_ids {
        let set = segs_for(ctx, id);
        // 꺾임: 이 노드에서 가로+세로가 모두 선택됨.
        if set.contains(&v_seg) && set.contains(&h_seg) {
            return true;
        }
        // 가로선 캡: 이 가로선의 이 쪽 끝 캡이 이 영역 꼭짓점을 향함.
        if set.contains(&h_seg) {
            let cap = h_cap(ctx, id, h_seg, right);
            if cap == SegCap::Up && !is_top && zone_bottom_level(zone, dock_on) == level {
                return true;
            }
            if cap == SegCap::Down && is_top && zone_top_level(zone, menu_on) == level {
                return true;
            }
        }
    }
    false
}

/// 영역별 곡률 사전. (Swift `zoneRadiiDict`)
fn zone_radii<'a>(ctx: &SegContext<'a>, z: ScreenZone) -> &'a BTreeMap<BorderCorner, f64> {
    match z {
        ScreenZone::MenuBar => ctx.menu_radii,
        ScreenZone::Main => ctx.main_radii,
        ScreenZone::Dock => ctx.dock_radii,
    }
}

/// 이 AI가 노드에서 (가로 h + 세로 v) 꺾일 때 쓰는 곡률 - 겹침 분할 클립 경계 계산용.
/// 캡이 있으면 캡이 향한 영역 꼭짓점, 없으면 세로선이 속한 영역 꼭짓점. 음수는 0. (Swift :831)
pub fn bend_radius_estimate(ctx: &SegContext, id: &str, h: SegPart, v: SegPart, node: SegNode) -> f64 {
    let (menu_on, dock_on) = (ctx.menu_on, ctx.dock_on);
    if lateral_menu(menu_on, dock_on, ctx.side) {
        for zone in ScreenZone::ALL {
            let perimeter = zone_perimeter_on(&[zone].into(), menu_on, dock_on, ctx.side);
            if !perimeter.contains(&h) || !perimeter.contains(&v) { continue; }
            for corner in BorderCorner::ALL {
                if side_corner_node(ctx, zone, corner) == node {
                    return zone_radii(ctx, zone).get(&corner).copied().unwrap_or(0.0).max(0.0);
                }
            }
        }
        return 0.0;
    }
    let zone_of = |s: SegPart| match s {
        SegPart::LMenuBar | SegPart::RMenuBar => ScreenZone::MenuBar,
        SegPart::LDock | SegPart::RDock => ScreenZone::Dock,
        _ => ScreenZone::Main,
    };
    let zone_above = |level: u8| match level {
        1 => menu_on.then_some(ScreenZone::MenuBar),
        2 => Some(ScreenZone::Main),
        3 => Some(if dock_on { ScreenZone::Dock } else { ScreenZone::Main }),
        _ => None,
    };
    let zone_below = |level: u8| match level {
        0 => Some(if menu_on { ScreenZone::MenuBar } else { ScreenZone::Main }),
        1 => Some(ScreenZone::Main),
        2 => dock_on.then_some(ScreenZone::Dock),
        _ => None,
    };
    let radius = |z: ScreenZone, c: BorderCorner| zone_radii(ctx, z).get(&c).copied().unwrap_or(0.0).max(0.0);
    let cap = h_cap(ctx, id, h, node.right);
    if cap == SegCap::Up || cap == SegCap::Down {
        let z = if cap == SegCap::Up { zone_above(node.level) } else { zone_below(node.level) };
        if let Some(z) = z {
            let corner = match (cap == SegCap::Up, node.right) {
                (true, true) => BorderCorner::BottomRight,
                (true, false) => BorderCorner::BottomLeft,
                (false, true) => BorderCorner::TopRight,
                (false, false) => BorderCorner::TopLeft,
            };
            return radius(z, corner);
        }
    }
    let z = zone_of(v);
    let is_bottom = zone_bottom_level(z, dock_on) == node.level;
    let corner = match (is_bottom, node.right) {
        (true, true) => BorderCorner::BottomRight,
        (true, false) => BorderCorner::BottomLeft,
        (false, true) => BorderCorner::TopRight,
        (false, false) => BorderCorner::TopLeft,
    };
    radius(z, corner)
}

/// 커서 호버 판정용: 화면의 이 변에 그려진 세그먼트가 하나라도 있는지. (Swift `edgeActive`)
pub fn edge_active(ctx: &SegContext, e: BorderEdge) -> bool {
    ctx.provider_ids.iter().any(|&id| {
        let s = segs_for(ctx, id);
        if lateral_menu(ctx.menu_on, ctx.dock_on, ctx.side) {
            return s.iter().any(|&part| {
                let (a,b) = seg_ends_on(part, ctx.menu_on, ctx.dock_on, ctx.side);
                let (a,b) = (side_node_point(a,ctx.side),side_node_point(b,ctx.side));
                match e {
                    BorderEdge::Top => a.1 == 0.0 && b.1 == 0.0,
                    BorderEdge::Bottom => a.1 == 1.0 && b.1 == 1.0,
                    BorderEdge::Left => a.0 == 0.0 && b.0 == 0.0,
                    BorderEdge::Right => a.0 == 1.0 && b.0 == 1.0,
                }
            });
        }
        match e {
            BorderEdge::Top => s.contains(&SegPart::HTop),
            BorderEdge::Bottom => s.contains(&SegPart::HBottom),
            BorderEdge::Left => LEFT_SEGS.iter().any(|x| s.contains(x)),
            BorderEdge::Right => RIGHT_SEGS.iter().any(|x| s.contains(x)),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layouts_follow_swift_distribution() {
        let m = default_layouts();
        let one = |id: &str| m[id].segments.clone().unwrap().into_iter().collect::<Vec<_>>();
        assert_eq!(one("claude"), vec![SegPart::HTop]);
        assert_eq!(one("gemini"), vec![SegPart::HBottom]);
        assert_eq!(one("codex"), vec![SegPart::LMain]);
        assert_eq!(m["claude"].anchor_corner, BorderCorner::TopLeft);
        assert_eq!(m["gemini"].anchor_corner, BorderCorner::BottomLeft);
        assert_eq!(m["codex"].anchor_corner, BorderCorner::BottomLeft);
        assert_eq!(default_layout(3).anchor_corner, BorderCorner::TopRight);
        assert_eq!(default_layout(4).segments, default_layout(0).segments);
        assert!(m.values().all(|l| l.clockwise && l.anchor_side == AnchorSide::Center));
    }

    #[test]
    fn h_cap_key_format() {
        assert_eq!(h_cap_key(SegPart::HMenu, false), "hMenu.L");
        assert_eq!(h_cap_key(SegPart::HBottom, true), "hBottom.R");
    }
}
