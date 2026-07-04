# 진행 상황 & 다음 단계

이 파일은 **매 단계 끝날 때마다 갱신**한다. 초보자 기준으로 "지금 어디까지 했고, 바로 다음에 뭘 하면 되는지"를 항상 여기서 확인할 수 있게 유지하는 것이 목적.

---

## 🔖 다음 세션에서 여기부터 시작 (2026-07-03 마지막 지점)

**상태:** PoC ①② 합격 + **MVP 그룹 1~4 + 로그인 실동작 완료.** 메뉴바에 실제 잔여율(예 5시간 95%) 표시 + 오버레이 띠가 잔여율만큼 깎여 렌더 + **앱 재시작해도 세션 유지** 확인(2026-07-04).

**로그인 = "브라우저로 로그인"(권장, 실동작 검증됨):** 내장 WebView가 Google/패스키를 막아서, **기본 브라우저에서 로그인(패스키/구글 자유) → sessionKey 복사 → 앱에 붙여넣기** 방식으로 전환. 앱이 그 키를 WKWebView 쿠키스토어에 주입(`injectSessionKey`)하면, WebView가 claude.ai 로드하며 **Cloudflare를 스스로 통과**하고 인증됨(예전 curl 방식의 403 문제 없음). 쿠키는 `WKWebsiteDataStore.default()`에 저장돼 재시작 후에도 유지. 메뉴 `브라우저로 로그인… (권장)` + `앱 안에서 로그인…`(폴백) 병행.

**핵심 버그 3종 수정(로그인이 계속 안 되던 원인):** ① 엔진 WKWebView가 **어떤 창에도 없으면 로드가 멈춤** → 화면 밖 호스트 창(`ClaudeSession.hostWindow`)에 상주시켜 해결(시작 조회·재조회 안정). ② OAuth 팝업을 메인에 합치면 opener 끊김 → 자식 창(`PopupWebView`). ③ 시작 조회 타이밍 → 재시도 루프.

**MVP 그룹 1~4 (사용자 피드백 반영, 2026-07-03):**
- **그룹 1 로그인 버그 수정:** 절대경로 `https://claude.ai/api/...` + 로그인 **폴링(1.2초)** + `ensureReady()`(claude.ai 로드 보장)로 "자동 닫힘 안 됨/여러 번 눌러야 표시" 해결. `ClaudeSession`이 navigationDelegate 겸함.
- **그룹 2 굵기·색:** 기본 굵기 6→**3pt(얇게)**, 기본색 초록. 메뉴 `선 색`(프리셋+직접선택/NSColorPanel)·`선 굵기`.
- **그룹 3 면 선택·둥근 모서리:** `BorderShape`(Shape)로 선택된 변만 그림. 인접 변은 둥글게 잇고 한쪽 끝은 직각 마감. 메뉴 `변(면) 선택`(4변 토글+프리셋 ㄷ자/ㄴ자 등)·`둥근 모서리`(0~32). 스크린샷으로 둥근 모서리 렌더 확인.
- **그룹 4 잔여율 띠 깎임:** 선택 변들을 **하나의 연속 경로**로 만들어 `.trim(0, ratio)`로 좌상단 앵커부터 잔여율만큼만 채움 + 옅은 트랙 배경(0.18). refresh()가 `settings.ratio`에 remainingRatio 반영. 24pt·0.5로 "상+우만 채워짐" 검증 완료. 메뉴 `트랙 배경 표시` 토글.
- 파일: `OverlaySettings.swift`(공유 설정, ObservableObject) 추가. `BorderView.swift`/`OverlayWindow.swift`/`AppDelegate.swift`/`StatusBarController.swift` 갱신.

**로그인 방식 수정 (2026-07-03):** 내장 WKWebView 인증 벽 대응 — ① Safari User-Agent 위장(구글 "안전하지 않은 브라우저" 회피) ② **OAuth 팝업을 진짜 자식 창(`PopupWebView.swift`)으로** 띄워 opener 관계 유지(합치면 로그인 완료 신호 끊김) → Google 앱 내 완료 가능. **패스키는 내장 WebView 구조상 불가**(Apple web-browser 엔타이틀먼트/associated-domains 필요, 획득 불가). 확실한 대안 = **이메일 로그인 코드**. 로그아웃 버튼은 로그아웃 상태면 비활성화(menu `autoenablesItems=false`).

**앱 실행:** `cd ~/usagemeter && nohup swift run UsageMeter >/tmp/usagemeter.log 2>&1 & disown`. 종료는 메뉴 "종료" 또는 `pkill -f UsageMeter`. (print는 파이프로 버퍼링돼 로그에 잘 안 뜸 — 상태 확인은 스크린샷 권장.)

**디자인 개선 (2026-07-04, 검증 완료):** ① 기본 선 굵기 3→**2pt**(1pt 옵션 추가). ② **모서리별 곡률 개별 조정**(`cornerRadii: [BorderCorner: CGFloat]`, 좌상/우상/우하/좌하) — 메뉴 `모서리 곡률`(전체 프리셋 + 모서리별 프리셋/직접입력). 맥미니 외장모니터처럼 위만 둥근 경우 대응. ③ **노치 감싸기**: 상변이 노치를 아래로 우회(사각 detour). `NSScreen.safeAreaInsets.top`+`auxiliaryTopLeft/RightArea`로 자동 감지(노치 있으면 자동 on), 너비/높이 사용자 조절(메뉴 `노치 감싸기`: 토글+너비 프리셋/직접입력+높이 직접입력). 맥미니는 노치 없어 자동감지 nil→수동 on. BorderShape 재작성(per-corner 접점 + 상변 polyline에 노치 detour, trim 호환 유지). 숫자 정밀입력은 `promptNumber` NSAlert.

**설정 창 + 세부 조정 (2026-07-04):** SwiftUI **설정 창**(`SettingsView`/`SettingsWindowController`, 메뉴 "설정 창 열기…" ⌘,) 추가 — 모든 값 슬라이더+**숫자입력칸+Stepper(화살표)**로 조절(#1). 반영 완료 7건: ①슬라이더 화살표/숫자입력 ②단일 변 선택해도 끝 모서리 곡률 포함(BorderShape runVertices에 이웃 접점 stub+둥근corner, 반경>0일 때만; 검증됨) ③④색 프리셋을 AI 브랜드색으로(Claude/ChatGPT/Gemini/Copilot/Perplexity/Mistral/Grok+초록/흰) ⑤"맥북 Air 15용" 명명 프리셋=기본값(`applyMacBookAir15`, `presetNames`/`applyPreset`) ⑥앵커=잔여율 시작 지점 선택(`anchorCorner`/`anchorSide`(곡률좌/꼭짓점/곡률우)/`anchorClockwise`; BorderShape closed 렌더에서 순서 재배열+방향) ⑦커서를 테두리에 올리면 정보 패널(`HoverInfoController`, 전역 마우스 모니터로 선택된 변 근처 감지→borderless 패널, refresh가 infoText 갱신). BorderShape을 "정점별 곡률 일반화"로 재작성해 본체/노치/부분선택/앵커 모두 일관 처리.

**세부 조정 2차 (2026-07-04):** ①앵커 방향 수정(시계/반시계 뒤집힘 → 소모 방향 기준으로 정정, BorderShape reorder). ②**설정 저장/불러오기 프리셋**(`SavedPreset`/`SettingsSnapshot` Codable, UserDefaults 영속, 최대 5개, 저장/불러오기/삭제 — 설정창 UI + 메뉴 프리셋). ③부분 변 선택 곡률 훅 짧게(hookMax 8) + **소모 끝부분 투명 그라데이션**(BorderView에서 tail을 12조각 점점 투명하게 trim). ④좌상·우상 곡률 기본 22 ⑤우하·좌하 0 ⑥노치 기본 너비 186·깊이 33 ⑦노치 곡률 위(바깥) 5·아래(안쪽) 12 ⑧기존 "맥북 Air 15용" 명명 프리셋 제거→기본 init 값으로, 프리셋은 사용자 저장식으로 대체 ⑨저장 5개 제한, 저장분 있으면 불러오기/삭제 활성. 색 프리셋은 AI 브랜드색(이전 커밋). 모두 빌드+스크린샷 검증(방향·기본값·설정창 UI).

**세부 조정 3차 (2026-07-04):** ①부분 변 선택 시 시작/끝 끝에 **곡률 모서리 추가 토글**(`extendStartCorner`/`extendEndCorner`) — 켜면 이웃 변으로 곡선이 뻗고 그 끝점이 띠 시작점. 기존 자동 훅 제거하고 토글 제어로. ②`startCornerAbove`(곡률 시작점 위/아래 — 현재 구현은 '아래=이웃변 접점' 확장, '위'는 꼭짓점 시작. 미세감 사용자 피드백 필요). ③**선 투명도**(`lineOpacity` 0~1, BorderView에 `.opacity()`, 설정창 슬라이더). 스냅샷(저장 프리셋)에 세 값 옵셔널로 추가(구버전 호환). 검증: 좌상 시작 곡률+반투명 스샷 확인. **주의: 끝 곡률 추가는 밴드 끝 페이드와 겹쳐 흐려질 수 있음(정상).**

**영속화·패키징·아이콘 (2026-07-04):** ①**모든 설정 UserDefaults 영속화**(`currentKey`, objectWillChange 300ms 디바운스 저장, init에서 로드·적용). ②시작 꼭짓점은 **유효한 것만 선택**(`validAnchorCorners`=루프면 4개/부분선택은 run 양 끝, 픽커 필터+edges didSet clampAnchorCorner). ③**`.app` 패키징**: `build-app.sh`(swift build -c release → UsageMeter.app + Info.plist LSUIElement/bundleId `com.usagemeter.app` + ad-hoc codesign). 더블클릭/`open UsageMeter.app` 실행 확인. ④메뉴바 **잔여율 링(도넛) 아이콘**(`ringImage`, 테두리 색 매칭, 잔여율만큼 12시부터 시계방향 채움, 색변경 시 objectWillChange로 갱신; 로그아웃=흐린 회색 링). **주의: `.app`은 swift-run과 쿠키·UserDefaults 분리 → .app에서 새로 로그인·설정 필요.** 미서명이라 첫 실행 시 우클릭>열기 필요할 수 있음.

**앱 실행(배포용):** `./build-app.sh` 후 `open UsageMeter.app`. 개발 중엔 여전히 `swift run UsageMeter`.

**멀티 AI 구조 (2026-07-04, 대규모 리팩터링):** 단일 Claude → **여러 AI(Claude+Gemini) 동시 지원**. ①`Providers.swift`(ProviderSpec: home/usageJS/색 등, Claude=API fetch, **Gemini=`gemini.google.com/usage` DOM 스크랩** `[data-test-id=gxu-currently/weekly]`, 2026-05 도입 확인). ②`ClaudeSession`→범용 `WebSession(spec:)`. ③`ProviderManager`(세션·스냅샷·로그인 관리, refreshAll/login/logout/active). ④**같은 레일에 여러 띠 겹침, 잔여율 높은 게 뒤·낮은(급한) 게 앞**(BorderView `bands` sorted desc). ⑤메뉴바 링=가장 급한 AI 색·잔여율. ⑥호버=로그인된 모든 AI 정보(멀티라인). ⑦메뉴 "AI 로그인/로그아웃" 서브메뉴(provider별). ⑧AI별 색·`keepLoggedIn` 옵션(끄면 시작 시 로그아웃) 설정창. ⑨투명 그라데이션 on/off+길이 슬라이더. Claude 링 코랄색 표시·크래시 없음 확인. **Gemini는 실기기 로그인 후 /usage DOM 셀렉터 실검증 필요(못 해봄, 셀렉터 튜닝 가능성).** 세션·설정 모두 UserDefaults/WKDataStore 영속.

**ChatGPT 불가 결론 (2026-07-04, 재조사):** OpenAI는 **잔여 사용량을 어디에도 노출 안 함**(웹앱·backend-api 모두 없음). 기존 트래커는 전부 클라이언트에서 메시지 수를 세어 추정. 게다가 우리 앱의 숨겨진 조회 웹뷰는 사용자가 실제 대화하는 브라우저가 아니라 **메시지 세기조차 불가**. → 클로드/제미나이식(계정 서버 카운터 읽기) 적용 불가. **사용자가 A안(추가 안 함) 선택.** OpenAI가 usage API를 열기 전엔 재검토 불필요.

**🔴 렌더링 버그 근본 수정 (2026-07-04): 사용량 바 안 보임 = SwiftUI Shape/ForEach 조합 버그.**
- 증상: 멀티밴드를 `ForEach(밴드){ shape.trim().stroke() }`(특히 페이드용 상수범위 ForEach 중첩)로 그리면 **도형이 아예 안 그려짐**(직접 호출은 됨). 진단으로 데이터(active=2, segs=28)·오버레이(빨강 Rectangle)·직접 도형은 정상, ForEach 경로만 실패로 특정.
- **해결: BorderView를 `Canvas`(imperative 드로잉)로 재작성.** 모든 조각을 `segments`(트랙+본체+페이드 평탄화)로 만들고 `Canvas{ ctx in for seg { ctx.stroke(seg.shape.path(in:rect).trimmedPath(from:to:), style) } }`. SwiftUI 뷰-식별 이슈 원천 차단 → 재발 불가. 좌·상 코랄/블루 띠 정상 렌더 확인.
- 부수: 테스트로 오염된 설정 1회 초기화(settingsVersion 2, 프리셋 유지).

**다음 세션 후보:** Gemini 실로그인 후 /usage 셀렉터 검증, 폴링 자동갱신, .app 아이콘.

---

**(이하 이전 기록)** PoC 단계는 종료됨.

**트랙 ② 검증 결과 (2026-07-03):**
- 엔드포인트 확정: `GET https://claude.ai/api/organizations` → org UUID, 그다음 `GET https://claude.ai/api/organizations/{uuid}/usage`. 인증은 쿠키 `sessionKey` 하나(+Cloudflare 통과용 `cf_clearance`).
- **curl 직접 호출은 Cloudflare 403("Just a moment...")로 막힘.** → **브라우저 자동화(로그인된 claude.ai 탭 안에서 `fetch('/api/...')`)로 우회 성공.** 이게 현재 검증된 유일한 경로. (curl로 하려면 cf_clearance + 정확히 일치하는 User-Agent 필요.)
- 실제 응답 확인: `five_hour.utilization`(5시간 세션, 0~100 %used) + `seven_day.utilization`(주간) + 각 `resets_at`(ISO-8601 UTC). **잔여율 = 1 - utilization/100.** `seven_day_opus/sonnet` 등 세부 버킷은 null 가능 → 방어적 파싱.
- 공통 출력 포맷 **코드로 확정**: `Sources/UsageMeter/Provider.swift` (UsageSnapshot / UsageStatus / UsageProvider). `swift build` 통과.

**MVP에서 할 일 (순서 초안):**
1. **인증 획득 방식 결정** — PoC는 브라우저 자동화로 우회했지만, 배포용 앱은 그 방식이 안 됨. 실기기 선택지: (a) 앱 내 WebView 로그인으로 sessionKey+cf_clearance 캡처, (b) 브라우저 쿠키 DB 복호화 읽기(macOS `Claude Safe Storage` 키체인). → Keychain 로컬 저장.
2. **ClaudeProvider 구현** — Provider.swift의 `UsageProvider`를 실제 `/usage` 호출로 구현(응답 Codable + 잔여율 변환). Cloudflare 대응 포함.
3. **빨간 PoC 띠 → 실제 잔여율 띠로 교체** — BorderView가 `remainingRatio`에 비례해 길이 변하게.
4. 폴링(30초~수분) + stale/만료 상태 표시, 메뉴바 설정(변/색상/폴링), 앱 종료 수단.

**주의:** 세션 키/쿠키 = 민감정보. 로그·파일·외부 전송 금지(이번 검증도 셸 변수/페이지 컨텍스트로만 다룸). 계획서 3-5, 7장, 11장 보안 항목 준수.

로드맵 원본은 노션 계획서 10장. 큰 흐름:
`PoC(오버레이 + 데이터수집 검증) → MVP(단일 띠) → 중첩 다중 AI → 구역/화면 대응 → 안정화 → 멀티 디바이스`

---

## ✅ / 🔄 / ⬜ 체크리스트

### PoC 트랙 ① — 빈 투명 테두리 오버레이 창
- ✅ **합격 (2026-07-03)**: 3대 검증 모두 통과
  - ✅ Package.swift (SPM 실행파일)
  - ✅ 오버레이 창 소스 4종 (main / AppDelegate / OverlayWindow / BorderView)
  - ✅ `swift run` 으로 빌드·실행 성공 확인
  - ✅ 3대 검증: 투명 표시 / always-on-top / **click-through** (macOS 26에서 클릭통과 정상 — 회귀 버그 없음)
  - ✅ 스크린샷으로 테두리 렌더링 증빙

### PoC 트랙 ② — Claude 사용량 데이터 수집 검증
- ✅ **합격 (2026-07-03)**: claude.ai `/organizations/{uuid}/usage`로 실제 잔여율 읽기 성공(브라우저 자동화 우회)
  - ✅ 엔드포인트·인증 확정 (sessionKey 쿠키, org UUID 선행 조회, Cloudflare 우회 필요)
  - ✅ 실제 응답 검증 (five_hour/seven_day utilization + resets_at)
  - ✅ 공통 Provider 출력 포맷 코드화 (`Sources/UsageMeter/Provider.swift`, 빌드 통과)

### MVP (진행 중)
- ✅ **인증 방식 결정 (2026-07-03): WebView 로그인 캡처.** 세부 설계 = "WebView를 조회 엔진으로 유지"(로그인용 WKWebView 안에서 `fetch('/api/...')` → Cloudflare 자동 통과. curl/URLSession은 403이라 불가). `WKWebsiteDataStore.default()`로 쿠키 디스크 저장 → 재시작해도 로그인 유지.
- ✅ **인증/조회 구현 (빌드 통과):** `ClaudeSession.swift`(로그인 유지+fetch 사용량+sessionKey 캡처), `LoginWindowController.swift`(로그인 창, 감지 시 자동 캡처·닫기), `StatusBarController.swift`(메뉴바: 로그인/사용량 새로고침/종료 — 종료 수단도 겸함). AppDelegate에서 StatusBarController 기동.
- 🔄 **사용자 실기기 로그인 테스트 대기** — 메뉴바 `◔` → "Claude 로그인…" → claude.ai 로그인 → 사용량 표시 확인. (앱 WebView는 Chrome과 별개 쿠키스토어라 최초 1회 로그인 필요. 비밀번호 입력은 Claude가 대신 못 함.)
- ⬜ sessionKey Keychain 저장(현재는 WebView 비휘발성 쿠키스토어에만 의존, 캡처값은 미저장)
- ⬜ 폴링(30초~수분) 자동 갱신
- ⬜ 단일 띠가 잔여율에 따라 길이 변하기 (빨간 PoC 띠 교체) ← 로그인 테스트 통과 후 다음 단계
- ⬜ 메뉴바 설정(변/색상/폴링)
- ⬜ 중첩 다중 AI + "짧을수록 앞" z-순서
- ⬜ 구역 지정 + 노치/메뉴바/Dock 회피
- ⬜ 안정화(인증 만료 처리, stale 표시, Keychain 보안)

---

## ▶️ 지금 바로 다음에 할 일

**MVP 진입 — 배포 가능한 인증 획득 방식 결정** *(권장 첫 단추)*
- 목표: PoC는 "로그인된 브라우저 탭 안에서 fetch"로 Cloudflare를 우회했지만, **독립 실행 앱은 그 방식이 안 됨.** 실기기에서 sessionKey(+cf_clearance)를 어떻게 얻을지 결정.
- 두 선택지: (a) **앱 내 WebView 로그인** — 사용자가 앱 안 브라우저로 claude.ai 로그인하면 쿠키 캡처(선행 사례 다수). (b) **브라우저 쿠키 DB 복호화** — macOS `Claude Safe Storage` 키체인으로 Chrome 쿠키 파일 복호화해 읽기.
- 결정 후: Provider.swift의 `UsageProvider`를 실제 `/usage` 호출로 구현 → 빨간 PoC 띠를 잔여율 띠로 교체.

**참고 — 오버레이 다듬기(트랙 ①의 남은 개선, 급하지 않음):**
- 앱 종료 수단(메뉴바 아이콘 또는 단축키) 추가 — 지금은 `pkill -f UsageMeter` 로만 종료.

**참고 — MVP 진입 직전 정할 것(계획서 12장, 아래 미결 사항):** 붙일 AI 우선순위 / 기본 트랙 변 / 기본 띠 기준(5시간 vs 주간) / 최소 macOS 버전.

---

## 아직 결정 안 한 것 (계획서 12장, MVP 진입 직전에 정함)
- 붙일 AI 우선순위 (권장: Claude → Gemini → ChatGPT)
- 기본 트랙 변 (권장: 좌/우)
- 기본 띠 기준: 5시간 세션 한도 vs 주간 한도
- 대상 최소 macOS 버전
