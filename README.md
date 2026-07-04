# UsageMeter

화면 **테두리에 색 띠**로 Claude · Gemini 의 남은 사용량을 보여주는 macOS 메뉴바 오버레이 위젯.
사용량을 쓸수록 띠가 짧아져서, 창을 열지 않고도 곁눈질로 잔여량을 파악할 수 있다.

> A macOS menu-bar overlay that shows your remaining Claude / Gemini usage as colored bands along the screen edges.

<br>

## ✨ 주요 기능

- **화면 테두리 오버레이** — 잔여율만큼 색 띠가 채워지고, 쓸수록 앵커에서부터 깎인다.
- **멀티 AI** — Claude(코랄) · Gemini(블루)를 같은 레일에 겹치거나 AI별로 변을 나눠 표시.
- **메뉴바** — 잔여율 링 아이콘(기준 AI 선택 가능) + 아이콘 옆 % 표시(AI별) + 드롭다운에 5시간/주간 잔여·리셋 시각 등.
- **커스터마이즈** — 굵기·투명도·색(브랜드 프리셋)·모서리 곡률·**노치 감싸기**·선 끝 마감·% 차감 시작 위치/방향.
- **프리셋** 저장/불러오기(최대 10개) + 기본 상태 저장.
- **다국어** — 한국어 / English / 日本語 전환.
- 모든 설정은 저장되어 재시작해도 유지된다.

## 🧩 동작 원리

- 로그인 세션이 살아있는 **WKWebView 안에서 in-page `fetch`** 로 사용량을 읽어 Cloudflare를 통과한다(curl/URLSession은 403).
- Claude: `GET /api/organizations/{uuid}/usage` (5시간·주간 utilization). Gemini: `gemini.google.com/usage` DOM.
- 오버레이는 borderless 투명 창(screenSaver 레벨, 클릭 통과)에 SwiftUI **Canvas** 로 테두리를 그린다.
- 쿠키는 `WKWebsiteDataStore.default()`(로컬)에만 저장되고 **외부로 전송하지 않는다**.

## 🚀 실행 (개발)

전체 Xcode 없이 Command Line Tools의 Swift만으로 실행된다. **macOS 14+**.

```sh
swift run UsageMeter
```

메뉴바 아이콘 → **AI 로그인 / 로그아웃** 에서 Claude/Gemini 로그인 후 사용량이 표시된다.

## 📦 앱으로 패키징 (v1.0.0)

```sh
./build-app.sh          # UsageMeter.app 생성 (아이콘 = icon.png)
open UsageMeter.app
```

- 메뉴바 전용 앱(Dock 아이콘 없음, LSUIElement).
- ad-hoc 서명이라 첫 실행 시 **우클릭 → 열기** 가 필요할 수 있다.
- `.app` 은 `swift run` 과 쿠키·설정 저장소가 분리되므로 앱에서 새로 로그인해야 한다.

## 🗂 프로젝트 구조

```
usagemeter/
  Package.swift
  build-app.sh                  .app 패키징 (icon.png → .icns)
  icon.png                      앱 아이콘 원본
  Sources/UsageMeter/
    main.swift / AppDelegate.swift
    OverlayWindow.swift         투명·항상위·클릭통과 창
    BorderView.swift            Canvas 테두리 렌더 + BorderShape
    OverlaySettings.swift       공유 설정(영속화)
    Localization.swift          한/영/일 문자열
    Providers.swift / WebSession.swift / ProviderManager.swift
    StatusBarController.swift   메뉴바
    SettingsView.swift / SettingsWindowController.swift
    LoginWindowController.swift / PopupWebView.swift / HoverInfoController.swift
  PROGRESS.md                   개발 진행 기록
```

## ⚠️ 참고

- claude.ai / gemini 의 비공식 엔드포인트·DOM에 의존하므로, 상대가 구조를 바꾸면 조회가 깨질 수 있다.
- ChatGPT는 잔여 사용량을 노출하지 않아 지원하지 않는다.

## 🙌 후원

무료·오픈소스입니다. 도움이 되었다면 후원으로 응원해 주세요.

> 🚧 후원 채널 준비 중 — 계정이 열리면 이 자리에 링크(Ko-fi / Buy Me a Coffee / 토스 등)가 추가됩니다.
> 준비 템플릿은 [`.github/FUNDING.yml`](.github/FUNDING.yml) 에 있습니다.

## 📄 라이선스

[MIT](LICENSE)
