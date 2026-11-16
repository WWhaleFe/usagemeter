# UsageMeter Rust 이식 계획서 (macOS + Windows)

작성: 2026-11-16 · 기준 버전: Swift v1.5.0 (커밋 `1b168f6`) · 상태: **진행 중 (결정 확정, 0단계 시작)**

이 문서는 Swift(AppKit·SwiftUI)로 만든 UsageMeter를 Rust로 다시 만들어 macOS와 Windows에서 같은 코드로 돌리기 위한 계획이다. 순서와 원칙은 세 가지다.

1. **기반 교체가 먼저다.** 지금 앱이 하는 일을 기능·동작·설정값까지 그대로 옮기는 것(기능 동등성, parity)을 최우선으로 한다. 새 기능은 동등성을 확인한 뒤에 붙인다.
2. **메모리 최적화는 기능과 같은 무게의 요구사항이다.** 단계마다 메모리 목표치를 두고, 측정해서 통과해야 다음 단계로 넘어간다.
3. **로컬 사용 기록 분석(토큰 통계)은 이식 과정에서 함께 만든다.** v1.5.0 때 추천한 "Claude Code·Codex·Antigravity 로컬 기록을 읽어 모델별·프로젝트별 토큰 통계를 내는 분석"을 6단계에서 구현한다.

---

## 목차

1. 현재 앱 요약과 이식 범위
2. 기술 선택과 이유
3. 전체 구조(크레이트 구성)
4. 메모리 최적화 설계와 목표치
5. 기능별 이식 설계
6. Windows 대응표
7. 데이터 이전(Swift 버전 → Rust 버전)
6-1. Windows 디자인 원칙 (Fluent)
8. 새 기능: 로컬 사용 기록 분석
9. 단계별 진행 계획과 통과 기준
10. 테스트 전략
11. 배포·업데이트 (11-1. Microsoft Store)
12. 위험 요소와 대응
13. 결정 사항 (확정)
14. 부록 A: 기능 동등성 체크리스트
15. 부록 B: 그대로 옮길 상수·규칙
16. 부록 C: 고칠지 유지할지 정할 기존 동작

---

## 1. 현재 앱 요약과 이식 범위

### 1-1. 지금 앱이 하는 일

| 영역 | 내용 | 주요 Swift 파일 |
|---|---|---|
| 사용량 수집 | Claude·Codex는 숨은 WKWebView(빈 오리진 문서)에서 `fetch`로 API 호출, Gemini는 `agy` CLI 우선 + 웹 DOM 파싱 대체 | `WebSession`, `Providers`, `AgyIntegration`, `ProviderManager` |
| 테두리 오버레이 | 화면마다 클릭이 통과하는 띠 창 최대 6개(좌·우·상·하·메뉴선·Dock선), 세그먼트 그래프 기반 경로, 잔여율만큼 채움, 페이드, 트랙, 노치 우회, 겹침 분할 | `BorderView`, `OverlayFrameController`, `DisplayRegistry` |
| 메뉴바 | 잔여율 링 아이콘 + AI별 색 % 글자, 드롭다운(정보 카드·미니 차트·27개 항목) | `StatusBarController`, `MenuInfoCardView`, `MiniChartView` |
| 호버 | 테두리 가장자리·모서리·메뉴바 아이콘 위에 마우스를 올리면 사용량 패널 | `HoverInfoController` |
| 설정 | 11개 탭, 디스플레이별 테두리 설정, 프리셋 10개, 기본 상태 | `SettingsView`, `OverlaySettings`, `SupportView`, `LineDragOverlay` |
| 분석 | 1.5.0 분석 창(AI 필터, 세부 항목, 잔여량 추이, 히트맵, 상세 표) | `AnalyticsView`, `AnalyticsModel`, `HistoryStore` |
| 기타 | 업데이트 확인·다운로드, 번들 교체 감지 후 재실행(Homebrew), 사용량 알림, 로그인 시 자동 실행, 한·영·일 335개 문자열 | `UpdateChecker`, `NotificationManager`, `Localization` |

### 1-2. 범위

- **포함:** 위 표의 모든 기능(부록 A 체크리스트 전부), Windows 대응, 데이터 이전, 로컬 기록 분석.
- **제외:** Swift 코드의 죽은 코드(`buildCornerMenu`, `buildNotchMenu`, `buildColorMenu`, `buildThicknessMenu`, `swatch`, `trackItem`, `notchToggleItem`, `toggleTrack`, `describe`, `friendlyReason`), 설정 스키마의 레거시 필드는 **읽기(이전)만** 하고 새로 쓰지 않는다.
- **Swift 버전 처리:** Rust 버전이 부록 A를 모두 통과할 때까지 Swift 버전을 계속 배포·유지보수한다. 전환 시점에 Rust 버전을 2.0.0으로 낸다.

---

## 2. 기술 선택과 이유

### 2-1. 결론

| 역할 | 선택 | 이유 |
|---|---|---|
| 앱 뼈대·창·트레이 | **Tauri 2** (내부적으로 tao·wry·tray-icon·muda) | macOS·Windows 단일 코드, 트레이·메뉴·단일 인스턴스·자동 실행·업데이터 플러그인 제공. 로그인용 웹뷰가 어차피 필요하므로 웹뷰 엔진을 하나로 통일 |
| 설정·분석 창 UI | Tauri 웹뷰 창 + **Svelte 5 + TypeScript**, 차트는 **uPlot**(약 50KB) + 직접 그리는 canvas. 컴포넌트는 **OS별 스킨 2벌**(macOS = 지금 모양, Windows = Fluent 2) | 11개 탭과 차트를 빠르게 만들 수 있고 번들이 작다. **열 때 만들고 닫을 때 파기**해서 평상시 메모리 0 |
| 테두리·호버 패널·트레이 팝업·아이콘 | **네이티브 그리기**: `kurbo`(경로 기하) + `tiny-skia`(래스터) + 글자는 `cosmic-text` | 오버레이를 웹뷰로 만들면 화면당 수십~수백 MB가 든다. CPU로 띠 크기만큼만 그려서 OS 창에 올린다 |
| macOS 플랫폼 코드 | `objc2`, `objc2-app-kit`, `objc2-quartz-core`, `objc2-web-kit`, `objc2-foundation` | 오버레이 NSWindow·CALayer, NSEvent 전역 감시, 노치(safeAreaInsets), WKWebView 세부 제어 |
| Windows 플랫폼 코드 | `windows`(windows-rs), `webview2-com` | 레이어드 창, 저수준 마우스 훅, 모니터·DPI·작업표시줄, WebView2 요청 가로채기, DWM 배경(Mica·Acrylic)·둥근 모서리, DirectComposition |
| HTTP | `reqwest` | 업데이트 확인, (가능하면) 사용량 API 직접 호출 |
| 비동기 | `tokio` | 타이머·디바운스·프로세스·파일 감시 |
| 저장 | `serde`/`serde_json`(설정), **`rusqlite`(bundled SQLite, 이력·토큰 집계)**, `plist`(Swift 설정 이전) | 이력을 JSON 통째 읽고 쓰는 방식의 메모리·IO 문제 해결 |
| 파일 감시 | `notify` | FSEvents(macOS)·ReadDirectoryChangesW(Windows) 공통 |
| 기타 | `qrcode`, `open`, `arboard`(클립보드), `directories`, `tracing`(로그) | 후원 QR, 링크 열기, 주소 복사, 경로, 진단 로그 |

**버전 고정:** 각 크레이트는 0단계 시작 시점의 최신 안정판으로 `Cargo.lock`에 고정한다. 이 문서는 이름만 정하고 버전 숫자는 적지 않는다.

### 2-2. 고려했다가 뺀 선택지

| 선택지 | 뺀 이유 |
|---|---|
| 전부 Tauri 웹뷰(오버레이 포함) | 화면마다 투명 웹뷰 창이 필요해 메모리 목표(4장)를 맞출 수 없다 |
| egui | 한글 IME 입력 품질과 macOS·Windows 네이티브 느낌이 약하다. GPU 컨텍스트 메모리가 창마다 든다 |
| Slint | 메모리는 가장 좋다. 차트·컬러 피커·대화형 다이어그램을 직접 만들어야 해 작업량이 크다. **0단계 측정에서 Tauri 창의 메모리 회수가 실패하면 대안으로 채택**(13장 결정 항목) |
| iced | 위젯 성숙도와 IME, 트레이 연동이 부족하다 |

---

## 3. 전체 구조(크레이트 구성)

같은 저장소의 `rust/` 폴더에 Cargo 워크스페이스를 만든다. Swift 코드는 전환 전까지 그대로 둔다.

```
rust/
  Cargo.toml                워크스페이스
  crates/
    um-core/                UI·OS와 무관한 순수 로직 (테스트 대부분이 여기)
      model.rs              UsageSnapshot, UsageSubItem, UsageStatus, ProviderSpec
      settings/             설정 스키마(SettingsSnapshot 필드명 그대로), 기본값, 검증, 마이그레이션
      segments.rs           세그먼트 그래프(SegPart, SegNode, 유효성, chainEnds, endOverride 등)
      i18n/                 ko·en·ja 문자열(Localization.swift에서 생성한 JSON)
      history.rs            SQLite 이력 저장, 표본 기록 규칙, 35일 보관
      analytics.rs          소비 이벤트, 리셋·창 사용률·한도 도달·소진 속도, 다운샘플
      notify_rules.rs       알림 임계값 판정(발송은 플랫폼 층)
      update.rs             GitHub 릴리스 확인, 버전 비교
    um-providers/           사용량 수집
      claude.rs codex.rs gemini.rs agy.rs
      fetch_http.rs         웹뷰 없는 직접 호출(가능할 때)
      fetch_webview.rs      임시 웹뷰로 호출(대체 경로)
      scheduler.rs          갱신 주기, 재시도, agy 활동 기반 빠른 갱신
    um-locallogs/           새 기능: Claude Code·Codex·Antigravity 로컬 기록 수집(8장)
    um-render/              네이티브 그리기 (OS 무관)
      border.rs             SegmentChainShape 이식, 채움·페이드·트랙·겹침 분할
      panel.rs              호버 패널·트레이 팝업(카드·미니 차트)
      icon.rs               링 아이콘·퍼센트 글자 이미지
      text.rs               cosmic-text 래퍼(시스템 글꼴 지연 로딩)
    um-platform/            OS별 코드 (cfg로 분리)
      mac/                  overlay_window, displays(노치·UUID), mouse_monitor, status_item 보조, login_item
      win/                  layered_window, monitors(DPI·작업표시줄), mouse_hook, autostart, toast
    um-app/                 Tauri 앱 (조립 담당)
      src/                  상태 관리, 명령(IPC), 트레이, 창 수명 관리
      ui/                   Svelte 프런트엔드(설정·분석·로그인 안내 창)
  tools/
    gen-i18n/               Localization.swift → i18n JSON 변환기
    border-fixtures/        Swift BorderView를 PNG로 렌더하는 기준 이미지 생성기(10장)
    measure-memory.sh / measure-memory.ps1
```

**원칙**
- `um-core`는 OS 의존 코드를 넣지 않는다. 이식 정확도의 대부분을 여기서 단위 테스트로 보장한다.
- 프런트엔드는 표시만 한다. 계산(분석 통계, 세그먼트 유효성)은 Rust가 하고 결과만 넘긴다. 같은 로직을 TS로 다시 쓰지 않는다.
- 상태의 원본은 Rust 쪽 하나다. 창이 다시 열리면 Rust에서 다시 받아 그린다(창 파기 전략과 맞물림).

---

## 4. 메모리 최적화 설계와 목표치

### 4-1. 지금 수치(기준선)

| 시점 | 수치 | 출처 |
|---|---|---|
| 최적화 전 Swift 앱 | 약 1.55GB (웹뷰 3개 SPA 상주 1.1GB + 오버레이 전체화면 백스토어 280MB) | `MEMORY-REPORT.md` |
| 최적화 후 Swift 앱 | 약 121MB 합계 | `MEMORY-REPORT.md` |
| v1.5.0 실측(2026-11-16) | 본체 footprint 102MB (+ Claude·Codex 빈 문서 WebContent 약 16MB·15MB) | `footprint` 명령 |
| Gemini 웹 조회 순간 | 약 300~360MB를 4~6초 | `MEMORY-REPORT.md` |

### 4-2. Rust 버전 목표치 (통과 기준)

macOS는 `footprint`(본체 + WebKit 하위 프로세스 합계), Windows는 비공개 작업 집합(Private Working Set, 본체 + `msedgewebview2.exe` 하위 프로세스 합계)으로 잰다.

| 상태 | macOS 목표 | Windows 목표 | 비고 |
|---|---|---|---|
| **평상시**(트레이 + 테두리, 창 없음, 웹뷰 없음 경로) | **≤ 45MB** | **≤ 40MB** | 지금의 약 1/3 |
| 평상시인데 웹뷰 대체 경로를 쓰는 경우 | ≤ 70MB | ≤ 90MB | WebView2 환경 자체 비용이 커서 Windows 기준을 높게 둔다 |
| Gemini 웹 조회 순간 | +300MB 이하, 15초 안에 평상시로 복귀 | 같음 | agy가 있으면 웹뷰 자체를 안 띄운다 |
| 설정 또는 분석 창 열림 | 평상시 + 150MB 이하 | 평상시 + 180MB 이하 | |
| 창을 닫은 뒤 60~90초 | **평상시 + 20MB 이하, 여러 번 열고 닫아도 늘지 않음** | 같음 | Swift 버전은 닫아도 150~200MB에 머문다. 0단계 S1 실측으로 조정(ADR 0001) |
| 화면 1개당 테두리 | ≤ 6MB (Retina 기준) | ≤ 4MB (4K 150% 기준) | 지금은 약 21MB |
| 로컬 기록 첫 수집(1.2GB 로그) | 작업 중 +40MB 이하 | 같음 | 끝나면 평상시로 복귀 |
| 24시간 연속 실행 후 평상시 | 시작 직후 대비 +10% 이하 | 같음 | 누수 감시 |

### 4-3. 메모리를 줄이는 방법

**① 평상시에 웹뷰를 하나도 띄우지 않는다 (가장 큰 효과)**
- Claude: 로그인할 때만 웹뷰를 쓰고, 로그인 후에는 웹뷰 쿠키 저장소에서 세션 쿠키를 읽어 `reqwest`로 같은 API를 직접 부른다. 브라우저와 비슷한 헤더(User-Agent 등)를 맞춘다.
- Codex: 위와 같은 방식. 추가로 Codex CLI 로컬 기록의 `rate_limits`(8장)를 웹뷰 없는 보조 소스로 쓴다.
- Gemini: `agy` CLI가 1순위(지금과 같음). 웹 DOM 파싱은 대체 경로로만 쓰고, 조회 때 웹뷰를 만들었다가 곧바로 파기한다(지금과 같음). Google 내부 RPC(`batchexecute`)로 직접 읽는 방법은 연구 항목으로 둔다(`MEMORY-REPORT.md`의 미해결 항목).
- **직접 호출이 막히면**(Cloudflare 봇 차단 등) 자동으로 "임시 웹뷰" 경로로 바꾼다. 조회할 때 웹뷰를 만들고, 서비스 오리진의 빈 문서를 띄워 `fetch`한 뒤 바로 파기한다. 어느 경로를 쓰는지 AI별로 기억해서 매번 시도하지 않는다.

**② 테두리 띠 창을 실제로 그리는 크기만큼만 만든다**
- 지금은 세로 띠 150pt, 가로 띠 170pt로 고정이라 Retina 화면 하나에 약 21MB를 쓴다.
- 띠 두께를 그때그때 계산한다: `inset + 선 두께 + 해당 변의 최대 곡률 + (윗변이면 노치 깊이) + 여유 4pt`. 기본 설정(두께 2, 곡률 22)이면 세로 띠가 약 30pt로 줄어 백스토어가 1/5이 된다.
- 어떤 AI도 쓰지 않는 변의 띠 창은 만들지 않는다(예: 왼쪽 세그먼트를 아무도 안 쓰면 vLeft 창 없음).
- 그린 결과를 OS에 넘긴 뒤 우리 쪽 픽셀 버퍼는 버린다. macOS는 `CALayer.contents`가, Windows는 `UpdateLayeredWindow`가 사본을 가진다. 다시 그릴 때 새로 만든다.
- 애니메이션이 없으므로 데이터·설정이 바뀔 때만 다시 그린다. 평상시 CPU는 0%.

**③ 설정·분석 창은 열 때 만들고 닫을 때 파기한다**
- Swift 버전에서는 SwiftUI 창을 닫고 해제하면 오히려 늘었다(`MEMORY-REPORT.md`). Tauri 웹뷰 창은 파기하면 렌더러 프로세스가 종료되므로 회수될 것으로 본다. **0단계에서 실측해서 확인한다.** 실패하면 Slint로 바꾼다.
- 창 상태(선택한 탭, 스크롤 위치)는 Rust 쪽에 저장했다가 다시 열 때 복원한다.

**④ 이력과 토큰 집계는 SQLite에 두고 필요한 만큼만 읽는다**
- 지금은 35일 치 표본 전체(약 1.2만 개, 1.4MB JSON)를 메모리에 올리고, 저장할 때마다 파일을 통째로 다시 읽어 합친 뒤 통째로 쓴다.
- Rust 버전은 표본 한 개당 한 행만 추가한다(`INSERT`). 분석 창이 요청하면 SQL로 기간·AI·항목을 걸러 집계하고, 차트용 계열은 Rust에서 최대 240칸으로 줄여 보낸다.
- 메뉴 미니 차트·소진 예측은 최근 24시간만 조회한다.

**⑤ 로컬 기록은 스트리밍으로 한 줄씩 읽는다**
- 파일마다 "어디까지 읽었는지(오프셋)"를 저장해 새로 추가된 부분만 읽는다.
- 필요한 필드만 담는 작은 구조체로 역직렬화하고 내용(대화 본문)은 버린다.
- 백그라운드 스레드 하나에서 낮은 우선순위로 돌린다.

**⑥ 그 밖의 작은 것들**
- 글꼴: 시스템 글꼴 전체를 읽지 않고 쓰는 글꼴(macOS: SF·Apple SD Gothic Neo·Hiragino, Windows: Segoe UI·Malgun Gothic·Yu Gothic UI)만 지연 로딩한다.
- 로그인하지 않은 AI는 세션 객체를 만들지 않는다(Swift 보고서의 "미적용" 항목).
- `tokio` 런타임은 워커 스레드 2개로 제한한다.
- 릴리스 빌드: `opt-level = "s"`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.

### 4-4. 측정 방법

- `tools/measure-memory.sh`(macOS): `footprint -p <pid>`와 WebKit 하위 프로세스를 합산하고 `vmmap --summary`를 남긴다.
- `tools/measure-memory.ps1`(Windows): 본체와 `msedgewebview2.exe` 하위 프로세스의 Private Working Set을 합산한다.
- 시나리오: 시작 후 2분 대기 → 평상시 측정 → 설정 창 열기 → 측정 → 닫고 60초 → 측정 → 분석 창 → 측정 → 닫고 60초 → 측정 → Gemini 강제 조회 → 15초 → 측정.
- 결과는 `rust/MEMORY-REPORT-RUST.md`에 단계마다 표로 쌓는다. 목표치를 넘기면 그 단계는 끝난 것으로 치지 않는다.

---

## 5. 기능별 이식 설계

### 5-1. 사용량 수집 (`um-providers`)

**공통 출력**은 Swift `UsageSnapshot`과 같은 모양이다: `remaining_ratio`, `secondary_ratio`, `opus_ratio`, 각 리셋 시각, `status`, `last_updated`, `plan`, `model_buckets`, `five_hour_ratio`, `raw_buckets`. 세부 항목 id 규칙(`5h`, `weekly`, `model:<라벨>:5h|weekly`)과 순서도 그대로다.

**AI별 조회 순서**

| AI | 1순위 | 2순위 | 3순위 |
|---|---|---|---|
| Claude | 직접 호출: `GET /api/organizations` → `orgs[0]` → `GET /api/organizations/{uuid}/usage` (쿠키는 로그인 웹뷰 저장소에서) | 임시 웹뷰 + 빈 오리진 문서 + 같은 JS | - |
| Codex | 직접 호출: `/api/auth/session` → `accounts/check/v4-2023-04-27` → `backend-api/wham/usage` | 임시 웹뷰 | 로컬 Codex CLI 기록의 `rate_limits`(가장 최근 값과 시각, 오래되면 흐리게) |
| Gemini | `agy -p /usage --output-format json` | 임시 웹뷰로 `gemini.google.com/usage` DOM 파싱(지금 JS 그대로) | - |

- 응답 해석 규칙(플랜 이름 매핑, `utilization` → 잔여율, 리셋 시각 파싱, 모델 버킷, Codex의 코드 리뷰·추가 한도, 창 길이 6시간 이하 = 5시간)은 Swift와 같게 옮긴다. JS 문자열(`usageJS`)은 웹뷰 경로에서 그대로 재사용하고, 직접 호출 경로는 같은 규칙을 Rust로 구현한다. 두 경로의 결과가 같은지 테스트한다(10장).
- **상태 반영 규칙**(ProviderManager.apply): `ok`면 저장·로그인 표시, `authExpired`면 로그아웃 표시, 그 밖의 실패는 직전 정상값을 유지한다. 시작할 때 최대 4번, 2초 간격 재시도도 같다.
- **갱신 주기**: 프리셋 `[1,2,3,4,5,10,15,20,30,60,120]`분, 직접 입력 1~180분, 실제 값 `max(1, …)`, 기본 5분. 바뀌었을 때만 타이머를 다시 건다.
- **agy 활동 감지**: `~/.gemini/antigravity-cli`의 `conversations/`, `brain/`, `history.jsonl`을 `notify`로 감시한다. 앞 트리거(7초 간격, 1.2초 뒤)와 뒤 트리거(마지막 이벤트 2.5초 뒤 + 6.5초 뒤 한 번 더) 규칙을 그대로 옮긴다. 600초 안에 활동이 있었으면 25초마다 Gemini를 갱신하는 빠른 타이머도 같다.
- **agy 실행**: 위치 탐색 순서(`~/.local/bin/agy` → `~/.gemini/antigravity-cli/bin/agy` → `/opt/homebrew/bin` → `/usr/local/bin` → `/usr/bin` → `PATH`), 12초 제한, 동시 호출 합치기, 단조 감소 규칙(같은 `reset_time` 안에서 0.12 미만 상승은 무시)을 그대로 옮긴다. Windows는 9장 참고.
- **로그인 창**: 460×720 창에 웹뷰를 띄워 서비스 로그인 페이지로 보낸다. 1.2초마다 완료를 확인한다(Gemini는 인증 쿠키 `__Secure-1PSID`·`__Secure-3PSID`·`SID`, 나머지는 사용량 조회 성공). 패스키를 끄는 문서 시작 스크립트, Safari 17.4.1 User-Agent, 팝업 창(같은 쿠키·`window.opener` 유지), 로드 실패 시 재시도 링크가 있는 안내 페이지도 같다.
- **로그아웃**: 서비스별 쿠키 도메인 조각(`claude`·`anthropic` / `google.com`·`gemini` / `chatgpt`·`openai`)에 해당하는 쿠키를 지운다. Gemini가 Google 쿠키를 전부 지우는 문제는 부록 C에서 결정한다.

### 5-2. 테두리 오버레이 (`um-render::border` + `um-platform`)

**기하 계산은 Swift `SegmentChainShape.build`를 한 줄씩 옮긴다.** 부록 B에 핵심 규칙을 모았다. 요점:
- 기준 좌표: `inset = 두께/2`, 레벨 y 4개(위·메뉴선·Dock선·아래).
- 순회 규칙: 루프는 `anchorCorner`에서 시작하고 `clockwise`가 참이면 가로 세그먼트부터 간다(실제 의미는 "가로 먼저"이고 시계 방향과 다를 수 있음, 그대로 유지).
- 꺾임 곡률은 세로 세그먼트가 속한 구역의 반지름을 쓴다. 곡선은 원호가 아니라 **2차 베지어**다(그대로 유지해야 모양이 같다).
- 가로 끝 처리(위·아래로 말기), "스쿱"(오목 필렛, 꼭짓점 3개), `endOverride`(두 AI가 모서리를 반씩 나눠 그림), 노치 우회(꼭짓점 4개, 좌우 1pt 여유, 레인 오프셋이 있으면 경사 점 추가).
- 채움: 경로를 길이 기준 0~잔여율로 자른다(`kurbo` 호 길이). 페이드는 끝부분 `min(fadeFraction, r)`를 12조각으로 나눠 불투명도를 `1-(i+1)/12`로 낮춘다. 실선은 둥근 끝, 트랙·페이드는 평평한 끝, 이음은 둥글게.
- 트랙은 AI마다 한 번, AI 색 0.18 불투명도. 전체에 `lineOpacity`를 곱한다.
- 쌓는 순서: AI(또는 세부 항목 분할 시 항목)별 띠를 **잔여율 높은 순**으로 그려 적은 것이 위에 온다. 노치 층을 먼저, 일반 층을 나중에.
- 겹침 분할(`splitOverlapLines`): 공유하는 AI 수 k에 대해 레인 오프셋 `(i-(k-1)/2)·t/k`, 폭 `t/k`, 오프셋은 화면 중심 쪽. 폭 전환은 6pt 조각으로 다시 샘플링하고 3회 원형 상자 블러를 한다(48~2048조각).

**창 (OS별)**
- 화면마다 띠 창을 만든다(4-3②대로 크기 계산). 각 띠는 **화면 전체 경로를 자기 원점만큼 옮겨 그리고** 창 크기로 잘린다(지금 방식 그대로).
- macOS: 테두리 없는 투명 NSWindow, 레벨 `.statusBar`, 마우스 무시, `canJoinAllSpaces | stationary | fullScreenAuxiliary`, 키·메인 창이 될 수 없음. 내용은 CALayer에 CGImage로 올린다. 띠로 나누는 이유(화면 가운데를 덮으면 macOS 보안 확인 버튼·Touch ID 창이 숨겨짐)는 그대로 유효하다.
- Windows: `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` 창에 `UpdateLayeredWindow`로 픽셀별 알파를 올린다. 모니터별 DPI(Per-Monitor v2)로 pt → px를 바꾼다.
- 다시 만드는 시점: 디스플레이 구성 변경(macOS `didChangeScreenParameters`, Windows `WM_DISPLAYCHANGE`·`WM_DPICHANGED`·작업 영역 변경), 설정 변경, 테두리 숨기기, 디스플레이 끄기.

**디스플레이 관리**: 안정적인 화면 id(macOS `CGDisplayCreateUUIDFromDisplayID`, Windows는 DisplayConfig 장치 경로), 내장 화면 우선 주 설정, 다른 화면은 주 설정을 씨앗으로 한 자식 설정(언어·AI 색·세부 항목·호버·숨기기는 항상 주 설정을 따름), 화면 끄기, 다른 화면에 복사, 장치 맞춤(노치 감지·곡률 22×화면폭/1710 비율), 해상도 변경 시 곡률 비례 조정(1% 넘을 때, 0.5~2배)을 그대로 옮긴다.

### 5-3. 메뉴바·트레이 (`um-app` + `um-render::icon/panel`)

- **아이콘**: 18×18pt 링(inset 2.5, 선 2.4, 반지름 6.5, 트랙 알파 0.28, 12시부터 시계 방향). 표시할 AI는 `menuBarIconID`가 활성이면 그것, 아니면 잔여율이 가장 낮은 활성 AI.
- **퍼센트 글자**: macOS는 링 + AI별 색 글자(11pt semibold)를 **한 장의 이미지로 그려** 상태 항목에 넣는다(색 글자를 tray-icon의 제목 기능으로는 못 넣기 때문). Windows 트레이는 정사각 아이콘만 가능하므로 아이콘 안에 숫자 하나 + 툴팁으로 대체한다(6장).
- **드롭다운**: 13장 결정 항목 1번. 권장안은 두 OS 공통으로 **왼쪽 클릭 = 네이티브로 그린 팝업 패널**(정보 카드 + 미니 차트 + 자주 쓰는 동작), **오른쪽 클릭(또는 패널의 더보기 버튼) = 네이티브 메뉴**(전체 27개 항목과 하위 메뉴)이다. 메뉴 구성·순서·단축키(⌘R·⌘,·⌘Q)·체크 상태는 부록 A의 목록 그대로다.
- **정보 줄 형식**: `<모델 라벨 ><5시간 잔여|주간 잔여>  NN%   ·   리셋 M/d HH:mm   ·   X시간 Y분 후`, 소진 예측 줄, 마지막 갱신 줄, 항목 색 점. 형식 문자열은 i18n 키를 그대로 쓴다.
- **미니 차트**: 6/12/24시간, 높이 92, y 0~100 고정, 단조 보간 선, 호버 시 점선 기준선과 툴팁.

### 5-4. 호버

- macOS: `NSEvent` 전역·로컬 마우스 이동 감시(지금과 같음, 별도 권한 불필요).
- Windows: 저수준 마우스 훅(`WH_MOUSE_LL`). 훅 안에서는 좌표만 채널로 넘기고 즉시 반환한다. 훅이 막히는 환경을 대비해 30Hz `GetCursorPos` 폴링을 대체 경로로 둔다(테두리가 켜진 동안만).
- 판정 규칙: 가장자리에서 `max(6, 두께)+7`pt 이내, 그 변을 어떤 AI가 쓰고 있을 것(메뉴선·Dock선은 대상 아님), `hoverEdges` 또는 `hoverCorners`·`hoverCornerSize`(기본 80). 메뉴바 아이콘 영역 판정 공식도 그대로(부록 B).
- 패널: 커서 +14,+14, 화면 밖이면 반대쪽. 메뉴바 호버면 아이콘 아래 가운데. 반투명 배경(macOS `NSVisualEffectView .hudWindow`, Windows는 Acrylic 배경 또는 반투명 단색), 12pt 글자, 여백 10/6, 모서리 7. 드롭다운이 열려 있으면 숨긴다.

### 5-5. 설정 창

- Svelte로 11개 탭을 만든다. 각 컨트롤의 범위·단계·비활성 조건은 부록 A의 탭별 목록 그대로다. 숫자 조절은 "슬라이더 + 숫자 칸 + 화살표" 묶음 컴포넌트 하나로 통일하고, 포커스가 있을 때 방향키로 한 단계씩 바뀌게 한다.
- 레이아웃 탭의 대화형 다이어그램(209×133, 클릭 = 유효할 때만 토글, 더블클릭 = 단독 선택, 겹침 금지 경고 1.6초)은 SVG로 그린다. 유효성 판정은 Rust 명령을 부른다.
- 테두리 관련 값은 "편집 중인 디스플레이"에, 나머지는 주 설정에 쓴다. 값이 바뀌면 즉시 Rust에 반영되고 테두리가 실시간으로 다시 그려진다. 저장은 300ms 디바운스.
- 선 위치 끌기(`LineDragOverlay`)는 해당 화면을 덮는 반투명 네이티브 창(마우스 받음)으로 만든다. 메뉴선 주황·Dock선 청록 손잡이, 5pt~화면 높이 60% 제한, 완료 버튼. Esc로도 닫히게 한다(작은 개선).
- 지원 탭: 메일 열기·주소 복사, 후원 링크, 카카오페이 QR(`qrcode` 크레이트, M 레벨).

### 5-6. 분석 창

- v1.5.0의 화면 구성을 그대로 옮긴다: 기간(1일·3일·주간·한달), AI 칩(전체 / AI 하나), 세부 항목 칩, 요약 4칸, AI별 소비 비중·항목별 소비량, 시간대별 패턴, 잔여량 추이, 기간별 소비 추이, 요일 × 시간대 히트맵, 세부 항목 상세 표, 안내·주의.
- 계산은 `um-core::analytics`(Swift `AnalyticsMath` 이식: 리셋 = +3%p 이상, 한도 도달 = 2% 이하 진입, 창 평균 사용률, 최근 3시간 소진 속도, 최저·최고 다운샘플)가 하고, 프런트엔드는 받은 숫자만 그린다.
- 여기에 8장의 "토큰" 화면을 더한다.

### 5-7. 알림·업데이트·자동 실행·기타

- **알림**: 기본 꺼짐, 임계값 {75, 90, 95}(사용률). 키 `id|임계값|리셋시각` 단위로 한 번만 보낸다. 리셋 시각이 바뀌면 다시 보낼 수 있다. macOS는 UNUserNotificationCenter(번들이 있어야 함), Windows는 토스트(앱 사용자 모델 ID 필요, 설치 프로그램이 시작 메뉴 바로가기와 함께 등록).
- **업데이트 확인**: `api.github.com/repos/WWhaleFe/usagemeter/releases/latest`, 15초 제한, 시작 시 + 24시간마다, 버전 비교 규칙(앞의 v 제거, 점으로 나눠 숫자 비교), 새 버전 알림은 버전당 한 번. 다운로드는 macOS zip / Windows 설치 파일을 받아 위치만 보여준다(macOS). Windows는 11장 업데이터.
- **번들 교체 감지 후 재실행(macOS·Homebrew)**: 10초마다 디스크의 `Info.plist` 빌드 번호와 실행 중인 번호를 비교, 두 번 연속 다르면 셸 도우미(`while kill -0 pid; do sleep 0.2; done; exec open -g 번들`)를 띄우고 종료. 그대로 옮긴다.
- **자동 실행**: macOS `SMAppService`(objc2로 직접 호출하거나 `tauri-plugin-autostart`의 LaunchAgent 방식 중 0단계에서 결정), Windows `HKCU\...\Run`.
- **Dock 아이콘 표시**: 설정·분석 창이 열려 있는 동안만 macOS 활성화 정책을 `.regular`로, 닫히면 `.accessory`로. Windows는 작업표시줄 버튼이 창과 함께 생겼다 사라지므로 따로 할 일이 없다.
- **외관**: 시스템·밝게·어둡게. 웹뷰 창은 CSS 변수로, 네이티브 패널은 테마 색 표로 그린다.
- **단일 인스턴스**: `tauri-plugin-single-instance`. 두 번째 실행 시 설정 창을 연다.

---

## 6. Windows 대응표

| macOS 기능 | Windows에서 | 비고 |
|---|---|---|
| 메뉴바 상태 항목 + 색 글자 | 알림 영역(트레이) 아이콘. 링 안에 가장 급한 AI의 잔여 숫자를 그리고, 툴팁에 전체 요약 | 트레이는 글자를 옆에 못 붙인다(기능 차이, README에 명시) |
| 메뉴바 아이콘 호버 | 트레이 아이콘 Enter/Leave 이벤트로 호버 패널 표시, 위치는 `Shell_NotifyIconGetRect` 기준 | |
| 드롭다운(NSMenu + 커스텀 뷰) | 5-3 권장안의 팝업 패널 + 오른쪽 클릭 메뉴 | 트레이 메뉴는 커스텀 뷰를 못 넣는다 |
| 메뉴선(화면 위 메뉴바 아래 구분선) | **만들지 않는다.** Windows는 화면 테두리만 그린다 | 사용자 결정(2026-11-16) |
| Dock선 | **만들지 않는다.** 작업표시줄과 바탕화면 사이 경계선에는 선을 그리지 않는다 | 사용자 결정(2026-11-16) |
| 노치 | 노치 기능 없음. 노치 탭은 Windows에서 숨긴다 | |
| 모든 Space에 표시 | 모든 가상 데스크톱에 표시: 도구 창(`WS_EX_TOOLWINDOW`)은 가상 데스크톱 전환 시 따라오는지 0단계에서 확인. 안 되면 `IVirtualDesktopManager`로 데스크톱 전환 이벤트에 맞춰 옮긴다 | |
| 전체화면 앱 위 표시 | 테두리 없는 전체화면 앱 위에는 보인다. 독점 전체화면 게임 위에는 못 그린다 | README에 명시 |
| `agy` CLI | `%USERPROFILE%\.local\bin\agy.exe`, `%USERPROFILE%\.gemini\antigravity-cli\bin\agy(.exe/.cmd)`, `PATH`(+`PATHEXT`). 실행 시 `CREATE_NO_WINDOW`, `/dev/null` → `NUL`, 작업 폴더 `%TEMP%` | Windows용 agy 존재 여부는 0단계 확인 |
| 로그인 웹뷰 | WebView2, 사용자 데이터 폴더 `%LOCALAPPDATA%\UsageMeter\WebView2` | WebView2 런타임이 없으면 설치 프로그램이 Evergreen 부트스트래퍼로 설치 |
| 빈 오리진 문서 트릭 | `AddWebResourceRequestedFilter("https://claude.ai/*")` + `CreateWebResourceResponse`로 홈 주소만 빈 HTML을 돌려준다. `NavigateToString`은 오리진이 about:blank가 되어 쿠키가 안 붙으므로 쓰지 않는다 | macOS는 `loadSimulatedRequest`를 objc2로 호출 |
| `callAsyncJavaScript`(Promise 대기) | `ExecuteScriptAsync`는 Promise를 기다리지 않으므로, JS를 async 즉시실행 함수로 감싸 결과를 `chrome.webview.postMessage`로 보낸다(wry의 IPC 핸들러로 받음). 요청마다 id를 붙여 응답을 짝짓는다 | macOS도 같은 방식으로 통일 가능 |
| 자동 실행 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` | |
| 알림 | 토스트(AUMID 등록 필요) | |
| 다크 모드 | 레지스트리 `AppsUseLightTheme` 감지, 창에 몰입형 다크 모드 속성 | |
| Finder에서 보기 | `explorer.exe /select,"경로"` | |
| 업데이트 | Tauri 업데이터(서명된 매니페스트) 또는 winget·Scoop. 실행 중인 exe는 덮어쓸 수 없으므로 도우미가 종료를 기다린 뒤 교체·재실행 | 11장 |
| 저장 위치 | 설정·이력: `%APPDATA%\UsageMeter\`, 웹뷰·캐시: `%LOCALAPPDATA%\UsageMeter\` | macOS: `~/Library/Application Support/UsageMeter/` |

**Windows 테두리 범위(확정):** Windows는 **화면 가장자리 테두리만** 그린다. 세그먼트는 `hTop`·`hBottom`·`lMain`·`rMain` 네 개만 쓰고(메뉴선·Dock선이 꺼진 상태와 같아 `lMain`·`rMain`이 화면 위에서 아래까지 이어진다), 메뉴선·Dock선·노치 관련 설정과 "구분선"·"노치" 탭, 선 위치 끌기 화면은 Windows에서 보이지 않는다. 테두리는 작업표시줄 영역을 피하지 않고 화면 바깥 가장자리를 따른다. macOS에서 옮겨 온 설정에 메뉴선·Dock선 값이 있어도 Windows에서는 무시한다(설정 파일은 OS별로 따로 저장되므로 실제로 섞이지 않는다).

### 6-1. Windows 디자인 원칙 (Fluent)

Windows 버전은 macOS 화면을 그대로 옮기지 않고 **Windows 11 기본 앱(설정 앱, 빠른 설정 플라이아웃)과 같은 디자인 언어(Fluent 2)**로 만든다. 기능과 설정값은 같고 모양과 배치만 OS에 맞춘다.

| 요소 | macOS | Windows |
|---|---|---|
| 설정 창 배치 | 위쪽 탭 11개(지금과 같음), Windows는 구분선·노치를 뺀 9개 | **왼쪽 탐색 창**(NavigationView) + 오른쪽 내용, Windows 11 설정 앱 구조. 항목마다 Segoe Fluent Icons 아이콘 |
| 창 배경 | 시스템 창 배경 | **Mica** 배경(Windows 11), Windows 10은 단색으로 대체 |
| 컨트롤 | macOS 모양(토글·세그먼트·슬라이더) | Fluent 2 모양: 토글 스위치, 콤보 박스, 슬라이더, 숫자 상자(NumberBox), 설정 카드(SettingsCard: 아이콘·제목·설명·오른쪽 컨트롤 한 줄) |
| 모서리 | macOS 기본 | 창·카드·플라이아웃 8px, 컨트롤 4px |
| 글꼴 | 시스템(SF, Apple SD Gothic Neo, Hiragino) | Segoe UI Variable + 한글 Malgun Gothic + 일본어 Yu Gothic UI |
| 색 | 시스템 강조색 | **Windows 강조색**(UISettings에서 읽음)과 밝게/어둡게 테마를 따름 |
| 메뉴바/트레이 클릭 | 팝업 패널(정보 카드·차트·주요 동작) | 작업표시줄 위로 올라오는 **플라이아웃**(빠른 설정 패널 스타일): Acrylic 배경, 8px 모서리, 위로 미끄러지는 등장, 바깥 클릭·Esc로 닫힘 |
| 오른쪽 클릭 메뉴 | 네이티브 메뉴 | Windows 11 네이티브 메뉴(둥근 모서리, 어두운 테마 적용) |
| 호버 패널 | `NSVisualEffectView` HUD 모양 | Windows 툴팁·플라이아웃 모양(Acrylic, 8px, 그림자) |
| 트레이 아이콘 | 링 + AI별 색 % 글자 | 16·20·24·32px 링(작업표시줄 밝기에 맞춰 트랙 색 조정), 숫자는 툴팁. 설정으로 "아이콘 안에 숫자" 선택 가능 |
| 분석 창 | 지금 모양 | Fluent 카드(레이어 배경·얇은 테두리), Mica 배경, 같은 차트 |
| 선 위치 끌기 | 지금 모양 | 없음(Windows는 구분선 기능이 없다) |
| 접근성 | VoiceOver | 내레이터·키보드 탐색(Tab 순서, 방향키), 고대비 테마에서 시스템 색 사용 |

**구현 방법**
- 웹뷰 창(설정·분석)은 Svelte 컴포넌트 층을 하나 두고 `mac`·`win` 두 스킨을 만든다. 값·동작 로직은 공유하고, 배치(위쪽 탭 / 왼쪽 탐색)와 스타일만 갈라진다. Fluent 2 색·간격·모서리 값은 디자인 토큰으로 정리한다.
- Tauri 창 효과로 Mica를 켜고, 웹뷰 배경은 투명하게 둔다.
- 플라이아웃·호버 패널은 네이티브 창이다. Windows는 일반 창에 DWM 시스템 배경(`DWMWA_SYSTEMBACKDROP_TYPE` = 일시 창용 Acrylic)과 둥근 모서리(`DWMWA_WINDOW_CORNER_PREFERENCE`)를 켜고, 우리가 그린 반투명 내용(tiny-skia)을 DirectComposition으로 그 위에 올린다. macOS는 `NSVisualEffectView` 위에 같은 그림을 CALayer로 올린다. 이 방식은 0단계 S9에서 먼저 확인한다.
- 지원 대상: **Windows 11(22H2 이상)을 기준으로 디자인**하고, Windows 10 22H2에서는 Mica·Acrylic 없이 동작만 보장한다.

---

## 7. 데이터 이전 (Swift 버전 → Rust 버전)

Rust 버전을 처음 실행할 때 한 번만 한다. 원본은 지우지 않는다(되돌릴 수 있게).

| 대상 | 원본 | 방법 |
|---|---|---|
| 주 설정 | `~/Library/Preferences/com.usagemeter.app.plist`의 `usagemeter.currentSettings`(JSON 데이터) | `plist`로 읽고 같은 필드명의 serde 구조체로 해석. 레거시 필드 마이그레이션(`edges`/`zones` → `segments`, `showPercentInMenuBar` → 전체 AI, `providerOrder` 정규화) 규칙도 옮긴다 |
| 디스플레이별 설정 | `usagemeter.display.<UUID>` | 같은 방식. 화면 UUID는 같은 API라 그대로 맞는다 |
| 프리셋·기본 상태 | `usagemeter.savedPresets`, `usagemeter.defaultSettings` | 같은 방식 |
| 화면 관련 | `usagemeter.mainDisplayUUID`, `usagemeter.disabledDisplays` | 그대로 |
| 기억해 둔 세부 항목 | `knownSubItems` | 그대로 |
| 마지막 알린 버전 | `usagemeter.lastNotifiedVersion` | 그대로 |
| 사용 이력 | `~/Library/Application Support/UsageMeter/history.json` | SQLite로 가져온다. 시각은 **2001-01-01 기준 초**이므로 `+978307200`해서 Unix 시각으로 바꾼다. `ratios`·`buckets`·`items` 모두 행으로 펼친다 |
| 로그인 쿠키 | `~/Library/WebKit/com.usagemeter.app/` | 번들 id를 `com.usagemeter.app`로 유지하면 WKWebView 기본 저장소를 그대로 쓸 가능성이 높다. **0단계에서 확인**. 안 되면 첫 실행 때 한 번 다시 로그인 안내 |

- 설정 스키마 버전은 Swift의 11 다음인 **12**부터 시작한다.
- Rust 설정은 JSON 파일(`settings.json`, `displays/<UUID>.json`, `presets.json`)로 저장한다. 필드명은 Swift와 같게 유지해 이전·디버깅을 쉽게 한다.

---

## 8. 새 기능: 로컬 사용 기록 분석 (토큰 통계)

지금 분석은 "한도의 몇 %가 줄었나"만 본다. 각 도구가 이 컴퓨터에 남기는 기록 파일을 읽으면 **실제 토큰 수, 모델, 프로젝트, 세션** 단위로 볼 수 있다. 이식 6단계에서 만든다.

### 8-1. 기록 소스 (2026-11-16 이 컴퓨터에서 구조 확인)

| 도구 | 위치(macOS / Windows) | 읽을 내용 | 확인한 필드 |
|---|---|---|---|
| Claude Code | `~/.claude/projects/**/*.jsonl` / `%USERPROFILE%\.claude\projects` (환경변수 `CLAUDE_CONFIG_DIR`가 있으면 그 아래) | `type == "assistant"` 줄의 `message.usage`, `message.model`, `timestamp`, `cwd`, `sessionId`, `isSidechain` | `usage`: `input_tokens`, `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`, `output_tokens_details.thinking_tokens`, `cache_creation.ephemeral_1h/5m_input_tokens`, `service_tier` |
| Codex CLI | `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` / `%USERPROFILE%\.codex\sessions` (`CODEX_HOME` 반영) | `token_usage_record`(새 형식) 또는 `event_msg/token_count`의 `last_token_usage`, `turn_context.model`, `session_meta.cwd`, **`rate_limits`** | `input_tokens`, `cached_input_tokens`, `cache_write_input_tokens`, `output_tokens`, `reasoning_output_tokens`; `rate_limits.primary/secondary.used_percent·window_minutes·resets_at` |
| Antigravity CLI | `~/.gemini/antigravity-cli/` / `%USERPROFILE%\.gemini\antigravity-cli` | `conversation_summaries.db`(대화별 `step_count`, `last_modified_time`, `workspace_uris`), `conversations/*.db`(대화별 SQLite) | 토큰 수가 있는지는 **0단계에서 스키마 조사**. 없으면 대화 수·단계 수·프로젝트별 활동량만 낸다 |

**보너스:** Codex 기록의 `rate_limits`는 로그인 없이 Codex 5시간·주간 사용률을 주는 소스다. CLI를 쓸 때마다 갱신되므로 5-1에서 Codex 3순위 소스로 쓴다.

### 8-2. 수집 방식

- SQLite 테이블
  - `ingest_files(path, size, mtime, offset, source)`: 파일별 읽은 위치.
  - `token_events(t, source, model, project, session, input, output, cache_read, cache_write, reasoning, sidechain, dedup_key UNIQUE)`.
  - `codex_limits(t, primary_used, primary_window_min, primary_resets_at, secondary_used, …)`.
- 중복 제거: Claude Code는 한 응답이 여러 줄로 기록되며 같은 `usage`가 반복되므로 `message.id`(+`requestId`)로 한 번만 센다. Codex는 `token_usage_record`가 있으면 그것을, 없으면 `token_count.last_token_usage`를 턴 id 기준으로 한 번만 센다.
- 처음 실행: 보관 기간(기본 35일, 설정 가능) 안에 수정된 파일만 읽는다. 이 컴퓨터의 Claude Code 기록은 1.2GB라 처음엔 수십 초 걸릴 수 있으므로 진행률을 보여준다.
- 이후: `notify`로 세 폴더를 감시하고 2초 디바운스 후 새로 붙은 줄만 읽는다. 파일 크기가 줄었으면(교체·압축) 처음부터 다시 읽는다.
- **개인정보:** 숫자, 모델 이름, 프로젝트 폴더 이름(경로 끝부분만), 세션 id만 저장한다. 대화 내용·프롬프트·파일 내용은 읽어도 저장하지 않는다. 소스마다 켜고 끄는 설정(기본 꺼짐, 처음에 설명과 함께 켤지 묻기), "프로젝트 이름 숨기기" 옵션을 둔다.

### 8-3. 분석 화면 ("토큰" 보기)

분석 창 위쪽에 보기 전환 `한도 | 토큰`을 두고, 기간·AI 필터는 공유한다.

1. **요약**: 기간 총 토큰, 일 평균, 캐시 적중률(`cache_read / (input + cache_read)`), 세션 수, 턴당 평균 토큰.
2. **모델별 토큰 추이**: 기간별 막대를 모델별로 쌓는다.
3. **토큰 구성**: 입력·출력·캐시 읽기·캐시 쓰기·추론(생각) 비율.
4. **프로젝트별 상위 10개**: 토큰과 세션 수.
5. **요일 × 시간대 히트맵**(토큰 기준).
6. **한도 환산**: 같은 시간대의 토큰 사용과 한도 % 하락을 맞춰 "5시간 한도 1% ≈ 약 N 토큰"을 모델별로 추정한다. 표본 수와 신뢰도를 같이 보여주고, 표본이 적으면 표시하지 않는다.
7. **API 환산 비용(선택, 기본 꺼짐)**: 사용자가 고칠 수 있는 단가표로 계산한 추정치. "구독 요금과 무관한 참고값"이라고 표시한다.
8. **Codex 한도 이력**: 로컬 기록의 `rate_limits`로 그린 사용률 추이(로그인 없이도 보임).

---

## 9. 단계별 진행 계획과 통과 기준

기간은 혼자 + AI 도움 기준의 대략적인 추정이다. 각 단계는 "통과 기준"을 모두 만족해야 끝난다. 모든 단계에 **메모리 측정**이 포함된다.

### 0단계: 사전 검증(스파이크)과 결정 · 약 1~1.5주

작은 실험으로 위험한 가정을 먼저 확인한다. 결과는 `rust/docs/adr/`에 결정 기록(ADR)으로 남긴다.

| 번호 | 실험 | 통과 기준 |
|---|---|---|
| S1 | Tauri 트레이만 띄운 빈 앱의 평상시 메모리, 웹뷰 창 열고 닫은 뒤 회수 (macOS·Windows) | 평상시 macOS ≤ 30MB, Windows ≤ 25MB. 창 닫고 60초 뒤 +10MB 이하. 실패하면 Slint로 같은 실험 후 결정 |
| S2 | 네이티브 띠 창 시제품(tiny-skia로 그린 선 1개): 클릭 통과, 최상위, 모든 Space·가상 데스크톱, 전체화면 앱 위, 다중 모니터·혼합 DPI | 두 OS에서 모두 표시·클릭 통과. 화면당 메모리 목표 충족 |
| S3 | Claude·Codex 직접 호출(reqwest + 웹뷰에서 꺼낸 쿠키) 성공률, Cloudflare 차단 여부 | 24시간 동안 5분 간격 성공률 측정. 95% 이상이면 1순위 채택, 아니면 임시 웹뷰를 1순위로 |
| S4 | 임시 웹뷰 + 빈 오리진 문서: macOS `loadSimulatedRequest`, Windows `WebResourceRequested`로 같은 결과 | 두 OS에서 Claude·Codex 조회 성공, 조회 후 웹뷰 파기 시 메모리 회수 |
| S5 | 번들 id `com.usagemeter.app` 유지 시 Swift 버전 로그인 쿠키를 wry가 그대로 쓰는지 | 재로그인 없이 조회 성공(실패하면 재로그인 안내로 결정). **결과: 통과(ADR 0004)** |
| S6 | Windows 저수준 마우스 훅의 CPU 사용과 안정성 | 평상시 CPU 0.1% 이하 |
| S7 | Antigravity `conversations/*.db` 스키마에 토큰 수가 있는지, Windows용 `agy` 존재 여부 | 조사 결과 기록 |
| S8 | `cosmic-text`로 한·영·일 글자 렌더 품질과 메모리 | 12pt 글자가 Swift 패널과 비교해 읽기에 문제없음, +8MB 이하 |
| S9 | Windows 플라이아웃: 일반 창 + Acrylic 시스템 배경 + DirectComposition으로 올린 반투명 내용, macOS는 NSVisualEffectView + CALayer | 두 OS에서 배경 흐림 위에 내용이 깨끗하게 보이고, 열고 닫을 때 메모리가 돌아온다 |
| S10 | MSIX 패키징 시제품(x64·ARM64), `startupTask`, 토스트 | 로컬 설치(개발자 모드) 후 자동 실행·알림 동작 |

### 1단계: 핵심 로직 (`um-core`) · 약 1.5~2주

- 모델·설정 스키마(모든 필드·기본값·범위), 세그먼트 그래프, i18n(생성기로 335개 키 변환), 이력 SQLite, 분석 계산, 알림 판정, 버전 비교.
- Swift 설정·이력 가져오기(7장).
- **통과 기준:** 단위 테스트 통과. 실제 `history.json`을 가져와 Swift 분석 창과 **같은 숫자**(누적 소비, 리셋 횟수, 창 평균 사용률 등)가 나온다. 세그먼트 유효성 판정이 Swift와 모든 조합(2^10)에서 같다.

### 2단계: 사용량 수집 (`um-providers`) · 약 1.5~2주

- Claude·Codex·Gemini(agy·웹), 조회 순서와 자동 전환, 로그인 창·팝업·로그아웃, 갱신 스케줄러, agy 활동 감지.
- 이 단계 끝에는 화면 없이 콘솔에 스냅샷을 찍는 개발용 실행 파일을 만든다.
- **통과 기준:** 세 AI 모두 두 OS에서 Swift 버전과 같은 값. 평상시(웹뷰 없음 경로) 메모리 목표 충족. 24시간 연속 실행 누수 없음.

### 3단계: 테두리 오버레이 · 약 2~2.5주 (가장 정교한 부분)

- `SegmentChainShape` 이식, 채움·페이드·트랙·쌓기·겹침 분할·노치·스쿱·endOverride.
- OS별 띠 창, 디스플레이 관리, 장치 맞춤.
- **통과 기준:** 기준 이미지 테스트(10장)에서 Swift 렌더와 픽셀 차이가 허용치 이하(안티에일리어싱 차이만). 화면당 메모리 목표 충족. 화면 연결·해제·해상도 변경에서 정상.

### 4단계: 트레이·호버·팝업 · 약 1.5주

- 링 아이콘·퍼센트 이미지, 팝업 패널(카드·미니 차트), 네이티브 메뉴 전체 항목, 호버(가장자리·모서리·트레이), 알림 발송.
- **통과 기준:** 부록 A의 메뉴·호버 항목 전부. 평상시 메모리 목표 유지.

### 5단계: 설정 창 · 약 2주

- 11개 탭 전부, 디스플레이 선택·복사, 프리셋·기본 상태, 업데이트 줄, 선 위치 끌기, 지원 탭.
- **통과 기준:** 부록 A 설정 항목 전부. 창을 열고 닫았을 때 메모리 목표 충족.

### 6단계: 분석 창 + 로컬 기록 분석 · 약 2주

- v1.5.0 분석 화면 이식, 8장의 토큰 분석 추가.
- **통과 기준:** 1.5.0 분석 창과 같은 숫자. 1.2GB 로그 첫 수집이 메모리 목표 안에서 끝남. 이후 새 줄만 읽음.

### 7단계: Windows 마무리 · 약 1.5주

- DPI, 가상 데스크톱, 토스트, 자동 실행, Fluent 스킨 마무리, 플라이아웃, NSIS·MSIX 패키지(x64·ARM64), Store판 기능 플래그, GitHub판 업데이터, winget·Scoop, Store 제출 자료(설명·스크린샷·개인정보 처리방침).
- **통과 기준:** Windows 11 x64·ARM64(및 Windows 10 22H2 x64)에서 설치 → 로그인 → 테두리 → 분석 → 업데이트(GitHub판) → 제거까지 수동 점검표 통과. Store판은 Partner Center 사전 검증(패키지 업로드 검사) 통과.

### 8단계: macOS 마무리와 전환 · 약 1주

- 유니버설 빌드, ad-hoc 서명, Homebrew cask 갱신, 번들 교체 감지 재실행, 데이터 이전 확인.
- 베타(GitHub 사전 릴리스) 1~2주 운영 → 2.0.0 정식 → Swift 코드는 `legacy-swift` 태그 후 정리.
- **통과 기준:** 부록 A 전 항목 체크, 메모리 목표 전 항목 충족, 베타 기간 치명적 문제 없음.

**합계: 약 14~17주.** 0단계 결과(특히 S1·S3)에 따라 크게 달라질 수 있어, 0단계가 끝나면 이 일정을 다시 쓴다.

---

## 10. 테스트 전략

| 종류 | 방법 |
|---|---|
| 핵심 로직 단위 테스트 | `um-core` 전부. 세그먼트 그래프 2^10 조합, 설정 기본값·범위, 버전 비교, 알림 판정, 분석 계산 |
| Swift와의 숫자 비교 | 실제 `history.json`을 고정 자료로 두고, Swift 쪽에서 같은 계산 결과를 JSON으로 내보내는 작은 도구를 만들어 Rust 결과와 비교 |
| 응답 해석 | Claude·Codex·agy 응답 예시(개인정보 제거)를 고정 자료로 두고 파싱 결과 비교. 응답 필드가 바뀌는 경우(누락·이름 변형) 포함 |
| 테두리 기준 이미지 | `tools/border-fixtures`: Swift `BorderView`를 `ImageRenderer`로 PNG로 뽑는 macOS 전용 도구. 대표 설정 40가지 안팎(세그먼트 조합, 곡률, 노치, 스쿱, 겹침 분할, 페이드, 잔여율 0·0.37·1)을 Swift와 Rust로 각각 렌더해 비교 |
| 로컬 기록 | 기록 파일 예시(숫자만 남기고 내용 제거)로 중복 제거·증분 읽기·파일 교체 처리 검증 |
| 메모리 | 4-4의 시나리오를 단계마다 두 OS에서 실행해 기록 |
| 수동 점검표 | 부록 A를 체크리스트로 사용, 두 OS 각각 |
| CI | GitHub Actions에서 macOS·Windows 빌드와 단위 테스트. GUI·메모리 측정은 로컬에서 |

---

## 11. 배포·업데이트

| | macOS | Windows |
|---|---|---|
| 빌드 | `universal-apple-darwin`(arm64 + x86_64) | **x86_64 + ARM64** (`x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`) |
| 패키지 | `.app` zip (지금과 같음) | **Store용 MSIX 번들**(x64 + ARM64), 직접 배포용 NSIS 설치 파일(`-setup.exe`, 아키텍처별), 포터블 zip |
| 서명 | ad-hoc (지금과 같음, 첫 실행 시 우클릭 → 열기) | **Store MSIX는 Microsoft가 서명**(인증서 불필요). GitHub 직접 배포 NSIS는 베타 동안 서명 없이(SmartScreen 경고), 정식 전에 서명 도입 여부 결정 |
| 배포 채널 | GitHub 릴리스 + Homebrew cask(`wwhalefe/tap/usagemeter`, 탭의 30분 주기 bump 유지) | **Microsoft Store(주 채널)** + GitHub 릴리스(NSIS·zip) + winget(Store 원본 `msstore` id로 연결) + Scoop 버킷 |
| 앱 내 업데이트 | 지금처럼 확인·다운로드·번들 교체 감지 재실행 | **Store판: 앱 내 업데이트 기능을 빼고 Store가 업데이트**(Store 정책). GitHub판: Tauri 업데이터(서명된 `latest.json`). Scoop판: 안내만 |
| 버전 | 2.0.0부터. `Cargo.toml` 버전 하나에서 Info.plist·exe 버전 리소스를 만든다 | 같음 |

### 11-1. Microsoft Store

**빌드 구분:** Cargo 기능 플래그 `store`로 Store판을 따로 빌드한다. Store판은 업데이트 확인·다운로드 UI, 번들 교체 재실행, 레지스트리 자동 실행 코드를 빼고 아래 MSIX 기능으로 대체한다.

| 항목 | 내용 |
|---|---|
| 패키지 | MSIX(전체 신뢰 데스크톱 앱, `runFullTrust`), x64·ARM64를 `.msixbundle`로 묶음. Windows SDK의 `makeappx`로 만들고 `AppxManifest.xml`은 저장소에서 관리 |
| 자동 실행 | 레지스트리 Run 키 대신 MSIX `windows.startupTask` 확장. 사용자가 설정에서 켜고 끄며, Windows 설정 > 시작 앱에서도 바뀐다 |
| 알림 | 패키지 id가 앱 사용자 모델 ID가 되어 토스트가 바로 동작 |
| 저장 위치 | `%APPDATA%`·`%LOCALAPPDATA%` 쓰기는 패키지 전용 위치로 자동 리디렉션됨(제거 시 함께 삭제). 코드 변경 불필요, 경로 표시만 주의 |
| 다른 도구의 기록 읽기 | 전체 신뢰 앱이라 `%USERPROFILE%\.claude` 등 실제 경로를 읽을 수 있다. 읽기 전용, 사용자 동의 후(8장) |
| WebView2 | Windows 11에는 기본 포함. 매니페스트에 의존성 표시 |
| 정책 대응 | ① 개인정보 처리방침 URL 필수(로그인 세션·사용량·로컬 기록 처리 방식 설명, 외부 전송 없음 명시) ② 스스로 업데이트 금지 → Store판 업데이트 기능 제거 ③ 설명·스크린샷·연령 등급 설문 ④ 제3자 서비스 로그인 화면을 앱 안에서 띄운다는 점을 설명에 명시 |
| 사용자가 직접 해야 하는 일 | Partner Center 개발자 계정 등록, 앱 이름 예약(`UsageMeter`가 이미 쓰였으면 대안 이름), 개인정보 처리방침 페이지 공개(GitHub Pages 등), 최종 제출 버튼 |
| 제출 자동화 | 이름 예약 후 Partner Center의 패키지 id·게시자 id를 매니페스트에 넣고, 릴리스 때 `msixbundle`을 GitHub 릴리스 자산으로 함께 만든다. 업로드는 Partner Center(또는 Store 제출 API)에서 |

---

## 12. 위험 요소와 대응

| 위험 | 영향 | 대응 |
|---|---|---|
| 직접 호출이 Cloudflare에 막힘 | 평상시 웹뷰가 필요해 메모리 목표 미달 | 자동으로 임시 웹뷰 경로. 그래도 평상시엔 웹뷰가 없고 조회 순간만 뜬다 |
| 서비스 API·화면 변경 | 조회 실패 | 지금처럼 방어적 파싱, 실패 원인 로그, 응답 키 목록을 세션당 한 번 기록 |
| Tauri 창을 닫아도 메모리가 안 돌아옴 | 목표 미달 | S1에서 먼저 확인, 실패 시 Slint |
| 테두리 모양이 Swift와 미묘하게 다름 | 사용자 체감 품질 저하 | 기준 이미지 테스트, 2차 베지어 그대로 이식 |
| Windows 가상 데스크톱·전체화면 동작 | 테두리가 사라짐 | S2에서 확인, 데스크톱 전환 이벤트 대응 |
| 혼합 DPI 다중 모니터 | 선 두께·위치 오차 | Per-Monitor v2, 모든 계산을 pt로 하고 그릴 때만 px 변환 |
| WebView2 런타임 없음(오래된 Windows 10) | 로그인 불가 | 설치 프로그램에 부트스트래퍼 포함 |
| Google이 앱 내장 웹뷰의 로그인을 막음(Windows Gemini 웹 로그인) | Gemini 웹 경로 사용 불가 | Gemini는 agy가 1순위라 영향이 작다. 막히면 Gemini 웹 로그인은 macOS 전용으로 두고 안내 |
| Store 심사 반려(정책·이름 중복) | 출시 지연 | 정책 대응표(11-1)를 제출 전 점검, 이름 후보 2개 준비 |
| 로컬 기록 형식 변경(도구 업데이트) | 토큰 통계 누락 | 필드는 모두 선택적으로 해석, 모르는 줄은 건너뜀, 소스별 "마지막 성공 시각" 표시 |
| 1.2GB 로그 첫 수집 부하 | 시작 직후 느려짐 | 백그라운드·낮은 우선순위·보관 기간 밖 파일 건너뛰기·진행률 표시 |
| 이식 기간 중 Swift 버전 수정 | 두 코드가 어긋남 | Swift 수정은 버그만, 수정할 때마다 부록 A·B에 반영 |

---

## 13. 결정 사항 (확정, 2026-11-16)

| 번호 | 항목 | 확정 내용 |
|---|---|---|
| D1 | 메뉴바 드롭다운 모양 | 두 OS 공통: 왼쪽 클릭 = 정보 패널(macOS 팝업 패널, Windows 플라이아웃), 오른쪽 클릭 = 전체 네이티브 메뉴 |
| D2 | 설정·분석 창 UI 기술 | Tauri 웹뷰 + Svelte, OS별 스킨. 0단계 S1 메모리 기준을 못 넘으면 Slint로 바꾼다 |
| D3 | Codex CLI 토큰 파일(`~/.codex/auth.json`) 사용 | 쓰지 않는다. 로컬 기록의 `rate_limits`와 웹 로그인으로 충분 |
| D4 | 로컬 기록 분석 기본값 | 기본 꺼짐. **처음 분석 창을 열 때** 설명과 함께 켤지 묻는다. 나중에 **설정(표시 탭의 "로컬 사용 기록" 묶음)과 분석 창 토큰 보기의 버튼**에서 소스별로 다시 켜고 끌 수 있다. 끄면 수집을 멈추고, "수집한 기록 지우기" 버튼도 둔다 |
| D5 | Windows 코드 서명 | Store판은 Microsoft 서명. GitHub 직접 배포판은 베타 동안 서명 없이, 정식 전에 결정 |
| D6 | 저장소 구조 | 같은 저장소의 `rust/` 폴더 |
| D7 | Windows ARM64 | 처음부터 x64와 함께 빌드·배포(Store 번들 포함) |
| D8 | Windows 배포 | Microsoft Store를 주 채널로 한다(11-1) |
| D9 | Windows 디자인 | Fluent 2 / Windows 11 기본 앱 디자인을 따른다(6-1) |

---|---|---|---|
| D1 | 메뉴바 드롭다운 모양 | 두 OS 공통: 왼쪽 클릭 = 팝업 패널(카드·차트·주요 동작), 오른쪽 클릭 = 전체 네이티브 메뉴 | macOS만 지금과 똑같은 NSMenu(커스텀 뷰 포함)를 objc2로 따로 구현. 작업량 약 1주 추가 |
| D2 | 설정·분석 창 UI 기술 | Tauri 웹뷰 + Svelte (S1 통과 조건) | Slint |
| D3 | Codex CLI의 `~/.codex/auth.json` 토큰으로 사용량 조회 허용 여부 | 쓰지 않음. 로컬 기록의 `rate_limits`와 웹 로그인으로 충분 | 선택 설정으로 허용(로그인 없이 정확한 값) |
| D4 | 로컬 기록 분석 기본값 | 기본 꺼짐, 처음 분석 창을 열 때 설명 후 켤지 묻기 | 기본 켜짐 |
| D5 | Windows 코드 서명 | 처음엔 서명 없이 베타, 정식 전에 결정 | 처음부터 서명 |
| D6 | 저장소 구조 | 같은 저장소 `rust/` 폴더 | 새 저장소 |
| D7 | Windows ARM64 빌드 | 나중에 | 처음부터 |

---

## 부록 A: 기능 동등성 체크리스트

두 OS 각각 체크한다. (W) 표시는 Windows에서 6장의 대체 동작으로 확인하는 항목이다.

### A-1. 수집·로그인
- [ ] Claude 플랜 이름 매핑(Max 20x·5x, Max, Team, Enterprise, Pro, Free, 그 외는 접두어 제거)
- [ ] Claude 5시간·주간·Opus·Sonnet·모델별 주간(`limits[].kind == weekly_scoped`)
- [ ] Codex 플랜(Plus·Pro·Team·Free·Enterprise·Business), 5시간·주간, 코드 리뷰·추가 한도, 창 길이로 5시간 판정
- [ ] Gemini agy: 버킷 분류, 대표 잔여율 = 가장 급한 값, 5시간 원값 별도, 단조 규칙, 플랜 "Antigravity"
- [ ] Gemini 웹: 셀렉터 → 키워드 근처 % → 로그아웃 판정, 리셋 문구(한·영·일·중 오전/오후, 월·일, 내일·내년 넘김)
- [ ] 상태 반영 규칙(실패 시 직전 정상값 유지), 시작 재시도 4회·2초
- [ ] 로그인 창(이미 로그인돼 있으면 2초 안에 창 없이 끝), 1.2초 완료 확인, 닫기 = 숨김
- [ ] 패스키 비활성 스크립트, User-Agent, 팝업 창, 로드 실패 안내 페이지
- [ ] 로그아웃(도메인 조각별 쿠키 삭제), "로그인 유지" 꺼짐 시 시작할 때 전체 로그아웃
- [ ] 갱신 주기 프리셋·직접 입력, agy 활동 감지(앞·뒤 트리거, 25초 빠른 갱신)

### A-2. 테두리
- [ ] 세그먼트 10종, 유효성(연결·차수 ≤ 2·끝점 0 또는 2), AI별 기본 레이아웃
- [ ] 시작 모서리·방향(가로 먼저 의미)·시작 쪽(왼·가운데·오른), 열린 사슬 뒤집기
- [ ] 가로 끝 위·아래 말기, 스쿱, endOverride 반쪽 곡선
- [ ] 구역별 곡률(메뉴·본문·Dock), 경계 연동 토글
- [ ] 노치 우회(폭·높이·네 곡률), 장치 맞춤, 해상도 변경 비례 조정
- [ ] 채움(길이 기준), 페이드 12조각, 트랙 0.18, 전체 불투명도, 끝 모양
- [ ] 잔여율 순 쌓기, 세부 항목 색 겹침(항목별 표시·색)
- [ ] 겹침 금지, 겹침 분할(레인·폭 전환 블러·노치 공유 규칙)
- [ ] 메뉴선·Dock선 높이(macOS만, Windows는 해당 없음), 띠 창 구성, 클릭 통과, 모든 Space(W: 가상 데스크톱), 전체화면 위
- [ ] 화면별 설정·끄기·다른 화면에 복사, 테두리 숨기기(시작 시 숨김 옵션)

### A-3. 메뉴바·트레이·호버
- [ ] 링 아이콘 규칙, 퍼센트 글자(W: 아이콘 숫자 + 툴팁)
- [ ] 정보 카드(이름·플랜 배지·항목 줄·색 점·소진 예측·마지막 갱신·빈 상태 안내)
- [ ] 미니 차트(6/12/24시간, 호버 툴팁)
- [ ] 메뉴 항목 전부: 분석 보기, 표시 정보(8개 체크, 메뉴가 닫히지 않음), 자동 갱신 주기, 언어, AI 로그인/로그아웃, 사용량 새로고침(⌘R), 앱 설정(⌘,), 프리셋(불러오기·삭제·현재 저장 + 이름 입력), 테두리 숨기기, 이 기기에 맞춤, 개발자에게 리포트(메일·주소 복사), 후원, 버전·업데이트 상태·확인·다운로드·릴리스 페이지, 종료(⌘Q)
- [ ] 호버: 가장자리 판정 거리, 변·모서리 선택, 모서리 크기, 메뉴바 아이콘 호버(W: 트레이), 패널 위치 규칙, 드롭다운 열림 중 숨김

### A-4. 설정 창
- [ ] 상단: 기본 상태 저장·되돌리기, 분석 버튼, 업데이트 줄(상태·확인·다운로드·릴리스·자동 확인), 디스플레이 선택(2대 이상)
- [ ] 로그인 탭(상태 점, Gemini 실험적 표시, 로그인 유지, 로그인 시 자동 실행)
- [ ] (새 기능) 로컬 사용 기록: 소스별 켜기·끄기, 프로젝트 이름 숨기기, 수집한 기록 지우기, 처음 분석 창 열 때 묻기
- [ ] 프리셋 탭(최대 10개, 디스플레이별 불러오기·저장 규칙)
- [ ] 표시 탭(언어, 외관, AI 순서, 호버 설정, 메뉴바 아이콘 AI, % 표시 AI, 드롭다운 정보 8개, 차트 시간·호버, 알림·임계값)
- [ ] 주기 탭, 선 탭(두께 1~15·0.5, 투명도, 트랙, 시작 시 숨김, 페이드, 겹침 분할, AI 색 + 프리셋 7색)
- [ ] 세부 항목 탭(분할 토글, 항목별 색·바·메뉴·되돌리기, 마지막으로 읽은 항목 표시)
- [ ] 구분선 탭(macOS만: 메뉴선 5~200, Dock선 5~400, 끌어서 조절)
- [ ] 레이아웃 탭(겹침 금지, 끝 위·아래, 사슬·루프 컨트롤, 모양 프리셋 4개, 대화형 다이어그램)
- [ ] 모서리 탭(구역별 4개 0~80, 사용 안 하는 모서리 비활성, 연동 토글)
- [ ] 노치 탭(macOS만): 켜기, 폭 0~500, 높이 0~100, 곡률 4개 0~40, 감지
- [ ] 지원 탭(메일·복사·후원·카카오페이 QR)

### A-5. 분석·기타
- [ ] 분석 창 v1.5.0 전부(5-6)
- [ ] 알림(기본 꺼짐, 임계값, 리셋별 1회)
- [ ] 업데이트 확인 24시간, 새 버전 알림 1회, 다운로드, 번들 교체 재실행(macOS)
- [ ] 한·영·일 335개 문자열, 외관(시스템·밝게·어둡게)
- [ ] 설정 저장 300ms 디바운스, 스키마 버전, 첫 실행 장치 맞춤
- [ ] Dock 아이콘 표시 규칙(macOS), 단일 인스턴스

---

## 부록 B: 그대로 옮길 상수·규칙

| 항목 | 값 | Swift 위치 |
|---|---|---|
| 띠 창 크기(지금) | 세로 150pt, 가로 170pt, 이음 겹침 2pt, 가로 띠는 좌우 148pt 안쪽 | `OverlayFrameController.swift:39-42, 92-108` |
| 페이드 | 12조각, 기본 0.012(0~0.2) | `BorderView.swift:679`, `OverlaySettings.swift:399` |
| 트랙 불투명도 | 0.18 | `BorderView.swift:752-756` |
| 겹침 분할 폭 전환 | 6pt 조각, 48~2048조각, 블러 반지름 `(max(600, t·140)/2)/조각길이`, 3회, 0.02 미만 차이 병합 | `BorderView.swift:768-815` |
| 호버 판정 | 가장자리 `max(6, 두께)+7`, 모서리 기본 80(20~300) | `HoverInfoController.swift:98-108` |
| 메뉴바 호버 영역 | `barH = max(화면위-visibleFrame.maxY, 아이콘 높이, 메뉴바 두께)`, `padX = clamp(화면폭·0.006, 8, 24)`, `padBelow = max(4, barH·0.25)` | `HoverInfoController.swift:121-134` |
| 호버 패널 | 커서 +14, 글자 12pt medium, 여백 10/6, 모서리 7 | `HoverInfoController.swift:33-62, 148-159` |
| 링 아이콘 | 18×18, inset 2.5, 선 2.4, 반지름 6.5, 트랙 알파 0.28(흐림 0.25) | `StatusBarController.swift:904-932` |
| 정보 카드 | 간격 9, 여백 16/10, 최소 폭 340, 배지 알파 0.16 | `MenuInfoCardView.swift` |
| 미니 차트 | 높이 92, 뷰 340×118, y 0/50/100 | `MiniChartView.swift` |
| 디바운스 | 아이콘·호버 150ms, 설정 저장 300ms, 이력 기록 500ms, 알림 판정 500ms | 여러 곳 |
| 이력 | 35일 보관, 45초 최소 간격(변화 0.0005 초과 시 즉시) | `HistoryStore.swift` |
| 소진 예측 | 최근 3시간, 0.05시간 초과, 시간당 0.0005 초과 | `HistoryStore.swift` |
| 분석 | 리셋 +0.03, 한도 도달 0.02, 다운샘플 240칸 최저·최고 | `AnalyticsModel.swift` |
| 장치 맞춤 | 위 곡률 `round(22·clamp(화면폭/1710, 0.5, 2))` 최대 80 | `OverlaySettings.swift:1078-1092` |
| 화면 변경 비례 | 1% 넘을 때, 0.5~2배, 구역 곡률 최대 80, 노치 곡률 최대 40 | `OverlaySettings.swift:1049-1073` |
| agy | 12초 제한, 단조 규칙 0.12, 감시 지연 0.4초, 앞 트리거 7초/1.2초, 뒤 트리거 2.5초 + 6.5초, 빠른 갱신 25초·활동 600초 | `AgyIntegration.swift`, `RefreshScheduler.swift` |
| 웹 세션 | 준비 대기 12초, 재로드 후 1.5초, 로그인 확인 1.2초, 로그인 창 460×720, 팝업 480×680 | `WebSession.swift`, `LoginWindowController.swift`, `PopupWebView.swift` |
| 업데이트 | 15초 제한, 24시간 주기, 번들 확인 10초·2회 연속 | `UpdateChecker.swift` |
| 프리셋 | 최대 10개, 내장 표(곡률 0·6·12·20·32, 두께 1·1.5·2·3·5, 노치 폭 160·200·230·260, 브랜드 7색) | `OverlaySettings.swift:950, 1362-1389` |
| 기본 AI 색 | Claude (0.85, 0.47, 0.34), Gemini (0.26, 0.52, 0.96), Codex (0.06, 0.64, 0.50) | `Providers.swift` |
| 세부 항목 팔레트 | 첫 항목 = AI 색, 이후 보라·호박·청록·분홍·연두 | `OverlaySettings.swift:901-907` |

---

## 부록 C: 고칠지 유지할지 정할 기존 동작

이식하면서 발견한 Swift 버전의 특이한 동작이다. 기본 방침은 "그대로 옮기고, 명백한 버그만 고친다"이다.

| 동작 | 방침 |
|---|---|
| `clockwise`가 실제로는 "가로 먼저"를 뜻해 TR·BL 시작이면 반시계 방향 | **유지**(기존 사용자 설정 모양이 바뀌면 안 됨). 설정 화면 문구만 정확하게 |
| Gemini 로그아웃이 `google.com`이 들어간 모든 쿠키를 지움 | **고침 후보**: 앱 전용 웹뷰 저장소라 다른 브라우저에는 영향이 없지만, Gemini 관련 쿠키만 지우도록 좁힌다 |
| `UsageStatus.stale`이 정의만 있고 쓰이지 않음, "오래된 값 흐리게 표시"도 미구현 | 유지(동등성 우선). 2.x에서 개선 후보 |
| 알림 중복 방지 기록이 메모리에만 있어 재시작하면 같은 알림이 다시 옴 | **고침**: SQLite에 저장 |
| 프리셋 기본 이름 "프리셋 N"이 한국어로 고정 | **고침**: i18n 키로 |
| 프리셋 최대 개수 주석(5)과 실제(10) 불일치 | 실제 10을 따름 |
| 선 위치 끌기 화면에서 Esc로 닫히지 않음 | **고침**: Esc 지원 |
| 이력 저장 시 파일 전체를 다시 읽어 합침 | SQLite로 바꾸며 자연히 해결 |
