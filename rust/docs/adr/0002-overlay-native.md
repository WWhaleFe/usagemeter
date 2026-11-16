# ADR 0002: 테두리는 네이티브 띠 창 + tiny-skia로 그린다 (S2 결과, macOS)

- 날짜: 2026-11-16
- 상태: macOS 채택, Windows는 실제 기기 확인 대기
- 관련: 계획서 4-3 ②, 5-2

## 측정 (macOS 26.6.2, Apple M4, 주 화면 2560×1600pt @2x, 릴리스 빌드)

`rust/spikes/s2-overlay-mac`: 띠 창 4개(좌·우·상·하). 띠 폭 = inset + 두께 + 곡률 + 4pt = 30.5pt(두께 3, 곡률 22).
화면 전체 경로(kurbo)를 띠 원점만큼 옮겨 tiny-skia로 그리고, `CFData` 사본으로 CGImage를 만들어 `CALayer.contents`에 올린 뒤 픽셀 버퍼는 버린다.

| 항목 | 결과 |
|---|---|
| 프로세스 전체 footprint | **21 MB** (Swift 버전 본체 약 102 MB) |
| 경로 자르기(길이 기준 62%) | 정상: 왼쪽 → 위 → 오른쪽 약 1000pt 지점에서 끝남(계산값 약 1000pt) |
| 클릭 통과·statusBar 레벨·모든 Space | 설정 적용, 화면에서 표시 확인 |

## 판단

- 계획대로 네이티브 띠 창 + tiny-skia 방식을 채택한다.
- 띠 폭을 그리는 만큼만 잡는 것(150pt → 30pt)이 메모리에 가장 크게 기여한다.

## 남은 확인

- Windows: `UpdateLayeredWindow` 레이어드 창, 가상 데스크톱, 혼합 DPI(실제 Windows 기기 필요).
- 여러 화면, Space 전환, 전체화면 앱 위 표시 수동 점검(3단계).

## 부록: S8 글자 렌더(cosmic-text)

`rust/spikes/s8-text`: 12pt @2x로 한·영·일 혼합 3줄을 그려 PNG 저장. 필요한 글꼴 3개(23개 얼굴)만 읽으면 footprint 3.9 MB, 시스템 글꼴 전체(914개 얼굴)를 읽어도 4.6 MB(글꼴 파일은 메모리 매핑). 글자 품질은 패널 용도로 충분. **채택.** 기본은 필요한 글꼴만 읽는다.
