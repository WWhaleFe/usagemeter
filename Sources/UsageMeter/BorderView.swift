import SwiftUI

/// 선택된 세그먼트(가로 4 + 세로 좌·우 각 3)를 **하나의 연속 경로**로 그리는 모양.
///
/// - 세그먼트 선택은 '한 줄로 이어진 선(체인) 또는 고리'만 유효.
/// - 가로·세로가 만나 꺾이는 노드는 해당 영역 꼭짓점의 곡률로 둥글게, 직선 통과는 그대로.
/// - 열린 체인의 양 끝에는 모서리 캡(가로선: 위/아래, 세로선: 포함)을 붙일 수 있다.
/// - 상단 가로선(hTop)을 지날 때 노치를 우회한다.
///
/// 겹침 분할은 **경로 위 구간(트림 비율)** 단위로 굵기를 계산한다(#노치 단차 근본 수정):
/// `widthSpans(in:)`가 경로를 따라 (구간, 굵기) 목록을 돌려주고, BorderView가 그 구간별로
/// 트림해 스트로크한다. 공간(사각형 클립) 기반이 아니라서 노치 우회처럼 다른 선의 영역을
/// 지나가는 부분이 엉뚱한 굵기로 잘리는 문제가 원천적으로 없다.
struct SegmentChainShape: Shape {
    let segments: Set<SegPart>
    let menuOn: Bool
    let dockOn: Bool
    let menuH: CGFloat
    let dockH: CGFloat
    let inset: CGFloat
    let menuRadii: [BorderCorner: CGFloat]
    let mainRadii: [BorderCorner: CGFloat]
    let dockRadii: [BorderCorner: CGFloat]
    let notchEnabled: Bool
    let notchWidth: CGFloat
    let notchHeight: CGFloat
    let notchRadii: [NotchCorner: CGFloat]
    let anchorCorner: BorderCorner
    let anchorSide: AnchorSide
    let clockwise: Bool
    /// 가로선 좌/우 끝의 위/아래 둥글게. 키 = OverlaySettings.hCapKey(세그먼트, 우?).
    let hCaps: [String: SegCap]
    // 정렬(위→아래, 좌→우) 기준 첫/마지막 끝의 교차 AI 보정(#모서리 반씩 양분).
    let endAHalf: Bool       // 첫 끝의 캡/아크를 반쪽만 그리기
    let endAAuto: Bool       // 첫 끝(세로선)에 자동 아크 붙이기
    let endBHalf: Bool
    let endBAuto: Bool
    /// 겹침 분할용 레인 오프셋: 세그먼트별 수직 이동량(가로선=Δy, 세로선=Δx).
    var laneOffsets: [SegPart: CGFloat] = [:]
    /// 겹침 분할용 구간 굵기: 공유 세그먼트 → t/N. 없으면 fullWidth.
    var laneWidths: [SegPart: CGFloat] = [:]
    /// 원래 선 굵기(분할 안 된 구간·모서리·노치·캡).
    var fullWidth: CGFloat = 0
    /// 이 AI가 노치를 우회할지(#노치 겹침 제외). 상단선을 여러 AI가 나눌 때는 최상단
    /// 레이어 한 명만 원래 굵기로 우회하고, 나머지는 직선으로 통과한다(물리 노치에 가려 안 보임).
    var notchDetour: Bool = true
    /// 노드(모서리)별 분할 굵기(#모서리 겹침): 한 노드를 2개 이상의 AI가 지나가면
    /// 그 모서리의 아크·스쿱 요소를 t/k로 그린다. 세그먼트 공유가 아니어도 모서리에서
    /// 실제로 겹치는 경우(스쿱 오버슛 등)를 처리한다.
    var nodeWidths: [SegNode: CGFloat] = [:]

    func path(in rect: CGRect) -> Path {
        build(in: rect, record: nil)
    }

    /// 경로를 따라 굵기가 달라지는 구간 목록: (트림 시작, 끝, 굵기, 노치 여부) + 전체 길이(pt).
    /// 인접한 같은 (굵기, 노치) 구간은 병합된다. 노치 구간은 레이어상 가장 뒤에 그린다.
    func widthSpans(in rect: CGRect)
        -> (spans: [(from: CGFloat, to: CGFloat, width: CGFloat, notch: Bool)], total: CGFloat) {
        var items: [(CGFloat, CGFloat, Bool)] = []
        _ = build(in: rect) { len, w, notch in items.append((len, w, notch)) }
        let total = items.reduce(CGFloat(0)) { $0 + $1.0 }
        guard total > 0 else { return ([], 0) }
        var out: [(CGFloat, CGFloat, CGFloat, Bool)] = []
        var acc: CGFloat = 0
        for (len, w, notch) in items {
            let a = acc / total, b = (acc + len) / total
            if var last = out.last, abs(last.2 - w) < 0.01, last.3 == notch {
                last.1 = b
                out[out.count - 1] = last
            } else if len > 0.001 {
                out.append((a, b, w, notch))
            }
            acc += len
        }
        return (out.map { (from: $0.0, to: $0.1, width: $0.2, notch: $0.3) }, total)
    }

    // MARK: - 경로 생성(+선택적 구간 기록)

    /// 경로를 만들고, record가 있으면 방출 순서대로 (요소 길이, 굵기, 노치 여부)를 기록한다.
    private func build(in rect: CGRect, record: ((CGFloat, CGFloat, Bool) -> Void)?) -> Path {
        var p = Path()
        let segs = segments.filter { OverlaySettings.segAvailable($0, menuOn: menuOn, dockOn: dockOn) }
        guard !segs.isEmpty,
              OverlaySettings.isValidSegments(segs, menuOn: menuOn, dockOn: dockOn) else { return p }

        let xL = rect.minX + inset, xR = rect.maxX - inset
        func yOf(_ level: Int) -> CGFloat {
            switch level {
            case 0: return rect.minY + inset
            case 1: return rect.minY + menuH
            case 2: return rect.maxY - dockH
            default: return rect.maxY - inset
            }
        }
        func pt(_ n: SegNode) -> CGPoint {
            var dx: CGFloat = 0, dy: CGFloat = 0
            // 이 노드에 붙은 내 세그먼트의 레인 오프셋을 합성(가로=Δy, 세로=Δx).
            if !laneOffsets.isEmpty {
                for s in segs {
                    guard let o = laneOffsets[s], o != 0 else { continue }
                    let (a, b) = OverlaySettings.segEnds(s, menuOn: menuOn, dockOn: dockOn)
                    guard a == n || b == n else { continue }
                    if s.isHorizontal { dy = o } else { dx = o }
                }
            }
            return CGPoint(x: (n.right ? xR : xL) + dx, y: yOf(n.level) + dy)
        }
        /// 세그먼트별 구간 굵기(분할 시 t/N, 아니면 원굵기).
        func segW(_ s: SegPart) -> CGFloat { laneWidths[s] ?? fullWidth }

        // MARK: 영역/곡률 매핑
        func zoneDict(_ z: ScreenZone) -> [BorderCorner: CGFloat] {
            switch z { case .menuBar: return menuRadii; case .main: return mainRadii; case .dock: return dockRadii }
        }
        func zoneOf(_ v: SegPart) -> ScreenZone {
            switch v {
            case .lMenuBar, .rMenuBar: return .menuBar
            case .lDock, .rDock: return .dock
            default: return .main
            }
        }
        func bottomLevel(_ z: ScreenZone) -> Int {
            switch z { case .menuBar: return 1; case .main: return dockOn ? 2 : 3; case .dock: return 3 }
        }
        func zoneAbove(_ level: Int) -> ScreenZone? {
            switch level {
            case 1: return menuOn ? .menuBar : nil
            case 2: return .main
            case 3: return dockOn ? .dock : .main
            default: return nil
            }
        }
        func zoneBelow(_ level: Int) -> ScreenZone? {
            switch level {
            case 0: return menuOn ? .menuBar : .main
            case 1: return .main
            case 2: return dockOn ? .dock : nil
            default: return nil
            }
        }

        typealias PV = (pt: CGPoint, r: CGFloat, pointed: Bool, over: CGPoint?)

        /// 접합부(가로 h + 세로 v) 노드의 정점들. 대개 곡률 하나짜리 정점 1개.
        /// 캡 방향이 세로선과 반대면 **스쿱**(#모서리 모양 2)으로 그 방향을 살린다.
        /// 반환: [(정점, 그 정점을 떠나는 구간의 굵기, 이 정점 아크의 굵기 상한)]
        /// 아크 굵기 상한 = 노드 분할 굵기(#모서리 겹침 — 여러 AI가 지나는 모서리는 t/k).
        func bendVertices(prevSeg: SegPart, nextSeg: SegPart, at node: SegNode)
            -> [(PV, CGFloat, CGFloat?)] {
            let h = prevSeg.isHorizontal ? prevSeg : nextSeg
            let v = prevSeg.isHorizontal ? nextSeg : prevSeg
            let N = pt(node)
            let wNext = segW(nextSeg)
            let nw = nodeWidths[node]                 // 이 모서리의 분할 굵기(없으면 원굵기)
            let scoopW = nw ?? fullWidth
            let cap = hCaps[OverlaySettings.hCapKey(h, right: node.right)] ?? SegCap.none
            if cap == .up || cap == .down {
                let z: ScreenZone? = (cap == .up) ? zoneAbove(node.level) : zoneBelow(node.level)
                if let z {
                    let corner: BorderCorner = (cap == .up)
                        ? (node.right ? .bottomRight : .bottomLeft)
                        : (node.right ? .topRight : .topLeft)
                    let r = max(0, zoneDict(z)[corner] ?? 0)
                    let (a, b) = OverlaySettings.segEnds(v, menuOn: menuOn, dockOn: dockOn)
                    let vDown = ((a == node) ? b.level : a.level) > node.level
                    let matches = (cap == .down && vDown) || (cap == .up && !vDown)
                    if matches {
                        return [((N, r, false, nil), wNext, nw)]
                    }
                    if r > 0.01 {
                        // 스쿱: O = 경계선 너머 지나친 점, T = 가로선 위 접점, 제어점 = 노드.
                        // 오버슛(N↔O)과 오목 아크는 노드 분할 굵기로 — 다른 AI의 세로선과
                        // 실제로 겹치는 구간이다(#모서리 겹침).
                        let O = CGPoint(x: N.x, y: N.y + (cap == .down ? r : -r))
                        let T = CGPoint(x: N.x + (node.right ? -r : r), y: N.y)
                        if prevSeg.isHorizontal {
                            // 가로 → 세로: T까지, N 제어로 O까지 오목, N을 지나 세로로.
                            return [((T, 0, false, nil), scoopW, nil),
                                    ((O, 0, false, N), scoopW, nil),
                                    ((N, 0, false, nil), wNext, nil)]
                        } else {
                            // 세로 → 가로: N을 지나 O까지, N 제어로 T까지 오목.
                            return [((N, 0, false, nil), scoopW, nil),
                                    ((O, 0, false, nil), scoopW, nil),
                                    ((T, 0, false, N), wNext, nil)]
                        }
                    }
                }
            }
            // 기본: 세로선 영역 꼭짓점의 값으로 자연 꺾임.
            let z = zoneOf(v)
            let isBottom = bottomLevel(z) == node.level
            let corner: BorderCorner = isBottom
                ? (node.right ? .bottomRight : .bottomLeft)
                : (node.right ? .topRight : .topLeft)
            return [((N, max(0, zoneDict(z)[corner] ?? 0), false, nil), wNext, nw)]
        }

        // MARK: 체인 순회
        let adj = OverlaySettings.segAdjacency(segs, menuOn: menuOn, dockOn: dockOn)
        let eps = adj.filter { $0.value.count == 1 }.keys.sorted()
        let isLoop = eps.isEmpty

        var startNode: SegNode
        var sHalf = false, sAuto = false, eHalf = false, eAuto = false
        if isLoop {
            let levels = adj.keys.map(\.level)
            let tl = levels.min() ?? 0, bl = levels.max() ?? 3
            let want: SegNode = {
                switch anchorCorner {
                case .topLeft: return SegNode(right: false, level: tl)
                case .topRight: return SegNode(right: true, level: tl)
                case .bottomRight: return SegNode(right: true, level: bl)
                case .bottomLeft: return SegNode(right: false, level: bl)
                }
            }()
            startNode = adj[want] != nil ? want : (adj.keys.sorted().first ?? SegNode(right: false, level: 0))
        } else {
            // 방향 반전(clockwise=false)이면 반대 끝에서 시작하되 끝 보정은 끝에 고정된 채 따라간다.
            startNode = clockwise ? eps[0] : eps[1]
            sHalf = clockwise ? endAHalf : endBHalf
            sAuto = clockwise ? endAAuto : endBAuto
            eHalf = clockwise ? endBHalf : endAHalf
            eAuto = clockwise ? endBAuto : endAAuto
        }

        var nodesOrder: [SegNode] = [startNode]
        var segsOrder: [SegPart] = []
        var prevSegVisit: SegPart? = nil
        var cur = startNode
        while segsOrder.count < segs.count {
            var cands = (adj[cur] ?? []).filter { $0 != prevSegVisit }
            if segsOrder.isEmpty && cands.count == 2 {
                // 고리 시작: 시계방향이면 가로 우선(좌상에서 오른쪽으로 진행).
                cands.sort { a, _ in clockwise ? a.isHorizontal : !a.isHorizontal }
            }
            guard let s = cands.first else { break }
            let (a, b) = OverlaySettings.segEnds(s, menuOn: menuOn, dockOn: dockOn)
            let next = (a == cur) ? b : a
            segsOrder.append(s)
            nodesOrder.append(next)
            prevSegVisit = s
            cur = next
        }
        guard segsOrder.count == segs.count else { return p }

        // MARK: 정점 목록 (정점, 그 정점을 떠나는 구간의 굵기)
        func notchVertices(reversed: Bool) -> [(PV, CGFloat)] {
            guard notchEnabled, notchWidth > 0, notchHeight > 0 else { return [] }
            // 노치 우회는 레인 오프셋 없이 **원래 위치·원굵기**로 그린다(#노치 겹침 제외).
            // 오프셋이 있으면 노치 앞뒤 경사 램프로 0까지 수렴시킨다.
            let cx = rect.midX, y0 = yOf(0)
            let lane = laneOffsets[.hTop] ?? 0
            let nl = max(cx - notchWidth / 2, xL + 1)
            let nr = min(cx + notchWidth / 2, xR - 1)
            guard nr > nl else { return [] }
            func nrr(_ c: NotchCorner) -> CGFloat { max(0, notchRadii[c] ?? 0) }
            var list: [(PV, CGFloat)] = [
                ((CGPoint(x: nl, y: y0), nrr(.outerLeft), false, nil), fullWidth),
                ((CGPoint(x: nl, y: y0 + notchHeight), nrr(.innerLeft), false, nil), fullWidth),
                ((CGPoint(x: nr, y: y0 + notchHeight), nrr(.innerRight), false, nil), fullWidth),
                ((CGPoint(x: nr, y: y0), nrr(.outerRight), false, nil), fullWidth),
            ]
            if abs(lane) > 0.01 {
                list.insert(((CGPoint(x: nl - 20, y: y0), 0, false, nil), fullWidth), at: 0)
                list.insert(((CGPoint(x: nl - 52, y: y0 + lane), 0, false, nil), fullWidth), at: 0)
                list.append(((CGPoint(x: nr + 20, y: y0), 0, false, nil), fullWidth))
                list.append(((CGPoint(x: nr + 52, y: y0 + lane), 0, false, nil), fullWidth))
            }
            var out = reversed ? Array(list.reversed()) : list
            // 마지막 노치 정점을 떠나는 구간은 다시 상단선(분할 굵기)이다.
            if !out.isEmpty { out[out.count - 1].1 = segW(.hTop) }
            return out
        }

        /// 열린 체인 끝의 캡 정점들. 시작이면 [접점, 꼭짓점], 끝이면 [꼭짓점, 접점] 순.
        func capVertices(_ cap: SegCap, node: SegNode, seg: SegPart, atStart: Bool) -> [(PV, CGFloat)]? {
            let P = pt(node)
            var r: CGFloat = 0
            var tangent = P
            if seg.isHorizontal {
                switch cap {
                case .up:
                    guard let z = zoneAbove(node.level) else { return nil }
                    let c: BorderCorner = node.right ? .bottomRight : .bottomLeft
                    r = max(0, zoneDict(z)[c] ?? 0)
                    tangent = CGPoint(x: P.x, y: P.y - r)
                case .down:
                    guard let z = zoneBelow(node.level) else { return nil }
                    let c: BorderCorner = node.right ? .topRight : .topLeft
                    r = max(0, zoneDict(z)[c] ?? 0)
                    tangent = CGPoint(x: P.x, y: P.y + r)
                default: return nil
                }
            } else {
                guard cap == .include else { return nil }
                let z = zoneOf(seg)
                let isBottom = bottomLevel(z) == node.level
                let corner: BorderCorner = isBottom
                    ? (node.right ? .bottomRight : .bottomLeft)
                    : (node.right ? .topRight : .topLeft)
                r = max(0, zoneDict(z)[corner] ?? 0)
                tangent = CGPoint(x: node.right ? P.x - r : P.x + r, y: P.y)
            }
            guard r > 0.01 else { return nil }
            let w = segW(seg)   // 캡 아크는 끝 세그먼트의 굵기를 따른다(반쪽 아크 페어링과 일관).
            return atStart ? [((tangent, 0, false, nil), w), ((P, r, false, nil), w)]
                           : [((P, r, false, nil), w), ((tangent, 0, false, nil), w)]
        }

        func endCap(node: SegNode, seg: SegPart, auto: Bool) -> SegCap {
            if seg.isHorizontal { return hCaps[OverlaySettings.hCapKey(seg, right: node.right)] ?? SegCap.none }
            return auto ? .include : SegCap.none
        }

        var vs: [PV] = []
        var ew: [CGFloat] = []          // ew[i] = 정점 i를 떠나는 구간의 굵기
        var en: [Bool] = []             // en[i] = 정점 i를 떠나는 구간이 노치 우회인지
        var av: [CGFloat?] = []         // av[i] = 정점 i 아크의 굵기 상한(노드 겹침 분할)
        func addV(_ items: [(PV, CGFloat, CGFloat?)], notch: Bool = false) {
            for (idx, item) in items.enumerated() {
                vs.append(item.0); ew.append(item.1); av.append(item.2)
                // 노치 목록의 마지막 정점을 떠나는 구간은 다시 상단선이다.
                en.append(notch && idx < items.count - 1)
            }
        }
        var startHasCap = false, endHasCap = false
        var anchorVertCount = 1          // 앵커 노드가 만든 정점 수(스쿱이면 3)
        if isLoop {
            let k = segsOrder.count
            for i in 0..<k {
                let prevSeg = segsOrder[(i - 1 + k) % k]
                let nextSeg = segsOrder[i]
                if prevSeg.isHorizontal != nextSeg.isHorizontal {
                    addV(bendVertices(prevSeg: prevSeg, nextSeg: nextSeg, at: nodesOrder[i]))
                } else {
                    addV([((pt(nodesOrder[i]), 0, false, nil), segW(nextSeg), nil)])
                }
                if i == 0 { anchorVertCount = vs.count }
                if nextSeg == .hTop, notchDetour {
                    addV(notchVertices(reversed: nodesOrder[i].right).map { ($0.0, $0.1, nil) }, notch: true)
                }
            }
        } else {
            let sCapV = endCap(node: nodesOrder[0], seg: segsOrder[0], auto: sAuto)
            if let capVs = capVertices(sCapV, node: nodesOrder[0], seg: segsOrder[0], atStart: true) {
                addV(capVs.map { ($0.0, $0.1, nil) })
                startHasCap = true
            } else {
                addV([((pt(nodesOrder[0]), 0, false, nil), segW(segsOrder[0]), nil)])
            }
            for i in 0..<segsOrder.count {
                if segsOrder[i] == .hTop, notchDetour {
                    addV(notchVertices(reversed: nodesOrder[i].right).map { ($0.0, $0.1, nil) }, notch: true)
                }
                if i < segsOrder.count - 1 {
                    if segsOrder[i].isHorizontal != segsOrder[i + 1].isHorizontal {
                        addV(bendVertices(prevSeg: segsOrder[i], nextSeg: segsOrder[i + 1],
                                          at: nodesOrder[i + 1]))
                    } else {
                        addV([((pt(nodesOrder[i + 1]), 0, false, nil), segW(segsOrder[i + 1]), nil)])
                    }
                }
            }
            let last = segsOrder.count
            let eCapV = endCap(node: nodesOrder[last], seg: segsOrder[last - 1], auto: eAuto)
            if let capVs = capVertices(eCapV, node: nodesOrder[last], seg: segsOrder[last - 1], atStart: false) {
                addV(capVs.map { ($0.0, $0.1, nil) })
                endHasCap = true
            } else {
                addV([((pt(nodesOrder[last]), 0, false, nil), segW(segsOrder[last - 1]), nil)])
            }
        }

        // MARK: 정점 → 둥근 폴리라인 경로 (길이·굵기 기록 포함)
        func unit(_ a: CGPoint, _ b: CGPoint) -> CGPoint {
            let dx = b.x - a.x, dy = b.y - a.y
            let len = max(0.0001, (dx * dx + dy * dy).squareRoot())
            return CGPoint(x: dx / len, y: dy / len)
        }
        func dist(_ a: CGPoint, _ b: CGPoint) -> CGFloat {
            ((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)).squareRoot()
        }
        func tangents(_ vs: [PV]) -> (t1: [CGPoint], t2: [CGPoint]) {
            let n = vs.count
            var t1 = [CGPoint](repeating: .zero, count: n), t2 = t1
            for i in 0..<n {
                let cur = vs[i].pt
                let prevV = vs[(i - 1 + n) % n], nextV = vs[(i + 1) % n]
                let maxPrev = (prevV.r > 0.01 ? 0.5 : 1.0) * dist(cur, prevV.pt)
                let maxNext = (nextV.r > 0.01 ? 0.5 : 1.0) * dist(cur, nextV.pt)
                let r = max(0, min(vs[i].r, maxPrev, maxNext))
                t1[i] = CGPoint(x: cur.x + unit(cur, prevV.pt).x * r, y: cur.y + unit(cur, prevV.pt).y * r)
                t2[i] = CGPoint(x: cur.x + unit(cur, nextV.pt).x * r, y: cur.y + unit(cur, nextV.pt).y * r)
            }
            return (t1, t2)
        }

        // 방출 헬퍼: 이동/직선/곡선을 그리면서 (길이, 굵기)를 기록한다.
        var cp = CGPoint.zero
        func quadLen(_ a: CGPoint, _ c: CGPoint, _ b: CGPoint) -> CGFloat {
            var total: CGFloat = 0
            var prevPt = a
            for i in 1...12 {
                let t = CGFloat(i) / 12
                let mt = 1 - t
                let q = CGPoint(x: mt * mt * a.x + 2 * mt * t * c.x + t * t * b.x,
                                y: mt * mt * a.y + 2 * mt * t * c.y + t * t * b.y)
                total += dist(prevPt, q)
                prevPt = q
            }
            return total
        }
        func mv(_ q: CGPoint) { p.move(to: q); cp = q }
        func ln(_ q: CGPoint, _ w: CGFloat, _ notch: Bool = false) {
            p.addLine(to: q); record?(dist(cp, q), w, notch); cp = q
        }
        func qd(_ q: CGPoint, _ c: CGPoint, _ w: CGFloat, _ notch: Bool = false) {
            p.addQuadCurve(to: q, control: c); record?(quadLen(cp, c, q), w, notch); cp = q
        }
        /// 모서리 아크: 중간점에서 반으로 갈라 앞 절반은 들어온 구간, 뒤 절반은 나가는 구간의
        /// 굵기를 쓴다(#모서리 겹침 굵기 — 공유 구간 쪽 절반은 분할 굵기로 그려진다).
        func cornerArc(_ i: Int, _ t1: [CGPoint], _ t2: [CGPoint],
                       _ wInRaw: CGFloat, _ wOutRaw: CGFloat, _ nIn: Bool, _ nOut: Bool) {
            let C = vs[i].pt
            // 노드 겹침 분할(#모서리 겹침): 여러 AI가 지나는 모서리는 아크를 t/k 이하로.
            let cap = av[i] ?? CGFloat.greatestFiniteMagnitude
            let wIn = min(wInRaw, cap), wOut = min(wOutRaw, cap)
            if abs(wIn - wOut) < 0.01 && nIn == nOut {
                qd(t2[i], C, max(wIn, wOut), nIn || nOut)
                return
            }
            let mid = CGPoint(x: 0.25 * t1[i].x + 0.5 * C.x + 0.25 * t2[i].x,
                              y: 0.25 * t1[i].y + 0.5 * C.y + 0.25 * t2[i].y)
            qd(mid, CGPoint(x: (t1[i].x + C.x) / 2, y: (t1[i].y + C.y) / 2), wIn, nIn)
            qd(t2[i], CGPoint(x: (C.x + t2[i].x) / 2, y: (C.y + t2[i].y) / 2), wOut, nOut)
        }

        if isLoop {
            // 앵커 정점에 곡률이 없고(스쿱 모서리) 같은 노드에 정점이 여럿이면,
            // 차감 시작 지점을 정점 회전으로 구현한다(#모서리 시작 수정5 — 세 지점):
            // 위 = T(가로선 접점), 꼭짓점 = N(모서리, 파란 원), 아래 = O(오버슛 끝).
            // over(오목 곡선) 정점이 시작점이 되는 경우는 닫힘에서 곡선을 복원한다.
            if vs[0].r < 0.01, anchorVertCount >= 3, vs.count >= 3 {
                let idxs = Array(0..<anchorVertCount)
                // O = 다른 정점들과 y가 가장 다른 정점(오버슛), N = 화면 가장자리에 가장
                // 가까운(중앙에서 먼 x) 나머지 정점, T = 남는 정점(가로선 위 접점).
                let oIdx = idxs.max { a, b in
                    let da = idxs.reduce(CGFloat(0)) { $0 + abs(vs[a].pt.y - vs[$1].pt.y) }
                    let db = idxs.reduce(CGFloat(0)) { $0 + abs(vs[b].pt.y - vs[$1].pt.y) }
                    return da < db
                } ?? 0
                let rest = idxs.filter { $0 != oIdx }
                let nIdx = rest.max {
                    abs(vs[$0].pt.x - rect.midX) < abs(vs[$1].pt.x - rect.midX)
                } ?? 0
                let tIdx = rest.first { $0 != nIdx } ?? 0
                let pick: Int
                switch anchorSide {
                case .left: pick = tIdx
                case .center: pick = nIdx
                case .right: pick = oIdx
                }
                if pick > 0 {
                    vs = Array(vs[pick...] + vs[..<pick])
                    ew = Array(ew[pick...] + ew[..<pick])
                    en = Array(en[pick...] + en[..<pick])
                    av = Array(av[pick...] + av[..<pick])
                }
            }
            let n = vs.count
            guard n >= 3 else { return p }
            let (t1, t2) = tangents(vs)
            /// 정점 i의 곡선(모서리) 굵기: 양옆 구간 중 굵은 쪽 + 노드 분할 상한.
            func aw(_ i: Int) -> CGFloat {
                min(max(ew[(i - 1 + n) % n], ew[i]), av[i] ?? CGFloat.greatestFiniteMagnitude)
            }
            let C0 = vs[0].pt
            // 차감 시작 지점(#위/아래 매핑): 시작 모서리 곡선의 두 접점 중 위(작은 y) 또는
            // 아래에서 시작한다. 순회 방향과 무관하게 y좌표로 판정해 항상 정확하다.
            // (.left = 위, .right = 아래. 구버전 .center는 위로 취급.)
            let wantDown = (anchorSide == .right)
            let t1IsUp = t1[0].y <= t2[0].y
            let startAtT1 = (t1IsUp != wantDown)
            if startAtT1 {
                mv(t1[0]); qd(t2[0], C0, aw(0))
            } else {
                mv(t2[0])
            }
            for i in 1..<n {
                if let ov = vs[i].over {
                    // 스쿱 정점: 직전 점에서 명시적 제어점으로 오목 곡선(#모서리 모양 2).
                    qd(vs[i].pt, ov, ew[i - 1], en[i - 1])
                } else {
                    ln(t1[i], ew[i - 1], en[i - 1])
                    cornerArc(i, t1, t2, ew[i - 1], ew[i], en[i - 1], en[i])
                }
            }
            if let ov0 = vs[0].over {
                // 시작 정점이 오목(스쿱) 곡선의 종점이면 seam을 그 곡선으로 닫는다.
                qd(vs[0].pt, ov0, ew[n - 1], en[n - 1])
            } else {
                ln(t1[0], ew[n - 1], en[n - 1])
                if !startAtT1 { qd(t2[0], C0, aw(0)) }
            }
            p.closeSubpath()
        } else {
            let n = vs.count
            guard n >= 2 else { return p }
            let (t1, t2) = tangents(vs)
            func aw(_ i: Int) -> CGFloat { max(ew[max(0, i - 1)], ew[min(i, n - 1)]) }
            // 반쪽 아크(#모서리 반씩 양분): 캡 모서리 곡선을 중간점까지만 그린다.
            func arcMid(_ i: Int) -> CGPoint {
                let C = vs[i].pt
                return CGPoint(x: 0.25 * t1[i].x + 0.5 * C.x + 0.25 * t2[i].x,
                               y: 0.25 * t1[i].y + 0.5 * C.y + 0.25 * t2[i].y)
            }
            let halfAtStart = sHalf && startHasCap && n >= 3
            let halfAtEnd = eHalf && endHasCap && n >= 3
            var from = 1
            if halfAtStart {
                let C = vs[1].pt
                mv(arcMid(1))
                qd(t2[1], CGPoint(x: (C.x + t2[1].x) / 2, y: (C.y + t2[1].y) / 2), aw(1))
                from = 2
            } else {
                mv(vs[0].pt)
            }
            let to = halfAtEnd ? n - 2 : n - 1
            for i in from..<to {
                if let ov = vs[i].over {
                    qd(vs[i].pt, ov, ew[i - 1], en[i - 1])
                } else {
                    ln(t1[i], ew[i - 1], en[i - 1])
                    cornerArc(i, t1, t2, ew[i - 1], ew[min(i, n - 1)], en[i - 1], en[min(i, n - 1)])
                }
            }
            if halfAtEnd {
                let i = n - 2
                let C = vs[i].pt
                ln(t1[i], ew[i - 1])
                qd(arcMid(i), CGPoint(x: (t1[i].x + C.x) / 2, y: (t1[i].y + C.y) / 2), aw(i))
            } else {
                ln(vs[n - 1].pt, ew[n - 2], en[n - 2])
            }
        }
        return p
    }
}

/// 화면 가장자리를 따라 각 AI의 잔여율만큼 색 띠를 겹쳐 그리는 오버레이 뷰.
///
/// 겹침 규칙: ① 완전히 같은 모양끼리는 잔여율 높은 것부터(뒤) 그려 %가 가장 적은
/// 급한 띠가 최상단에 오게 한다. ② 모양이 다른데 일부 세그먼트가 겹치면 그 구간만
/// 굵기를 공유자 수로 나눠 나란히 표시한다. ③ '겹침 구간 굵기 분할' 옵션을 켜면
/// 같은 모양끼리도 ②처럼 나눠 표시한다. 노치 우회는 항상 원굵기 한 줄이다.
struct BorderView: View {
    @ObservedObject var settings: OverlaySettings
    @ObservedObject var manager: ProviderManager

    /// 한 AI의 그리기 항목: 모양(레인·구간 굵기 포함) + 잔여율 + 색 + 분할 여부.
    private struct DrawItem: Identifiable {
        let id: String
        let shape: SegmentChainShape
        let ratio: Double
        let color: Color
        let split: Bool          // 구간 굵기(widthSpans) 사용 여부
    }

    private func makeShape(for id: String, laneOffsets: [SegPart: CGFloat],
                           laneWidths: [SegPart: CGFloat], nodeWidths: [SegNode: CGFloat],
                           notchDetour: Bool) -> SegmentChainShape {
        let l = settings.layout(for: id)
        let segs = settings.segs(for: id)
        var aHalf = false, aAuto = false, bHalf = false, bAuto = false
        if let ends = OverlaySettings.chainEnds(segs, menuOn: settings.menuLineEnabled,
                                                dockOn: settings.dockLineEnabled) {
            (aHalf, aAuto) = settings.endOverride(for: id, node: ends.start.node, seg: ends.start.seg)
            (bHalf, bAuto) = settings.endOverride(for: id, node: ends.end.node, seg: ends.end.seg)
        }
        return SegmentChainShape(
            segments: segs,
            menuOn: settings.menuLineEnabled, dockOn: settings.dockLineEnabled,
            menuH: settings.menuLineHeight, dockH: settings.dockLineHeight,
            inset: settings.thickness / 2,
            menuRadii: settings.menuZoneRadii, mainRadii: settings.cornerRadii,
            dockRadii: settings.dockZoneRadii,
            notchEnabled: settings.notchEnabled, notchWidth: settings.notchWidth,
            notchHeight: settings.notchHeight, notchRadii: settings.notchRadii,
            anchorCorner: l.anchorCorner, anchorSide: l.anchorSide, clockwise: l.clockwise,
            hCaps: l.hCaps ?? [:],
            endAHalf: aHalf, endAAuto: aAuto, endBHalf: bHalf, endBAuto: bAuto,
            laneOffsets: laneOffsets, laneWidths: laneWidths,
            fullWidth: settings.thickness, notchDetour: notchDetour,
            nodeWidths: nodeWidths
        )
    }

    /// 그릴 AI 목록(뒤→앞 순서). 겹침 분할 레인/굵기를 여기서 계산한다.
    private var drawItems: [DrawItem] {
        let t = settings.thickness
        let active = manager.active
        // 세그먼트별 공유자 목록(활성 AI만, 스펙 순서로 고정 → 레인 배정 결정적).
        var sharers: [SegPart: [String]] = [:]
        for spec in ProviderSpec.all where active.contains(where: { $0.spec.id == spec.id }) {
            for s in settings.segs(for: spec.id) { sharers[s, default: []].append(spec.id) }
        }
        // 상단선이 분할될 때 노치 우회 소유자: 상단선 공유자 중 잔여율 최소(최상단 레이어).
        let hTopSharers = active.filter { settings.segs(for: $0.spec.id).contains(.hTop) }
        let notchOwner = hTopSharers.min { $0.snap.remainingRatio < $1.snap.remainingRatio }?.spec.id

        // 노드(모서리)별로 몇 개의 AI가 지나가는지(#모서리 겹침) — 스쿱 오버슛·아크 분할용.
        let menuOn = settings.menuLineEnabled, dockOn = settings.dockLineEnabled
        var nodeAIs: [SegNode: Set<String>] = [:]
        for spec in ProviderSpec.all where active.contains(where: { $0.spec.id == spec.id }) {
            for s in settings.segs(for: spec.id) {
                let (a, b) = OverlaySettings.segEnds(s, menuOn: menuOn, dockOn: dockOn)
                nodeAIs[a, default: []].insert(spec.id)
                nodeAIs[b, default: []].insert(spec.id)
            }
        }
        var nodeWidthMap: [SegNode: CGFloat] = [:]
        for (n, ids) in nodeAIs where ids.count > 1 { nodeWidthMap[n] = t / CGFloat(ids.count) }

        // 잔여율 높은 띠부터(뒤) → %가 가장 적은 띠가 최상단(#규칙1).
        let ordered = active.sorted { $0.snap.remainingRatio > $1.snap.remainingRatio }
        var out: [DrawItem] = []
        for item in ordered {
            let myId = item.spec.id
            let mySegs = settings.segs(for: myId)
            var lanes: [SegPart: CGFloat] = [:]
            var widths: [SegPart: CGFloat] = [:]
            for s in mySegs {
                let sh = sharers[s] ?? [myId]
                let k = sh.count
                // 분할 조건(#규칙2,3): 다른 모양과 부분 겹침이거나, 분할 옵션이 켜져 있으면.
                let differs = sh.contains { settings.segs(for: $0) != mySegs }
                if k > 1, differs || settings.splitOverlapLines, let i = sh.firstIndex(of: myId) {
                    lanes[s] = (CGFloat(i) - CGFloat(k - 1) / 2) * (t / CGFloat(k))
                    widths[s] = t / CGFloat(k)
                }
            }
            let hTopSplit = lanes[.hTop] != nil
            // 노드 분할 굵기는 겹침 분할이 실제로 동작 중일 때만(내가 분할에 참여할 때) 적용.
            let myNodeWidths = lanes.isEmpty ? [:] : nodeWidthMap
            let sh = makeShape(for: myId, laneOffsets: lanes, laneWidths: widths,
                               nodeWidths: myNodeWidths,
                               notchDetour: !hTopSplit || myId == notchOwner)
            out.append(DrawItem(id: myId, shape: sh, ratio: item.snap.remainingRatio,
                                color: settings.color(forProvider: myId),
                                split: !lanes.isEmpty || !myNodeWidths.isEmpty))
        }
        return out
    }

    private let fadeSteps = 12

    var body: some View {
        let items = drawItems
        let op = settings.lineOpacity
        let t = settings.thickness
        let showTrack = settings.showTrack
        let fadeOn = settings.fadeEnabled
        let fadeFrac = settings.fadeFraction
        return Canvas { context, size in
            let rect = CGRect(origin: .zero, size: size)
            context.opacity = op
            // AI별 (경로, 굵기 구간) 준비. 노치 우회가 있으면 분할이 없어도 구간을 계산해
            // 노치를 별도 레이어(가장 뒤)로 분리한다(#노치 레이어 뒤).
            typealias WSpan = (from: CGFloat, to: CGFloat, width: CGFloat, notch: Bool)
            var prepared: [(item: DrawItem, path: Path, spans: [WSpan])] = []
            for item in items {
                let path = item.shape.path(in: rect)
                var spans: [WSpan] = [(0, 1, t, false)]
                let hasNotch = item.shape.notchDetour && item.shape.notchEnabled
                    && item.shape.segments.contains(.hTop)
                if item.split || hasNotch {
                    let m = item.shape.widthSpans(in: rect)
                    if !m.spans.isEmpty {
                        spans = Self.tapered(m.spans, total: m.total, thickness: t)
                    }
                }
                prepared.append((item, path, spans))
            }
            // 트랙 → 본체 → 그라데이션을 노치(뒤)·본체(앞) 두 레이어로 나눠 그린다.
            func draw(_ item: DrawItem, _ path: Path, _ spans: [WSpan], notchLayer: Bool) {
                // roundAll: 불투명 동일색(본체)은 모든 조각 끝을 둥글게(이음매 무해, 끝이 자연스러움).
                // 그 외(트랙·페이드)는 반투명 겹침 얼룩을 피해 butt 유지(#선 끝 둥글게).
                func strokeRange(_ a: CGFloat, _ b: CGFloat, _ color: Color, roundAll: Bool = false) {
                    guard b > a else { return }
                    for sp in spans where sp.notch == notchLayer {
                        let lo = max(a, sp.from), hi = min(b, sp.to)
                        // 테이퍼 슬라이스(~0.3pt)도 그려야 하므로 아주 미세한 것만 거른다(#끊김 수정).
                        guard hi > lo + 0.000001 else { continue }
                        let cap: CGLineCap = roundAll ? .round : .butt
                        context.stroke(path.trimmedPath(from: lo, to: hi), with: .color(color),
                                       style: StrokeStyle(lineWidth: sp.width, lineCap: cap, lineJoin: .round))
                    }
                }
                if showTrack { strokeRange(0, 1, item.color.opacity(0.18)) }
                let r = CGFloat(max(0, min(1, item.ratio)))
                let fade = fadeOn ? min(fadeFrac, r) : 0
                let solid = max(0, r - fade)
                strokeRange(0, solid, item.color, roundAll: true)
                if fade > 0 {
                    for i in 0..<fadeSteps {
                        let a = solid + fade * CGFloat(i) / CGFloat(fadeSteps)
                        let b = solid + fade * CGFloat(i + 1) / CGFloat(fadeSteps)
                        let opStep = 1 - Double(i + 1) / Double(fadeSteps)
                        strokeRange(a, b, item.color.opacity(opStep))
                    }
                }
            }
            // 1) 노치 우회 — 모든 선보다 뒤(메뉴바선 등과 겹치면 뒤에 깔린다).
            for pr in prepared { draw(pr.item, pr.path, pr.spans, notchLayer: true) }
            // 2) 본체.
            for pr in prepared { draw(pr.item, pr.path, pr.spans, notchLayer: false) }
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
    }

    /// 경로 전체를 균일 슬라이스로 리샘플한 뒤 **가우시안 평활**로 굵기를 흐르듯 섞는다
    /// (#인지 불가 전환). 경계별 테이퍼와 달리 곡률 지점처럼 짧은 구간이 껴 있어도
    /// 전환 길이(≈600pt+)가 줄지 않는다. 노치 슬라이스는 매 패스마다 원굵기로 고정(pin)해
    /// 노치는 정확히 원굵기를 유지하고 그 옆으로 긴 램프가 자연스럽게 생긴다.
    private static func tapered(_ spans: [(from: CGFloat, to: CGFloat, width: CGFloat, notch: Bool)],
                                total: CGFloat, thickness: CGFloat)
        -> [(from: CGFloat, to: CGFloat, width: CGFloat, notch: Bool)] {
        guard spans.count >= 2, total > 0 else { return spans }
        // 6pt 균일 슬라이스로 리샘플.
        let slicePts: CGFloat = 6
        let n = max(48, min(2048, Int(total / slicePts)))
        var w = [CGFloat](repeating: 0, count: n)
        var pin = [Bool](repeating: false, count: n)     // 노치 = 고정
        var isNotch = [Bool](repeating: false, count: n)
        var si = 0
        for i in 0..<n {
            let f = (CGFloat(i) + 0.5) / CGFloat(n)
            while si < spans.count - 1 && f > spans[si].to { si += 1 }
            w[i] = spans[si].width
            isNotch[i] = spans[si].notch
            pin[i] = spans[si].notch
        }
        let pinned = w
        // 원형(고리) 박스 블러 3패스 ≈ 가우시안. 반경 = 전환 길이의 절반.
        let taperPts = max(600, thickness * 140)
        let radius = max(1, min(n / 2 - 1, Int((taperPts / 2) / (total / CGFloat(n)))))
        for _ in 0..<3 {
            // 3배 패딩 프리픽스(고리 순환) — 창이 배열을 벗어나지 않게.
            var pre = [CGFloat](repeating: 0, count: 3 * n + 1)
            for i in 0..<(3 * n) { pre[i + 1] = pre[i] + w[i % n] }
            var out = w
            for i in 0..<n {
                let lo = i + n - radius, hi = i + n + radius
                out[i] = (pre[hi + 1] - pre[lo]) / CGFloat(2 * radius + 1)
            }
            w = out
            for i in 0..<n where pin[i] { w[i] = pinned[i] }   // 노치 원굵기 고정
        }
        // 비슷한 굵기의 인접 슬라이스 병합 → 스팬 목록.
        var out: [(CGFloat, CGFloat, CGFloat, Bool)] = []
        for i in 0..<n {
            let a = CGFloat(i) / CGFloat(n), b = CGFloat(i + 1) / CGFloat(n)
            if var last = out.last, abs(last.2 - w[i]) < 0.02, last.3 == isNotch[i] {
                last.1 = b
                out[out.count - 1] = last
            } else {
                out.append((a, b, w[i], isNotch[i]))
            }
        }
        if var last = out.last { last.1 = 1; out[out.count - 1] = last }
        return out.map { (from: $0.0, to: $0.1, width: $0.2, notch: $0.3) }
    }
}