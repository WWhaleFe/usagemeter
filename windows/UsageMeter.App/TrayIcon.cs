using System.Drawing;
using System.Windows;
using Application = System.Windows.Application;
using UsageMeter.Core;
using WinForms = System.Windows.Forms;

namespace UsageMeter.App;

/// <summary>
/// 시스템 트레이 아이콘 + 컨텍스트 메뉴 — macOS StatusBarController(NSStatusItem) 대응.
/// 잔여율 링 아이콘을 GDI+로 그린다.
/// </summary>
public sealed class TrayIcon : IDisposable
{
    private readonly AppSettings _settings;
    private readonly ProviderManager _manager;
    private readonly OverlayController _overlays;
    private readonly WinForms.NotifyIcon _icon;

    private string T(string key) => Loc.Tr(key, _settings.Language);

    public TrayIcon(AppSettings settings, ProviderManager manager, OverlayController overlays)
    {
        _settings = settings;
        _manager = manager;
        _overlays = overlays;
        _icon = new WinForms.NotifyIcon
        {
            Visible = true,
            Text = "UsageMeter",
        };
        UpdateIcon();
        _icon.ContextMenuStrip = BuildMenu();
    }

    public void UpdateIcon()
    {
        // 첫 번째 로그인된 AI 기준 링 아이콘.
        UsageSnapshot? snap = null;
        (byte R, byte G, byte B) color = (128, 128, 128);
        foreach (var spec in ProviderSpec.All)
        {
            var s = _manager.SnapshotFor(spec.Id);
            if (s is { Ok: true }) { snap = s; color = spec.DefaultColor; break; }
        }
        _icon.Icon = DrawRingIcon(snap?.RemainingRatio ?? 0, Color.FromArgb(color.R, color.G, color.B));
        _icon.Text = snap != null ? $"UsageMeter — {snap.RemainingRatio * 100:F0}%" : "UsageMeter";
        _icon.ContextMenuStrip = BuildMenu();
    }

    private static Icon DrawRingIcon(double remaining, Color color)
    {
        const int size = 32;
        using var bmp = new Bitmap(size, size);
        using var g = Graphics.FromImage(bmp);
        g.SmoothingMode = System.Drawing.Drawing2D.SmoothingMode.AntiAlias;
        var rect = new RectangleF(4, 4, size - 8, size - 8);
        using var track = new Pen(Color.FromArgb(70, color), 5);
        g.DrawEllipse(track, rect);
        if (remaining > 0.005)
        {
            using var pen = new Pen(color, 5) { StartCap = System.Drawing.Drawing2D.LineCap.Round, EndCap = System.Drawing.Drawing2D.LineCap.Round };
            g.DrawArc(pen, rect, -90, (float)(360 * remaining));
        }
        var h = bmp.GetHicon();
        return Icon.FromHandle(h);
    }

    private WinForms.ContextMenuStrip BuildMenu()
    {
        var menu = new WinForms.ContextMenuStrip();

        // AI별 상태 + 로그인/로그아웃.
        foreach (var spec in ProviderSpec.All)
        {
            var snap = _manager.SnapshotFor(spec.Id);
            string status = snap is { Ok: true }
                ? $"{spec.Name}: {snap.RemainingRatio * 100:F0}%"
                : $"{spec.Name}: {T("acct.loggedOut")}";
            var item = new WinForms.ToolStripMenuItem(status);
            var login = new WinForms.ToolStripMenuItem(T("menu.login"));
            login.Click += async (_, _) => await _manager.ShowLoginAsync(spec.Id);
            var logout = new WinForms.ToolStripMenuItem(T("menu.logout"));
            logout.Click += async (_, _) =>
            {
                await _manager.SessionFor(spec.Id).LogoutAsync();
                await _manager.RefreshAllAsync();
            };
            item.DropDownItems.Add(login);
            item.DropDownItems.Add(logout);
            menu.Items.Add(item);
        }
        menu.Items.Add(new WinForms.ToolStripSeparator());

        var refresh = new WinForms.ToolStripMenuItem(T("menu.refresh"));
        refresh.Click += async (_, _) => await _manager.RefreshAllAsync();
        menu.Items.Add(refresh);

        var settings = new WinForms.ToolStripMenuItem(T("menu.settings"));
        settings.Click += (_, _) => SettingsWindow.Open(_settings, _manager, _overlays);
        menu.Items.Add(settings);

        var quit = new WinForms.ToolStripMenuItem(T("menu.quit"));
        quit.Click += (_, _) => Application.Current.Shutdown();
        menu.Items.Add(quit);
        return menu;
    }

    public void Dispose()
    {
        _icon.Visible = false;
        _icon.Dispose();
    }
}
