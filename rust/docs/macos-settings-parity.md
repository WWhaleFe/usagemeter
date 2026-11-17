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

초기 대조에서는 macOS의 로컬 토큰 기록 섹션과 한도/토큰 보기 전환을 숨겼으나, 아래 세 번째 후속 수정에서 결정 D4에 따라 복원했다. 이 기능은 Swift 대조와 별개로 요청된 Rust 포트 기능이다.

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

## 후속 대조: abfc052 이후 남은 차이

2026-11-17. 제공된 `cmp2/rs_*.png` 12장을 보기 도구로 모두 열었고, Swift 기준 캡처와 현재 Swift 소스를 함께 확인했다. 목록의 일부 전제는 실제 소스와 달랐다. 소스가 최종 기준이라는 지시에 따라 해당 속성은 유지했다. Swift 파일과 Windows 스킨은 수정하지 않았다.

### 번호별 처리 결과

| 번호 | 결과 | 수정 또는 유지 근거 |
| --- | --- | --- |
| 1 | 수정 | macOS 빌더에 Visible·hidden_title(false)를 명시. Tauri Visible이 추가하는 FullSizeContentView를 네이티브 창에서 해제하고 Expanded 제목 스타일을 적용했다. 최종 가운데 정렬과 콘텐츠 영역은 실제 캡처로 확인이 필요하다. |
| 2 | 유지 | `SettingsView.swift:125`에 `Divider().padding(.vertical, 2)`가 실제로 있고 기준 캡처에도 보인다. 상단 구분선을 제거하지 않았다. |
| 3 | 수정 | 분석 보기에는 Swift 소스의 `chart.bar.xaxis`, 누적 소비에는 `chart.bar.fill` 원본 SF Symbol을 연결했다. 두 이름을 구분해서 사용한다. |
| 4 | 수정 | macOS 체크박스만 14px·반경 3.5px, 어두운 회색 채움, 약한 경계로 변경. 선택은 기존 테마 토큰의 다크 #0a84ff·라이트 #007aff와 흰 체크를 사용한다. |
| 5 | 수정 | 탭 줄 안쪽에 위아래 2px 여백, 바깥 반경 7px·선택 반경 5px, 글자 굵기 500을 적용했다. |
| 6 | 수정 | macOS body에 `-webkit-font-smoothing: auto`를 적용했다. 버튼·체크박스 전체에 추가 굵기를 강제하지 않았고 탭·슬라이더·호버 소제목만 500으로 보정했다. 최종 글자 인상은 캡처로 확인이 필요하다. |
| 7 | 수정 | 스테퍼의 글꼴 화살표를 동일한 SVG와 180도 회전으로 교체. 두 버튼을 각각 13×10px로 중앙 정렬하고 구분선을 내부 그림자로 그려 하단 잘림을 방지했다. |
| 8 | 일부 수정 | medium 500 적용. 크기는 Swift `LabeledSlider`의 `.subheadline`에 맞춰 기존 12px를 유지했다. 11px caption으로 바꾸지 않았다. |
| 9 | 일부 수정·기존 조건 확인 | 입력창 배경을 다크 #262626으로 변경. 설명은 기존 `preset.displayNote` 키와 `s.editing` 조건을 유지했다. `ipc.rs`의 editing_id는 주 화면이면 None을 반환하므로 Swift의 `border !== settings` 조건과 같다. Rust 캡처에 문구가 없는 것은 다른 화면을 편집하지 않는 상태다. 안내를 항상 표시하지 않았다. |
| 10 | 수정 | 위아래 모두 원본 chevron 심볼·18×16px 버튼·가운데 정렬. 각 행 상하 여백을 2px씩 줄였다. |
| 11 | 일부 수정 | 소제목에 medium 500 적용. Swift의 `.subheadline`을 따라 12px를 유지했고 11px caption으로 바꾸지 않았다. |
| 12 | 수정 | macOS 설정·분석 세그먼트의 비선택 세로 경계를 글자색 4%로 약하게 만들었다. |
| 13 | 유지 | `SettingsView.swift:453`에 구분선이 실제로 있다. 사용자 주기는 sliderRow 전체에 `.disabled(!settings.refreshUseCustom)`가 적용되므로 기존 라벨·컨트롤 비활성 표시를 유지했다. |
| 14 | 수정 | 선 색상 칸의 회색 베젤 안에 1px 어두운 테두리를 넣었다. 기존 네이티브 색 패널 호출은 유지한다. |
| 15 | 수정 | 원형 스와치의 외곽 outline과 간격을 없애고 2.5px 파란 테두리를 색에 밀착시켰다. 비선택 경계는 Swift secondary 40%에 맞춰 검은색도 구별되게 했다. |
| 16 | 기존 조건 확인·보강 | 기존 disabled={!menuOn}/{!dockOn}과 흐림 표시가 Swift:760/768과 동일하고 제공 캡처에도 적용돼 있다. 적용 핸들러에도 같은 조건을 추가해 꺼진 영역 프리셋을 거부한다. Windows 분기는 그대로다. |
| 17 | 유지 | Swift:776/777에 미리보기 크기 209×133이 명시돼 있다. 왼쪽 내용에 따라 다이어그램을 늘리면 소스와 달라지므로 고정 크기를 유지했다. |
| 18 | 수정 | Swift의 너비 범위 0..500·step 1을 유지. 별도 눈금 개수는 소스에 없다. 기존 2px 눈금이 좁은 간격에서 겹쳐 단색으로 보이는 문제를 1px 선·최소 3px 간격으로 해결했다. 값 단계와 숫자 입력은 바뀌지 않는다. |
| 19 | 일부 수정 | 메일 아이콘은 원본 `paperplane.fill`로 교체. 파란 이메일 테두리는 `SupportView.swift:73`에 accent 30%로 명시된 장식이며 포커스 링이 아니므로 유지했다. |
| 20 | 수정 | 카카오페이 묶음을 Swift처럼 별도 세로 그룹으로 배치. 제목 12px·600, 안내 11px, 내부 간격 6px·위 여백 4px로 변경했다. |
| 21 | 수정 | 설정 버튼을 원본 `gearshape`로 교체했다. |
| 22 | 후속 대체 | 당시에는 SF Symbol PNG를 추출했으나, 아래 세 번째 후속 수정에서 원본·번들 PNG와 추출 도구를 삭제하고 macOS 시스템 런타임 생성으로 교체했다. Swift 심볼 이름과 Windows 기존 SVG 경로는 보존했다. |
| 23 | 수정 | 시간대 막대·기간별 막대·잔여량 선 차트의 세로 값 축을 macOS에서 오른쪽으로 옮겼다. Swift Charts의 기본 trailing을 따르고 Windows는 기존 왼쪽 축을 유지한다. |

줄표를 하이픈으로 바꾼 문구와 1.5.0 분석 구성(KPI 4칸, 전체 AI 칩, 차트 순서)은 유지했다. 새 주석은 한국어이며 수정 소스에 U+2014가 없다.

### 후속 검증

- `ui-src`의 `npm run check`: 0 오류, 0 경고.
- `ui-src`의 `npm run build`: 통과. 원본 SF Symbol 21개와 갱신된 UI 산출물을 포함한다.
- `cargo build -p um-app --release`: 통과. 최종 UI와 네이티브 제목줄 보정을 포함한다. 로그: `rust/target/macos-settings-followup-release.log`.
- `rust/tools/check.sh`: 통과. macOS 테스트 130개 통과·기존 1개 무시, macOS 및 Windows x64/ARM64 clippy 통과. XWIN_CACHE_DIR은 기존 저장소 내 `rust/target/cargo-xwin`을 사용했다. 로그: `rust/target/macos-settings-followup-check.log`.
- Windows 아이콘 SVG 경로가 HEAD와 동일하고 macOS SF Symbol 매핑에 빠진 PNG가 없는 것을 확인했다.
- `git diff --check` 및 수정 소스 U+2014 검사 통과.
- 캡처 도구 `rust/tools/winid 0`은 SIGTRAP(종료 코드 -5)으로 실패했다. 로그: `rust/target/macos-settings-followup-capture.log`. 앱은 실행하지 않았고 새 창 캡처는 생략했다. 최종 화면·라이트/다크 외형·다중 화면 안내·제목 가운데 정렬의 실기 검증은 남아 있다.
- 커밋·푸시·reset·checkout·브랜치 변경은 하지 않았다.

### 후속 변경 파일과 이유

| 파일 | 이유 |
| --- | --- |
| `um-app/src/windows.rs` | macOS 네이티브 제목줄 스타일 복원 |
| `ui-src/src/app.css`, `components/Toggle.svelte` | macOS 글꼴 렌더링·체크박스·입력창 배경 토큰 |
| `components/Slider.svelte`, `Segmented.svelte` | 대칭 스테퍼·medium 라벨·눈금 표시·약한 세로 경계 |
| `components/BAiLayout.svelte` | 꺼진 메뉴바·Dock 프리셋의 적용 핸들러 방어 |
| `settings/Settings.svelte`, `TopArea.svelte` | 탭 캡슐·여백·굵기, Swift 분석 버튼 심볼 |
| `settings/tabs/Display.svelte`, `Line.svelte`, `Presets.svelte`, `Support.svelte` | 순서 화살표·소제목·색상 칸·입력창·후원 간격 |
| `analytics/Icon.svelte`, `Analytics.svelte` | macOS SF Symbol 원본 연결, KPI·세그먼트 보정 |
| `analytics/HourChart.svelte`, `LineChart.svelte`, `OverTime.svelte` | macOS 차트 Y축 오른쪽 배치 |
| `ui-src/src/assets/sf-symbols/*.png`, `rust/tools/export-sf-symbols.swift` | 당시 추가했던 심볼 21개와 재생성 도구. 세 번째 후속 수정에서 전부 삭제 |
| `ui-src/tsconfig.json` | Vite 자산 glob의 타입 검사 지원 |
| `um-app/ui/index.html`, `assets/*` | 최종 프런트엔드 빌드 산출물 |
| `rust/docs/macos-settings-parity.md` | 소스와 목록의 불일치, 번호별 결과와 검증 한계 기록 |

## 세 번째 후속 수정: 시스템 심볼·로컬 토큰 기록·제목줄

2026-11-17. 이 절이 현재 구현 기준이다. 앞선 두 작업의 기존 변경은 보존했으며 Windows 스킨은 유지했다.

### 1. SF Symbol 배포 자산 제거

- `ui-src/src/assets/sf-symbols`의 PNG 21개, `rust/tools/export-sf-symbols.swift`, `ui/assets`의 해당 PNG 산출물을 모두 삭제했다. 빌드 결과는 JS·CSS뿐이며 `git status --short`에도 SF Symbol PNG와 추출 도구가 없다. 해당 파일들은 작업 시작 시 미추적 상태였으므로 삭제 후 상태 목록에서 사라진다.
- `um-app/src/symbols.rs`의 동기 Tauri 명령 `ui_symbol(name, size, weight, color)`을 `main.rs`에 등록했다. macOS 외 플랫폼에서는 `None`만 반환한다. 기존 macOS 전용 objc2 의존성을 사용해 새 공용 의존성을 추가하지 않았다.
- macOS 메인 스레드에서 `NSImage::imageWithSystemSymbolName_accessibilityDescription`과 `NSImageSymbolConfiguration`으로 포인트 크기·굵기를 적용한다. 단색 심볼을 투명 RGBA 비트맵에 그려 2배 해상도 PNG data URL로 돌려준다. `color`는 선택적인 `#RRGGBB` 또는 `#RRGGBBAA` 색이며 기본은 불투명 검정이다.
- 인자가 같은 결과는 메인 스레드의 `HashMap`에 보관한다. 캐시는 최대 64 KiB이며 큰 응답은 저장하지 않는다. 파일·설정·브라우저 영구 저장소에는 기록하지 않는다.
- 공통 `analytics/Icon.svelte`가 macOS에서만 명령을 호출한다. 응답은 CSS 알파 마스크와 `currentColor`로 표시해 외형 변경에 즉시 반응한다. `None` 또는 IPC 실패 시 SVG로 대체하고, 늦게 도착한 이전 요청이 새 아이콘을 덮지 않게 했다. `lib/symbols.ts`는 같은 요청의 중복 IPC를 합친다.
- Windows의 기존 SVG 경로 사전을 HEAD와 대조해 전부 같은 것을 확인했다. macOS 전용 이름인 `clock.fill`, `chart.bar.xaxis`, 위아래 chevron에도 SVG 대체 경로가 있다.

### 2. macOS 로컬 사용 기록 복원

- 처음 분석 창을 열 때 플랫폼과 관계없이 `ui_locallogs.asked`를 확인하고 미응답이면 Consent를 표시한다. 기존 저장된 동의와 소스별 기본 꺼짐 정책을 유지한다.
- 기본 보기는 계속 `limits`다. macOS에서는 헤더 기간 선택 바로 아래 폭 180px의 공통 `Segmented`로 한도·토큰을 고른다. Windows의 기존 보기 전환은 유지했다.
- 설정 표시 탭의 `LocalLogsSection`을 모든 플랫폼에 표시한다. macOS 분석 창의 토큰 보기에도 같은 섹션을 표시해 소스별 수집·프로젝트 이름 숨기기·기록 지우기를 언제든 바꿀 수 있다.
- Consent와 LocalLogsSection은 macOS `Section` GroupBox 및 기존 스킨 버튼·Toggle 체크박스를 사용한다. Windows Consent의 기존 상자와 버튼 배치는 유지했다.
- Tokens 카드·빈 상태는 분석 창의 `--card-bg`, `--card-border`, `--card-r`를 사용한다. KPI는 같은 14px 여백·22px 숫자, 차트 카드는 16px 여백·13px 제목을 쓰며 칩은 분석 창과 같은 둥근 테두리·선택색·색 점을 쓴다. 추가 스타일은 모두 macOS 스킨에 한정했다.

### 3. 제목줄의 생성 방식 보정

- 로컬 의존성 `tauri-runtime-wry-2.12.3/src/lib.rs`의 `title_bar_style` 구현을 확인했다. `Visible`은 내부적으로 `with_fullsize_content_view(true)`를 강제하고 `Transparent`는 false를 설정한다. Swift `SettingsWindowController.swift`와 `AnalyticsWindowController.swift`는 둘 다 `.titled, .closable, .miniaturizable`만 쓰는 일반 창이다.
- 기존 코드는 이미 생성·표시된 창에서 FullSizeContentView를 제거했다. 이번에는 Tauri 빌더에서 `Transparent`로 생성 시점부터 전체 크기 콘텐츠를 끄고 창을 숨겨 둔다. 그런 다음 메인 스레드에서 툴바를 제거하고 Swift와 같은 정확한 style mask, 보이는 제목, 불투명 제목줄, `Expanded`를 설정한 뒤에 창을 표시한다. 콘텐츠 크기도 760×880 또는 1000×720으로 다시 지정하고 가운데 배치한다.
- Apple 문서상 [Expanded](https://developer.apple.com/documentation/appkit/nswindow/toolbarstyle-swift.enum/expanded?language=objc)는 툴바와 제목을 별도 줄에 배치한다. [FullSizeContentView](https://developer.apple.com/documentation/appkit/nswindow/stylemask-swift.struct/fullsizecontentview?language=objc)는 제목줄 아래로 콘텐츠를 확장하는 마스크다. 위 보정은 공개 AppKit API만 사용한다.
- 생성 옵션의 차이는 소스에서 확인했지만, 이것이 macOS 26에서 보인 왼쪽 정렬의 유일한 원인인지는 실기 재현을 못 했다. 이 환경에서 창 목록 도구가 SIGTRAP으로 종료돼 최종 제목 위치의 픽셀 검증은 남아 있다. 가운데 정렬을 실측 완료했다고 보고하지 않는다.

### 검증 결과

| 검사 | 결과 |
| --- | --- |
| `ui-src/npm run build` | 통과. PNG 없는 최종 UI 갱신 |
| `ui-src/npm run check` | 오류 0개, 경고 0개 |
| `cargo build -p um-app --release` | 통과. `rust/target/macos-runtime-symbols-release.log` |
| `rust/tools/check.sh` | 통과. 테스트 130개 통과, 기존 1개 무시. macOS 및 Windows x64·ARM64 clippy 통과. `rust/target/macos-runtime-symbols-check.log` |
| 시스템 심볼 실행 검사 | 현재 `symbols.rs`를 직접 사용하는 별도 Rust 실행 파일에서 심볼 21개 생성 성공, 14pt 요청의 28×28 PNG 헤더, 반복 응답, 굵기·색상 차이, 없는 이름·잘못된 크기·색상의 `None` 검증 통과. PNG 파일은 저장하지 않음. `rust/target/macos-runtime-symbols-probe.log` |
| 자산·회귀 검사 | 소스 및 번들 PNG 없음, 번들 SF 자산 경로 없음, 추출 도구 없음, Windows SVG 사전 동일 |
| `git diff --check`, 변경 소스·문서 U+2014 | 통과 |

`XWIN_CACHE_DIR`은 앞선 작업과 같은 저장소 내 `rust/target/cargo-xwin`을 사용했다. 브라우저 도구는 현재 승인 정책상 실행되지 않았고 `rust/tools/winid 0`은 종료 코드 -5로 실패했다. 따라서 실제 창 제목·Consent·토큰 UI의 라이트/다크 화면 비교와 클릭 검증은 남아 있다. 캐시 검사는 반복 응답이 같은 것을 확인한 것이며 별도 성능 계측은 하지 않았다.

### 이번에 바꾼 파일과 이유

| 파일 | 이유 |
| --- | --- |
| `um-app/src/symbols.rs`, `main.rs` | macOS 시스템 심볼 생성·메모리 캐시·명령 등록, 다른 OS의 None 반환 |
| `um-app/src/windows.rs` | 표시 전 기본 네이티브 제목줄과 콘텐츠 크기 복원 |
| `ui-src/src/lib/symbols.ts`, `analytics/Icon.svelte` | PNG 자산 참조 제거, 런타임 마스크·SVG 대체 |
| `analytics/Analytics.svelte`, `settings/tabs/Display.svelte` | macOS 보기 전환·첫 동의·설정 및 분석의 기록 제어 복원 |
| `tokens/Consent.svelte`, `tokens/Tokens.svelte` | macOS GroupBox와 분석 카드·칩·KPI 외형 |
| `ui-src/src/assets/sf-symbols/*.png`, `rust/tools/export-sf-symbols.swift`, `ui/assets/*.png` | SF Symbol 배포 자산과 추출 도구 삭제 |
| `um-app/ui/index.html`, `assets/*` | PNG 없는 프런트엔드 산출물 |
| `rust/docs/macos-settings-parity.md` | 현재 구현, 검증 결과와 실기 검증 한계 기록 |

저장소 밖 파일 수정이나 커밋·푸시·reset·checkout·브랜치 변경은 하지 않았다.
