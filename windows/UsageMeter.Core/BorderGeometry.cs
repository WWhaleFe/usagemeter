namespace UsageMeter.Core;

/// <summary>테두리 생성 입력 — macOS SegmentChainShape 파라미터 이식.
/// Windows에는 노치가 없으므로 노치 관련은 제외(가이드 결정).</summary>
public sealed class BorderConfig
{
    public required HashSet<SegPart> Segments { get; init; }
    public bool TaskOn { get; init; }
    public double TaskH { get; init; } = 48;
    public double Inset { get; init; } = 1;

    public Dictionary<BorderCorner, double> MainRadii { get; init; } = new();
    public Dictionary<BorderCorner, double> TaskRadii { get; init; } = new();

    public BorderCorner AnchorCorner { get; init; } = BorderCorner.TopLeft;
    public AnchorSide AnchorSide { get; init; } = AnchorSide.Left;
    public bool Clockwise { get; init; } = true;

    /// <summary>가로선 좌/우 끝의 위/아래 둥글게. 키 = SegGraph.HCapKey.</summary>
    public Dictionary<string, SegCap> HCaps { get; init; } = new();

    // 교차 AI 반쪽 아크 보정(정렬된 첫/마지막 끝).
    public bool EndAHalf { get; init; }
    public bool EndAAuto { get; init; }
    public bool EndBHalf { get; init; }
    public bool EndBAuto { get; init; }

    /// <summary>겹침 분할 레인 오프셋(가로=Δy, 세로=Δx) / 구간 굵기(t/N).</summary>
    public Dictionary<SegPart, double> LaneOffsets { get; init; } = new();
    public Dictionary<SegPart, double> LaneWidths { get; init; } = new();
    public double FullWidth { get; init; } = 2;

    /// <summary>노드(모서리)별 분할 굵기 — 여러 AI가 지나는 모서리는 t/k.</summary>
    public Dictionary<SegNode, double> NodeWidths { get; init; } = new();
}

public readonly record struct PathSample(double X, double Y, double Len);
public readonly record struct WidthSpan(double From, double To, double Width);   // From/To = 트림 비율(0~1)

/// <summary>생성된 테두리 경로: 평탄화 폴리라인(누적 길이) + 굵기 구간.</summary>
public sealed class BorderPath
{
    public required List<PathSample> Samples { get; init; }
    public required List<WidthSpan> Spans { get; init; }
    public required double Total { get; init; }
    public required bool IsLoop { get; init; }

    /// <summary>트림 비율 [a,b] 구간의 부분 폴리라인 좌표(경계는 보간).</summary>
    public List<(double X, double Y)> Extract(double a, double b)
    {
        var pts = new List<(double, double)>();
        if (Samples.Count < 2 || Total <= 0 || b <= a) return pts;
        double la = a * Total, lb = b * Total;
        for (int i = 0; i < Samples.Count - 1; i++)
        {
            var s0 = Samples[i];
            var s1 = Samples[i + 1];
            if (s1.Len < la || s0.Len > lb) continue;
            double segLen = s1.Len - s0.Len;
            double t0 = segLen <= 0 ? 0 : Math.Clamp((la - s0.Len) / segLen, 0, 1);
            double t1 = segLen <= 0 ? 1 : Math.Clamp((lb - s0.Len) / segLen, 0, 1);
            var p0 = (s0.X + (s1.X - s0.X) * t0, s0.Y + (s1.Y - s0.Y) * t0);
            var p1 = (s0.X + (s1.X - s0.X) * t1, s0.Y + (s1.Y - s0.Y) * t1);
            if (pts.Count == 0) pts.Add(p0);
            pts.Add(p1);
        }
        return pts;
    }
}

/// <summary>세그먼트 그래프 → 둥근 폴리라인 경로. macOS SegmentChainShape.build() 이식.</summary>
public static class BorderGeometry
{
    private const int QuadSamples = 14;   // 2차 베지어 평탄화 표본 수

    public static BorderPath? Build(BorderConfig c, double width, double height)
    {
        var segs = c.Segments.Where(s => SegGraph.Available(s, c.TaskOn)).ToHashSet();
        if (segs.Count == 0 || !SegGraph.IsValid(segs, c.TaskOn)) return null;

        double xL = c.Inset, xR = width - c.Inset;
        double YOf(int level) => level switch
        {
            0 => c.Inset,
            1 => height - c.TaskH,
            _ => height - c.Inset,
        };
        (double X, double Y) Pt(SegNode n)
        {
            double dx = 0, dy = 0;
            foreach (var s in segs)
            {
                if (!c.LaneOffsets.TryGetValue(s, out var o) || o == 0) continue;
                var (a, b) = SegGraph.Ends(s, c.TaskOn);
                if (a != n && b != n) continue;
                if (s.IsHorizontal()) dy = o; else dx = o;
            }
            return ((n.Right ? xR : xL) + dx, YOf(n.Level) + dy);
        }
        double SegW(SegPart s) => c.LaneWidths.TryGetValue(s, out var w) ? w : c.FullWidth;

        Dictionary<BorderCorner, double> ZoneDict(ScreenZone z) =>
            z == ScreenZone.Main ? c.MainRadii : c.TaskRadii;
        ScreenZone ZoneOf(SegPart v) =>
            v is SegPart.LTask or SegPart.RTask ? ScreenZone.Task : ScreenZone.Main;
        int BottomLevel(ScreenZone z) =>
            z == ScreenZone.Main ? (c.TaskOn ? 1 : 2) : 2;
        ScreenZone? ZoneAbove(int level) => level switch
        {
            1 => ScreenZone.Main,
            2 => c.TaskOn ? ScreenZone.Task : ScreenZone.Main,
            _ => null,
        };
        ScreenZone? ZoneBelow(int level) => level switch
        {
            0 => ScreenZone.Main,
            1 => c.TaskOn ? ScreenZone.Task : null,
            _ => null,
        };
        double Radius(ScreenZone z, BorderCorner corner) =>
            Math.Max(0, ZoneDict(z).TryGetValue(corner, out var r) ? r : 0);

        // 정점: (점, 곡률, 명시적 제어점[스쿱]) + 떠나는 구간 굵기 + 아크 굵기 상한
        var vs = new List<((double X, double Y) Pt, double R, (double X, double Y)? Over)>();
        var ew = new List<double>();
        var av = new List<double?>();
        void AddV(IEnumerable<(((double, double), double, (double, double)?) V, double W, double? A)> items)
        {
            foreach (var (v, w, a) in items) { vs.Add(v); ew.Add(w); av.Add(a); }
        }

        // 접합부 정점(자연 꺾임 / 캡 유지 / 스쿱) — macOS bendVertices 이식.
        List<(((double, double), double, (double, double)?) V, double W, double? A)>
            BendVertices(SegPart prevSeg, SegPart nextSeg, SegNode node)
        {
            var h = prevSeg.IsHorizontal() ? prevSeg : nextSeg;
            var v = prevSeg.IsHorizontal() ? nextSeg : prevSeg;
            var n = Pt(node);
            double wNext = SegW(nextSeg);
            double? nw = c.NodeWidths.TryGetValue(node, out var nwv) ? nwv : null;
            double scoopW = nw ?? c.FullWidth;
            var cap = c.HCaps.TryGetValue(SegGraph.HCapKey(h, node.Right), out var cp) ? cp : SegCap.None;
            if (cap is SegCap.Up or SegCap.Down)
            {
                var z = cap == SegCap.Up ? ZoneAbove(node.Level) : ZoneBelow(node.Level);
                if (z is { } zone)
                {
                    var corner = cap == SegCap.Up
                        ? (node.Right ? BorderCorner.BottomRight : BorderCorner.BottomLeft)
                        : (node.Right ? BorderCorner.TopRight : BorderCorner.TopLeft);
                    double r = Radius(zone, corner);
                    var (a, b) = SegGraph.Ends(v, c.TaskOn);
                    bool vDown = (a == node ? b.Level : a.Level) > node.Level;
                    bool matches = (cap == SegCap.Down && vDown) || (cap == SegCap.Up && !vDown);
                    if (matches)
                        return new() { ((n, r, null), wNext, nw) };
                    if (r > 0.01)
                    {
                        // 스쿱: 세로변이 경계선을 지나쳐 오목 아크로 되감김.
                        var o = (n.X, n.Y + (cap == SegCap.Down ? r : -r));
                        var t = (n.X + (node.Right ? -r : r), n.Y);
                        return prevSeg.IsHorizontal()
                            ? new() { ((t, 0, null), scoopW, null), ((o, 0, n), scoopW, null), ((n, 0, null), wNext, null) }
                            : new() { ((n, 0, null), scoopW, null), ((o, 0, null), scoopW, null), ((t, 0, n), wNext, null) };
                    }
                }
            }
            var zv = ZoneOf(v);
            bool isBottom = BottomLevel(zv) == node.Level;
            var cnr = isBottom
                ? (node.Right ? BorderCorner.BottomRight : BorderCorner.BottomLeft)
                : (node.Right ? BorderCorner.TopRight : BorderCorner.TopLeft);
            return new() { ((n, Radius(zv, cnr), null), wNext, nw) };
        }

        List<(((double, double), double, (double, double)?) V, double W, double? A)>?
            CapVertices(SegCap cap, SegNode node, SegPart seg, bool atStart)
        {
            var p = Pt(node);
            double r;
            (double, double) tangent;
            if (seg.IsHorizontal())
            {
                ScreenZone? z;
                BorderCorner cnr;
                if (cap == SegCap.Up)
                {
                    z = ZoneAbove(node.Level);
                    cnr = node.Right ? BorderCorner.BottomRight : BorderCorner.BottomLeft;
                    if (z is not { } zz) return null;
                    r = Radius(zz, cnr);
                    tangent = (p.X, p.Y - r);
                }
                else if (cap == SegCap.Down)
                {
                    z = ZoneBelow(node.Level);
                    cnr = node.Right ? BorderCorner.TopRight : BorderCorner.TopLeft;
                    if (z is not { } zz) return null;
                    r = Radius(zz, cnr);
                    tangent = (p.X, p.Y + r);
                }
                else return null;
            }
            else
            {
                if (cap != SegCap.Include) return null;
                var z = ZoneOf(seg);
                bool isBottom = BottomLevel(z) == node.Level;
                var cnr = isBottom
                    ? (node.Right ? BorderCorner.BottomRight : BorderCorner.BottomLeft)
                    : (node.Right ? BorderCorner.TopRight : BorderCorner.TopLeft);
                r = Radius(z, cnr);
                tangent = (node.Right ? p.X - r : p.X + r, p.Y);
            }
            if (r <= 0.01) return null;
            double w = SegW(seg);
            return atStart
                ? new() { ((tangent, 0, null), w, null), ((p, r, null), w, null) }
                : new() { ((p, r, null), w, null), ((tangent, 0, null), w, null) };
        }

        SegCap EndCapOf(SegNode node, SegPart seg, bool auto) =>
            seg.IsHorizontal()
                ? (c.HCaps.TryGetValue(SegGraph.HCapKey(seg, node.Right), out var cp) ? cp : SegCap.None)
                : (auto ? SegCap.Include : SegCap.None);

        // ── 체인 순회 ──
        var adj = SegGraph.Adjacency(segs, c.TaskOn);
        var eps = adj.Where(kv => kv.Value.Count == 1).Select(kv => kv.Key).OrderBy(n => n).ToList();
        bool isLoop = eps.Count == 0;

        SegNode startNode;
        bool sHalf = false, sAuto = false, eHalf = false, eAuto = false;
        if (isLoop)
        {
            var levels = adj.Keys.Select(n => n.Level).ToList();
            int tl = levels.Min(), bl = levels.Max();
            var want = c.AnchorCorner switch
            {
                BorderCorner.TopLeft => new SegNode(false, tl),
                BorderCorner.TopRight => new SegNode(true, tl),
                BorderCorner.BottomRight => new SegNode(true, bl),
                _ => new SegNode(false, bl),
            };
            startNode = adj.ContainsKey(want) ? want : adj.Keys.OrderBy(n => n).First();
        }
        else
        {
            startNode = c.Clockwise ? eps[0] : eps[1];
            sHalf = c.Clockwise ? c.EndAHalf : c.EndBHalf;
            sAuto = c.Clockwise ? c.EndAAuto : c.EndBAuto;
            eHalf = c.Clockwise ? c.EndBHalf : c.EndAHalf;
            eAuto = c.Clockwise ? c.EndBAuto : c.EndAAuto;
        }

        var nodesOrder = new List<SegNode> { startNode };
        var segsOrder = new List<SegPart>();
        SegPart? prevVisit = null;
        var cur = startNode;
        while (segsOrder.Count < segs.Count)
        {
            var cands = adj.TryGetValue(cur, out var lc)
                ? lc.Where(s => !prevVisit.HasValue || s != prevVisit.Value).ToList()
                : new List<SegPart>();
            if (segsOrder.Count == 0 && cands.Count == 2)
                cands = cands.OrderByDescending(s => c.Clockwise ? s.IsHorizontal() : !s.IsHorizontal()).ToList();
            if (cands.Count == 0) break;
            var seg = cands[0];
            var (a, b) = SegGraph.Ends(seg, c.TaskOn);
            var next = a == cur ? b : a;
            segsOrder.Add(seg);
            nodesOrder.Add(next);
            prevVisit = seg;
            cur = next;
        }
        if (segsOrder.Count != segs.Count) return null;

        // ── 정점 구성 ──
        bool startHasCap = false, endHasCap = false;
        int anchorVertCount = 1;
        if (isLoop)
        {
            int k = segsOrder.Count;
            for (int i = 0; i < k; i++)
            {
                var prevSeg = segsOrder[(i - 1 + k) % k];
                var nextSeg = segsOrder[i];
                if (prevSeg.IsHorizontal() != nextSeg.IsHorizontal())
                    AddV(BendVertices(prevSeg, nextSeg, nodesOrder[i]));
                else
                    AddV(new[] { ((Pt(nodesOrder[i]), 0.0, ((double, double)?)null), SegW(nextSeg), (double?)null) });
                if (i == 0) anchorVertCount = vs.Count;
            }
        }
        else
        {
            var sCapV = EndCapOf(nodesOrder[0], segsOrder[0], sAuto);
            var capVs = CapVertices(sCapV, nodesOrder[0], segsOrder[0], atStart: true);
            if (capVs != null) { AddV(capVs); startHasCap = true; }
            else AddV(new[] { ((Pt(nodesOrder[0]), 0.0, ((double, double)?)null), SegW(segsOrder[0]), (double?)null) });

            for (int i = 0; i < segsOrder.Count - 1; i++)
            {
                if (segsOrder[i].IsHorizontal() != segsOrder[i + 1].IsHorizontal())
                    AddV(BendVertices(segsOrder[i], segsOrder[i + 1], nodesOrder[i + 1]));
                else
                    AddV(new[] { ((Pt(nodesOrder[i + 1]), 0.0, ((double, double)?)null), SegW(segsOrder[i + 1]), (double?)null) });
            }
            int last = segsOrder.Count;
            var eCapV = EndCapOf(nodesOrder[last], segsOrder[last - 1], eAuto);
            var capVe = CapVertices(eCapV, nodesOrder[last], segsOrder[last - 1], atStart: false);
            if (capVe != null) { AddV(capVe); endHasCap = true; }
            else AddV(new[] { ((Pt(nodesOrder[last]), 0.0, ((double, double)?)null), SegW(segsOrder[last - 1]), (double?)null) });
        }

        // 스쿱 앵커: 차감 시작 지점(위/모서리/아래)을 안전한 정점 회전으로 구현.
        if (isLoop && vs.Count >= 3 && vs[0].R < 0.01 && anchorVertCount >= 3)
        {
            var idxs = Enumerable.Range(0, anchorVertCount).ToList();
            int oIdx = idxs.OrderByDescending(i2 =>
                idxs.Sum(j => Math.Abs(vs[i2].Pt.Y - vs[j].Pt.Y))).First();
            var rest = idxs.Where(i2 => i2 != oIdx).ToList();
            int nIdx = rest.OrderByDescending(i2 => Math.Abs(vs[i2].Pt.X - width / 2)).First();
            int tIdx = rest.First(i2 => i2 != nIdx);
            int pick = c.AnchorSide switch
            {
                AnchorSide.Left => tIdx,
                AnchorSide.Center => nIdx,
                _ => oIdx,
            };
            if (pick > 0)
            {
                vs = vs.Skip(pick).Concat(vs.Take(pick)).ToList();
                ew = ew.Skip(pick).Concat(ew.Take(pick)).ToList();
                av = av.Skip(pick).Concat(av.Take(pick)).ToList();
            }
        }

        // ── 접점 계산 ──
        int n2 = vs.Count;
        double Dist((double X, double Y) a, (double X, double Y) b) =>
            Math.Sqrt((a.X - b.X) * (a.X - b.X) + (a.Y - b.Y) * (a.Y - b.Y));
        (double X, double Y) Unit((double X, double Y) a, (double X, double Y) b)
        {
            double dx = b.X - a.X, dy = b.Y - a.Y;
            double len = Math.Max(1e-4, Math.Sqrt(dx * dx + dy * dy));
            return (dx / len, dy / len);
        }
        var t1 = new (double X, double Y)[n2];
        var t2 = new (double X, double Y)[n2];
        for (int i = 0; i < n2; i++)
        {
            var curP = vs[i].Pt;
            var prevV = vs[(i - 1 + n2) % n2];
            var nextV = vs[(i + 1) % n2];
            double maxPrev = (prevV.R > 0.01 ? 0.5 : 1.0) * Dist(curP, prevV.Pt);
            double maxNext = (nextV.R > 0.01 ? 0.5 : 1.0) * Dist(curP, nextV.Pt);
            double r = Math.Max(0, Math.Min(vs[i].R, Math.Min(maxPrev, maxNext)));
            var up = Unit(curP, prevV.Pt);
            var un = Unit(curP, nextV.Pt);
            t1[i] = (curP.X + up.X * r, curP.Y + up.Y * r);
            t2[i] = (curP.X + un.X * r, curP.Y + un.Y * r);
        }

        // ── 방출(평탄화 폴리라인 + 길이·굵기 기록) ──
        var samples = new List<PathSample>();
        var spanRaw = new List<(double Len, double W)>();
        double total = 0;
        (double X, double Y) cp = (0, 0);
        void Mv((double X, double Y) q)
        {
            cp = q;
            if (samples.Count == 0) samples.Add(new PathSample(q.X, q.Y, 0));
        }
        void Ln((double X, double Y) q, double w)
        {
            double d = Dist(cp, q);
            total += d;
            samples.Add(new PathSample(q.X, q.Y, total));
            spanRaw.Add((d, w));
            cp = q;
        }
        void Qd((double X, double Y) q, (double X, double Y) ctl, double w)
        {
            var a0 = cp;
            double segTotal = 0;
            var prevPt = a0;
            var buf = new List<(double X, double Y, double D)>();
            for (int i = 1; i <= QuadSamples; i++)
            {
                double t = (double)i / QuadSamples, mt = 1 - t;
                var q2 = (X: mt * mt * a0.X + 2 * mt * t * ctl.X + t * t * q.X,
                          Y: mt * mt * a0.Y + 2 * mt * t * ctl.Y + t * t * q.Y);
                double d = Dist(prevPt, (q2.X, q2.Y));
                segTotal += d;
                buf.Add((q2.X, q2.Y, d));
                prevPt = (q2.X, q2.Y);
            }
            foreach (var (x, y, d) in buf)
            {
                total += d;
                samples.Add(new PathSample(x, y, total));
            }
            spanRaw.Add((segTotal, w));
            cp = q;
        }
        double Aw(int i)
        {
            double baseW = Math.Max(ew[(i - 1 + n2) % n2], ew[i]);
            return av[i] is { } cap ? Math.Min(baseW, cap) : baseW;
        }

        if (isLoop)
        {
            if (n2 < 3) return null;
            var c0 = vs[0].Pt;
            // 차감 시작 지점(위/아래): 시작 모서리 곡선의 두 접점 중 y로 판정.
            bool wantDown = c.AnchorSide == AnchorSide.Right;
            bool t1IsUp = t1[0].Y <= t2[0].Y;
            bool startAtT1 = t1IsUp != wantDown;
            if (startAtT1) { Mv(t1[0]); Qd(t2[0], c0, Aw(0)); }
            else Mv(t2[0]);
            for (int i = 1; i < n2; i++)
            {
                if (vs[i].Over is { } ov)
                    Qd(vs[i].Pt, ov, ew[i - 1]);         // 스쿱 오목 곡선
                else
                {
                    Ln(t1[i], ew[i - 1]);
                    // 모서리 아크 반분할: 앞 절반=들어온 굵기, 뒤 절반=나가는 굵기.
                    double wIn = Math.Min(ew[i - 1], av[i] ?? double.MaxValue);
                    double wOut = Math.Min(ew[i], av[i] ?? double.MaxValue);
                    if (Math.Abs(wIn - wOut) < 0.01)
                        Qd(t2[i], vs[i].Pt, Math.Max(wIn, wOut));
                    else
                    {
                        var cc = vs[i].Pt;
                        var mid = (X: 0.25 * t1[i].X + 0.5 * cc.X + 0.25 * t2[i].X,
                                   Y: 0.25 * t1[i].Y + 0.5 * cc.Y + 0.25 * t2[i].Y);
                        Qd((mid.X, mid.Y), ((t1[i].X + cc.X) / 2, (t1[i].Y + cc.Y) / 2), wIn);
                        Qd(t2[i], ((cc.X + t2[i].X) / 2, (cc.Y + t2[i].Y) / 2), wOut);
                    }
                }
            }
            if (vs[0].Over is { } ov0)
                Qd(vs[0].Pt, ov0, ew[n2 - 1]);           // 시작이 스쿱 종점이면 오목 곡선으로 닫기
            else
            {
                Ln(t1[0], ew[n2 - 1]);
                if (!startAtT1) Qd(t2[0], c0, Aw(0));
            }
        }
        else
        {
            if (n2 < 2) return null;
            (double X, double Y) ArcMid(int i)
            {
                var cc = vs[i].Pt;
                return (0.25 * t1[i].X + 0.5 * cc.X + 0.25 * t2[i].X,
                        0.25 * t1[i].Y + 0.5 * cc.Y + 0.25 * t2[i].Y);
            }
            bool halfAtStart = sHalf && startHasCap && n2 >= 3;
            bool halfAtEnd = eHalf && endHasCap && n2 >= 3;
            int from = 1;
            if (halfAtStart)
            {
                var cc = vs[1].Pt;
                Mv(ArcMid(1));
                Qd(t2[1], ((cc.X + t2[1].X) / 2, (cc.Y + t2[1].Y) / 2), Aw(1));
                from = 2;
            }
            else Mv(vs[0].Pt);
            int to = halfAtEnd ? n2 - 2 : n2 - 1;
            for (int i = from; i < to; i++)
            {
                if (vs[i].Over is { } ov)
                    Qd(vs[i].Pt, ov, ew[i - 1]);
                else
                {
                    Ln(t1[i], ew[i - 1]);
                    double wIn = Math.Min(ew[i - 1], av[i] ?? double.MaxValue);
                    double wOut = Math.Min(ew[Math.Min(i, n2 - 1)], av[i] ?? double.MaxValue);
                    if (Math.Abs(wIn - wOut) < 0.01)
                        Qd(t2[i], vs[i].Pt, Math.Max(wIn, wOut));
                    else
                    {
                        var cc = vs[i].Pt;
                        var mid = ArcMid(i);
                        Qd(mid, ((t1[i].X + cc.X) / 2, (t1[i].Y + cc.Y) / 2), wIn);
                        Qd(t2[i], ((cc.X + t2[i].X) / 2, (cc.Y + t2[i].Y) / 2), wOut);
                    }
                }
            }
            if (halfAtEnd)
            {
                int i = n2 - 2;
                var cc = vs[i].Pt;
                Ln(t1[i], ew[i - 1]);
                Qd(ArcMid(i), ((t1[i].X + cc.X) / 2, (t1[i].Y + cc.Y) / 2), Aw(i));
            }
            else Ln(vs[n2 - 1].Pt, ew[n2 - 2]);
        }

        if (total <= 0) return null;

        // 스팬 병합(같은 굵기 이어붙임) → 트림 비율.
        var spans = new List<WidthSpan>();
        double acc = 0;
        foreach (var (len, w) in spanRaw)
        {
            double a = acc / total, b = (acc + len) / total;
            if (spans.Count > 0 && Math.Abs(spans[^1].Width - w) < 0.01)
                spans[^1] = spans[^1] with { To = b };
            else if (len > 0.001)
                spans.Add(new WidthSpan(a, b, w));
            acc += len;
        }
        if (spans.Count > 0) spans[^1] = spans[^1] with { To = 1 };

        return new BorderPath { Samples = samples, Spans = spans, Total = total, IsLoop = isLoop };
    }

    /// <summary>굵기 전환 평활: 균일 리샘플 + 원형 가우시안(박스 3패스) — macOS tapered() 이식.
    /// 전환 길이 ≈ max(600, 굵기×140)pt로 사용자가 변화를 인지하지 못한다.</summary>
    public static List<WidthSpan> Smooth(List<WidthSpan> spans, double total, double thickness)
    {
        if (spans.Count < 2 || total <= 0) return spans;
        const double slicePts = 6;
        int n = Math.Max(48, Math.Min(2048, (int)(total / slicePts)));
        var w = new double[n];
        int si = 0;
        for (int i = 0; i < n; i++)
        {
            double f = (i + 0.5) / n;
            while (si < spans.Count - 1 && f > spans[si].To) si++;
            w[i] = spans[si].Width;
        }
        double taperPts = Math.Max(600, thickness * 140);
        int radius = Math.Max(1, Math.Min(n / 2 - 1, (int)((taperPts / 2) / (total / n))));
        for (int pass = 0; pass < 3; pass++)
        {
            var pre = new double[3 * n + 1];
            for (int i = 0; i < 3 * n; i++) pre[i + 1] = pre[i] + w[i % n];
            var outW = new double[n];
            for (int i = 0; i < n; i++)
            {
                int lo = i + n - radius, hi = i + n + radius;
                outW[i] = (pre[hi + 1] - pre[lo]) / (2 * radius + 1);
            }
            w = outW;
        }
        var res = new List<WidthSpan>();
        for (int i = 0; i < n; i++)
        {
            double a = (double)i / n, b = (double)(i + 1) / n;
            if (res.Count > 0 && Math.Abs(res[^1].Width - w[i]) < 0.02)
                res[^1] = res[^1] with { To = b };
            else
                res.Add(new WidthSpan(a, b, w[i]));
        }
        if (res.Count > 0) res[^1] = res[^1] with { To = 1 };
        return res;
    }
}
