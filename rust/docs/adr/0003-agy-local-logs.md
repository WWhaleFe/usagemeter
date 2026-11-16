# ADR 0003: Antigravity 로컬 기록에서 토큰 정보를 읽을 수 있다 (S7 결과)

- 날짜: 2026-11-16
- 상태: 조사 완료, 필드 의미 검증은 6단계 시작 때
- 관련: 계획서 8-1

## 확인한 것 (읽기 전용 사본, 대화 내용은 보지 않음)

- `~/.gemini/antigravity-cli/conversations/<id>.db`는 대화별 SQLite. 표: `steps`, `gen_metadata`, `executor_metadata`, `trajectory_meta`, `trajectory_metadata_blob`, `parent_references`, `battle_mode_infos`.
- 앱이 쓰는 중에는 원본을 바로 열 수 없었다(`unable to open database file`). **사본(+ `-wal`)을 만들어 읽는다.**
- `gen_metadata.data`는 protobuf. 응답 1건마다 1행. 필드 번호 구조(스키마 없이 해독):
  - `1.19` = 모델 이름 (예: `gemini-3.8-flash-n`)
  - `1.4` (= `1.17.2`와 같은 값) = 사용량으로 보이는 숫자 묶음: `.1`, `.2`, `.3`, `.5`, `.6`, `.9`, `.10`
    - 예: `.1=1318, .2=9650, .3=1280, .6=24, .9=1116, .10=164` / `.1=1318, .2=3626, .3=128, .5=7617, ...`
  - `1.20` = 키·값 목록(`request_id`, `model_enum`, `used_claude`, `used_non_gemini_model`, `cascade_id` …)
  - `1.11`, `1.12` = (초, 나노초) 형태의 시간 값으로 보임
- `conversation_summaries.db`(대화 목록: `step_count`, `last_modified_time`, `workspace_uris`)는 별도.

## 판단

- 토큰 정보가 있으므로 8장의 Antigravity 소스를 "대화 수만"이 아니라 **모델별 토큰**까지 지원하는 것으로 진행한다.
- `1.4.*` 각 숫자가 입력·출력·캐시·추론 중 무엇인지는 아직 추정이다. 6단계 시작 때 agy `/usage` 잔여율 하락과 맞춰 보고, 확신이 없으면 "총량"만 보여준다.
- 비공개 내부 형식이라 언제든 바뀔 수 있다. 해석 실패 시 조용히 건너뛰고 소스 상태에 "형식 변경" 표시.
