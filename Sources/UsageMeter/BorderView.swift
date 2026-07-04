import SwiftUI

/// 선택된 변(면)들을 **하나의 연속 경로**로 그리는 테두리 모양.
///
/// - 정점마다 곡률을 주는 일반화 방식 → 본체 모서리·노치 모서리를 똑같이 다룬다.
/// - 한쪽 변만 선택돼도 그 끝의 둥근 모서리(곡률)를 포함해 그린다(#2).
/// - 앵커(anchor)로 잔여율 띠의 시작 꼭짓점·방향·곡률 좌/우 지점을 고른다(#6).
/// - 상단 노치가 켜져 있으면 상변이 노치를 아래로 감싸며 우회한다.
struct BorderShape: Shape {
    let edges: Set<BorderEdge>
    let radii: [BorderCorner: CGFloat]
    let inset: CGFloat
    let notchEnabled: Bool
    let notchWidth: CGFloat
    let notchHeight: CGFloat
    let notchRadii: [NotchCorner: CGFloat]
    let anchorCorner: BorderCorner
    let anchorSide: AnchorSide
    let anchorClockwise: Bool
    let extendStartCorner: Bool
    let extendEndCorner: Bool
    let startCornerAbove: Bool

    private static let cw: [BorderEdge] = [.top, .right, .bottom, .left]
    private func cwSuccessor(_ e: BorderEdge) -> BorderEdge { Self.cw[(Self.cw.firstIndex(of: e)! + 1) % 4] }
    private func ccwPred(_ e: BorderEdge) -> BorderEdge { Self.cw[(Self.cw.firstIndex(of: e)! + 3) % 4] }

    func path(in rect: CGRect) -> Path {
        var p = Path()
        let R = rect.insetBy(dx: inset, dy: inset)
        guard R.width > 0, R.height > 0, !edges.isEmpty else { return p }
        let x = R.minX, y = R.minY, w = R.width, h = R.height

        let cornerPt: [BorderCorner: CGPoint] = [
            .topLeft: CGPoint(x: x, y: y), .topRight: CGPoint(x: x + w, y: y),
            .bottomRight: CGPoint(x: x + w, y: y + h), .bottomLeft: CGPoint(x: x, y: y + h),
        ]
        func mainRadius(_ c: BorderCorner) -> CGFloat { max(0, radii[c] ?? 0) }
        func startCorner(_ e: BorderEdge) -> BorderCorner {
            switch e { case .top: return .topLeft; case .right: return .topRight
                       case .bottom: return .bottomRight; case .left: return .bottomLeft }
        }
        func endCorner(_ e: BorderEdge) -> BorderCorner {
            switch e { case .top: return .topRight; case .right: return .bottomRight
                       case .bottom: return .bottomLeft; case .left: return .topLeft }
        }
        // 꼭짓점 corner에서 변 edge를 따라 곡률 r만큼 떨어진 접점.
        func tangent(_ corner: BorderCorner, along edge: BorderEdge) -> CGPoint {
            let r = mainRadius(corner)
            switch (corner, edge) {
            case (.topLeft, .top): return CGPoint(x: x + r, y: y)
            case (.topLeft, .left): return CGPoint(x: x, y: y + r)
            case (.topRight, .top): return CGPoint(x: x + w - r, y: y)
            case (.topRight, .right): return CGPoint(x: x + w, y: y + r)
            case (.bottomRight, .right): return CGPoint(x: x + w, y: y + h - r)
            case (.bottomRight, .bottom): return CGPoint(x: x + w - r, y: y + h)
            case (.bottomLeft, .bottom): return CGPoint(x: x + r, y: y + h)
            case (.bottomLeft, .left): return CGPoint(x: x, y: y + h - r)
            default: return cornerPt[corner]!
            }
        }

        func notchVertices() -> [(CGPoint, CGFloat)] {
            guard notchEnabled, notchWidth > 0, notchHeight > 0 else { return [] }
            let cx = R.midX
            let nl = max(cx - notchWidth / 2, x + mainRadius(.topLeft))
            let nr = min(cx + notchWidth / 2, x + w - mainRadius(.topRight))
            guard nr > nl else { return [] }
            func nrr(_ c: NotchCorner) -> CGFloat { max(0, notchRadii[c] ?? 0) }
            return [
                (CGPoint(x: nl, y: y), nrr(.outerLeft)),
                (CGPoint(x: nl, y: y + notchHeight), nrr(.innerLeft)),
                (CGPoint(x: nr, y: y + notchHeight), nrr(.innerRight)),
                (CGPoint(x: nr, y: y), nrr(.outerRight)),
            ]
        }

        // 한 run의 정점((점, 곡률)). closed=고리. open은 양끝이 끝점(직각)이되,
        // 곡률>0인 끝 모서리는 이웃 변 접점까지 곡선을 포함한다(#2).
        func runVertices(_ run: [BorderEdge], closed: Bool) -> [(pt: CGPoint, r: CGFloat)] {
            var vs: [(CGPoint, CGFloat)] = []
            // 곡률 옵션을 '앵커(=% 차감 시작) 코너'에 묶는다. run의 두 끝 중 앵커인 쪽이
            // extendStartCorner(=% 차감 시작쪽), 반대쪽이 extendEndCorner(=반대쪽 끝)를 쓴다.
            let sC0 = startCorner(run.first!), eC0 = endCorner(run.last!)
            let sIsAnchor = (sC0 == anchorCorner)
            let eIsAnchor = (eC0 == anchorCorner)
            let sExtend = sIsAnchor ? extendStartCorner : extendEndCorner
            let eExtend = eIsAnchor ? extendStartCorner : extendEndCorner
            let sAbove = sIsAnchor ? startCornerAbove : false
            let eAbove = eIsAnchor ? startCornerAbove : false

            if !closed {
                let e0 = run[0], sC = startCorner(e0), sr = mainRadius(sC)
                if sExtend, sr > 0.01 {
                    if sAbove {
                        vs.append((tangent(sC, along: e0), 0))            // 위: 선택 변 위에서 시작
                    } else {
                        vs.append((tangent(sC, along: ccwPred(e0)), 0))   // 아래: 이웃 변으로 곡선 뻗음
                        vs.append((cornerPt[sC]!, sr))
                    }
                } else {
                    vs.append((cornerPt[sC]!, 0))                         // 직각(꼭짓점)
                }
            }
            for (i, e) in run.enumerated() {
                if e == .top { vs.append(contentsOf: notchVertices()) }
                let isLast = i == run.count - 1
                if closed || !isLast {
                    let c = endCorner(e)
                    vs.append((cornerPt[c]!, mainRadius(c)))
                }
            }
            if !closed {
                let em = run.last!, eC = endCorner(em), er = mainRadius(eC)
                if eExtend, er > 0.01 {
                    if eAbove {
                        vs.append((tangent(eC, along: em), 0))           // 위: 선택 변 위에서 끝
                    } else {
                        vs.append((cornerPt[eC]!, er))                   // 아래: 이웃 변으로 곡선 뻗음
                        vs.append((tangent(eC, along: cwSuccessor(em)), 0))
                    }
                } else {
                    vs.append((cornerPt[eC]!, 0))
                }
            }
            return vs.map { (pt: $0.0, r: $0.1) }
        }

        func unit(_ a: CGPoint, _ b: CGPoint) -> CGPoint {
            let dx = b.x - a.x, dy = b.y - a.y
            let len = max(0.0001, (dx * dx + dy * dy).squareRoot())
            return CGPoint(x: dx / len, y: dy / len)
        }
        func dist(_ a: CGPoint, _ b: CGPoint) -> CGFloat {
            ((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)).squareRoot()
        }
        // 정점별 접점 두 개(이전/다음 방향) 계산.
        func tangents(_ vs: [(pt: CGPoint, r: CGFloat)]) -> (t1: [CGPoint], t2: [CGPoint]) {
            let n = vs.count
            var t1 = [CGPoint](repeating: .zero, count: n), t2 = t1
            for i in 0..<n {
                let cur = vs[i].pt
                let prevV = vs[(i - 1 + n) % n], nextV = vs[(i + 1) % n]
                // 이웃도 둥글면 절반까지만(겹침 방지), 이웃이 직각 끝점이면 전체 거리까지 허용.
                // → 부분 선택의 추가 곡률 모서리도 설정한 곡률 그대로 나온다(반값 버그 수정).
                let maxPrev = (prevV.r > 0.01 ? 0.5 : 1.0) * dist(cur, prevV.pt)
                let maxNext = (nextV.r > 0.01 ? 0.5 : 1.0) * dist(cur, nextV.pt)
                let r = max(0, min(vs[i].r, maxPrev, maxNext))
                t1[i] = CGPoint(x: cur.x + unit(cur, prevV.pt).x * r, y: cur.y + unit(cur, prevV.pt).y * r)
                t2[i] = CGPoint(x: cur.x + unit(cur, nextV.pt).x * r, y: cur.y + unit(cur, nextV.pt).y * r)
            }
            return (t1, t2)
        }

        func addOpen(_ vs: [(pt: CGPoint, r: CGFloat)]) {
            let n = vs.count; guard n >= 2 else { return }
            let (t1, t2) = tangents(vs)
            p.move(to: vs[0].pt)
            for i in 1..<(n - 1) { p.addLine(to: t1[i]); p.addQuadCurve(to: t2[i], control: vs[i].pt) }
            p.addLine(to: vs[n - 1].pt)
        }

        // 고리를 앵커(시작 정점·방향·좌/우)에서 시작하도록 그린다.
        func addClosed(_ vsIn: [(pt: CGPoint, r: CGFloat)]) {
            var vs = vsIn
            let n = vs.count; guard n >= 3 else { return }
            // 앵커 꼭짓점으로 순서 재배열(+방향).
            if let ai = vs.firstIndex(where: { dist($0.pt, cornerPt[anchorCorner]!) < 0.5 }) {
                // 정점 목록은 시계 반대로 쌓여 있으므로, 시계방향이면 역순으로 순회한다.
                let order = (0..<n).map { anchorClockwise ? ((ai - $0) % n + n) % n : (ai + $0) % n }
                vs = order.map { vs[$0] }
            }
            let (t1, t2) = tangents(vs)
            switch anchorSide {
            case .right:  p.move(to: t2[0])
            case .center: p.move(to: vs[0].pt); p.addQuadCurve(to: t2[0], control: vs[0].pt)
            case .left:   p.move(to: t1[0]); p.addQuadCurve(to: t2[0], control: vs[0].pt)
            }
            for i in 1..<n { p.addLine(to: t1[i]); p.addQuadCurve(to: t2[i], control: vs[i].pt) }
            p.addLine(to: t1[0])
            switch anchorSide {
            case .right:  p.addQuadCurve(to: t2[0], control: vs[0].pt)
            case .center: p.addQuadCurve(to: vs[0].pt, control: vs[0].pt)
            case .left:   break   // 이미 t1[0]로 닫힘
            }
            p.closeSubpath()
        }

        let sel = Self.cw.filter { edges.contains($0) }
        if sel.count == 4 { addClosed(runVertices(Self.cw, closed: true)); return p }

        guard let gap = Self.cw.firstIndex(where: { !edges.contains($0) }) else { return p }
        let ordered = (0..<4).map { Self.cw[(gap + 1 + $0) % 4] }.filter { edges.contains($0) }
        var runs: [[BorderEdge]] = []
        for e in ordered {
            if let last = runs.last?.last, cwSuccessor(last) == e { runs[runs.count - 1].append(e) }
            else { runs.append([e]) }
        }
        for run in runs {
            var vs = runVertices(run, closed: false)
            // 앵커 꼭짓점이 경로의 '끝'이 되도록 맞춘다 → 닫힌 루프와 동일하게
            // %가 앵커(= % 차감 시작 위치)에서부터 줄어든다.
            if anchorCorner == startCorner(run.first!) { vs = Array(vs.reversed()) }
            addOpen(vs)
        }
        return p
    }
}

/// 화면 가장자리를 따라 각 AI의 잔여율만큼 색 띠를 겹쳐 그리는 오버레이 뷰.
/// 같은 레일에 여러 AI를 겹치고, **잔여율 높은 띠는 뒤·낮은(급한) 띠는 앞**에 둔다.
struct BorderView: View {
    @ObservedObject var settings: OverlaySettings
    @ObservedObject var manager: ProviderManager

    /// 주어진 변·앵커·선끝마감으로 BorderShape 생성(굵기·모서리·노치는 공용).
    private func makeShape(edges: Set<BorderEdge>, anchorCorner: BorderCorner,
                          anchorSide: AnchorSide, clockwise: Bool,
                          extendStart: Bool, extendEnd: Bool, noCurve: Bool) -> BorderShape {
        BorderShape(
            edges: edges, radii: settings.cornerRadii, inset: settings.thickness / 2,
            notchEnabled: settings.notchEnabled, notchWidth: settings.notchWidth,
            notchHeight: settings.notchHeight, notchRadii: settings.notchRadii,
            anchorCorner: anchorCorner, anchorSide: anchorSide, anchorClockwise: clockwise,
            extendStartCorner: extendStart, extendEndCorner: extendEnd, startCornerAbove: noCurve
        )
    }

    private var globalShape: BorderShape {
        makeShape(edges: settings.edges, anchorCorner: settings.anchorCorner,
                  anchorSide: settings.anchorSide, clockwise: settings.anchorClockwise,
                  extendStart: settings.extendStartCorner, extendEnd: settings.extendEndCorner,
                  noCurve: settings.startCornerAbove)
    }

    private var style: StrokeStyle {
        StrokeStyle(lineWidth: settings.thickness, lineCap: .butt, lineJoin: .round)
    }

    private let fadeSteps = 12

    /// 그릴 스트로크 조각 하나(트랙/본체/페이드 모두 동일 구조로 평탄화).
    /// 동적/상수 ForEach를 중첩하지 않고 하나의 ForEach로 그려 렌더링 버그를 원천 차단한다.
    private struct Seg: Identifiable {
        let id: Int
        let shape: BorderShape
        let from: CGFloat
        let to: CGFloat
        let color: Color
    }

    /// 그릴 모든 조각을 뒤→앞 순서의 평평한 배열로 만든다.
    private var segments: [Seg] {
        var segs: [Seg] = []
        var idx = 0
        func add(shape: BorderShape, ratio: Double, color: Color, trackColor: Color) {
            // 트랙(옅은 전체 길이).
            if settings.showTrack {
                segs.append(Seg(id: idx, shape: shape, from: 0, to: 1, color: trackColor)); idx += 1
            }
            let r = CGFloat(max(0, min(1, ratio)))
            let fade = settings.fadeEnabled ? min(settings.fadeFraction, r) : 0
            let solid = max(0, r - fade)
            // 채운 본체.
            segs.append(Seg(id: idx, shape: shape, from: 0, to: solid, color: color)); idx += 1
            // 끝부분 투명 그라데이션(여러 조각).
            if fade > 0 {
                for i in 0..<fadeSteps {
                    let a = solid + fade * CGFloat(i) / CGFloat(fadeSteps)
                    let b = solid + fade * CGFloat(i + 1) / CGFloat(fadeSteps)
                    let op = 1 - Double(i + 1) / Double(fadeSteps)
                    segs.append(Seg(id: idx, shape: shape, from: a, to: b, color: color.opacity(op))); idx += 1
                }
            }
        }

        if settings.separateByAI {
            for item in manager.active {
                let l = settings.layout(for: item.spec.id)
                let sh = makeShape(edges: l.edges, anchorCorner: l.anchorCorner,
                                   anchorSide: l.anchorSide, clockwise: l.clockwise,
                                   extendStart: l.xStart, extendEnd: l.xEnd, noCurve: l.xNoCurve)
                let c = settings.color(forProvider: item.spec.id)
                add(shape: sh, ratio: item.snap.remainingRatio, color: c, trackColor: c.opacity(0.18))
            }
        } else {
            let sh = globalShape
            // 잔여율 높은 것부터(뒤), 낮은(급한) 게 앞.
            let ordered = manager.active.sorted { $0.snap.remainingRatio > $1.snap.remainingRatio }
            for item in ordered {
                add(shape: sh, ratio: item.snap.remainingRatio,
                    color: settings.color(forProvider: item.spec.id), trackColor: settings.color.opacity(0.18))
            }
        }
        return segs
    }

    var body: some View {
        // Canvas로 각 조각을 직접 그린다. SwiftUI Shape/ForEach 조합의 렌더링 버그를
        // 근본적으로 피하고, 조각 수·순서에 상관없이 항상 안정적으로 그려진다.
        let segs = segments
        let op = settings.lineOpacity
        let st = style
        return Canvas { context, size in
            let rect = CGRect(origin: .zero, size: size)
            context.opacity = op
            for seg in segs {
                let path = seg.shape.path(in: rect).trimmedPath(from: seg.from, to: seg.to)
                context.stroke(path, with: .color(seg.color), style: st)
            }
        }
        .ignoresSafeArea()
        .allowsHitTesting(false)
    }
}
