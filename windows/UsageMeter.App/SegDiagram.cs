using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Shapes;
using UsageMeter.Core;
using Color = System.Windows.Media.Color;
using Point = System.Windows.Point;
using Cursors = System.Windows.Input.Cursors;
using Brushes = System.Windows.Media.Brushes;

namespace UsageMeter.App;

/// <summary>
/// 인터랙티브 화면 다이어그램(2분할) — macOS segDiagram 이식.
/// 화면 모양 위의 세그먼트 선을 직접 클릭해 토글. 선택 = AI 색, 미선택 = 회색.
/// 무효(끊긴) 조합이 되는 클릭은 되돌리고 경고를 띄운다.
/// </summary>
public sealed class SegDiagram : Canvas
{
    public const double W = 209, H = 133;
    private const double TaskFrac = 0.80;     // 작업표시줄선 위치(다이어그램 비율)

    private readonly Func<AppSettings> _getSettings;
    private readonly string _providerId;
    private readonly Color _accent;
    private readonly Action _onChanged;
    private readonly Action<string> _warn;

    public SegDiagram(Func<AppSettings> getSettings, string providerId, Color accent,
                      Action onChanged, Action<string> warn)
    {
        _getSettings = getSettings;
        _providerId = providerId;
        _accent = accent;
        _onChanged = onChanged;
        _warn = warn;
        Width = W;
        Height = H;
        Background = new SolidColorBrush(Color.FromArgb(28, 128, 128, 128));
        Rebuild();
    }

    private (Point A, Point B) Line(SegPart s, bool taskOn)
    {
        double taskY = taskOn ? H * TaskFrac : H;
        return s switch
        {
            SegPart.HTop => (new Point(0, 0), new Point(W, 0)),
            SegPart.HTask => (new Point(0, taskY), new Point(W, taskY)),
            SegPart.HBottom => (new Point(0, H), new Point(W, H)),
            SegPart.LMain => (new Point(0, 0), new Point(0, taskOn ? taskY : H)),
            SegPart.LTask => (new Point(0, taskY), new Point(0, H)),
            SegPart.RMain => (new Point(W, 0), new Point(W, taskOn ? taskY : H)),
            _ => (new Point(W, taskY), new Point(W, H)),           // RTask
        };
    }

    public void Rebuild()
    {
        Children.Clear();
        var settings = _getSettings();
        bool taskOn = settings.TaskLineEnabled;
        var layout = settings.LayoutFor(_providerId);

        foreach (SegPart s in Enum.GetValues<SegPart>())
        {
            if (!SegGraph.Available(s, taskOn)) continue;
            var (a, b) = Line(s, taskOn);
            bool on = layout.Segments.Contains(s);
            var line = new Line
            {
                X1 = a.X, Y1 = a.Y, X2 = b.X, Y2 = b.Y,
                Stroke = new SolidColorBrush(on ? _accent : Color.FromArgb(90, 128, 128, 128)),
                StrokeThickness = on ? 5 : 3,
                StrokeStartLineCap = PenLineCap.Round,
                StrokeEndLineCap = PenLineCap.Round,
                Cursor = Cursors.Hand,
                Tag = s,
            };
            // 클릭 판정을 넓히는 투명 히트 영역.
            var hit = new Line
            {
                X1 = a.X, Y1 = a.Y, X2 = b.X, Y2 = b.Y,
                Stroke = Brushes.Transparent,
                StrokeThickness = 14,
                Cursor = Cursors.Hand,
                Tag = s,
            };
            hit.MouseLeftButtonDown += (_, e) => Toggle(s, e);
            Children.Add(line);
            Children.Add(hit);
        }
    }

    private void Toggle(SegPart s, MouseButtonEventArgs e)
    {
        var settings = _getSettings();
        var layout = settings.LayoutFor(_providerId);
        bool taskOn = settings.TaskLineEnabled;
        var next = new HashSet<SegPart>(layout.Segments);

        if (e.ClickCount >= 2)
        {
            next = new HashSet<SegPart> { s };   // 더블클릭 = 그 선만
        }
        else if (!next.Remove(s))
        {
            next.Add(s);
        }
        if (next.Count > 0 && !SegGraph.IsValid(next, taskOn))
        {
            _warn(Loc.Tr("lay.invalid", settings.Language));
            return;
        }
        if (next.Count == 0) return;             // 최소 1개 유지
        layout.Segments = next;
        Rebuild();
        _onChanged();
    }
}
