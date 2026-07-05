using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using UsageMeter.Core;
using Color = System.Windows.Media.Color;
using TabControl = System.Windows.Controls.TabControl;
using Brushes = System.Windows.Media.Brushes;
using Button = System.Windows.Controls.Button;
using CheckBox = System.Windows.Controls.CheckBox;
using ComboBox = System.Windows.Controls.ComboBox;
using TextBlock = System.Windows.Controls.TextBlock;
using Orientation = System.Windows.Controls.Orientation;
using HorizontalAlignment = System.Windows.HorizontalAlignment;
using UniformGrid = System.Windows.Controls.Primitives.UniformGrid;

namespace UsageMeter.App;

/// <summary>
/// 설정 창 — macOS SettingsView 이식(2분할 모델).
/// 탭: 로그인 / 표시 / 선 / 화면 분할 / 배치 / 모서리 곡률.
/// 모든 변경은 즉시 저장 + 오버레이 다시 그림.
/// </summary>
public sealed class SettingsWindow : Window
{
    private readonly AppSettings _settings;
    private readonly ProviderManager _manager;
    private readonly OverlayController _overlays;
    private readonly List<SegDiagram> _diagrams = new();
    private TabControl _tabs = null!;
    private TextBlock _warnText = null!;

    private static SettingsWindow? _instance;

    public static void Open(AppSettings settings, ProviderManager manager, OverlayController overlays)
    {
        if (_instance is { IsLoaded: true })
        {
            _instance.Activate();
            return;
        }
        _instance = new SettingsWindow(settings, manager, overlays);
        _instance.Show();
    }

    private string T(string key) => Loc.Tr(key, _settings.Language);

    private SettingsWindow(AppSettings settings, ProviderManager manager, OverlayController overlays)
    {
        _settings = settings;
        _manager = manager;
        _overlays = overlays;
        Title = T("settings.title");
        Width = 640;
        Height = 640;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;
        BuildContent();
    }

    private void Apply()
    {
        _settings.Save();
        _overlays.Redraw();
    }

    private void BuildContent()
    {
        _tabs = new TabControl { Margin = new Thickness(10) };
        _tabs.Items.Add(Tab(T("tab.login"), LoginTab()));
        _tabs.Items.Add(Tab(T("tab.display"), DisplayTab()));
        _tabs.Items.Add(Tab(T("tab.line"), LineTab()));
        _tabs.Items.Add(Tab(T("tab.partition"), PartitionTab()));
        _tabs.Items.Add(Tab(T("tab.layout"), LayoutTab()));
        _tabs.Items.Add(Tab(T("tab.radius"), RadiusTab()));
        Content = _tabs;
    }

    private static TabItem Tab(string header, UIElement content) => new()
    {
        Header = header,
        Content = new ScrollViewer
        {
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            Content = content,
        },
    };

    private static StackPanel Column(double spacing = 10) => new()
    {
        Orientation = Orientation.Vertical,
        Margin = new Thickness(12),
    };

    private static DockPanel Row(string label, UIElement control, double labelWidth = 170)
    {
        var dock = new DockPanel { Margin = new Thickness(0, 3, 0, 3) };
        var text = new TextBlock
        {
            Text = label, Width = labelWidth,
            VerticalAlignment = VerticalAlignment.Center,
        };
        DockPanel.SetDock(text, Dock.Left);
        dock.Children.Add(text);
        dock.Children.Add(control);
        return dock;
    }

    private Slider MakeSlider(double min, double max, double value, Action<double> onChange, double step = 1)
    {
        var s = new Slider
        {
            Minimum = min, Maximum = max, Value = value,
            TickFrequency = step, IsSnapToTickEnabled = true,
            VerticalAlignment = VerticalAlignment.Center,
        };
        s.ValueChanged += (_, e) => { onChange(e.NewValue); Apply(); };
        return s;
    }

    // ── 로그인 ──
    private UIElement LoginTab()
    {
        var col = Column();
        foreach (var spec in ProviderSpec.All)
        {
            var snap = _manager.SnapshotFor(spec.Id);
            var status = new TextBlock
            {
                Text = snap is { Ok: true } ? T("acct.loggedIn") : T("acct.loggedOut"),
                VerticalAlignment = VerticalAlignment.Center,
                Margin = new Thickness(8, 0, 8, 0),
                Foreground = snap is { Ok: true } ? Brushes.Green : Brushes.Gray,
            };
            var login = new Button { Content = T("menu.login"), Padding = new Thickness(10, 2, 10, 2) };
            login.Click += async (_, _) => await _manager.ShowLoginAsync(spec.Id);
            var logout = new Button
            {
                Content = T("menu.logout"),
                Padding = new Thickness(10, 2, 10, 2),
                Margin = new Thickness(6, 0, 0, 0),
            };
            logout.Click += async (_, _) =>
            {
                await _manager.SessionFor(spec.Id).LogoutAsync();
                await _manager.RefreshAllAsync();
            };
            var row = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 4, 0, 4) };
            row.Children.Add(new TextBlock
            {
                Text = spec.Name, Width = 90, FontWeight = FontWeights.SemiBold,
                VerticalAlignment = VerticalAlignment.Center,
            });
            row.Children.Add(status);
            row.Children.Add(login);
            row.Children.Add(logout);
            col.Children.Add(row);
        }
        return col;
    }

    // ── 표시 ──
    private UIElement DisplayTab()
    {
        var col = Column();
        var lang = new ComboBox { Width = 160, HorizontalAlignment = HorizontalAlignment.Left };
        foreach (var (code, name) in new[] { ("ko", "한국어"), ("en", "English"), ("ja", "日本語") })
            lang.Items.Add(new ComboBoxItem { Content = name, Tag = code });
        lang.SelectedIndex = _settings.Language switch { "ko" => 0, "ja" => 2, _ => 1 };
        lang.SelectionChanged += (_, _) =>
        {
            _settings.Language = (string)((ComboBoxItem)lang.SelectedItem).Tag;
            Apply();
            int keep = _tabs.SelectedIndex;
            BuildContent();                     // 언어 즉시 반영
            _tabs.SelectedIndex = keep;
            Title = T("settings.title");
        };
        col.Children.Add(Row(T("display.language"), lang));
        col.Children.Add(Row(T("display.refresh"),
            MakeSlider(1, 120, _settings.RefreshMinutes, v => _settings.RefreshMinutes = (int)v)));
        return col;
    }

    // ── 선 ──
    private UIElement LineTab()
    {
        var col = Column();
        col.Children.Add(Row(T("line.thickness"),
            MakeSlider(1, 12, _settings.Thickness, v => _settings.Thickness = v)));
        col.Children.Add(Row(T("line.opacity"),
            MakeSlider(0.2, 1.0, _settings.LineOpacity, v => _settings.LineOpacity = v, 0.05)));
        var track = new CheckBox { Content = T("line.track"), IsChecked = _settings.ShowTrack };
        track.Checked += (_, _) => { _settings.ShowTrack = true; Apply(); };
        track.Unchecked += (_, _) => { _settings.ShowTrack = false; Apply(); };
        col.Children.Add(track);
        var split = new CheckBox
        {
            Content = T("line.splitOverlap"),
            IsChecked = _settings.SplitOverlapLines,
            Margin = new Thickness(0, 6, 0, 0),
        };
        split.Checked += (_, _) => { _settings.SplitOverlapLines = true; Apply(); };
        split.Unchecked += (_, _) => { _settings.SplitOverlapLines = false; Apply(); };
        col.Children.Add(split);
        return col;
    }

    // ── 화면 분할 ──
    private UIElement PartitionTab()
    {
        var col = Column();
        var enable = new CheckBox { Content = T("part.enable"), IsChecked = _settings.TaskLineEnabled };
        enable.Checked += (_, _) => { _settings.TaskLineEnabled = true; OnPartitionChanged(); };
        enable.Unchecked += (_, _) => { _settings.TaskLineEnabled = false; OnPartitionChanged(); };
        col.Children.Add(enable);
        col.Children.Add(Row(T("part.height"),
            MakeSlider(24, 120, _settings.TaskLineHeight, v => _settings.TaskLineHeight = v)));
        return col;
    }

    private void OnPartitionChanged()
    {
        // 경계선 off 시 무효가 된 배치를 정리.
        foreach (var spec in ProviderSpec.All)
        {
            var layout = _settings.LayoutFor(spec.Id);
            if (!SegGraph.IsValid(layout.Segments, _settings.TaskLineEnabled))
                layout.Segments = new HashSet<SegPart> { SegPart.HTop };
        }
        Apply();
        foreach (var d in _diagrams) d.Rebuild();
        RebuildLayoutTab();
    }

    // ── 배치 ──
    private int _layoutTabIndex = 4;

    private void RebuildLayoutTab()
    {
        if (_tabs.Items[_layoutTabIndex] is TabItem item)
            ((ScrollViewer)item.Content).Content = LayoutTabContent();
    }

    private UIElement LayoutTab() => LayoutTabContent();

    private UIElement LayoutTabContent()
    {
        _diagrams.Clear();
        var col = Column();
        _warnText = new TextBlock
        {
            Foreground = Brushes.OrangeRed,
            Visibility = Visibility.Collapsed,
            Margin = new Thickness(0, 0, 0, 6),
        };
        col.Children.Add(_warnText);
        foreach (var spec in ProviderSpec.All)
            col.Children.Add(AiLayoutBox(spec));
        return col;
    }

    private void Warn(string msg)
    {
        _warnText.Text = msg;
        _warnText.Visibility = Visibility.Visible;
        var timer = new System.Windows.Threading.DispatcherTimer { Interval = TimeSpan.FromSeconds(2) };
        timer.Tick += (_, _) => { _warnText.Visibility = Visibility.Collapsed; timer.Stop(); };
        timer.Start();
    }

    private UIElement AiLayoutBox(ProviderSpec spec)
    {
        var layout = _settings.LayoutFor(spec.Id);
        var accent = Color.FromRgb(spec.DefaultColor.R, spec.DefaultColor.G, spec.DefaultColor.B);

        var left = new StackPanel { Orientation = Orientation.Vertical, Width = 330 };
        left.Children.Add(new TextBlock
        {
            Text = spec.Name, FontWeight = FontWeights.Bold, Margin = new Thickness(0, 0, 0, 6),
        });

        // 가로선 끝 캡(선택된 가로선만).
        var hs = new[] { SegPart.HTop, SegPart.HTask, SegPart.HBottom }.Where(layout.Segments.Contains).ToList();
        if (hs.Count > 0)
        {
            left.Children.Add(new TextBlock
            {
                Text = T("lay.hCapsTitle"), Foreground = Brushes.Gray, FontSize = 11,
            });
            foreach (var h in hs) left.Children.Add(HCapRow(spec.Id, h));
        }

        // 앵커 / 방향.
        bool isLoop = SegGraph.ChainEnds(layout.Segments, _settings.TaskLineEnabled) == null
                      && SegGraph.IsValid(layout.Segments, _settings.TaskLineEnabled);
        if (isLoop)
        {
            var corner = new ComboBox { Width = 130 };
            var corners = new[]
            {
                (BorderCorner.TopLeft, T("corner.tl")), (BorderCorner.TopRight, T("corner.tr")),
                (BorderCorner.BottomRight, T("corner.br")), (BorderCorner.BottomLeft, T("corner.bl")),
            };
            foreach (var (c, name) in corners) corner.Items.Add(new ComboBoxItem { Content = name, Tag = c });
            corner.SelectedIndex = Array.FindIndex(corners, x => x.Item1 == layout.AnchorCorner);
            corner.SelectionChanged += (_, _) =>
            {
                layout.AnchorCorner = (BorderCorner)((ComboBoxItem)corner.SelectedItem).Tag;
                Apply();
            };
            left.Children.Add(Row(T("lay.anchor"), corner, 140));

            var side = new ComboBox { Width = 130 };
            var sides = new[]
            {
                (AnchorSide.Left, T("side.up")), (AnchorSide.Center, T("side.corner")),
                (AnchorSide.Right, T("side.down")),
            };
            foreach (var (s, name) in sides) side.Items.Add(new ComboBoxItem { Content = name, Tag = s });
            side.SelectedIndex = Array.FindIndex(sides, x => x.Item1 == layout.AnchorSide);
            side.SelectionChanged += (_, _) =>
            {
                layout.AnchorSide = (AnchorSide)((ComboBoxItem)side.SelectedItem).Tag;
                Apply();
            };
            left.Children.Add(Row(T("lay.anchorSide"), side, 140));
        }
        else
        {
            var reverse = new CheckBox { Content = T("lay.reverse"), IsChecked = !layout.Clockwise };
            reverse.Checked += (_, _) => { layout.Clockwise = false; Apply(); };
            reverse.Unchecked += (_, _) => { layout.Clockwise = true; Apply(); };
            left.Children.Add(reverse);
        }

        // 우측: 프리셋(다이어그램 폭 정렬) + 다이어그램.
        var right = new StackPanel { Orientation = Orientation.Vertical };
        var presetRow = new UniformGrid { Rows = 1, Columns = 3, Width = SegDiagram.W, Margin = new Thickness(0, 0, 0, 4) };
        void AddPreset(string labelKey, ScreenZone[] zones, Dictionary<string, SegCap> caps)
        {
            var b = new Button { Content = T(labelKey), Margin = new Thickness(0, 0, 3, 0), FontSize = 11 };
            b.Click += (_, _) =>
            {
                layout.Segments = SegGraph.ZonePerimeter(zones, _settings.TaskLineEnabled);
                layout.HCaps = new Dictionary<string, SegCap>(caps);
                Apply();
                RebuildLayoutTab();
            };
            presetRow.Children.Add(b);
        }
        AddPreset("lay.zoneAll", new[] { ScreenZone.Main, ScreenZone.Task }, new());
        AddPreset("zone.main", new[] { ScreenZone.Main }, new());
        AddPreset("zone.task", new[] { ScreenZone.Task }, new()
        {
            // 작업표시줄 감싸개: 윗선 양끝 위로 둥글게(스쿱).
            [SegGraph.HCapKey(SegPart.HTask, false)] = SegCap.Up,
            [SegGraph.HCapKey(SegPart.HTask, true)] = SegCap.Up,
        });
        right.Children.Add(presetRow);
        var diagram = new SegDiagram(() => _settings, spec.Id, accent,
            onChanged: () => { Apply(); RebuildLayoutTab(); }, warn: Warn);
        _diagrams.Add(diagram);
        right.Children.Add(diagram);

        var row = new StackPanel { Orientation = Orientation.Horizontal };
        row.Children.Add(left);
        row.Children.Add(right);
        var box = new Border
        {
            BorderBrush = Brushes.LightGray,
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(6),
            Padding = new Thickness(10),
            Margin = new Thickness(0, 0, 0, 8),
            Child = row,
        };
        return box;
    }

    private UIElement HCapRow(string providerId, SegPart h)
    {
        var layout = _settings.LayoutFor(providerId);
        string segName = h switch
        {
            SegPart.HTop => T("seg.hTop"),
            SegPart.HTask => T("seg.hTask"),
            _ => T("seg.hBottom"),
        };
        var row = new StackPanel { Orientation = Orientation.Horizontal, Margin = new Thickness(0, 2, 0, 2) };
        row.Children.Add(new TextBlock
        {
            Text = segName, Width = 90, VerticalAlignment = VerticalAlignment.Center, FontSize = 12,
        });
        foreach (bool rightEnd in new[] { false, true })
        {
            row.Children.Add(new TextBlock
            {
                Text = rightEnd ? T("lay.right") : T("lay.left"),
                Margin = new Thickness(6, 0, 4, 0),
                VerticalAlignment = VerticalAlignment.Center,
                Foreground = Brushes.Gray, FontSize = 11,
            });
            var combo = new ComboBox { Width = 70, FontSize = 11 };
            var opts = new[] { (SegCap.None, "—"), (SegCap.Up, T("cap.upShort")), (SegCap.Down, T("cap.downShort")) };
            foreach (var (cap, name) in opts) combo.Items.Add(new ComboBoxItem { Content = name, Tag = cap });
            string key = SegGraph.HCapKey(h, rightEnd);
            var current = layout.HCaps.TryGetValue(key, out var cv) ? cv : SegCap.None;
            combo.SelectedIndex = Array.FindIndex(opts, o => o.Item1 == current);
            combo.SelectionChanged += (_, _) =>
            {
                var cap = (SegCap)((ComboBoxItem)combo.SelectedItem).Tag;
                if (cap == SegCap.None) layout.HCaps.Remove(key);
                else layout.HCaps[key] = cap;
                Apply();
            };
            row.Children.Add(combo);
        }
        return row;
    }

    // ── 모서리 곡률 ──
    private UIElement RadiusTab()
    {
        var col = Column();
        void RadiiGroup(string title, Dictionary<BorderCorner, double> dict, BorderCorner[] corners)
        {
            col.Children.Add(new TextBlock
            {
                Text = title, FontWeight = FontWeights.Bold, Margin = new Thickness(0, 8, 0, 4),
            });
            foreach (var c in corners)
            {
                string name = c switch
                {
                    BorderCorner.TopLeft => T("corner.tl"),
                    BorderCorner.TopRight => T("corner.tr"),
                    BorderCorner.BottomRight => T("corner.br"),
                    _ => T("corner.bl"),
                };
                double cur = dict.TryGetValue(c, out var v) ? v : 0;
                col.Children.Add(Row(name, MakeSlider(0, 80, cur, val =>
                {
                    dict[c] = val;
                    if (_settings.LinkMainTaskRadii) SyncLinkedRadii(dict, c, val);
                }), 90));
            }
        }
        RadiiGroup(T("zone.main"), _settings.MainRadii,
            new[] { BorderCorner.TopLeft, BorderCorner.TopRight, BorderCorner.BottomRight, BorderCorner.BottomLeft });
        RadiiGroup(T("zone.task"), _settings.TaskRadii,
            new[] { BorderCorner.TopLeft, BorderCorner.TopRight, BorderCorner.BottomRight, BorderCorner.BottomLeft });

        var link = new CheckBox
        {
            Content = T("radius.link"),
            IsChecked = _settings.LinkMainTaskRadii,
            Margin = new Thickness(0, 10, 0, 0),
        };
        link.Checked += (_, _) => { _settings.LinkMainTaskRadii = true; Apply(); };
        link.Unchecked += (_, _) => { _settings.LinkMainTaskRadii = false; Apply(); };
        col.Children.Add(link);
        return col;
    }

    /// <summary>본문 하단 모서리 ↔ 작업표시줄 상단 모서리(경계에서 맞닿음) 동기화.</summary>
    private void SyncLinkedRadii(Dictionary<BorderCorner, double> changed, BorderCorner corner, double v)
    {
        if (ReferenceEquals(changed, _settings.MainRadii))
        {
            if (corner == BorderCorner.BottomLeft) _settings.TaskRadii[BorderCorner.TopLeft] = v;
            if (corner == BorderCorner.BottomRight) _settings.TaskRadii[BorderCorner.TopRight] = v;
        }
        else
        {
            if (corner == BorderCorner.TopLeft) _settings.MainRadii[BorderCorner.BottomLeft] = v;
            if (corner == BorderCorner.TopRight) _settings.MainRadii[BorderCorner.BottomRight] = v;
        }
    }
}
