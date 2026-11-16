import SwiftUI

// 실제 ProviderManager.swift는 WebSession(WKWebView)·LoginWindowController 등 UI 연결부를 끌고 오므로
// 이 도구에서는 BorderView가 읽는 `active`만 가진 대역으로 바꿔 컴파일한다.
// 나머지(BorderView, OverlaySettings, Provider, Providers, Localization)는 Sources/UsageMeter의 실제 파일을 쓴다.
@MainActor
final class ProviderManager: ObservableObject {
    var active: [(spec: ProviderSpec, snap: UsageSnapshot)] = []
}
