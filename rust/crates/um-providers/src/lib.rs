//! 사용량 수집의 순수 부분(네트워크·웹뷰 없음). 계획서 5-1.
//!
//! - `spec`   Providers.swift (ProviderSpec, usageJS)
//! - `parse`  WebSession.swift의 결과 -> 스냅샷 변환
//! - `direct` 웹뷰 없는 직접 호출 경로의 요청 기술과 응답 해석(JS와 같은 `raw`를 만든다)
//! - `agy`    AgyIntegration.swift (실행 파일 탐색, 명령줄, 출력 파서)
//! - `state`  ProviderManager.swift (상태 반영 규칙, 세부 항목 기억, 시작 재시도)

pub mod agy;
pub mod direct;
pub mod parse;
pub mod spec;
pub mod state;
