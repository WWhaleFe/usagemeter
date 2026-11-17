# macOS 설정 창 Swift 대조 기록

2026-11-17. 최종 기준은 Swift 소스이며, 1.4.2 설정·분석 기준 캡처 15장도 직접 열어 확인했다. macOS 전용 스타일은 `data-skin="macos"`로 분리했고 기존 Windows `win` 분기와 Fluent 토큰은 유지했다.

## 탭별 수정과 소스 대조

| 영역 | 수정 또는 확인한 내용 | 설정 연결 및 조건 |
| --- | --- | --- |
| 공통 상단 | 제목이 상자 안에 있는 기본 상태 섹션, 회색 저장·초기화 버튼, 아이콘 분석 버튼, 구분선, 굵은 버전, 업데이트 버튼 3개, 설치 안내와 작은 체크박스 | 기존 defaultSave/defaultReset 및 업데이트 action 유지. 저장된 기본 상태가 없으면 초기화 비활성. 다중 화면 선택과 복사 안내도 별도 상자로 배치 |
| 탭 패널 | 회색 선택 탭, 가운데 탭 줄이 패널 윗변에 걸치는 배치, 내부 스크롤과 여백 | 탭 순서는 Swift와 동일. macOS 설정 760×880, 분석 1000×720 고정 크기 |
| 로그인 | 회색 로그인 버튼, 9px 상태 점, 이름·상태 글꼴, AI 사이 구분선, 체크박스 | Claude/Gemini/Codex 고정 순서. keepLoggedIn, autostart action 연결. 번들이 아니면 자동 실행 비활성 |
| 저장 불러오기 | 상자 안 제목, 저장된 이름 버튼 폭, 테두리 없는 삭제 버튼 | 최대 10개 제한, presetSave/Load/Delete 유지. 다른 화면 편집 시 안내 조건 확인 |
| 표시 | 전체 폭의 회색 언어·외형 세그먼트, 테두리 없는 순서 버튼, 호버 체크박스와 행 간격, 넓은 메뉴 팝업 | providerOrder, hoverEdges/Corners, hoverCornerSize 20..300/5, 메뉴 아이콘·%·드롭다운·차트·75/90/95 알림 설정 연결 확인. 차트가 꺼지면 기간과 호버 옵션 비활성 |
| 주기 | 넓은 팝업, 구분선, 위쪽 레이블과 숫자 칸·세로 스테퍼 | 프리셋 1/2/3/4/5/10/15/20/30/60/120. 사용자 주기 1..180/1, 정수 반올림. 사용자 설정과 프리셋의 상호 비활성 조건 유지 |
| 선 | 두께·투명도·그라데이션 슬라이더, 체크박스, 겹침 안내 두 줄과 순서, AI 선색 문구와 22px 팔레트 | thickness 1..15/0.5, 투명도 0..100/1 → lineOpacity, 그라데이션 0..20/1 → fadeFraction. macOS의 불필요한 정수 반올림 제거. fadeEnabled에 따른 비활성 유지 |
| 세부 항목 | 색·230px 이름·70px 미니 바·잔여/차감·바/메뉴·색 초기화를 한 행에 배치 | subItemSplitIDs/Colors/BarHidden/MenuHidden 확인. 색과 바만 분리 설정에 따라 비활성. 로그아웃·조회 실패의 마지막 항목은 0.6 불투명도로 계속 편집 가능 |
| 화면 분할 | 공통 상자·체크박스·슬라이더·회색 드래그 버튼 적용 | 메뉴 5..200/1, Dock 5..400/1. 각 선이 꺼지면 해당 슬라이더 비활성. 둘 다 꺼지면 dragLines 비활성 |
| 배치 | 370px 옵션 열, 209px 프리셋/다이어그램 열, 작은 2×2 버튼, 상자 간격, 끝 캡 행 정렬 | providerLayouts 맵을 보존하는 기존 연결 확인. 열린 체인은 반전만 표시. 닫힌 고리는 시작 모서리·방향·필요한 곡선 시작점 표시. 가로선 맨 위의 위 캡과 맨 아래의 아래 캡 비활성 |
| 모서리 곡률 | 공통 상자와 두 열 슬라이더 적용, 단위 중복 표시 제거 | 각 영역 0..80/1. menuBar/main/dock 노출 조건, cornerUsed 비활성, linkMenuMainRadii/linkMainDockRadii의 이웃 꼭짓점 연결 확인 |
| 노치 | 크기·곡률 슬라이더, 세부 그룹 전체 비활성/흐림 처리 | 폭 0..500/1, 높이 0..100/1, 곡률 0..40/1. outerLeft/innerLeft/innerRight/outerRight 순서. detectNotch action은 감지 실패 시 끄는 기존 Rust 동작 확인 |
| 개발자에게 | 가운데 제목·설명·이메일 상자·버튼, 보내기/복사/링크 아이콘, 후원과 QR 간격 | 메일·복사·후원 링크·QR action 유지. 외부 메일 작성이나 송금 링크 실행은 검증 과정에서 호출하지 않음 |
| 분석 | Swift 제목, 17px 헤더, 회색 기간 선택, 기본 상자 색 | 최신 Swift 소스의 KPI·차트·항목 필터·상세 표·주의 문구 구성을 유지. 오래된 17_analytics.png와 달라진 차트 순서는 최종 Swift 소스를 따름 |

macOS에서 Swift에 없는 로컬 토큰 기록 섹션과 한도/토큰 보기 전환은 표시하지 않는다. Windows의 해당 기능과 동의 안내는 유지한다.

## 공통 컨트롤과 네이티브 기능

- Section 제목을 둥근 상자 안으로 옮겼다. 기본 13px, 보조 11px와 Swift의 12px 하위 레이블을 적용했다.
- macOS Toggle은 오른쪽 레이블의 체크박스다. Windows 스위치는 기존 구현을 유지한다.
- macOS Slider는 레이블 아래에 눈금 트랙, 48px 숫자 칸, 세로 스테퍼를 둔다. Swift가 문구 안에 둔 단위만 표시한다. 방향키와 범위 제한을 연결했다.
- macOS 선 색과 세부 항목 색은 NSColorPanel에 연결했다. 선 색은 알파를 허용하고 세부 항목은 불투명하다. 선택 즉시 전역 색 설정을 저장하고 기존 changed 경로로 테두리와 메뉴를 다시 그린다.
- 설정 제목은 한국어 `UsageMeter 설정`이며 영어·일본어 제목도 추가했다. 분석은 기존 `analytics.title`을 사용한다. 언어 변경 시 열린 창 제목도 갱신한다.
- 외형 변경과 기본 상태/프리셋 적용은 NSApp 테마도 갱신한다.

## 검사 결과

- `npm run build`: 통과. 생성된 `um-app/ui`도 갱신했다.
- `npm run check`: 0 오류, 0 경고.
- `cargo build -p um-app --release`: 통과. 로그: `rust/target/macos-settings-release.log`.
- `tools/check.sh`: 통과. macOS 130개 테스트 통과, 기존 1개 무시. macOS와 Windows x64/ARM64 clippy 통과. 로그: `rust/target/macos-settings-check.log`.
- Windows 검사 캐시는 쓰기 제한 때문에 기존 캐시를 저장소의 `rust/target/cargo-xwin`에 복사하고 `XWIN_CACHE_DIR`로 지정했다. 검사 스크립트와 사용자 캐시는 수정하지 않았다.
- 설정 화면에서 정적으로 찾은 patch 첫 필드 43개를 SettingsSnapshot 직렬화 이름과 대조했고 불일치가 없었다. 설정 화면의 정적 문구 키도 모두 문자열 표에 존재한다. 동적 맵 필드는 해당 Rust 타입과 직접 대조했다.
- `git diff --check`: 통과. 변경한 소스에 U+2014가 없는 것을 확인했다.
- 추가로 실행한 `cargo fmt --all -- --check`는 기존 워크스페이스 전반의 서식 차이로 실패했다. 범위 밖 파일은 재정렬하지 않았고 새 native_color.rs만 정리했다.

## 남은 검증과 캡처

네이티브 캡처는 현재 실행 환경의 제한으로 실패했다. 앱 실행에서는 macOS LaunchServices/XPC 연결 오류가 났고, 창 목록 도구 winid가 종료 코드 133으로 실패했다. 브라우저 도구도 현재 승인 정책에서 실행할 수 없었고, 로컬 Vite 서버는 listen EPERM, 직접 실행한 헤드리스 브라우저는 종료 코드 133으로 실패했다.

따라서 실제 Rust 창과 Swift 창의 픽셀 비교 2회, 로그인/다중 화면/노치 감지/드래그/네이티브 색 패널 조작과 실제 테두리의 즉시 반영은 검증하지 못했다. CSS로 재현한 탭·체크박스·슬라이더의 AppKit 렌더링 차이도 남아 있을 수 있다. 시각적으로 완전히 같다고 확인한 상태는 아니다. 개발 바이너리는 .app 번들이 아니므로 자동 실행이 비활성화될 수 있고, 버전·로그인·사용량 값은 기준 캡처와 실행 환경에 따라 다르다.

최종 릴리스로 전체 탭 캡처를 두 차례 시도한 위치:

- `rust/target/macos-settings-review/pass1`
- `rust/target/macos-settings-review/pass2`

이 폴더의 capture.log는 실패 기록이며 PNG는 생성되지 않았다. 실패한 캡처를 최종 이미지로 대체하거나 기준 Swift 이미지를 복사하지 않았다.

실제 앱 `/Applications/UsageMeter.app`과 실제 `~/Library/Application Support/UsageMeter` 데이터는 수정하지 않았다. 기존에 수정돼 있던 rust/.gitignore와 기존 미추적 capture-settings.sh/winid.swift는 그대로 보존했다. 커밋·푸시·reset·checkout·브랜치 변경은 하지 않았다.

## 바꾼 파일과 이유

| 파일 | 이유 |
| --- | --- |
| ui-src/src/App.svelte, app.css | macOS 스킨 구분과 Swift 색·글꼴·버튼·팝업·구분선 토큰 |
| ui-src/src/components/Section.svelte, Toggle.svelte, Slider.svelte, Segmented.svelte | 상자 안 제목, 체크박스, 눈금과 세로 스테퍼, 회색 세그먼트 |
| ui-src/src/components/BAiLayout.svelte | Swift 배치 상자의 고정 열 너비와 작은 프리셋 |
| ui-src/src/settings/Settings.svelte, TopArea.svelte | 패널에 걸친 탭과 기본 상태·업데이트·다중 화면 상단 |
| ui-src/src/settings/tabs/Login.svelte, Presets.svelte, Display.svelte, Interval.svelte | 각 탭의 macOS 컨트롤·문구·행 간격·비활성 상태 |
| ui-src/src/settings/tabs/Line.svelte, SubItems.svelte, Layout.svelte, Notch.svelte, Support.svelte | 색상 행과 네이티브 선택, 한 행 세부 항목, 배치, 노치 비활성 그룹, 후원 버튼 |
| ui-src/src/lib/api.svelte.ts | Swift 설정용 AI 순서와 Windows 순서 분리 |
| ui-src/src/analytics/Analytics.svelte, Icon.svelte | 분석의 macOS 외형과 공통 버튼 아이콘 |
| um-app/src/windows.rs, ipc.rs, main.rs, native_color.rs | 창 제목·크기·테마, macOS 네이티브 색 패널과 저장·재그리기 연결 |
| um-core/src/i18n/extra.json | 번역된 설정 창 제목 |
| um-app/Cargo.toml, rust/Cargo.lock | macOS 전용 AppKit 색 패널 의존성 |
| um-app/ui/index.html, assets/* | 프런트엔드 빌드 산출물 |
| rust/docs/macos-settings-parity.md | 탭 대조, 검사 결과, 남은 검증 기록 |
