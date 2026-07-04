import AppKit
import WebKit

/// 한 AI 서비스(ProviderSpec)의 로그인 상태를 유지하며 사용량을 읽는 조회 엔진.
///
/// 로그인된 WebView 안에서 조회(fetch/DOM)하므로 Cloudflare 등도 브라우저가 스스로 통과한다.
/// `WKWebsiteDataStore.default()`(비휘발성)라 재시작해도 로그인이 유지된다.
/// 평소엔 투명·클릭통과 호스트 창에 담겨 화면에 있으나 안 보인다(로드 유지). 로그인할 때만 보인다.
@MainActor
final class WebSession: NSObject, WKNavigationDelegate, WKUIDelegate {

    let spec: ProviderSpec
    let webView: WKWebView
    private let hostWindow: NSWindow

    private var lastLoadedMatches = false
    private var readyWaiters: [CheckedContinuation<Void, Never>] = []
    private var loadWaiters: [CheckedContinuation<Void, Never>] = []
    private var popups: [PopupWebView] = []

    private let desktopUA =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Safari/605.1.15"

    /// 패스키(WebAuthn) 비활성화 스크립트. 내장 WebView는 패스키 미지원이라, 구글 등이
    /// 패스키를 요구하면 막힌다. PublicKeyCredential을 없애 비밀번호 등으로 자동 전환시킨다.
    static func disablePasskeyScript() -> WKUserScript {
        let js = """
        (function () {
          try { Object.defineProperty(window, 'PublicKeyCredential', { value: undefined, configurable: true }); } catch (e) {}
          try {
            if (navigator.credentials) {
              navigator.credentials.get = function () { return Promise.reject(new DOMException('disabled', 'NotAllowedError')); };
              navigator.credentials.create = function () { return Promise.reject(new DOMException('disabled', 'NotAllowedError')); };
            }
          } catch (e) {}
        })();
        """
        return WKUserScript(source: js, injectionTime: .atDocumentStart, forMainFrameOnly: false)
    }

    init(spec: ProviderSpec) {
        self.spec = spec
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .default()
        config.userContentController.addUserScript(Self.disablePasskeyScript())
        self.webView = WKWebView(frame: NSRect(x: 0, y: 0, width: 460, height: 720), configuration: config)
        self.hostWindow = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 460, height: 720),
            styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false
        )
        super.init()
        webView.navigationDelegate = self
        webView.uiDelegate = self
        webView.customUserAgent = desktopUA

        webView.autoresizingMask = [.width, .height]
        hostWindow.title = "\(spec.name) 로그인"
        hostWindow.isReleasedWhenClosed = false
        hostWindow.contentView?.addSubview(webView)
        webView.frame = hostWindow.contentView?.bounds ?? webView.frame
        hostWindow.alphaValue = 0
        hostWindow.ignoresMouseEvents = true
        hostWindow.orderFrontRegardless()
    }

    func load() { webView.load(URLRequest(url: spec.homeURL)) }

    // MARK: - 로그인 창 표시/숨김

    func presentLoginWindow(delegate: NSWindowDelegate?) {
        hostWindow.delegate = delegate
        hostWindow.ignoresMouseEvents = false
        hostWindow.alphaValue = 1
        hostWindow.setContentSize(NSSize(width: 460, height: 720))
        hostWindow.center()
        load()
        NSApp.activate(ignoringOtherApps: true)
        hostWindow.makeKeyAndOrderFront(nil)
    }

    func dismissLoginWindow() {
        hostWindow.delegate = nil
        hostWindow.alphaValue = 0
        hostWindow.ignoresMouseEvents = true
        hostWindow.orderBack(nil)
    }

    // MARK: - 팝업(OAuth) 처리
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                 for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        let popup = PopupWebView(configuration: configuration, userAgent: desktopUA)
        popup.onClose = { [weak self, weak popup] in self?.popups.removeAll { $0 === popup } }
        popups.append(popup)
        popup.present()
        return popup.webView
    }

    // MARK: - 로드 추적
    private func matches() -> Bool {
        lastLoadedMatches && (webView.url?.host?.contains(spec.matchHost) ?? false)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        lastLoadedMatches = webView.url?.host?.contains(spec.matchHost) ?? false
        if lastLoadedMatches { resume(&readyWaiters) }
        resume(&loadWaiters)
    }

    private func resume(_ waiters: inout [CheckedContinuation<Void, Never>]) {
        let w = waiters; waiters = []; w.forEach { $0.resume() }
    }

    func ensureReady(timeoutSeconds: Double = 12) async {
        if matches() { return }
        let onOther = (webView.url != nil) && !(webView.url?.host?.contains(spec.matchHost) ?? false)
        if !onOther { load() }
        await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
            readyWaiters.append(c)
            Task { [weak self] in
                try? await Task.sleep(nanoseconds: UInt64(timeoutSeconds * 1_000_000_000))
                if let self { self.resume(&self.readyWaiters) }
            }
        }
    }

    /// 홈을 다시 로드하고 완료까지 기다린다(제미나이 /usage 최신 DOM 확보용).
    private func reloadAndWait(timeoutSeconds: Double = 12) async {
        load()
        await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
            loadWaiters.append(c)
            Task { [weak self] in
                try? await Task.sleep(nanoseconds: UInt64(timeoutSeconds * 1_000_000_000))
                if let self { self.resume(&self.loadWaiters) }
            }
        }
        try? await Task.sleep(nanoseconds: 1_500_000_000)   // 클라이언트 렌더 대기
    }

    // MARK: - 사용량 조회
    private func probeRaw() async -> [String: Any] {
        do {
            let result = try await webView.callAsyncJavaScript(spec.usageJS, arguments: [:], in: nil, contentWorld: .page)
            return (result as? [String: Any]) ?? ["ok": false, "reason": "bad_result"]
        } catch { return ["ok": false, "reason": "eval_error"] }
    }

    /// 로그인 여부(강제 이동 없이 현재 페이지에서 시도).
    func probeLoggedIn() async -> Bool { (await probeRaw())["ok"] as? Bool == true }

    func fetchUsage() async -> UsageSnapshot {
        await ensureReady()
        if spec.reloadBeforeFetch { await reloadAndWait() }
        let now = Date()
        let raw = await probeRaw()
        guard raw["ok"] as? Bool == true else {
            let reason = raw["reason"] as? String ?? "unknown"
            if reason == "not_logged_in" || reason == "no_data" { return snapshot(.authExpired, now) }
            return snapshot(.unavailable(reason), now)
        }
        let five = raw["five_hour"] as? [String: Any]
        let week = raw["seven_day"] as? [String: Any]
        let opus = raw["seven_day_opus"] as? [String: Any]
        return UsageSnapshot(
            id: spec.id, remainingRatio: remaining(five), secondaryRatio: week.map(remaining),
            opusRatio: opus.map(remaining),
            resetAt: resetDate(five), secondaryResetAt: resetDate(week),
            opusResetAt: resetDate(opus),
            status: .ok, lastUpdated: now
        )
    }

    func logout() async {
        let store = webView.configuration.websiteDataStore.httpCookieStore
        let cookies = await store.allCookies()
        for c in cookies where spec.cookieDomains.contains(where: { c.domain.contains($0) }) {
            await store.deleteCookie(c)
        }
        lastLoadedMatches = false
        load()
    }

    // MARK: - 변환
    private func remaining(_ bucket: [String: Any]?) -> Double {
        guard let u = (bucket?["utilization"] as? NSNumber)?.doubleValue else { return 1.0 }
        return max(0.0, min(1.0, 1.0 - u / 100.0))
    }
    private func resetDate(_ bucket: [String: Any]?) -> Date? {
        guard let s = bucket?["resets_at"] as? String else { return nil }
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return f.date(from: s) ?? { f.formatOptions = [.withInternetDateTime]; return f.date(from: s) }()
    }
    private func snapshot(_ status: UsageStatus, _ now: Date) -> UsageSnapshot {
        UsageSnapshot(id: spec.id, remainingRatio: 0, secondaryRatio: nil, opusRatio: nil,
                      resetAt: nil, secondaryResetAt: nil, opusResetAt: nil, status: status, lastUpdated: now)
    }
}
