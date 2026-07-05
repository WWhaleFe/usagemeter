using System.Runtime.InteropServices;
using System.Windows;
using Color = System.Windows.Media.Color;
using Brushes = System.Windows.Media.Brushes;
using Pen = System.Windows.Media.Pen;
using Point = System.Windows.Point;
using System.Windows.Interop;
using System.Windows.Media;

namespace UsageMeter.App;

/// <summary>
/// 화면 하나를 덮는 투명·항상위·클릭통과 오버레이 창 — macOS OverlayWindow 대응.
/// WS_EX_TRANSPARENT(클릭 통과) + WS_EX_LAYERED + WS_EX_TOOLWINDOW(작업표시줄/Alt-Tab 숨김)
/// + WS_EX_NOACTIVATE(포커스 안 가져감).
/// </summary>
public sealed class OverlayWindow : Window
{
    private const int GWL_EXSTYLE = -20;
    private const int WS_EX_TRANSPARENT = 0x00000020;
    private const int WS_EX_LAYERED = 0x00080000;
    private const int WS_EX_TOOLWINDOW = 0x00000080;
    private const int WS_EX_NOACTIVATE = 0x08000000;

    [DllImport("user32.dll")]
    private static extern int GetWindowLong(IntPtr hWnd, int nIndex);
    [DllImport("user32.dll")]
    private static extern int SetWindowLong(IntPtr hWnd, int nIndex, int dwNewLong);

    public BorderCanvas Canvas { get; }

    public OverlayWindow(Rect screenBounds)
    {
        WindowStyle = WindowStyle.None;
        ResizeMode = ResizeMode.NoResize;
        AllowsTransparency = true;
        Background = Brushes.Transparent;
        Topmost = true;
        ShowInTaskbar = false;
        ShowActivated = false;
        Focusable = false;
        IsHitTestVisible = false;

        Left = screenBounds.Left;
        Top = screenBounds.Top;
        Width = screenBounds.Width;
        Height = screenBounds.Height;

        Canvas = new BorderCanvas();
        Content = Canvas;

        SourceInitialized += (_, _) =>
        {
            var hwnd = new WindowInteropHelper(this).Handle;
            int ex = GetWindowLong(hwnd, GWL_EXSTYLE);
            SetWindowLong(hwnd, GWL_EXSTYLE,
                ex | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE);
        };
    }
}

/// <summary>테두리를 그리는 캔버스 — macOS BorderView(Canvas) 대응. DrawItems를 받아 즉시 렌더.</summary>
public sealed class BorderCanvas : FrameworkElement
{
    public sealed record Stroke(List<(double X, double Y)> Points, double Width, Color Color, double Opacity);

    private List<Stroke> _strokes = new();

    public void SetStrokes(List<Stroke> strokes)
    {
        _strokes = strokes;
        InvalidateVisual();
    }

    protected override void OnRender(DrawingContext dc)
    {
        foreach (var s in _strokes)
        {
            if (s.Points.Count < 2) continue;
            var geo = new StreamGeometry();
            using (var ctx = geo.Open())
            {
                ctx.BeginFigure(new Point(s.Points[0].X, s.Points[0].Y), isFilled: false, isClosed: false);
                for (int i = 1; i < s.Points.Count; i++)
                    ctx.LineTo(new Point(s.Points[i].X, s.Points[i].Y), isStroked: true, isSmoothJoin: true);
            }
            geo.Freeze();
            var brush = new SolidColorBrush(s.Color) { Opacity = s.Opacity };
            brush.Freeze();
            var pen = new Pen(brush, s.Width)
            {
                StartLineCap = PenLineCap.Round,
                EndLineCap = PenLineCap.Round,
                LineJoin = PenLineJoin.Round,
            };
            pen.Freeze();
            dc.DrawGeometry(null, pen, geo);
        }
    }
}
