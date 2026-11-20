//! UsageMeter OS별 코드(계획서 3장 `um-platform`).
//!
//! 지금 있는 것: 테두리 띠 창(`OverlayStrip`), 디스플레이 목록(`display`). 그림은 `um-render`가 만든
//! 미리곱한(premultiplied) RGBA 픽셀을 받아 OS 창에 올리기만 한다.

pub mod dialog;
pub mod display;
pub mod mouse;
pub mod toast;
pub mod webview_proxy;

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod win;

#[cfg(target_os = "macos")]
pub use mac::overlay::OverlayStrip;
#[cfg(target_os = "macos")]
pub use mac::panel::InfoPanel;
#[cfg(windows)]
pub use win::overlay::OverlayStrip;
#[cfg(windows)]
pub use win::panel::InfoPanel;

/// 시스템이 어두운 모드인지(앱 외관이 "시스템"일 때 패널 글자색을 정한다).
pub fn system_dark() -> bool {
    #[cfg(target_os = "macos")]
    {
        let Some(mtm) = objc2::MainThreadMarker::new() else { return false };
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        app.effectiveAppearance().name().to_string().contains("Dark")
    }
    #[cfg(windows)]
    {
        // HKCU\...\Personalize\AppsUseLightTheme = 0 이면 어두운 모드.
        use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
        use windows::core::w;
        let mut v: u32 = 1;
        let mut n = size_of::<u32>() as u32;
        let r = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("AppsUseLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut v as *mut u32 as *mut _),
                Some(&mut n),
            )
        };
        r.is_ok() && v == 0
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        false
    }
}

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

/// Mica는 Windows 11(빌드 22000)부터 지원한다. 버전 조회 실패 시 일반 창을 쓴다.
#[cfg(windows)]
pub fn mica_supported() -> bool {
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
    let mut version = OSVERSIONINFOW { dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32, ..Default::default() };
    unsafe { RtlGetVersion(&mut version) }.is_ok() && version.dwBuildNumber >= 22000
}
