using System.Windows;
using Application = System.Windows.Application;
using UsageMeter.Core;

namespace UsageMeter.App;

/// <summary>트레이 상주 앱 진입점 — macOS AppDelegate 대응.</summary>
public partial class App : Application
{
    private AppSettings _settings = null!;
    private ProviderManager _manager = null!;
    private OverlayController _overlays = null!;
    private TrayIcon _tray = null!;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        _settings = AppSettings.Load();
        _settings.EnsureDefaultLayouts();

        _manager = new ProviderManager(_settings);
        _overlays = new OverlayController(_settings, _manager);
        _tray = new TrayIcon(_settings, _manager, _overlays);

        _manager.SnapshotsChanged += () => Dispatcher.Invoke(() =>
        {
            _overlays.Redraw();
            _tray.UpdateIcon();
        });

        _overlays.ShowAll();
        _ = _manager.StartAsync();
    }

    protected override void OnExit(ExitEventArgs e)
    {
        _tray.Dispose();
        _settings.Save();
        base.OnExit(e);
    }
}
