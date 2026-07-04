import AppKit
import WebKit

/// OAuth 팝업(예: "Google로 계속하기")을 위한 **진짜 자식 창** WebView.
///
/// 팝업을 메인 웹뷰에 합쳐버리면 `window.opener` 관계가 끊겨, 로그인 완료 시
/// 원래 창으로 결과를 돌려주는 마지막 단계가 실패한다. 그래서 별도 WKWebView를
/// **같은 configuration**(→ 프로세스 풀·쿠키 저장소 공유)으로 만들어 자식 창에 띄운다.
/// 로그인이 끝나면 팝업이 스스로 `window.close()`를 호출 → `webViewDidClose`에서 닫는다.
@MainActor
final class PopupWebView: NSObject, WKUIDelegate {

    let webView: WKWebView
    private let window: NSWindow
    /// 창이 닫힐 때(로그인 완료 등) 알림 — 소유자가 참조 정리용.
    var onClose: (() -> Void)?

    init(configuration: WKWebViewConfiguration, userAgent: String) {
        // 팝업(구글 로그인 등)에도 패스키 비활성화 주입 → 패스키 대신 비밀번호로 전환.
        configuration.userContentController.addUserScript(WebSession.disablePasskeyScript())
        webView = WKWebView(frame: NSRect(x: 0, y: 0, width: 480, height: 680), configuration: configuration)
        window = NSWindow(
            contentRect: webView.frame,
            styleMask: [.titled, .closable, .resizable],
            backing: .buffered, defer: false
        )
        super.init()
        webView.customUserAgent = userAgent
        webView.uiDelegate = self
        window.title = "로그인"
        window.isReleasedWhenClosed = false   // ARC가 잡고 있으므로 닫힐 때 이중 해제 방지(크래시 수정)
        window.contentView = webView
        window.center()
    }

    func present() {
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
    }

    /// 팝업이 또 팝업을 열면(중첩) 같은 창에서 잇는다.
    func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                 for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
        if navigationAction.targetFrame == nil { webView.load(navigationAction.request) }
        return nil
    }

    /// 팝업이 스스로 닫히면(로그인 완료) 자식 창도 닫는다.
    func webViewDidClose(_ webView: WKWebView) {
        window.close()
        onClose?()
    }
}
