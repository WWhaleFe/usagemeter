//! UsageMeter OS별 코드(계획서 3장 `um-platform`).
//!
//! 지금 있는 것: 테두리 띠 창(`OverlayStrip`), 디스플레이 목록(`display`). 그림은 `um-render`가 만든
//! 미리곱한(premultiplied) RGBA 픽셀을 받아 OS 창에 올리기만 한다.

pub mod display;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

#[cfg(target_os = "macos")]
pub use mac::overlay::OverlayStrip;
#[cfg(windows)]
pub use win::overlay::OverlayStrip;

/// 띠 창 위치·크기. macOS = 전역 AppKit 좌표(pt, 왼쪽 아래 원점),
/// Windows = 가상 화면 물리 픽셀(왼쪽 위 원점). 변환은 호출하는 디스플레이 층이 맡는다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripFrame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("메인 스레드가 아님")]
    NotMainThread,
    #[error("OS 호출 실패: {0}")]
    Os(String),
    #[error("픽셀 크기가 맞지 않음: {got} 바이트, 기대 {expected}")]
    BadPixels { got: usize, expected: usize },
}
