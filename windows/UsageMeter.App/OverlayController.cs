using System.Windows;
using Color = System.Windows.Media.Color;
using System.Windows.Media;
using UsageMeter.Core;

namespace UsageMeter.App;

/// <summary>
/// 화면마다 오버레이 창을 띄우고, Core 지오메트리로 스트로크를 계산해 그린다 —
/// macOS BorderView.drawItems + OverlayController 이식(겹침 레인 분할·모서리 분할 포함).
/// </summary>
public sealed class OverlayController
{
    private readonly AppSettings _settings;
    private readonly ProviderManager _manager;
    private readonly List<OverlayWindow> _windows = new();

    public OverlayController(AppSettings settings, ProviderManager manager)
    {
        _settings = settings;
        _manager = manager;
    }

    public void ShowAll()
    {
        foreach (var w in _windows) w.Close();
        _windows.Clear();
        foreach (var screen in System.Windows.Forms.Screen.AllScreens)
        {
            // WPF는 DIP(96dpi 기준) 좌표 — 물리 픽셀을 DPI로 나눈다.
            var b = screen.Bounds;
            using var g = System.Drawing.Graphics.FromHwnd(IntPtr.Zero);
            double scale = g.DpiX / 96.0;
            var rect = new Rect(b.X / scale, b.Y / scale, b.Width / scale, b.Height / scale);
            var w = new OverlayWindow(rect);
            w.Show();
            _windows.Add(w);
        }
        Redraw();
    }

    public void Redraw()
    {
        foreach (var w in _windows)
            w.Canvas.SetStrokes(BuildStrokes(w.Width, w.Height));
    }

    private sealed record Item(ProviderSpec Spec, ProviderLayout Layout, double Remaining, Color Color);

    private List<BorderCanvas.Stroke> BuildStrokes(double width, double height)
    {
        var strokes = new List<BorderCanvas.Stroke>();
        double t = _settings.Thickness;
        bool taskOn = _settings.TaskLineEnabled;

        var items = new List<Item>();
        foreach (var spec in ProviderSpec.All)
        {
            var snap = _manager.SnapshotFor(spec.Id);
            if (snap is not { Ok: true }) continue;
            var layout = _settings.LayoutFor(spec.Id);
            var rgb = layout.ColorRgb is { Length: 3 } c
                ? Color.FromRgb((byte)c[0], (byte)c[1], (byte)c[2])
                : Color.FromRgb(spec.DefaultColor.R, spec.DefaultColor.G, spec.DefaultColor.B);
            items.Add(new Item(spec, layout, snap.RemainingRatio, rgb));
        }
        if (items.Count == 0) return strokes;

        // ── 겹침 계산: 세그먼트별 공유자 → 레인 분할(t/N) — macOS drawItems 이식 ──
        var sharers = new Dictionary<SegPart, List<int>>();
        for (int i = 0; i < items.Count; i++)
            foreach (var s in items[i].Layout.Segments.Where(s => SegGraph.Available(s, taskOn)))
                (sharers.TryGetValue(s, out var l) ? l : sharers[s] = new()).Add(i);

        // 노드(모서리) 공유 → t/k.
        var nodeCount = new Dictionary<SegNode, HashSet<int>>();
        for (int i = 0; i < items.Count; i++)
            foreach (var s in items[i].Layout.Segments.Where(s => SegGraph.Available(s, taskOn)))
            {
                var (a, b) = SegGraph.Ends(s, taskOn);
                foreach (var n in new[] { a, b })
                    (nodeCount.TryGetValue(n, out var set) ? set : nodeCount[n] = new()).Add(i);
            }

        // 잔여율 낮은(급한) AI가 위에 오도록 정렬 후 뒤에서부터 그림.
        var order = Enumerable.Range(0, items.Count)
            .OrderByDescending(i => items[i].Remaining).ToList();

        foreach (int idx in order)
        {
            var item = items[idx];
            var lanes = new Dictionary<SegPart, double>();
            var widths = new Dictionary<SegPart, double>();
            foreach (var s in item.Layout.Segments)
            {
                if (!sharers.TryGetValue(s, out var sh) || sh.Count <= 1) continue;
                if (!_settings.SplitOverlapLines && sh.Count <= 1) continue;
                int k = sh.Count;
                int lane = sh.IndexOf(idx);
                double laneW = t / k;
                widths[s] = laneW;
                lanes[s] = (lane - (k - 1) / 2.0) * laneW;
            }
            var nodeWidths = new Dictionary<SegNode, double>();
            foreach (var (n, set) in nodeCount)
                if (set.Count > 1 && set.Contains(idx))
                    nodeWidths[n] = t / set.Count;

            var cfg = new BorderConfig
            {
                Segments = item.Layout.Segments,
                TaskOn = taskOn,
                TaskH = _settings.TaskLineHeight,
                Inset = t / 2,
                MainRadii = _settings.MainRadii,
                TaskRadii = _settings.TaskRadii,
                AnchorCorner = item.Layout.AnchorCorner,
                AnchorSide = item.Layout.AnchorSide,
                Clockwise = item.Layout.Clockwise,
                HCaps = item.Layout.HCaps,
                LaneOffsets = lanes,
                LaneWidths = widths,
                FullWidth = t,
                NodeWidths = nodeWidths,
            };
            var path = BorderGeometry.Build(cfg, width, height);
            if (path == null) continue;
            var spans = BorderGeometry.Smooth(path.Spans, path.Total, t);

            // 트랙(배경) — 전체 경로를 옅게.
            if (_settings.ShowTrack)
                foreach (var sp in spans)
                    AddStroke(strokes, path, sp.From, sp.To, sp.Width, item.Color, 0.15);

            // 잔여 띠 — 앵커(경로 시작)에서부터 차감: 실선 = [1-잔여, 1].
            double solidFrom = Math.Clamp(1 - item.Remaining, 0, 1);
            foreach (var sp in spans)
            {
                double a = Math.Max(sp.From, solidFrom), b = sp.To;
                if (b <= a + 1e-6) continue;
                AddStroke(strokes, path, a, b, sp.Width, item.Color, _settings.LineOpacity);
            }
        }
        return strokes;
    }

    private static void AddStroke(List<BorderCanvas.Stroke> outList, BorderPath path,
        double from, double to, double width, Color color, double opacity)
    {
        var pts = path.Extract(from, to);
        if (pts.Count >= 2)
            outList.Add(new BorderCanvas.Stroke(pts, width, color, opacity));
    }
}
