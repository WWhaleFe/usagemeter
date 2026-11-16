//! 세그먼트 체인 경로 생성. Swift `SegmentChainShape.build`(BorderView.swift:82-560)를 한 줄씩 옮긴다.
//! 좌표: pt, 왼쪽 위 원점. 화면 전체 좌표로 만들고 띠 창이 자기 원점만큼 옮겨 그린다.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kurbo::{BezPath, Point};
use um_core::segments::{h_cap_key, is_valid_segments, seg_adjacency, seg_available, seg_ends};
use um_core::types::{AnchorSide, BorderCorner, NotchCorner, ScreenZone, SegCap, SegNode, SegPart};

/// 경로 위 한 구간: 길이(pt)·굵기·노치 우회 여부. (Swift `record` 콜백 인자)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rec {
    pub len: f64,
    pub width: f64,
    pub notch: bool,
}

/// `build` 결과: 경로 + 방출 순서대로의 구간 기록.
#[derive(Debug, Clone, Default)]
pub struct Built {
    pub path: BezPath,
    pub rec: Vec<Rec>,
}

/// Swift `SegmentChainShape`의 저장 필드 전부. (BorderView.swift:14-50)
#[derive(Debug, Clone)]
pub struct ChainShape {
    pub segments: BTreeSet<SegPart>,
    pub menu_on: bool,
    pub dock_on: bool,
    pub menu_h: f64,
    pub dock_h: f64,
    pub inset: f64,
    pub menu_radii: BTreeMap<BorderCorner, f64>,
    pub main_radii: BTreeMap<BorderCorner, f64>,
    pub dock_radii: BTreeMap<BorderCorner, f64>,
    pub notch_enabled: bool,
    pub notch_width: f64,
    pub notch_height: f64,
    pub notch_radii: HashMap<NotchCorner, f64>,
    pub anchor_corner: BorderCorner,
    pub anchor_side: AnchorSide,
    pub clockwise: bool,
    pub h_caps: BTreeMap<String, SegCap>,
    /// 정렬(위 -> 아래, 좌 -> 우) 기준 첫 끝: 캡/아크를 반쪽만 그리기.
    pub end_a_half: bool,
    /// 첫 끝(세로선)에 자동 아크 붙이기.
    pub end_a_auto: bool,
    pub end_b_half: bool,
    pub end_b_auto: bool,
    /// 겹침 분할 레인 오프셋(가로선 = dy, 세로선 = dx).
    pub lane_offsets: BTreeMap<SegPart, f64>,
    /// 겹침 분할 구간 굵기(공유 세그먼트 -> t/N). 없으면 `full_width`.
    pub lane_widths: BTreeMap<SegPart, f64>,
    /// 원래 선 굵기(분할 안 된 구간·모서리·노치·캡).
    pub full_width: f64,
    /// 이 AI가 노치를 우회할지(상단선을 여럿이 나눌 때는 한 명만).
    pub notch_detour: bool,
    /// 노드(모서리)별 분할 굵기(한 노드를 2개 이상 AI가 지날 때 t/k).
    pub node_widths: BTreeMap<SegNode, f64>,
}

/// 정점: 위치, 곡률 반지름, 스쿱 오목 곡선의 명시적 제어점. (Swift `PV`, `pointed`는 항상 false라 뺀다)
#[derive(Debug, Clone, Copy)]
struct Pv {
    pt: Point,
    r: f64,
    over: Option<Point>,
}

fn pv(pt: Point, r: f64, over: Option<Point>) -> Pv {
    Pv { pt, r, over }
}

/// (정점, 그 정점을 떠나는 구간의 굵기, 이 정점 아크의 굵기 상한)
type Item = (Pv, f64, Option<f64>);

fn dist(a: Point, b: Point) -> f64 {
    ((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)).sqrt()
}

fn unit(a: Point, b: Point) -> Point {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = f64::max(0.0001, (dx * dx + dy * dy).sqrt());
    Point::new(dx / len, dy / len)
}

/// 2차 베지어 길이: 12구간 폴리라인 근사(Swift `quadLen`). 구간 굵기 기록에만 쓴다.
fn quad_len(a: Point, c: Point, b: Point) -> f64 {
    let mut total = 0.0;
    let mut prev = a;
    for i in 1..=12 {
        let t = f64::from(i) / 12.0;
        let mt = 1.0 - t;
        let q = Point::new(
            mt * mt * a.x + 2.0 * mt * t * c.x + t * t * b.x,
            mt * mt * a.y + 2.0 * mt * t * c.y + t * t * b.y,
        );
        total += dist(prev, q);
        prev = q;
    }
    total
}

/// 경로 방출기: 이동/직선/곡선을 그리며 (길이, 굵기, 노치)를 기록한다. (Swift `mv`/`ln`/`qd`)
struct Emitter {
    path: BezPath,
    cp: Point,
    rec: Vec<Rec>,
}

impl Emitter {
    fn mv(&mut self, q: Point) {
        self.path.move_to(q);
        self.cp = q;
    }
    fn ln(&mut self, q: Point, w: f64, notch: bool) {
        self.path.line_to(q);
        self.rec.push(Rec { len: dist(self.cp, q), width: w, notch });
        self.cp = q;
    }
    fn qd(&mut self, q: Point, c: Point, w: f64, notch: bool) {
        self.path.quad_to(c, q);
        self.rec.push(Rec { len: quad_len(self.cp, c, q), width: w, notch });
        self.cp = q;
    }
}

/// 정점별 접점(t1 = 들어오는 쪽, t2 = 나가는 쪽). 곡률은 이웃까지 거리의 절반(이웃도 둥글면)/전체로 제한.
/// (Swift `tangents`: 열린 체인에서도 이웃은 순환 인덱스)
fn tangents(vs: &[Pv]) -> (Vec<Point>, Vec<Point>) {
    let n = vs.len();
    let mut t1 = vec![Point::ZERO; n];
    let mut t2 = vec![Point::ZERO; n];
    for i in 0..n {
        let cur = vs[i].pt;
        let prev = vs[(i + n - 1) % n];
        let next = vs[(i + 1) % n];
        let max_prev = (if prev.r > 0.01 { 0.5 } else { 1.0 }) * dist(cur, prev.pt);
        let max_next = (if next.r > 0.01 { 0.5 } else { 1.0 }) * dist(cur, next.pt);
        let r = f64::max(0.0, vs[i].r.min(max_prev).min(max_next));
        let (up, un) = (unit(cur, prev.pt), unit(cur, next.pt));
        t1[i] = Point::new(cur.x + up.x * r, cur.y + up.y * r);
        t2[i] = Point::new(cur.x + un.x * r, cur.y + un.y * r);
    }
    (t1, t2)
}

fn zone_of(v: SegPart) -> ScreenZone {
    match v {
        SegPart::LMenuBar | SegPart::RMenuBar => ScreenZone::MenuBar,
        SegPart::LDock | SegPart::RDock => ScreenZone::Dock,
        _ => ScreenZone::Main,
    }
}

fn corner_of(is_bottom: bool, right: bool) -> BorderCorner {
    match (is_bottom, right) {
        (true, true) => BorderCorner::BottomRight,
        (true, false) => BorderCorner::BottomLeft,
        (false, true) => BorderCorner::TopRight,
        (false, false) => BorderCorner::TopLeft,
    }
}

/// `build` 중 쓰는 계산 컨텍스트(Swift의 중첩 함수들이 캡처하던 값).
struct Ctx<'a> {
    s: &'a ChainShape,
    segs: BTreeSet<SegPart>,
    w: f64,
    h: f64,
    xl: f64,
    xr: f64,
}

impl Ctx<'_> {
    fn y_of(&self, level: u8) -> f64 {
        match level {
            0 => self.s.inset,
            1 => self.s.menu_h,
            2 => self.h - self.s.dock_h,
            _ => self.h - self.s.inset,
        }
    }

    /// 노드 좌표 + 이 노드에 붙은 내 세그먼트의 레인 오프셋 합성(BorderView.swift:97-119).
    /// 오프셋 부호는 화면 안쪽(중앙) 방향으로 통일한다: 위쪽 선(hTop, hMenu)은 +y, 아래쪽 선은 -y,
    /// 좌변은 +x, 우변은 -x가 안쪽.
    fn pt(&self, n: SegNode) -> Point {
        let (mut dx, mut dy) = (0.0, 0.0);
        if !self.s.lane_offsets.is_empty() {
            // Swift는 Set 순회 순서가 매 실행 달라 마지막 대입이 불정. 여기서는 SegPart 순서로 고정.
            for s in &self.segs {
                let Some(&o) = self.s.lane_offsets.get(s) else { continue };
                if o == 0.0 {
                    continue;
                }
                let (a, b) = seg_ends(*s, self.s.menu_on, self.s.dock_on);
                if a != n && b != n {
                    continue;
                }
                if s.is_horizontal() {
                    dy = if matches!(s, SegPart::HTop | SegPart::HMenu) { o } else { -o };
                } else {
                    dx = if n.right { -o } else { o };
                }
            }
        }
        Point::new((if n.right { self.xr } else { self.xl }) + dx, self.y_of(n.level) + dy)
    }

    /// 세그먼트별 구간 굵기(분할 시 t/N, 아니면 원굵기).
    fn seg_w(&self, s: SegPart) -> f64 {
        self.s.lane_widths.get(&s).copied().unwrap_or(self.s.full_width)
    }

    /// 영역 곡률. 키가 없거나 음수면 0.
    fn zone_radius(&self, z: ScreenZone, c: BorderCorner) -> f64 {
        let d = match z {
            ScreenZone::MenuBar => &self.s.menu_radii,
            ScreenZone::Main => &self.s.main_radii,
            ScreenZone::Dock => &self.s.dock_radii,
        };
        f64::max(0.0, d.get(&c).copied().unwrap_or(0.0))
    }

    fn bottom_level(&self, z: ScreenZone) -> u8 {
        match z {
            ScreenZone::MenuBar => 1,
            ScreenZone::Main => {
                if self.s.dock_on {
                    2
                } else {
                    3
                }
            }
            ScreenZone::Dock => 3,
        }
    }

    fn zone_above(&self, level: u8) -> Option<ScreenZone> {
        match level {
            1 => self.s.menu_on.then_some(ScreenZone::MenuBar),
            2 => Some(ScreenZone::Main),
            3 => Some(if self.s.dock_on { ScreenZone::Dock } else { ScreenZone::Main }),
            _ => None,
        }
    }

    fn zone_below(&self, level: u8) -> Option<ScreenZone> {
        match level {
            0 => Some(if self.s.menu_on { ScreenZone::MenuBar } else { ScreenZone::Main }),
            1 => Some(ScreenZone::Main),
            2 => self.s.dock_on.then_some(ScreenZone::Dock),
            _ => None,
        }
    }

    fn h_cap(&self, h: SegPart, right: bool) -> SegCap {
        self.s.h_caps.get(&h_cap_key(h, right)).copied().unwrap_or(SegCap::None)
    }

    /// 접합부(가로 h + 세로 v) 노드의 정점들. 대개 곡률 하나짜리 정점 1개.
    /// 캡 방향이 세로선과 반대면 스쿱(오목 필렛, 정점 3개)으로 그 방향을 살린다. (BorderView.swift:160-209)
    fn bend_vertices(&self, prev_seg: SegPart, next_seg: SegPart, node: SegNode) -> Vec<Item> {
        let (h, v) = if prev_seg.is_horizontal() { (prev_seg, next_seg) } else { (next_seg, prev_seg) };
        let n = self.pt(node);
        let w_next = self.seg_w(next_seg);
        let nw = self.s.node_widths.get(&node).copied(); // 이 모서리의 분할 굵기(없으면 원굵기)
        let scoop_w = nw.unwrap_or(self.s.full_width);
        let cap = self.h_cap(h, node.right);
        if cap == SegCap::Up || cap == SegCap::Down {
            let z = if cap == SegCap::Up { self.zone_above(node.level) } else { self.zone_below(node.level) };
            if let Some(z) = z {
                let corner = corner_of(cap == SegCap::Up, node.right);
                let r = self.zone_radius(z, corner);
                let (a, b) = seg_ends(v, self.s.menu_on, self.s.dock_on);
                let v_down = (if a == node { b.level } else { a.level }) > node.level;
                let matches = (cap == SegCap::Down && v_down) || (cap == SegCap::Up && !v_down);
                if matches {
                    return vec![(pv(n, r, None), w_next, nw)];
                }
                if r > 0.01 {
                    // 스쿱: O = 경계선 너머 지나친 점, T = 가로선 위 접점, 제어점 = 노드.
                    let o = Point::new(n.x, n.y + if cap == SegCap::Down { r } else { -r });
                    let t = Point::new(n.x + if node.right { -r } else { r }, n.y);
                    return if prev_seg.is_horizontal() {
                        // 가로 -> 세로: T까지, N 제어로 O까지 오목, N을 지나 세로로.
                        vec![
                            (pv(t, 0.0, None), scoop_w, None),
                            (pv(o, 0.0, Some(n)), scoop_w, None),
                            (pv(n, 0.0, None), w_next, None),
                        ]
                    } else {
                        // 세로 -> 가로: N을 지나 O까지, N 제어로 T까지 오목.
                        vec![
                            (pv(n, 0.0, None), scoop_w, None),
                            (pv(o, 0.0, None), scoop_w, None),
                            (pv(t, 0.0, Some(n)), w_next, None),
                        ]
                    };
                }
            }
        }
        // 기본: 세로선 영역 꼭짓점의 값으로 자연 꺾임.
        let z = zone_of(v);
        let is_bottom = self.bottom_level(z) == node.level;
        let r = self.zone_radius(z, corner_of(is_bottom, node.right));
        vec![(pv(n, r, None), w_next, nw)]
    }

    /// 상단선을 지날 때 노치 우회 정점들(레인 오프셋이 있으면 경사 점 4개 추가). (BorderView.swift:260-286)
    fn notch_vertices(&self, reversed: bool) -> Vec<(Pv, f64)> {
        let s = self.s;
        if !(s.notch_enabled && s.notch_width > 0.0 && s.notch_height > 0.0) {
            return vec![];
        }
        // 노치 우회는 레인 오프셋 없이 원래 위치·원굵기로 그린다.
        let cx = self.w / 2.0;
        let y0 = self.y_of(0);
        let lane = s.lane_offsets.get(&SegPart::HTop).copied().unwrap_or(0.0);
        let nl = f64::max(cx - s.notch_width / 2.0, self.xl + 1.0);
        let nr = f64::min(cx + s.notch_width / 2.0, self.xr - 1.0);
        if nr <= nl {
            return vec![];
        }
        let nrr = |c: NotchCorner| f64::max(0.0, s.notch_radii.get(&c).copied().unwrap_or(0.0));
        let fw = s.full_width;
        let p = |x: f64, y: f64, r: f64| (pv(Point::new(x, y), r, None), fw);
        let mut list = vec![
            p(nl, y0, nrr(NotchCorner::OuterLeft)),
            p(nl, y0 + s.notch_height, nrr(NotchCorner::InnerLeft)),
            p(nr, y0 + s.notch_height, nrr(NotchCorner::InnerRight)),
            p(nr, y0, nrr(NotchCorner::OuterRight)),
        ];
        if lane.abs() > 0.01 {
            // insert(at: 0)를 두 번: 결과 순서 = [nl-52(경사), nl-20, ...]
            list.insert(0, p(nl - 20.0, y0, 0.0));
            list.insert(0, p(nl - 52.0, y0 + lane, 0.0));
            list.push(p(nr + 20.0, y0, 0.0));
            list.push(p(nr + 52.0, y0 + lane, 0.0));
        }
        let mut out: Vec<(Pv, f64)> = if reversed { list.into_iter().rev().collect() } else { list };
        // 마지막 노치 정점을 떠나는 구간은 다시 상단선(분할 굵기)이다.
        if let Some(last) = out.last_mut() {
            last.1 = self.seg_w(SegPart::HTop);
        }
        out
    }

    /// 열린 체인 끝의 캡 정점들. 시작이면 [접점, 꼭짓점], 끝이면 [꼭짓점, 접점] 순. (BorderView.swift:289-321)
    fn cap_vertices(&self, cap: SegCap, node: SegNode, seg: SegPart, at_start: bool) -> Option<Vec<(Pv, f64)>> {
        let p = self.pt(node);
        let r;
        let tangent;
        if seg.is_horizontal() {
            match cap {
                SegCap::Up => {
                    let z = self.zone_above(node.level)?;
                    r = self.zone_radius(z, corner_of(true, node.right));
                    tangent = Point::new(p.x, p.y - r);
                }
                SegCap::Down => {
                    let z = self.zone_below(node.level)?;
                    r = self.zone_radius(z, corner_of(false, node.right));
                    tangent = Point::new(p.x, p.y + r);
                }
                _ => return None,
            }
        } else {
            if cap != SegCap::Include {
                return None;
            }
            let z = zone_of(seg);
            let is_bottom = self.bottom_level(z) == node.level;
            r = self.zone_radius(z, corner_of(is_bottom, node.right));
            tangent = Point::new(if node.right { p.x - r } else { p.x + r }, p.y);
        }
        if r <= 0.01 {
            return None;
        }
        let w = self.seg_w(seg); // 캡 아크는 끝 세그먼트의 굵기를 따른다.
        Some(if at_start {
            vec![(pv(tangent, 0.0, None), w), (pv(p, r, None), w)]
        } else {
            vec![(pv(p, r, None), w), (pv(tangent, 0.0, None), w)]
        })
    }

    fn end_cap(&self, node: SegNode, seg: SegPart, auto: bool) -> SegCap {
        if seg.is_horizontal() {
            return self.h_cap(seg, node.right);
        }
        if auto { SegCap::Include } else { SegCap::None }
    }
}

/// 모서리 아크: 중간점에서 반으로 갈라 앞 절반은 들어온 구간, 뒤 절반은 나가는 구간의 굵기를 쓴다.
/// (BorderView.swift:435-449)
#[allow(clippy::too_many_arguments)]
fn corner_arc(
    em: &mut Emitter,
    vs: &[Pv],
    av: &[Option<f64>],
    i: usize,
    t1: &[Point],
    t2: &[Point],
    w_in_raw: f64,
    w_out_raw: f64,
    n_in: bool,
    n_out: bool,
) {
    let c = vs[i].pt;
    // 노드 겹침 분할: 여러 AI가 지나는 모서리는 아크를 t/k 이하로.
    let cap = av[i].unwrap_or(f64::MAX);
    let (w_in, w_out) = (w_in_raw.min(cap), w_out_raw.min(cap));
    if (w_in - w_out).abs() < 0.01 && n_in == n_out {
        em.qd(t2[i], c, f64::max(w_in, w_out), n_in || n_out);
        return;
    }
    let mid = Point::new(0.25 * t1[i].x + 0.5 * c.x + 0.25 * t2[i].x, 0.25 * t1[i].y + 0.5 * c.y + 0.25 * t2[i].y);
    em.qd(mid, Point::new((t1[i].x + c.x) / 2.0, (t1[i].y + c.y) / 2.0), w_in, n_in);
    em.qd(t2[i], Point::new((c.x + t2[i].x) / 2.0, (c.y + t2[i].y) / 2.0), w_out, n_out);
}

/// 정점 목록 + 정점별 부가 정보. (Swift `vs`/`ew`/`en`/`av`)
#[derive(Default)]
struct Verts {
    vs: Vec<Pv>,
    /// ew[i] = 정점 i를 떠나는 구간의 굵기
    ew: Vec<f64>,
    /// en[i] = 정점 i를 떠나는 구간이 노치 우회인지
    en: Vec<bool>,
    /// av[i] = 정점 i 아크의 굵기 상한(노드 겹침 분할)
    av: Vec<Option<f64>>,
}

impl Verts {
    fn add(&mut self, items: &[Item], notch: bool) {
        for (idx, item) in items.iter().enumerate() {
            self.vs.push(item.0);
            self.ew.push(item.1);
            self.av.push(item.2);
            // 노치 목록의 마지막 정점을 떠나는 구간은 다시 상단선이다.
            self.en.push(notch && idx + 1 < items.len());
        }
    }
}

impl ChainShape {
    /// 경로를 만들고 방출 순서대로 (요소 길이, 굵기, 노치 여부)를 기록한다. 화면 전체 크기 (w, h). (Swift `build`)
    pub fn build(&self, w: f64, h: f64) -> Built {
        let segs: BTreeSet<SegPart> =
            self.segments.iter().copied().filter(|&s| seg_available(s, self.menu_on, self.dock_on)).collect();
        if segs.is_empty() || !is_valid_segments(&segs, self.menu_on, self.dock_on) {
            return Built::default();
        }
        let cx = Ctx { s: self, segs, w, h, xl: self.inset, xr: w - self.inset };
        let segs = &cx.segs;
        let (menu_on, dock_on) = (self.menu_on, self.dock_on);

        // ---- 체인 순회 ----
        let adj = seg_adjacency(segs, menu_on, dock_on);
        let eps: Vec<SegNode> = adj.iter().filter(|(_, v)| v.len() == 1).map(|(k, _)| *k).collect(); // 정렬됨
        let is_loop = eps.is_empty();

        let start_node: SegNode;
        let (mut s_half, mut s_auto, mut e_half, mut e_auto) = (false, false, false, false);
        if is_loop {
            let tl = adj.keys().map(|n| n.level).min().unwrap_or(0);
            let bl = adj.keys().map(|n| n.level).max().unwrap_or(3);
            let want = match self.anchor_corner {
                BorderCorner::TopLeft => SegNode { right: false, level: tl },
                BorderCorner::TopRight => SegNode { right: true, level: tl },
                BorderCorner::BottomRight => SegNode { right: true, level: bl },
                BorderCorner::BottomLeft => SegNode { right: false, level: bl },
            };
            start_node = if adj.contains_key(&want) {
                want
            } else {
                adj.keys().next().copied().unwrap_or(SegNode { right: false, level: 0 })
            };
        } else {
            // 방향 반전(clockwise=false)이면 반대 끝에서 시작하되 끝 보정은 끝에 고정된 채 따라간다.
            start_node = if self.clockwise { eps[0] } else { eps[1] };
            s_half = if self.clockwise { self.end_a_half } else { self.end_b_half };
            s_auto = if self.clockwise { self.end_a_auto } else { self.end_b_auto };
            e_half = if self.clockwise { self.end_b_half } else { self.end_a_half };
            e_auto = if self.clockwise { self.end_b_auto } else { self.end_a_auto };
        }

        let mut nodes_order = vec![start_node];
        let mut segs_order: Vec<SegPart> = vec![];
        let mut prev_visit: Option<SegPart> = None;
        let mut cur = start_node;
        while segs_order.len() < segs.len() {
            let mut cands: Vec<SegPart> = adj
                .get(&cur)
                .map(|v| v.iter().copied().filter(|s| Some(*s) != prev_visit).collect())
                .unwrap_or_default();
            if segs_order.is_empty() && cands.len() == 2 {
                // 고리 시작: clockwise면 가로 우선, 아니면 세로 우선(안정 정렬). "시계 방향"이 아니라 "가로 먼저".
                cands.sort_by_key(|s| if self.clockwise { !s.is_horizontal() } else { s.is_horizontal() });
            }
            let Some(&s) = cands.first() else { break };
            let (a, b) = seg_ends(s, menu_on, dock_on);
            let next = if a == cur { b } else { a };
            segs_order.push(s);
            nodes_order.push(next);
            prev_visit = Some(s);
            cur = next;
        }
        if segs_order.len() != segs.len() {
            return Built::default();
        }

        // ---- 정점 목록 ----
        let mut vt = Verts::default();
        let (mut start_has_cap, mut end_has_cap) = (false, false);
        let mut anchor_vert_count = 1usize; // 앵커 노드가 만든 정점 수(스쿱이면 3)
        if is_loop {
            let k = segs_order.len();
            for i in 0..k {
                let prev_seg = segs_order[(i + k - 1) % k];
                let next_seg = segs_order[i];
                if prev_seg.is_horizontal() != next_seg.is_horizontal() {
                    vt.add(&cx.bend_vertices(prev_seg, next_seg, nodes_order[i]), false);
                } else {
                    vt.add(&[(pv(cx.pt(nodes_order[i]), 0.0, None), cx.seg_w(next_seg), None)], false);
                }
                if i == 0 {
                    anchor_vert_count = vt.vs.len();
                }
                if next_seg == SegPart::HTop && self.notch_detour {
                    let nv: Vec<Item> =
                        cx.notch_vertices(nodes_order[i].right).into_iter().map(|(p, w)| (p, w, None)).collect();
                    vt.add(&nv, true);
                }
            }
        } else {
            let s_cap = cx.end_cap(nodes_order[0], segs_order[0], s_auto);
            if let Some(cv) = cx.cap_vertices(s_cap, nodes_order[0], segs_order[0], true) {
                let items: Vec<Item> = cv.into_iter().map(|(p, w)| (p, w, None)).collect();
                vt.add(&items, false);
                start_has_cap = true;
            } else {
                vt.add(&[(pv(cx.pt(nodes_order[0]), 0.0, None), cx.seg_w(segs_order[0]), None)], false);
            }
            for i in 0..segs_order.len() {
                if segs_order[i] == SegPart::HTop && self.notch_detour {
                    let nv: Vec<Item> =
                        cx.notch_vertices(nodes_order[i].right).into_iter().map(|(p, w)| (p, w, None)).collect();
                    vt.add(&nv, true);
                }
                if i + 1 < segs_order.len() {
                    if segs_order[i].is_horizontal() != segs_order[i + 1].is_horizontal() {
                        vt.add(&cx.bend_vertices(segs_order[i], segs_order[i + 1], nodes_order[i + 1]), false);
                    } else {
                        vt.add(
                            &[(pv(cx.pt(nodes_order[i + 1]), 0.0, None), cx.seg_w(segs_order[i + 1]), None)],
                            false,
                        );
                    }
                }
            }
            let last = segs_order.len();
            let e_cap = cx.end_cap(nodes_order[last], segs_order[last - 1], e_auto);
            if let Some(cv) = cx.cap_vertices(e_cap, nodes_order[last], segs_order[last - 1], false) {
                let items: Vec<Item> = cv.into_iter().map(|(p, w)| (p, w, None)).collect();
                vt.add(&items, false);
                end_has_cap = true;
            } else {
                vt.add(
                    &[(pv(cx.pt(nodes_order[last]), 0.0, None), cx.seg_w(segs_order[last - 1]), None)],
                    false,
                );
            }
        }

        // ---- 정점 -> 둥근 폴리라인 경로 ----
        let mut em = Emitter { path: BezPath::new(), cp: Point::ZERO, rec: vec![] };
        if is_loop {
            // 앵커 정점에 곡률이 없고(스쿱 모서리) 같은 노드에 정점이 여럿이면 차감 시작 지점을
            // 정점 회전으로 구현한다: 위 = T(가로선 접점), 꼭짓점 = N(모서리), 아래 = O(오버슛 끝).
            if vt.vs[0].r < 0.01 && anchor_vert_count >= 3 && vt.vs.len() >= 3 {
                let idxs: Vec<usize> = (0..anchor_vert_count).collect();
                // O = 다른 정점들과 y가 가장 다른 정점, N = 중앙에서 x가 가장 먼 나머지 정점, T = 남는 정점.
                // Swift `max(by:)`는 엄격히 클 때만 교체하므로 동률이면 앞쪽을 고른다.
                let score = |a: usize| idxs.iter().map(|&i| (vt.vs[a].pt.y - vt.vs[i].pt.y).abs()).sum::<f64>();
                let mut o_idx = idxs[0];
                for &i in &idxs[1..] {
                    if score(o_idx) < score(i) {
                        o_idx = i;
                    }
                }
                let rest: Vec<usize> = idxs.iter().copied().filter(|&i| i != o_idx).collect();
                let dx = |i: usize| (vt.vs[i].pt.x - w / 2.0).abs();
                let mut n_idx = rest[0];
                for &i in &rest[1..] {
                    if dx(n_idx) < dx(i) {
                        n_idx = i;
                    }
                }
                let t_idx = rest.iter().copied().find(|&i| i != n_idx).unwrap_or(0);
                let pick = match self.anchor_side {
                    AnchorSide::Left => t_idx,
                    AnchorSide::Center => n_idx,
                    AnchorSide::Right => o_idx,
                };
                if pick > 0 {
                    vt.vs.rotate_left(pick);
                    vt.ew.rotate_left(pick);
                    vt.en.rotate_left(pick);
                    vt.av.rotate_left(pick);
                }
            }
            let n = vt.vs.len();
            if n < 3 {
                return Built { path: em.path, rec: em.rec };
            }
            let (t1, t2) = tangents(&vt.vs);
            let (vs, ew, en, av) = (&vt.vs, &vt.ew, &vt.en, &vt.av);
            // 정점 i의 곡선(모서리) 굵기: 양옆 구간 중 굵은 쪽 + 노드 분할 상한.
            let aw = |i: usize| f64::max(ew[(i + n - 1) % n], ew[i]).min(av[i].unwrap_or(f64::MAX));
            let c0 = vs[0].pt;
            // 차감 시작 지점: 시작 모서리 곡선의 두 접점 중 위(작은 y) 또는 아래에서 시작한다.
            // 순회 방향과 무관하게 y좌표로 판정한다(.left = 위, .right = 아래, .center는 위로 취급).
            let want_down = self.anchor_side == AnchorSide::Right;
            let t1_is_up = t1[0].y <= t2[0].y;
            let start_at_t1 = t1_is_up != want_down;
            if start_at_t1 {
                em.mv(t1[0]);
                em.qd(t2[0], c0, aw(0), false);
            } else {
                em.mv(t2[0]);
            }
            for i in 1..n {
                if let Some(ov) = vs[i].over {
                    // 스쿱 정점: 직전 점에서 명시적 제어점으로 오목 곡선.
                    em.qd(vs[i].pt, ov, ew[i - 1], en[i - 1]);
                } else {
                    em.ln(t1[i], ew[i - 1], en[i - 1]);
                    corner_arc(&mut em, vs, av, i, &t1, &t2, ew[i - 1], ew[i], en[i - 1], en[i]);
                }
            }
            if let Some(ov0) = vs[0].over {
                // 시작 정점이 오목(스쿱) 곡선의 종점이면 seam을 그 곡선으로 닫는다.
                em.qd(vs[0].pt, ov0, ew[n - 1], en[n - 1]);
            } else {
                em.ln(t1[0], ew[n - 1], en[n - 1]);
                if !start_at_t1 {
                    em.qd(t2[0], c0, aw(0), false);
                }
            }
            // closeSubpath: 마지막 점이 시작점과 같아 길이가 0이므로 닫는 선분은 만들지 않는다.
        } else {
            let n = vt.vs.len();
            if n < 2 {
                return Built { path: em.path, rec: em.rec };
            }
            let (t1, t2) = tangents(&vt.vs);
            let (vs, ew, en, av) = (&vt.vs, &vt.ew, &vt.en, &vt.av);
            let aw = |i: usize| f64::max(ew[i.saturating_sub(1)], ew[i.min(n - 1)]);
            // 반쪽 아크: 캡 모서리 곡선을 중간점까지만 그린다.
            let arc_mid = |i: usize| {
                let c = vs[i].pt;
                Point::new(0.25 * t1[i].x + 0.5 * c.x + 0.25 * t2[i].x, 0.25 * t1[i].y + 0.5 * c.y + 0.25 * t2[i].y)
            };
            let half_at_start = s_half && start_has_cap && n >= 3;
            let half_at_end = e_half && end_has_cap && n >= 3;
            let mut from = 1;
            if half_at_start {
                let c = vs[1].pt;
                em.mv(arc_mid(1));
                em.qd(t2[1], Point::new((c.x + t2[1].x) / 2.0, (c.y + t2[1].y) / 2.0), aw(1), false);
                from = 2;
            } else {
                em.mv(vs[0].pt);
            }
            let to = if half_at_end { n - 2 } else { n - 1 };
            for i in from..to {
                if let Some(ov) = vs[i].over {
                    em.qd(vs[i].pt, ov, ew[i - 1], en[i - 1]);
                } else {
                    em.ln(t1[i], ew[i - 1], en[i - 1]);
                    let j = i.min(n - 1);
                    corner_arc(&mut em, vs, av, i, &t1, &t2, ew[i - 1], ew[j], en[i - 1], en[j]);
                }
            }
            if half_at_end {
                let i = n - 2;
                let c = vs[i].pt;
                em.ln(t1[i], ew[i - 1], false);
                em.qd(arc_mid(i), Point::new((t1[i].x + c.x) / 2.0, (t1[i].y + c.y) / 2.0), aw(i), false);
            } else {
                em.ln(vs[n - 1].pt, ew[n - 2], en[n - 2]);
            }
        }
        Built { path: em.path, rec: em.rec }
    }
}
