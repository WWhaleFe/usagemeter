//! 간단한 확인 대화상자(Windows). 트레이 메뉴처럼 앱 창이 없는 곳에서 되돌리기 어려운 동작 전에 묻는다.

/// 예/아니요를 묻는다. 예를 고르면 true. 부르는 스레드를 막으므로 메인 스레드에서 부르지 않는다.
#[cfg(windows)]
pub fn confirm(title: &str, text: &str) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{IDYES, MB_ICONQUESTION, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MessageBoxW};
    use windows::core::HSTRING;
    let r = unsafe { MessageBoxW(None, &HSTRING::from(text), &HSTRING::from(title), MB_YESNO | MB_ICONQUESTION | MB_TOPMOST | MB_SETFOREGROUND) };
    r == IDYES
}
