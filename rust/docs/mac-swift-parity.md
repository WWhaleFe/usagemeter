# macOS Swift 1.5.0 동작 맞춤 결과

2026-11-17. 요청 번호 기준으로 실제 Swift·Rust 구현을 대조했다.

| 번호 | 결과 | 반영 내용 |
|---|---|---|
| 1 | 고침 | 쿠키 기반 사전 확인만 최대 2초 대기. Gemini는 agy를 호출하지 않으며 Claude·Codex는 쿠키가 있을 때만 직접 호출한다. 성공 결과도 화면에 반영한다. |
| 2 | 고침 | 로그인 폴링마다 새 토큰을 발급하고 이전 회차 응답을 무시한다. |
| 3 | 고침 | 조회·계정 변경 뒤 ipc::changed로 ui-state, 메뉴, 화면을 갱신한다. |
| 4 | 고침 | 직접 호출 클라이언트를 제거한다. 세대 번호로 로그아웃 전에 시작한 조회도 무효화한다. 시드 제거는 기존 구현을 유지했다. |
| 5 | 고침 | 전역 고유 팝업 라벨, 중첩 팝업 처리, 로그인 창 파괴 시 자식 팝업 즉시 정리를 추가했다. |
| 6 | 고침 | macOS NSApplication 활성화와 makeKeyAndOrderFront를 호출한다. |
| 7 | 고침 | 로그인 창이 열린 AI의 조회 및 진행 중 조회 결과 반영을 건너뛴다. |
| 8 | 고침 | 로그인 유지가 꺼져 있으면 시드를 표시하지 않고 시작 조회 전에 모든 AI의 웹 쿠키를 지운다. |
| 9 | 고침 | 시작 재시도·전체 갱신을 AI별 스레드로 실행하고 각 결과를 즉시 반영한다. |
| 10 | 고침 | 프리셋·기본 상태·기억 항목을 저장하고 복원한다. KnownItem은 Swift와 같은 id/ratio 객체 형식이며 이전 Rust 튜플 형식도 읽는다. |
| 11 | 고침 | 첫 실시간 결과가 실패여도 해당 AI의 시드를 제거한다. |
| 12 | 고침 | agy 활동 파일을 notify로 감시한다. 1.2초 선행, 2.5초 후행, 후행 뒤 6.5초 재확인과 최근 10분 활동 시 25초 보조 조회를 연결했다. |
| 13 | 고침 | 실행 중 agy 결과를 공유하고 프로세스 실행 동안 파서 잠금을 잡지 않는다. |
| 14 | 고침 | due_alerts와 UNUserNotificationCenter를 연결하고 리셋 주기별 알림 키를 저장한다. Swift의 notify.title/body 키를 사용한다. |
| 15 | 고침 | 같은 번호에서 프리릴리스를 정식보다 낮게 비교한다. 자동 확인을 켜면 즉시 확인하고 새 버전 알림 키를 저장한다. 메뉴·설정 문구에 v를 붙인다. |
| 16 | 고침 | 첫 화면·새 화면 기기 맞춤, 내장 화면 우선 선택, 저장한 폭을 이용한 해상도 적응을 연결했다. |
| 17 | 고침 | 내부 메뉴선 하단·Dock선 상단 곡률이 스트립 두께에서 빠질 수 있음을 확인하고 계산에 포함했다. |
| 18 | 고침 | 후원 메뉴에서 settings#support를 연다. |
| 19 | 고침 | 현재 설정 저장 메뉴에서 이름 입력 UI가 있는 settings#presets를 연다. |
| 20 | 고침 | 메뉴바 좌우·아래 여유 영역과 커서 위치 추적을 추가했다. NSMenu 추적 알림으로 메뉴가 열린 동안 호버를 억제한다. |
| 21 | 고침 | 차트 호버 x에 더하던 14pt 중복 패딩을 제거했다. |
| 22 | 고침 | 카드·차트 구분선, 스펙 순 차트 계열, menu.noAI, 해당 모니터 배율을 적용했다. AppKit 버튼의 실제 좌표를 이용한다. |
| 23 | 고침 | AI 메뉴를 Swift 스펙 순으로 맞추고 Gemini 실험적 접미어를 제거했다. ●/○ 표시는 이미 구현되어 유지했다. |

## 보고서에서 해당 없는 지적과 유지한 동작

- 메뉴바 % 누락은 해당 없음. tray_title.rs에 이미 구현되어 있으며 수정하지 않았다.
- 좌클릭·우클릭 통합은 해당 없음. 사용자 결정에 따라 정보 카드와 전체 메뉴 구분을 유지했다.
- 로그아웃 시 시드 제거 누락은 해당 없음. 기존 toggle_account에 이미 있었다.
- Swift가 presets.json·known_items.json에 저장한다는 전제는 해당 없음. Swift는 OverlaySettings.swift:1348-1357의 usagemeter.savedPresets, ProviderManager.swift:123-134의 knownSubItems UserDefaults JSON을 사용한다. Rust 저장 파일은 기존 읽기 경로와 호환되게 유지하고 객체·스냅샷 형식을 맞췄다.

## 요청 번호 밖의 보고서 잔여 사항

다음 차이는 실제 코드에 남아 있으며 이번 변경에는 포함하지 못했다.

- 표시 정보 체크 후 네이티브 메뉴가 닫히는 동작.
- 메뉴 SF Symbol 아이콘과 리포트의 2줄 커스텀 헤더.
- 로그인 페이지 로드 실패 안내 HTML과 재시도 버튼.
- 업데이트 알림 클릭 시 릴리스 페이지 열기.
- 비활성 링의 시스템 secondaryLabelColor 및 극소 잔여율 호 생략.

## 바꾼 파일과 이유

- um-app/src/main.rs, web.rs, direct.rs: 로그인 사전 확인·폴링·팝업·활성화·계정 캐시 무효화·조회 병렬화.
- um-app/src/state.rs: 프리셋·기억 항목·알림 키 저장, 시드 수명, 화면 기기 맞춤 및 폭 기억.
- um-app/src/agy_activity.rs, data.rs: 활동 감시와 agy 실행·파서 잠금 분리. 실행 합치기는 main.rs의 Agy에서 처리한다.
- um-app/src/notifications.rs: macOS 사용률·업데이트 알림.
- um-app/src/ipc.rs, menu.rs: 상태 이벤트, 즉시 업데이트 확인, 메뉴 이동 및 Swift 순서·문구.
- um-app/src/panels.rs: 메뉴 추적 억제, 호버 여유 영역, 카드 구분선·계열 순서·화면 배율.
- um-core/src/update.rs: 프리릴리스·빌드 메타데이터 버전 비교와 회귀 테스트.
- um-render/src/border/mod.rs, panel.rs: 스트립 곡률 범위와 차트 호버 좌표.
- um-app/ui-src/src/settings/TopArea.svelte, um-app/ui/: 새 버전 v 접두어와 프론트 빌드 결과.
- um-app/Cargo.toml, Cargo.lock: notify 및 macOS 알림·블록 의존성.

## 검증

- cargo test --workspace: 136개 통과, 기존 성능 테스트 1개 ignored.
- cargo clippy --workspace --all-targets -- -D warnings: 통과.
- npm run build: 통과, ui/ 갱신.
- Windows x64 및 ARM64: cargo xwin clippy --workspace --all-targets --offline --target <대상> -- -D warnings 통과. 저장소 내부 target/cargo-xwin 캐시를 사용했다.
- git diff --check: 통과.
- 추가 회귀 검증: 프리릴리스 순서, 프리셋·기억 항목·알림 키 재시작 복원, 기존 튜플 형식 읽기, 첫 조회 실패 시 시드 제거, 이전 회차 로그인 결과 무시, 닫힌 로그인 결과 거부, 내부 가로선 곡률 범위.
- 실제 계정 로그인·OAuth 서비스 흐름·macOS 알림 배너·모니터 물리 연결은 화면에서 검증하지 않았다. Windows 검증은 교차 정적 검사이며 Windows 실행 테스트는 아니다.
