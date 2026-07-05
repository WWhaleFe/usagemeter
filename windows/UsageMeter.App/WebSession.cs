using System.IO;
using System.Text.Json;
using System.Windows;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.Wpf;
using UsageMeter.Core;

namespace UsageMeter.App;

/// <summary>
/// AI 하나의 WebView2 세션 — macOS WebSession(WKWebView) 대응.
/// 숨은 WebView2에서 provider의 usage JS를 실행해 스냅샷을 얻는다.
/// 쿠키는 UserDataFolder(로컬)에만 저장, 외부 전송 금지.
/// </summary>
public sealed class WebSession : IAsyncDisposable
{
    private readonly ProviderSpec _spec;
    private Window? _host;              // WebView2는 HWND가 필요 — 화면 밖 숨김 창
    private WebView2? _webView;
    private bool _ready;

    public WebSession(ProviderSpec spec) => _spec = spec;

    private static string UserDataDir(string id) =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                     "UsageMeter", "WebView2", id);

    private async Task EnsureReadyAsync()
    {
        if (_ready && _webView != null) return;
        _webView = new WebView2();
        _host = new Window
        {
            Width = 900, Height = 700,
            ShowInTaskbar = false,
            WindowStyle = WindowStyle.None,
            ShowActivated = false,
            Left = -20000, Top = -20000,   // 화면 밖(숨김) — Visibility.Hidden은 WebView2 초기화 불가
            Content = _webView,
        };
        _host.Show();
        var env = await CoreWebView2Environment.CreateAsync(null, UserDataDir(_spec.Id));
        await _webView.EnsureCoreWebView2Async(env);
        _webView.CoreWebView2.Navigate(_spec.HomeUrl);
        await WaitForLoadAsync();
        _ready = true;
    }

    private Task WaitForLoadAsync(int timeoutMs = 20000)
    {
        var tcs = new TaskCompletionSource();
        void Handler(object? s, CoreWebView2NavigationCompletedEventArgs e)
        {
            _webView!.CoreWebView2.NavigationCompleted -= Handler;
            tcs.TrySetResult();
        }
        _webView!.CoreWebView2.NavigationCompleted += Handler;
        _ = Task.Delay(timeoutMs).ContinueWith(_ => tcs.TrySetResult());
        return tcs.Task;
    }

    /// <summary>usage JS를 실행해 스냅샷 반환. 로그인 안 됐으면 Ok=false.</summary>
    public async Task<UsageSnapshot> FetchAsync()
    {
        try
        {
            await EnsureReadyAsync();
            if (_spec.ReloadBeforeFetch)
            {
                _webView!.CoreWebView2.Reload();
                await WaitForLoadAsync();
                await Task.Delay(1500);   // DOM 렌더 대기(Gemini)
            }
            string raw = await _webView!.CoreWebView2.ExecuteScriptAsync(_spec.UsageJs);
            using var doc = JsonDocument.Parse(raw);
            return UsageSnapshot.Parse(_spec.Id, doc.RootElement, DateTimeOffset.Now);
        }
        catch
        {
            return new UsageSnapshot(_spec.Id, 0, null, null, null, null, null, false, DateTimeOffset.Now);
        }
    }

    /// <summary>로그인 창: 같은 UserDataFolder를 쓰는 보이는 창 — 로그인하면 세션 공유됨.</summary>
    public async Task ShowLoginAsync()
    {
        var wv = new WebView2();
        var win = new Window
        {
            Title = $"{_spec.Name} — Login",
            Width = 980, Height = 760,
            Content = wv,
        };
        win.Show();
        var env = await CoreWebView2Environment.CreateAsync(null, UserDataDir(_spec.Id));
        await wv.EnsureCoreWebView2Async(env);
        wv.CoreWebView2.Navigate(_spec.HomeUrl);
    }

    /// <summary>로그아웃: 쿠키 전부 삭제.</summary>
    public async Task LogoutAsync()
    {
        try
        {
            await EnsureReadyAsync();
            _webView!.CoreWebView2.CookieManager.DeleteAllCookies();
        }
        catch { /* 무시 */ }
    }

    public ValueTask DisposeAsync()
    {
        _host?.Close();
        _webView?.Dispose();
        return ValueTask.CompletedTask;
    }
}

/// <summary>모든 AI 세션 + 주기 갱신 — macOS ProviderManager + RefreshScheduler 대응.</summary>
public sealed class ProviderManager
{
    private readonly AppSettings _settings;
    private readonly Dictionary<string, WebSession> _sessions = new();
    private readonly Dictionary<string, UsageSnapshot> _snapshots = new();
    private System.Windows.Threading.DispatcherTimer? _timer;

    public event Action? SnapshotsChanged;

    public ProviderManager(AppSettings settings)
    {
        _settings = settings;
        foreach (var spec in ProviderSpec.All)
            _sessions[spec.Id] = new WebSession(spec);
    }

    public UsageSnapshot? SnapshotFor(string id) =>
        _snapshots.TryGetValue(id, out var s) ? s : null;

    public WebSession SessionFor(string id) => _sessions[id];

    public async Task StartAsync()
    {
        await RefreshAllAsync();
        _timer = new System.Windows.Threading.DispatcherTimer
        {
            Interval = TimeSpan.FromMinutes(Math.Max(1, _settings.RefreshMinutes)),
        };
        _timer.Tick += async (_, _) => await RefreshAllAsync();
        _timer.Start();
    }

    public async Task RefreshAllAsync()
    {
        foreach (var (id, session) in _sessions)
            _snapshots[id] = await session.FetchAsync();
        SnapshotsChanged?.Invoke();
    }
}
