# Rust 이식 진행 현황

계획: [`../docs/RUST_PORT_PLAN.md`](../docs/RUST_PORT_PLAN.md) · 결정 기록: [`docs/adr/`](docs/adr/) · 브랜치: `rust-port`
CI: macOS, Windows x64, Windows ARM64에서 `cargo test --workspace` + `clippy -D warnings`

## 단계별 상태 (2026-11-16)

| 단계 | 상태 | 내용 |
|---|---|---|
| 0 사전 검증 | 진행 중 | S1(Tauri 메모리) 통과, S2 macOS 통과, S7 조사 완료, S8 통과. 남음: S2·S9·S10(실제 Windows 기기), S3·S5(로그인 세션을 쓰므로 앱 안에서 사용자 실행), S4, S6 |
| 1 핵심 로직(`um-core`) | 완료 | 설정(Swift 가져오기 포함), 세그먼트 그래프(Swift 전수 비교 5,040건), 문자열 349개, 이력 SQLite·분석(Swift 기준값 비교), 알림 판정, 버전 비교 |
| 2 수집(`um-providers`) | 순수 로직 완료 | 응답 해석(웹뷰 경로·직접 호출 경로 결과 일치), agy 해석·실행 파일 탐색, 상태 반영 규칙. 남음: 웹뷰·HTTP 전송 계층, 로그인 창, 스케줄러 |
| 3 테두리 | 그리기·창 완료 | `um-render` 기하·그리기(Swift 기준 이미지 55장 비교 통과), `um-platform` 띠 창(macOS NSWindow+CALayer, Windows 레이어드 창, Windows CI에서 실제 창 생성). 남음: 디스플레이 관리·장치 맞춤 연결 |
| 4 트레이·호버·패널 | 시작 전 | |
| 5 설정 창 | 시작 전 | Svelte, macOS·Windows(Fluent) 스킨 |
| 6 분석 + 로컬 기록 | 수집기 완료 | `um-locallogs`(Claude Code·Codex·Antigravity 증분 수집, 기본 꺼짐). 남음: agy 필드 의미 검증, 분석 화면 |
| 7 Windows 마무리 | 시작 전 | MSIX(Store)·NSIS, ARM64 |
| 8 macOS 마무리·전환 | 시작 전 | |

## 측정값

| 항목 | 값 | 기준 |
|---|---|---|
| Tauri 트레이 앱 평상시 | 13 MB | Swift 본체 약 102 MB |
| 창 열고 닫은 뒤 | 30~32 MB, 반복해도 늘지 않음 | ADR 0001 |
| 테두리 4면(2560×1600@2x) | 프로세스 21 MB | ADR 0002 |
| 글자 렌더(한·영·일) | 3.9 MB | ADR 0002 부록 |
| 이력 가져오기 11.8k 표본 | 66 ms(릴리스), RSS 12.7 MB | |
| 로컬 기록 50k줄 | 0.69 s(디버그), 줄 버퍼 최대 1.3 KB | |

## 다음 할 일

1. 앱 뼈대(`um-app`, Tauri): 트레이, 테두리 연결(디스플레이 관리 + `strip_extent`에 메뉴선·Dock선 높이 반영), 상태 저장
2. 수집 전송 계층: wry 임시 웹뷰(macOS `loadSimulatedRequest`, Windows `WebResourceRequested`), 직접 호출(reqwest), 로그인 창 → 이 안에서 S3·S5를 사용자 동의하에 측정
3. 호버·플라이아웃(S9), 설정 창 스킨
4. 실제 Windows 기기에서 S2·S9·S10·메모리 측정

## 알아 둘 점

- `rust/tools/border-fixtures`: 실제 Swift `BorderView`로 기준 이미지를 다시 뽑는 도구(`./build.sh run`).
- `rust/tools/swift-ref`: 세그먼트·분석 기준값 생성 Swift 스크립트.
- `rust/tools/gen_i18n.py`: `Localization.swift` → 문자열 JSON(줄표는 하이픈으로 바꿈).
- 이 Mac에서는 SQLite C 코드를 Windows용으로 교차 컴파일할 수 없어, Windows 확인은 CI가 한다(`um-platform`만 단독으로 `cargo check --target x86_64-pc-windows-msvc` 가능).
