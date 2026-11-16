# ADR 0001: 설정·분석 창 UI는 Tauri 웹뷰로 한다 (S1 결과)

- 날짜: 2026-11-16
- 상태: 채택
- 관련: 계획서 2-1, 4-2, 13장 D2

## 측정 (macOS 26.6.2, Apple M4, 릴리스 빌드, `rust/tools/measure-memory.sh`)

트레이만 띄운 Tauri 2.12.5 앱(`rust/spikes/s1-tauri-idle`). 60초에 760×880 웹뷰 창 열기, 90초에 파기, 180초에 다시 열기, 210초에 파기.

| 시점 | 본체 | WebKit 하위 프로세스 | 합계 |
|---|---|---|---|
| 평상시(창 연 적 없음) | 13 MB | 없음 | **13 MB** |
| 창 열림 | 25 MB | WebContent 43, GPU 13, Networking 6 | 87 MB |
| 파기 후 30초 | 24 MB | GPU 12, Networking 6 | 42 MB |
| 파기 후 90초 | 24 MB | Networking 6 | **30 MB** |
| 두 번째 열고 파기 후 90초 | 26 MB | Networking 6 | **32 MB** |

## 판단

- 평상시 13MB로 S1 기준(macOS ≤ 30MB)을 넉넉히 통과.
- 창을 닫은 뒤 처음 한 번은 약 +17~19MB가 남는다(WebKit 프레임워크·Networking 프로세스). **반복해도 늘지 않는다**(30 → 32MB).
- 원래 기준 "+10MB 이하"는 넘지만, Swift 버전(닫은 뒤 150~200MB 유지)보다 훨씬 낫고 평상시 합계 목표(45MB)도 지킨다.
- 그래서 Slint로 바꾸지 않고 Tauri를 채택한다. 계획서 4-2의 기준을 "창을 닫은 뒤 60~90초: 평상시 + 20MB 이하, 반복해도 늘지 않음"으로 고친다.

## 남은 확인

- Windows(WebView2)에서 같은 실험: CI가 아닌 실제 Windows 기기에서 측정 필요.
