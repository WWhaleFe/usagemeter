# UsageMeter

**English** → [README.en.md](README.en.md)

화면 **테두리에 색 띠**로 Claude · Gemini · Codex(ChatGPT 플랜)의 남은 사용량을 보여주는 macOS 메뉴바 오버레이 위젯.
사용량을 쓸수록 띠가 짧아져서, 창을 열지 않고도 곁눈질로 잔여량을 파악할 수 있다.

> A macOS menu-bar overlay that shows your remaining Claude / Gemini / Codex usage as colored bands along the screen edges.

## ⬇️ 다운로드 (macOS)

**[최신 릴리스 받기 (Releases)](https://github.com/WWhaleFe/usagemeter/releases/latest)** — 두 가지 형태로 제공합니다:

- **`UsageMeter-vX.X.X.zip`** — 압축 해제 후 앱을 **응용 프로그램**으로 드래그.
- **`UsageMeter-vX.X.X.pkg`** — **더블클릭 설치 관리자**(자동으로 `/Applications`에 설치).

유니버설 바이너리(애플실리콘 + 인텔), macOS 14+.

> ⚠️ 아직 애플 공증(notarize) 전이라 첫 실행 시 **우클릭 → 열기**가 필요합니다. 안 열리면 시스템 설정 → 개인정보 보호 및 보안에서 "그래도 열기". (`.pkg`도 설치 시 동일하게 우클릭 → 열기 또는 "그래도 열기"가 필요할 수 있습니다.)

> 🪟 **Windows 버전**도 개발 중입니다 — 다만 완성까지는 **상당한 시간이 걸릴 것으로 예상**됩니다.

<br>

## ✨ 주요 기능

- **화면 테두리 오버레이** — 잔여율만큼 색 띠가 채워지고, 쓸수록 앵커에서부터 깎인다.
- **멀티 AI** — Claude(코랄) · Gemini(블루) · Codex(틸). 같은 레일에 겹치거나 AI별로 선을 나눠 표시.
- **화면 분할** — 메뉴바선/Dock선으로 화면을 나누고 원하는 영역(메뉴바만, Dock만 등)의 둘레를 감싼다. 경계선은 화면 위에서 직접 드래그로 조정.
- **세그먼트 배치** — 가로 4개(상단/메뉴바선/Dock선/하단) + 세로 좌우 각 3구간을 인터랙티브 다이어그램에서 클릭해 선택. 한 줄로 이어진 조합만 허용, 모양 프리셋 원클릭.
- **모서리 표현** — 영역별 모서리 곡률(경계 연결 토글), 가로선 끝 위/아래 둥글게, 스쿱(오목 감싸기) 모서리, 노치 감싸기(자동 감지).
- **겹침 렌더링** — 같은 모양끼리는 급한(잔여 최소) 띠가 맨 위, 부분 겹침은 실제 겹치는 구간만 굵기를 나눠 나란히. 굵기 전환은 수백 pt에 걸쳐 평활되어 눈치챌 수 없다.
- **메뉴바** — 잔여율 링 아이콘(기준 AI 선택) + 아이콘 옆 %(AI별) + 드롭다운 정보 카드(고대비, 라이트/다크 자동)에 5시간·주간·모델별 주간 잔여·리셋 카운트다운·소진 예측·24시간 미니 차트(호버 시 값 표시).
- **구독 플랜·모델별 사용량** — AI 이름 옆에 구독 플랜 뱃지(예: Claude `Max (5x)`), 그리고 5시간(현재 세션)·주간(모든 모델)·모델별 주간(예: `Fable`) 잔여를 함께 표시. 서버가 주는 항목만 자동 표시된다.
- **분석 대시보드** — 메뉴/설정의 "분석 보기"로 열림. 기간(1일·3일·주간·한달)별 **AI별 소비 비중(도넛)**·**시간대별 사용 패턴**·요약(누적/일평균/가장 활발한 시간). Claude·Gemini·Codex를 **모두·일부·하나만** 골라 볼 수 있다.
- **외형(라이트/다크/시스템)** — 기기 설정을 따르거나 라이트·다크를 직접 선택. 메뉴·설정·호버 색이 테마에 맞게 바뀐다.
- **AI 표시 순서** — 드롭다운·호버·메뉴바 % 표시 순서를 통일해서 조절.
- **알림** — 사용률 75/90/95% 도달 시 macOS 알림.
- **후원** — 앱 안(설정 → 후원 탭)에 **GitHub Sponsors** 버튼 + **카카오페이 QR**(둘 다 수수료 없음). 설정·분석 창이 열려 있는 동안엔 Dock 아이콘으로도 다시 불러올 수 있다.
- **편의** — 로그인 탭(AI별 로그인/로그아웃), 자동 갱신 주기(1분~2시간+직접 입력), 프리셋 10개 + 기본 상태 저장, Mac 로그인 시 자동 실행, 한국어/English/日本語.
- 모든 설정은 저장되어 재시작해도 유지된다.

## 🧩 동작 원리

- 로그인 세션이 살아있는 **WKWebView 안에서 in-page `fetch`** 로 사용량을 읽어 Cloudflare를 통과한다(curl/URLSession은 403).
- Claude: `GET /api/organizations/{uuid}/usage` (5시간·주간 + `limits`의 모델별 `weekly_scoped` 버킷) + `rate_limit_tier`로 플랜 판별. Gemini: `gemini.google.com/usage` DOM. Codex: chatgpt.com 세션으로 공식 대시보드가 쓰는 내부 API(`backend-api/wham/usage`) 호출.
- 오버레이는 borderless 투명 창(screenSaver 레벨, 클릭 통과)에 SwiftUI **Canvas** 로 그린다. 테두리는 세그먼트 그래프로 모델링되고, 겹침 구간은 경로 트림 스팬으로 계산해 가우시안 평활로 굵기를 섞는다.
- 쿠키는 `WKWebsiteDataStore.default()`(로컬)에만 저장되고 **외부로 전송하지 않는다**.

## 🚀 실행 (개발)

전체 Xcode 없이 Command Line Tools의 Swift만으로 실행된다. **macOS 14+**.

```sh
swift run UsageMeter
```

메뉴바 아이콘 → **AI 로그인 / 로그아웃**(또는 설정창 로그인 탭)에서 로그인하면 사용량이 표시된다.

## 📦 앱으로 패키징

```sh
./build-app.sh          # 유니버설 UsageMeter.app 생성 (아이콘 = icon.png)
open UsageMeter.app
```

- 메뉴바 전용 앱(Dock 아이콘 없음, LSUIElement).
- 유니버설 바이너리: arm64 + x86_64를 `lipo`로 결합(Xcode 불필요).
- ad-hoc 서명이라 첫 실행 시 **우클릭 → 열기** 가 필요할 수 있다.
- `.app` 은 `swift run` 과 쿠키·설정 저장소가 분리되므로 앱에서 새로 로그인해야 한다.

## 🗂 프로젝트 구조

```
usagemeter/
  Package.swift
  build-app.sh                  .app 패키징 (유니버설, icon.png → .icns)
  icon.png                      앱 아이콘 원본
  Sources/UsageMeter/
    main.swift / AppDelegate.swift
    OverlayWindow.swift         투명·항상위·클릭통과 창
    BorderView.swift            Canvas 렌더 + SegmentChainShape(세그먼트 그래프)
    OverlaySettings.swift       공유 설정(영속화)
    Localization.swift          한/영/일 문자열
    Providers.swift / WebSession.swift / ProviderManager.swift
    StatusBarController.swift   메뉴바
    MenuInfoCardView.swift      드롭다운 정보 카드(고대비·플랜/모델 표시)
    SettingsView.swift / SettingsWindowController.swift
    SupportView.swift           후원 탭(GitHub Sponsors + 카카오페이 QR)
    AnalyticsView.swift / AnalyticsWindowController.swift   분석 대시보드
    DockPresence.swift          창 열림 시 Dock 아이콘 표시
    LineDragOverlay.swift       경계선 화면 드래그 조정
    HistoryStore.swift / MiniChartView.swift / NotificationManager.swift
    RefreshScheduler.swift
    LoginWindowController.swift / PopupWebView.swift / HoverInfoController.swift
  .github/FUNDING.yml           저장소 스폰서 버튼
  PROGRESS.md                   개발 진행 기록
```

## ⚠️ 참고

- claude.ai / gemini / chatgpt.com 의 비공식 엔드포인트·DOM에 의존하므로, 상대가 구조를 바꾸면 조회가 깨질 수 있다.
- ChatGPT의 **일반 대화** 한도는 어디에도 노출되지 않아 모니터링할 수 없다 — 대신 같은 플랜의 **Codex(에이전트) 사용량**을 지원한다.
- Gemini 조회는 아직 실험적이다.

## 📄 라이선스

[MIT](LICENSE)
