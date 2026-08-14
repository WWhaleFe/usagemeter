# UsageMeter 메모리 점유 조사 보고서

작성일: 2026-08-14 · 대상: v1.2.5 (`UsageMeter.app`, 빌드 11)
측정 환경: macOS 26.2, Apple Silicon, 디스플레이 5376×3360(@2x, "2688×1680처럼 보임")

모든 수치는 **추정이 아니라 이 기기에서 직접 측정한 값**이다.
`phys_footprint`(= 활성 모니터의 '메모리' 열과 같은 값) 기준.

---

## 1. 현재 상태 (측정값)

앱을 실제로 실행하고 20분간 관측:

| 프로세스 | phys_footprint | 최대치 |
|---|---:|---:|
| `UsageMeter` (본체) | **330 MB** | 781 MB |
| `WebKit.WebContent` #1 | 354 MB | 434 MB |
| `WebKit.WebContent` #2 | 298 MB | 391 MB |
| `WebKit.WebContent` #3 | 450 MB | 536 MB |
| `WebKit.GPU` | 91 MB | 103 MB |
| `WebKit.Networking` | 29 MB | 239 MB |
| **합계** | **≈ 1.55 GB** | |

원인은 **딱 두 덩어리**다.

- **웹뷰 3개가 AI 사이트 SPA를 통째로 띄운 채 영구 상주** — 약 1.1 GB (71%)
- **오버레이 테두리 창의 전체화면 크기 레이어 백스토어 4장** — 약 280 MB (18%)

---

## 2. 원인 A — SPA 3개를 계속 로드해 둔다 (≈1.1 GB)

`ProviderManager.init()`이 시작하자마자 Claude·Gemini·Codex용 `WebSession`을 3개 만들고,
각각 `WKWebView`가 서비스 홈페이지를 **완전히 로드한 상태로 앱이 꺼질 때까지 유지**된다
(`WebSession.swift:41-68`, `ProviderManager.swift:24-27`).
WKWebView 하나 = `WebContent` 프로세스 하나이고, 그 안에 React/Angular 앱 전체가 살아 있다.

`vmmap` 상 각 WebContent의 300~450 MB 중 대부분이 `WebKit Malloc`(= DOM + JS 힙)이다.
이미지·폰트 같은 리소스가 아니라 **JS/DOM 그 자체**가 비용이다.

### A-1. Gemini — 재로드마다 계속 커진다 (최악)

`ProviderSpec.gemini`는 `reloadBeforeFetch: true`라서 **갱신 주기(기본 5분)마다 페이지를 통째로 재로드**한다
(`WebSession.swift:224`, `Providers.swift:89`).

같은 웹뷰를 14초 간격으로 6번 재로드하며 측정한 결과:

```
갓 로드했을 때   294 MB
재로드 1회 후    469 MB
재로드 2회 후    549 MB
재로드 3회 후    825 MB   ← 최대
재로드 4회 후    704 MB
재로드 5회 후    572 MB
안정화           510~550 MB
```

**사용자가 체감한 "제미나이 1기가"의 정체가 이것이다.** 페이지를 새로 그릴 때마다 이전 문서의
메모리가 완전히 회수되지 않고 누적된다(Google 페이지 쪽 문제라 우리가 고칠 수 없다).

### A-2. Codex — 로그인도 안 됐는데 283 MB를 쓴다

현재 이 기기는 ChatGPT에 로그인되어 있지 않다(실측: `/api/auth/session` → 401).
그런데도 `chatgpt.com` SPA는 로드된 채 **283 MB**를 점유한다.
**절대 결과를 못 주는 웹뷰가 283 MB를 쓰고 있는 것**이다.

### A-3. 핵심 발견 — Claude·Codex는 SPA가 아예 필요 없다

`Providers.swift`의 `usageJS`를 보면 Claude와 Codex는 **`fetch()`만 쓴다**. DOM을 전혀 안 본다.
필요한 것은 "그 오리진(claude.ai)에 속한 문서 안에서 실행"되는 것뿐이고, 그러면 쿠키가 실려 나간다.

그래서 `loadSimulatedRequest(_:response:responseData:)`로 **claude.ai 오리진의 빈 HTML 문서**를
띄워놓고 똑같은 `usageJS`를 실행하는 프로토타입을 만들어 검증했다:

| 방식 | WebContent | 조회 결과 |
|---|---:|---|
| 지금처럼 claude.ai SPA 전체 로드 | **314 MB** | `ok=1` + 5시간/주간 사용률·플랜 정상 반환 |
| **claude.ai 오리진의 빈 문서 (제안)** | **16 MB** | `ok=1` + 동일 필드 동일하게 정상 반환 |

**동일하게 정상 동작하면서 메모리는 1/20.** Cloudflare도 통과한다(같은 WebKit 네트워크 스택 +
같은 쿠키 저장소를 그대로 쓰기 때문). chatgpt.com도 같은 방식에서 15 MB였다(로그인 안 돼 있어
조회 정확성은 미검증이나, 호출하는 엔드포인트가 실제 페이지가 쓰는 것과 동일하다).

### A-4. 시도했다가 버린 방법 — 리소스 차단

`WKContentRuleList`로 image/media/font를 전부 차단하고 Gemini `/usage`를 로드해봤다:

```
차단 없음   294 MB
차단 적용   296 MB   ← 효과 없음
```

비용이 이미지가 아니라 JS/DOM이라 **의미 없다. 이 방향은 추천하지 않는다.**

---

## 3. 원인 B — 오버레이가 전체화면 백스토어를 4장 잡는다 (≈280 MB)

`vmmap`으로 본체 프로세스를 뜯어보면:

```
IOSurface  ...  [ 69.9M  69.9M  69.9M ] SurfaceID: 0xd1  5376x3360 (&BGA) 'QuartzCore'
IOSurface  ...  [ 69.9M  69.9M  69.9M ] SurfaceID: 0xd8  5376x3360 (&BGA) 'QuartzCore'
IOSurface  ...  [ 69.9M  69.9M  69.9M ] SurfaceID: 0xe0  5376x3360 (&BGA) 'QuartzCore'
IOSurface  ...  [ 69.9M  69.9M  69.9M ] SurfaceID: 0xe1  5376x3360 (&BGA) 'QuartzCore'
                                        총 279.6 MB (전부 dirty)
```

**5376×3360 = 화면 전체 해상도**다. 스트립 창 4개(vLeft·vRight·hTop·hBottom)가
각자 **화면 한 장짜리 레이어 백스토어**를 들고 있다.

원인은 `OverlayFrameController.swift:105`:

```swift
w.setFrame(f, display: true)                 // 창은 150pt(세로) / 170pt(가로) 짜리 얇은 띠
host.frame = CGRect(x: S.minX - f.minX, y: S.minY - f.minY,
                    width: S.width, height: S.height)   // ← 뷰는 화면 전체 크기
```

창은 얇지만 그 안의 `NSHostingView<BorderView>`가 **화면 전체 크기**로 배치되고(좌표를 맞추려고
일부러 어긋나게 둔 것), CoreAnimation은 그 레이어의 bounds대로 백스토어를 잡는다.
컨테이너의 `masksToBounds`는 **그리기만 자를 뿐 백스토어 크기는 안 줄인다.**

즉 눈에 보이는 건 150pt짜리 띠인데 메모리는 70 MB짜리 전체화면 한 장씩 쓰고 있다.
게다가 이건 **모니터 수만큼, 그리고 활성화된 선 개수만큼 배로 는다**
(메뉴선·Dock선을 켜면 스트립이 6개 → 약 420 MB, 외장 모니터를 붙이면 또 그만큼).

---

## 4. 원인 C — 웹뷰가 절대 유휴 절전에 들어가지 않는다 (CPU + 메모리 유지)

`WebSession.init()`은 숨김 창을 `alphaValue = 0` + `orderFrontRegardless()`로 띄운다
(`WebSession.swift:61-67`). WebKit 입장에서 이 창은 **"보이는 창"**이다.
그래서 백그라운드 타이머 스로틀링·페이지 절전이 전혀 걸리지 않는다.

실측: 아무것도 안 하는 유휴 상태에서 WebContent 프로세스들이 **각각 7~8% CPU를 지속 소모**했다.
Gemini/ChatGPT SPA의 애니메이션·폴링·웹소켓이 계속 도는 것이다. 배터리에도 직결된다.

---

## 5. 개선안 (우선순위 순)

### ① Claude·Codex를 "빈 오리진 문서 + fetch" 방식으로 전환 — 약 **566 MB** 절감 ✅검증완료

`WebSession.load()`를 provider 성격에 따라 갈라준다. `ProviderSpec`에 플래그 하나를 추가
(예: `needsRenderedDOM: Bool` — Claude/Codex는 `false`, Gemini는 `true`).

```swift
func load() {
    guard spec.needsRenderedDOM else { return loadStub() }
    webView.load(URLRequest(url: spec.homeURL))
}

/// SPA를 로드하지 않고, 해당 오리진에 속한 빈 문서만 띄운다.
/// 쿠키·credentials:'include'가 그대로 실려 usageJS의 fetch가 정상 동작한다.
private func loadStub() {
    let html = Data("<!doctype html><meta charset=utf-8><title>um</title>".utf8)
    let resp = URLResponse(url: spec.homeURL, mimeType: "text/html",
                           expectedContentLength: html.count, textEncodingName: "utf-8")
    webView.loadSimulatedRequest(URLRequest(url: spec.homeURL), response: resp, responseData: html)
}
```

- 로그인 창을 열 때만 `loadLogin()`으로 진짜 페이지를 로드하고, `dismissLoginWindow()`에서
  홈 대신 **stub으로 되돌린다**(`WebSession.swift:102`의 `load()` 호출이 그대로 stub이 됨).
- `matches()` / `ensureReady()` 판정은 `webView.url?.host`를 보므로 stub URL이 서비스 오리진이라
  **로직 수정이 거의 필요 없다.**
- `hasAuthCookie()`도 `websiteDataStore`를 직접 보므로 영향 없음.
- 314 MB → 16 MB, 283 MB → 15 MB.

### ② Gemini는 조회할 때만 웹뷰를 만들고 끝나면 파기 — 약 **510 MB** 절감 ✅검증완료

Gemini는 렌더된 DOM을 파싱해야 해서 stub을 못 쓴다. 대신 **상주를 없앤다.**

프로토타입으로 "로드 → 조회 → 웹뷰 해제" 사이클을 측정한 결과,
해제 직후 **WebContent 프로세스가 완전히 종료되고 294 MB가 전부 회수**됐다.

```
로드 후            294 MB
webView 해제 후    (프로세스 없음)
```

- 상주 510~550 MB → **0 MB**, 5분마다 약 10~12초 동안만 ~300 MB 피크(듀티 사이클 약 4%).
- 덤으로 **A-1의 무한 증가 문제가 원천 소멸한다**(매번 새 프로세스로 시작).
- 구현: `WebSession`이 `webView`를 `var webView: WKWebView?`로 갖고,
  `fetchUsage()` 진입 시 생성 → `ensureReady()` → `probeRaw()` → 마지막에
  `stopLoading()` / `removeFromSuperview()` / `webView = nil`.
  **로그인 창이 열려 있는 동안은 파기하지 않도록** 가드가 필요하다
  (`WebSession.swift:222`의 `loginWindowVisible` 체크와 같은 자리).

### ③ 오버레이 스트립을 창 크기 레이어로 — 약 **260 MB** 절감

`OverlayFrameController`가 `host.frame`을 화면 전체로 잡는 대신 **창 크기(스트립 크기)** 로 잡고,
`BorderView`가 그리기 원점을 옮기도록 바꾼다.

```swift
// OverlayFrameController.reconfigure()
host.frame = CGRect(origin: .zero, size: f.size)                 // 스트립 크기
host.rootView = BorderView(settings: settings, manager: manager,
                           screenSize: S.size,
                           // 화면 좌상단 기준 스트립 위치(SwiftUI는 y가 아래로 증가).
                           originInScreen: CGPoint(x: f.minX - S.minX,
                                                   y: S.maxY - f.maxY))
```

```swift
// BorderView.body 의 Canvas 안, 맨 앞에서
context.translateBy(x: -originInScreen.x, y: -originInScreen.y)
let rect = CGRect(origin: .zero, size: screenSize)   // 경로는 지금처럼 전체화면 좌표로 계산
```

- 세로 스트립 150×1680pt → 백스토어 약 **4 MB**, 가로 스트립 2388×170pt → 약 **6.5 MB**.
- 4장 합쳐 **280 MB → 약 21 MB**. 모니터·선 개수에 비례해 늘던 문제도 함께 사라진다.
- `WebKit.GPU` 프로세스(91 MB)도 이 표면들을 공유하고 있어 같이 줄어든다.
- 추가(선택): `vThick=150` / `hThick=170`은 "곡률 80 + 노치 100"의 최악값 상수다.
  현재 설정값에서 실제 필요한 두께를 계산해 쓰면 여기서 또 30~50% 줄어든다. 효과는 작다(수 MB).

### ④ 로그인 안 된 provider의 웹뷰는 아예 만들지 않는다 — 상황에 따라 최대 **283 MB**

`ProviderManager.init()`이 무조건 3개를 만든다. 로그인 이력이 없거나 사용자가 끈 provider는
`WebSession`을 지연 생성하고, 로그아웃 시 파기한다.
(①②를 적용하면 이 항목의 절대 효과는 작아지지만, 불필요한 프로세스가 안 뜨는 건 여전히 이득.)

### ⑤ 숨은 호스트 창을 `orderOut`으로 — CPU·배터리

`WebSession.init()`의 `orderFrontRegardless()`를 `orderOut(nil)`로 바꾸고, 조회/로그인 중에만 올린다.
①②를 적용하면 상주 웹뷰 자체가 없어져 대부분 무의미해지지만,
Gemini 조회 중(10초) 구간에는 여전히 의미가 있다.

### ⑥ (추가 조사 과제) Gemini도 fetch 전용으로 만들 수 있는지

`/usage` 페이지가 실제로 호출하는 XHR을 캡처해보니 데이터는 전부
`https://gemini.google.com/_/BardChatUi/data/batchexecute?rpcids=…` 로 온다.
관측된 rpcid: `otAQ7b, sJBwce, K4WWud, ozz5Z, whPPme, CNgdBe, I4z33b, GPRiHf, maGuAc,
Te6DCf, ESY5D, MaZiqc, VxUbXb, jSf9Qc, o30O0e, qpEbW, aPya6c, L5adhe`

이 중 사용량을 주는 rpcid를 특정하면 Gemini도 ①과 같은 16 MB짜리 fetch 전용 provider가 되고,
지금의 깨지기 쉬운 DOM 셀렉터 파싱도 함께 없앨 수 있다.
다만 `f.sid` / `bl`(빌드 라벨)을 페이지 HTML에서 뽑아야 하고 Google이 수시로 바꾸므로,
**DOM 파싱을 폴백으로 남긴 채** 별도 과제로 진행하는 것을 권한다. ②만으로도 효과는 충분하다.

---

## 6. 예상 결과

| | 현재 (실측) | ①②③ 적용 후 (예상) |
|---|---:|---:|
| UsageMeter 본체 | 330 MB | ~70 MB |
| WebContent 3개 | 354 + 298 + 450 MB | 16 + 16 MB (Gemini는 상주 안 함) |
| WebKit GPU / Networking | 120 MB | ~40 MB |
| **합계** | **≈ 1.55 GB** | **≈ 140 MB** |

(실행 중인 앱의 WebContent 3개는 프로세스만으로 어느 provider인지 구분되지 않는다.
provider별 수치는 아래 격리 측정값이 정확하다 — Claude 314 MB, ChatGPT 283 MB,
Gemini 294 MB(재로드 누적 시 825 MB). 450 MB짜리는 증가 양상으로 보아 Gemini일 가능성이 높다.)

**약 91% 감소.** 유휴 CPU도 20% 이상 → 거의 0에 수렴한다.

권장 적용 순서: **① → ② → ③**. ①과 ③은 서로 독립적이고 회귀 위험이 낮다.
②는 웹뷰 수명주기를 건드리므로 로그인 창이 열려 있는 동안의 가드를 반드시 함께 넣어야 한다.

---

## 7. 적용 결과 (2026-08-14, ①②③ 모두 반영)

동일한 디스플레이 구성(5376×3360)에서 앱을 다시 실행해 1시간 관측한 실측값:

| | 적용 전 | 적용 후 |
|---|---:|---:|
| UsageMeter 본체 | 330 MB | **65 MB** |
| ├ 창 백스토어(IOSurface) | 279.6 MB (5376×3360 ×4) | **21.7 MB** (320×3360 ×2, 4800×352 ×2) |
| WebContent (Claude·Codex 상주) | 354 + 450 MB | **16 + 16 MB** |
| WebContent (Gemini) | 298 MB (재로드 누적 시 825 MB) | **상주 0 MB**, 조회 중 4초만 ~360 MB |
| WebKit GPU / Networking | 120 MB | **24 MB** |
| **합계** | **≈ 1.55 GB** | **≈ 121 MB** |
| 유휴 CPU (WebContent 합) | 15~22% | **0.0%** |

**92% 감소.**

> ⚠️ 위 수치는 **메뉴바에 상주하며 사용량만 갱신하는 상태** 기준이다.
> 설정 창·세그먼트 다이어그램·분석 대시보드를 열어 쓰고 나면 앱 본체가 **147 MB**까지
> 올라가 합계 **약 197 MB**가 된다(실측). 늘어난 부분은 웹뷰도 오버레이 백스토어도 아니고
> (그 상태에서도 IOSurface는 11.2 MB), SwiftUI 창이 잡은 메모리가 창을 닫아도 바로
> 반환되지 않는 것이다. 이번 작업(①②③) 범위 밖의 별개 항목으로, 아래 "남은 과제"에 적는다.

메뉴선·Dock선을 모두 켜서 스트립이 6개가 된 구성에서도 백스토어는 띠 크기 그대로였다
(160×1600 ×2, 2272×192 ×2, 2272×160, 2272×128 등 합계 11.2 MB).
구버전이었다면 이 구성에서 화면 전체 버퍼가 6장(약 420 MB) 잡혔을 자리다.
같은 구성에서 테두리 모양·모서리 곡률·노치 우회가 이전과 동일함을 육안으로 확인했다.

### 동작 검증

- **Gemini 생성→조회→파기 사이클을 1초 간격으로 포착** (갱신 주기 tick = 13:25:31):
  ```
  13:25:24~29  상주 웹뷰 2개뿐 (Gemini 없음)
  13:25:31     새 WebContent  15 MB   ← 생성
  13:25:33                   339 MB   ← /usage 로드
  13:25:34                   363 MB   ← 피크
  13:25:35     프로세스 소멸           ← 파기, 전량 회수
  ```
  최종 유니버설 빌드에서도 동일하게 재현됐다(16:33:48 생성 → 216 MB 피크 → 16:33:54 소멸).
  5분 주기에서 **4~6초만** 존재한다(듀티 사이클 ≈ 1.5%).

- **조회 정확성**: `history.json`에 5분마다 두 provider가 모두 정상 기록됐고
  (12:35 · 12:40 · … · 13:25, 총 12회 연속), Claude 잔여율이 주기마다 실제로 변동해
  매번 새로 읽고 있음이 확인된다. 60분간 `fetch failed` 로그 0건.

- **오버레이 백스토어**: `vmmap` 상 전체화면 IOSurface가 사라지고 스트립 크기
  (320×3360 = 4.3 MB, 4800×352 = 6.7 MB)만 남았다.

### 변경 파일

| 파일 | 내용 |
|---|---|
| `Providers.swift` | `ProviderSpec.needsRenderedDOM` 추가(Gemini만 `true`) |
| `WebSession.swift` | `loadStub()`(①), 웹뷰 지연 생성·파기(`ensureWebView`/`releaseWebView`)(②), WebContent 강제 종료 복구 핸들러 |
| `OverlayFrameController.swift` | 호스팅 뷰를 스트립 크기로 + 그리기 원점 전달(③) |
| `BorderView.swift` | `screenSize`/`originInScreen` 파라미터 + Canvas 원점 이동(③) |

### 남은 과제

- 위 5항 ④(로그인 안 된 provider 웹뷰 생성 안 하기)와 ⑤(`orderOut`)는 미적용.
  ①② 적용 후 각각 16 MB 수준이라 효과가 작다.
- 위 5항 ⑥(Gemini batchexecute rpcid 특정)은 별도 과제로 남김.
- **설정·분석 창을 쓰고 나면 앱 본체가 49 MB → 147 MB로 늘고 창을 닫아도 돌아오지 않는다.**
  IOSurface는 11 MB뿐이라 오버레이와 무관하고, malloc·mapped file 쪽이다.
  SwiftUI 창(NSHostingView + Charts)의 해제 시점 문제로 보인다 — 미조사.
- 배포 시 `build-app.sh`의 `VERSION`/`BUILD` 상향 필요(업데이트 알림 기능이 버전 비교를 쓴다).

---

## 부록 — 재현 방법

측정에 쓴 명령:

```bash
open ./UsageMeter.app
PID=$(pgrep -x UsageMeter)
footprint -p $PID | grep phys_footprint          # 본체
vmmap $PID | grep QuartzCore                     # 전체화면 IOSurface 확인
for p in $(pgrep -f WebKit.WebContent); do footprint -p $p | grep -m1 phys_footprint:; done
```

검증용 프로토타입(빈 오리진 문서 / 파기 사이클 / 리소스 차단 / rpcid 캡처)은
`UsageMeter.app`과 같은 번들 ID(`com.usagemeter.app`)를 가진 별도 `.app`으로 만들어
같은 `WKWebsiteDataStore`(로그인 쿠키)를 공유한 상태에서 실행했다.
