import AppKit
import WebKit

/// 한 AI 서비스(ProviderSpec)의 로그인 상태를 유지하며 사용량을 읽는 조회 엔진.
///
/// 로그인된 WebView 안에서 조회(fetch/DOM)하므로 Cloudflare 등도 브라우저가 스스로 통과한다.
/// `WKWebsiteDataStore.default()`(비휘발성)라 재시작해도 로그인이 유지된다.
///
/// ## 메모리 전략 (v1.2.6 — 웹뷰가 앱 메모리의 70%를 먹던 문제)
/// 서비스 홈페이지(SPA)를 통째로 띄워두면 WebContent 프로세스 하나가 300~800MB를 쓴다.
/// 그래서 provider 성격에 따라 두 가지 방식으로 나눈다:
///
/// - **fetch 전용**(Claude·Codex — `needsRenderedDOM == false`):
///   SPA를 아예 로드하지 않고, 서비스 오리진에 속한 **빈 문서**만 띄워둔다(`loadStub`).
///   `usageJS`가 `fetch()`만 쓰므로 오리진과 쿠키만 맞으면 동작한다. 314MB → 16MB.
/// - **DOM 파싱**(Gemini — `needsRenderedDOM == true`):
///   렌더된 화면이 필요해 빈 문서를 못 쓴다. 대신 **상주시키지 않는다**.
///   조회할 때만 웹뷰를 만들고 끝나면 파기해 WebContent 프로세스를 통째로 회수한다.
///   상주 510MB → 0MB(조회 중 10여 초만 피크). 재로드마다 메모리가 누적되던 문제도 함께 사라진다.
///
/// 평소엔 투명·클릭통과 호스트 창에 담겨 화면에 있으나 안 보인다. 로그인할 때만 보인다.
@MainActor
final class WebSession: NSObject, WKNavigationDelegate, WKUIDelegate {

    let spec: ProviderSpec
    /// 조회용 웹뷰. `needsRenderedDOM` provider는 조회/로그인 중에만 존재하고 그 외엔 nil이다.
    private(set) var webView: WKWebView?
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
        self.hostWindow = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 460, height: 720),
            styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false
        )
        super.init()
        hostWindow.title = "\(spec.name) 로그인"
        hostWindow.isReleasedWhenClosed = false
        hostWindow.alphaValue = 0
        hostWindow.ignoresMouseEvents = true
        // 조회 엔진용 숨은 창: 미션 컨트롤/Exposé·창 순환·창 메뉴에서 감춘다.
        // (.transient = 미션 컨트롤에 노출 안 됨. 웹뷰는 창에 그대로 살아 조회는 계속 동작.)
        hostWindow.collectionBehavior = [.transient, .ignoresCycle]
        hostWindow.isExcludedFromWindowsMenu = true
        hostWindow.orderFrontRegardless()

        // fetch 전용 provider는 웹뷰가 빈 문서라 상주해도 싸다(16MB) — 미리 만들어 둔다.
        // DOM 파싱 provider는 조회 시점까지 만들지 않는다(그게 이 클래스의 메모리 절감 핵심).
        if !spec.needsRenderedDOM { _ = ensureWebView() }
    }

    // MARK: - 웹뷰 수명주기

    /// 웹뷰가 없으면 만들어 호스트 창에 붙인다. 이미 있으면 그대로 돌려준다.
    @discardableResult
    private func ensureWebView() -> WKWebView {
        if let wv = webView { return wv }
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .default()
        config.userContentController.addUserScript(Self.disablePasskeyScript())
        let wv = WKWebView(frame: NSRect(x: 0, y: 0, width: 460, height: 720), configuration: config)
        wv.navigationDelegate = self
        wv.uiDelegate = self
        wv.customUserAgent = desktopUA
        wv.autoresizingMask = [.width, .height]
        hostWindow.contentView?.addSubview(wv)
        wv.frame = hostWindow.contentView?.bounds ?? wv.frame
        webView = wv
        lastLoadedMatches = false
        return wv
    }

    /// 웹뷰를 완전히 파기해 WebContent 프로세스 메모리를 OS에 돌려준다.
    /// 로그인 창이 떠 있는 동안엔 절대 파기하지 않는다(로그인 절차가 통째로 날아간다).
    private func releaseWebView() {
        guard !loginWindowVisible, let wv = webView else { return }
        wv.stopLoading()
        wv.navigationDelegate = nil
        wv.uiDelegate = nil
        wv.removeFromSuperview()
        webView = nil
        lastLoadedMatches = false
    }

    // MARK: - 로드

    func load() {
        // 아직(또는 더 이상) 웹뷰가 없는 provider는 조회 시점에 만들어 로드한다.
        guard let wv = webView else { return }
        guard spec.needsRenderedDOM else { return loadStub(into: wv) }
        wv.load(URLRequest(url: spec.homeURL))
    }

    /// SPA를 로드하지 않고 **서비스 오리진에 속한 빈 문서**만 띄운다.
    ///
    /// `usageJS`가 `fetch()`만 쓰는 provider(Claude·Codex)는 문서 내용이 필요 없다.
    /// 오리진만 맞으면 `credentials:'include'` 요청에 쿠키가 그대로 실리고, WebKit 네트워크
    /// 스택을 쓰므로 Cloudflare도 실제 페이지와 똑같이 통과한다.
    private func loadStub(into wv: WKWebView) {
        let url = spec.homeURL
        let html = Data("<!doctype html><meta charset=\"utf-8\"><title>UsageMeter</title>".utf8)
        let response = URLResponse(url: url, mimeType: "text/html",
                                   expectedContentLength: html.count, textEncodingName: "utf-8")
        wv.loadSimulatedRequest(URLRequest(url: url), response: response, responseData: html)
    }

    /// 로그인 창에서 여는 주소(서비스에 따라 홈이 아닐 수 있다 — ProviderSpec.loginURL 참고).
    func loadLogin() { ensureWebView().load(URLRequest(url: spec.effectiveLoginURL)) }

    // MARK: - 로그인 창 표시/숨김

    /// 로그인 창이 화면에 떠 있는지(로드 실패 안내를 띄울지 판단에 쓴다).
    private var loginWindowVisible: Bool { hostWindow.alphaValue > 0 }

    func presentLoginWindow(delegate: NSWindowDelegate?) {
        hostWindow.delegate = delegate
        hostWindow.ignoresMouseEvents = false
        hostWindow.alphaValue = 1
        // 로그인 중엔 일반 창으로(사용자가 다루고 미션 컨트롤에서도 정상 취급).
        hostWindow.collectionBehavior = [.managed]
        hostWindow.setContentSize(NSSize(width: 460, height: 720))
        hostWindow.center()
        loadLogin()
        NSApp.activate(ignoringOtherApps: true)
        hostWindow.makeKeyAndOrderFront(nil)
    }

    func dismissLoginWindow() {
        hostWindow.delegate = nil
        hostWindow.alphaValue = 0
        hostWindow.ignoresMouseEvents = true
        // 다시 숨은 엔진 창으로: 미션 컨트롤에서 감춤.
        hostWindow.collectionBehavior = [.transient, .ignoresCycle]
        hostWindow.orderBack(nil)
        // DOM 파싱 provider는 로그인이 끝나면 웹뷰를 통째로 버린다(다음 조회 때 새로 만든다).
        if spec.needsRenderedDOM { return releaseWebView() }
        // 로그인을 도중에 닫았다면 웹뷰가 로그인 페이지(accounts.google.com 등)에 머문다.
        // 그대로 두면 이후 조회가 계속 그 페이지에서 실패하므로 서비스 오리진으로 되돌린다.
        if !(webView?.url?.host?.contains(spec.matchHost) ?? false) { load() }
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
        lastLoadedMatches && (webView?.url?.host?.contains(spec.matchHost) ?? false)
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        lastLoadedMatches = webView.url?.host?.contains(spec.matchHost) ?? false
        if lastLoadedMatches { resume(&readyWaiters) }
        resume(&loadWaiters)
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        handleLoadFailure(error)
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        handleLoadFailure(error)
    }

    /// 로드 실패 처리: 기다리는 조회를 즉시 깨우고, 로그인 창이라면 흰 화면 대신 이유를 보여준다.
    private func handleLoadFailure(_ error: Error) {
        let ns = error as NSError
        // 리다이렉트·재로드로 인한 취소는 실패가 아니다.
        guard ns.code != NSURLErrorCancelled else { return }
        NSLog("[UsageMeter] %@ load failed (%ld): %@", spec.id, ns.code, error.localizedDescription)
        resume(&loadWaiters)
        resume(&readyWaiters)
        guard loginWindowVisible, let wv = webView else { return }
        let url = spec.effectiveLoginURL.absoluteString
        let detail = error.localizedDescription
            .replacingOccurrences(of: "&", with: "&amp;")
            .replacingOccurrences(of: "<", with: "&lt;")
        wv.loadHTMLString("""
        <html><head><meta name="viewport" content="width=device-width,initial-scale=1">
        <style>body{font:14px -apple-system,sans-serif;color:#333;padding:32px 24px;line-height:1.6}
        a{display:inline-block;margin-top:16px;padding:8px 16px;background:#0a66ff;color:#fff;
        border-radius:8px;text-decoration:none}</style></head><body>
        <b>페이지를 열지 못했습니다 · Couldn't load the page</b>
        <p>\(detail)</p><a href="\(url)">다시 시도 · Retry</a></body></html>
        """, baseURL: nil)
    }

    /// WebContent 프로세스가 죽었을 때(메모리 압박에 의한 강제 종료 등) 빈 웹뷰로 남지 않게 복구한다.
    /// 상주 웹뷰는 이제 빈 문서라 아주 가벼운 대신, 시스템이 회수 대상으로 삼기도 더 쉽다.
    /// 이 처리가 없으면 이후 모든 조회가 eval_error로 조용히 실패한다.
    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        NSLog("[UsageMeter] %@ web content process terminated — reloading", spec.id)
        lastLoadedMatches = false
        resume(&readyWaiters)
        resume(&loadWaiters)
        // 조회 때만 웹뷰를 만드는 provider는 그냥 버린다(다음 조회에서 새로 만든다).
        if spec.needsRenderedDOM { return releaseWebView() }
        load()
    }

    private func resume(_ waiters: inout [CheckedContinuation<Void, Never>]) {
        let w = waiters; waiters = []; w.forEach { $0.resume() }
    }

    func ensureReady(timeoutSeconds: Double = 12) async {
        if matches() { return }
        let wv = ensureWebView()
        let onOther = (wv.url != nil) && !(wv.url?.host?.contains(spec.matchHost) ?? false)
        if !onOther { load() }
        await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
            readyWaiters.append(c)
            Task { [weak self] in
                try? await Task.sleep(nanoseconds: UInt64(timeoutSeconds * 1_000_000_000))
                if let self { self.resume(&self.readyWaiters) }
            }
        }
    }

    /// 홈을 다시 로드하고 완료까지 기다린다(상주 웹뷰에서 최신 DOM을 확보할 때).
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
        guard let wv = webView else { return ["ok": false, "reason": "no_webview"] }
        do {
            let result = try await wv.callAsyncJavaScript(spec.usageJS, arguments: [:], in: nil, contentWorld: .page)
            return (result as? [String: Any]) ?? ["ok": false, "reason": "bad_result"]
        } catch { return ["ok": false, "reason": "eval_error"] }
    }

    /// 사용량까지 정상적으로 읽히는지(= 가장 확실한 로그인 상태). 강제 이동 없이 현재 페이지에서 시도.
    ///
    /// 웹뷰를 상주시키지 않는 provider는 여기서 웹뷰를 새로 띄우지 않는다 — 이 호출은
    /// "로그인 창을 열기 전 빠른 확인"용이라, 그러자고 300MB짜리 페이지를 로드하면 본말전도다.
    /// 대신 인증 쿠키 유무로 판정한다(제미나이의 로그인 판정 기준과 같다).
    func probeUsageOK() async -> Bool {
        guard webView != nil else {
            return spec.authCookieNames.isEmpty ? false : await hasAuthCookie()
        }
        return (await probeRaw())["ok"] as? Bool == true
    }

    /// 로그인 창을 닫아도 되는지 판정.
    ///
    /// 제미나이처럼 사용량 조회가 DOM 파싱인 서비스는 화면 구조가 바뀌면 "로그인했는데도
    /// 조회 실패"가 나서, 조회 성공을 로그인 판정에 쓰면 창이 영영 안 닫힌다.
    /// 그래서 그런 서비스는 **인증 쿠키 + 서비스 도메인 복귀**로 판정한다.
    func probeLoginDone() async -> Bool {
        guard !spec.authCookieNames.isEmpty else { return await probeUsageOK() }
        // 로그인 절차 중(accounts.google.com 등)에 미리 닫히지 않도록 서비스 도메인 복귀를 함께 본다.
        guard webView?.url?.host?.contains(spec.matchHost) ?? false else { return false }
        return await hasAuthCookie()
    }

    private func hasAuthCookie() async -> Bool {
        let cookies = await WKWebsiteDataStore.default().httpCookieStore.allCookies()
        return cookies.contains { c in
            spec.authCookieNames.contains(c.name)
                && spec.cookieDomains.contains(where: { c.domain.contains($0) })
        }
    }

    func fetchUsage() async -> UsageSnapshot {
        // 로그인 창이 떠 있는 동안엔 조회하지 않는다.
        // 조회와 로그인이 **같은 웹뷰**를 쓰기 때문에, 로그인 중에 주기 갱신이 끼어들면
        // reload가 구글 로그인 화면을 덮어써서 로그인 절차가 통째로 날아간다
        // (사용자가 본 "Gemini 로그인 창이 흰 화면으로 멈춤"의 원인).
        if loginWindowVisible { return snapshot(.unavailable("logging_in"), Date()) }

        // 이번 조회에서 웹뷰를 새로 만들었다면(= 상주하지 않는 provider), 끝나고 반드시 파기한다.
        let freshlyCreated = (webView == nil)
        defer { if spec.needsRenderedDOM { releaseWebView() } }

        await ensureReady()
        if spec.reloadBeforeFetch && !freshlyCreated {
            // 상주 웹뷰에 남아 있는 옛 DOM을 버리고 새로 그린다.
            await reloadAndWait()
        } else if spec.needsRenderedDOM {
            // 갓 만든 웹뷰는 이미 최신이다 — 클라이언트 렌더 시간만 준다.
            try? await Task.sleep(nanoseconds: 1_500_000_000)
        }
        let now = Date()
        let raw = await probeRaw()
        guard raw["ok"] as? Bool == true else {
            let reason = raw["reason"] as? String ?? "unknown"
            // 실패 원인은 항상 남긴다(사용자 제보 때 원인 파악이 가능하도록).
            NSLog("[UsageMeter] %@ fetch failed reason=%@ host=%@ status=%@ snippet=%@",
                  spec.id, reason,
                  raw["host"] as? String ?? "-",
                  String(describing: raw["status"] ?? "-"),
                  raw["snippet"] as? String ?? "-")
            // 진짜 미로그인일 때만 재로그인을 유도한다.
            if reason == "not_logged_in" { return snapshot(.authExpired, now) }
            // 로그인은 됐으나 화면/응답을 못 읽음(예: Gemini DOM 변경, 서버 응답 없음) →
            // '재로그인 필요'가 아니라 '못 읽음'으로 구분해 오안내를 막는다.
            return snapshot(.unavailable(reason), now)
        }
        let five = raw["five_hour"] as? [String: Any]
        let week = raw["seven_day"] as? [String: Any]
        let opus = raw["seven_day_opus"] as? [String: Any]
        // 구독 플랜 + 모델별 주간 버킷(예: Claude "Fable") — 제공자가 주면 반영.
        let plan = (raw["plan"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        let buckets = (raw["model_buckets"] as? [[String: Any]])?.compactMap { b -> UsageModelBucket? in
            guard let label = b["label"] as? String, !label.isEmpty else { return nil }
            return UsageModelBucket(label: label, remainingRatio: remaining(b), resetAt: resetDate(b))
        }
        return UsageSnapshot(
            id: spec.id, remainingRatio: remaining(five), secondaryRatio: week.map(remaining),
            opusRatio: opus.map(remaining),
            resetAt: resetDate(five), secondaryResetAt: resetDate(week),
            opusResetAt: resetDate(opus),
            status: .ok, lastUpdated: now,
            plan: plan, modelBuckets: (buckets?.isEmpty == true ? nil : buckets)
        )
    }

    func logout() async {
        let store = WKWebsiteDataStore.default().httpCookieStore
        let cookies = await store.allCookies()
        for c in cookies where spec.cookieDomains.contains(where: { c.domain.contains($0) }) {
            await store.deleteCookie(c)
        }
        lastLoadedMatches = false
        // 상주하지 않는 provider는 웹뷰가 없다 — 다음 조회 때 새로 만들면 된다.
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
