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

    /// <summary>usage JS를 실행해 스냅샷 반환. 로그인 안 됐으면 Ok=false.
    /// ExecuteScriptAsync는 Promise를 await하지 않으므로(빈 객체 반환)
    /// postMessage(WebMessageReceived)로 async 결과를 받는다.</summary>
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
            var tcs = new TaskCompletionSource<string>();
            void OnMessage(object? s, CoreWebView2WebMessageReceivedEventArgs e)
            {
                try { tcs.TrySetResult(e.TryGetWebMessageAsString()); }
                catch { tcs.TrySetResult("{\"ok\":false,\"reason\":\"bad_message\"}"); }
            }
            _webView!.CoreWebView2.WebMessageReceived += OnMessage;
            try
            {
                string wrapped = "(async () => { try { const r = await (" + _spec.UsageJs + "); " +
                    "window.chrome.webview.postMessage(JSON.stringify(r)); } catch (e) { " +
                    "window.chrome.webview.postMessage(JSON.stringify({ok:false,reason:'exception',message:String(e)})); } })(); null";
                await _webView.CoreWebView2.ExecuteScriptAsync(wrapped);
                var done = await Task.WhenAny(tcs.Task, Task.Delay(25000));
                if (done != tcs.Task)
                    return new UsageSnapshot(_spec.Id, 0, null, null, null, null, null, false, DateTimeOffset.Now);
                using var doc = JsonDocument.Parse(tcs.Task.Result);
                return UsageSnapshot.Parse(_spec.Id, doc.RootElement, DateTimeOffset.Now);
            }
            finally
            {
                _webView.CoreWebView2.WebMessageReceived -= OnMessage;
            }
        }
        catch
        {
            return new UsageSnapshot(_spec.Id, 0, null, null, null, null, null, false, DateTimeOffset.Now);
        }
    }

    /// <summary>로그인 창: 같은 UserDataFolder를 쓰는 보이는 창 — 로그인하면 세션 공유됨.
    /// 창을 닫으면 onClosed(자동 새로고침) 호출.</summary>
    public async Task ShowLoginAsync(Action? onClosed = null)
    {
        var wv = new WebView2();
        var win = new Window
        {
            Title = $"{_spec.Name} — Login",
            Width = 980, Height = 760,
            Content = wv,
        };
        // 순서 중요: ① 숨은 조회 웹뷰 세션 리셋(새 쿠키 반영) → ② onClosed(새로고침).
        win.Closed += (_, _) =>
        {
            if (_webView != null)
            {
                _host?.Close();
                _webView.Dispose();
                _webView = null;
                _ready = false;
            }
            onClosed?.Invoke();
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

    /// <summary>로그인 창을 열고, 닫히면 자동으로 전체 새로고침.</summary>
    public async Task ShowLoginAsync(string id) =>
        await _sessions[id].ShowLoginAsync(onClosed: async () => await RefreshAllAsync());

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
