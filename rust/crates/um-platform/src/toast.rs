//! Windows 알림(토스트). macOS 알림은 앱 쪽(UserNotifications)에 있다.

/// 토스트를 띄운다. 설치판은 시작 메뉴 바로 가기의 AppUserModelID(`app_id`, 번들 ID)로,
/// Store(MSIX)판은 패키지 ID로 보낸다. 바로 가기가 없는 포터블판은 Windows가 표시하지 않을 수 있다.
#[cfg(windows)]
pub fn show(app_id: &str, title: &str, body: &str) -> Result<(), String> {
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager, ToastTemplateType};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use windows::core::HSTRING;
    // 조회 스레드에서 불리므로 WinRT를 쓰기 전에 COM을 연다(이미 열려 있으면 그대로 쓴다).
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let run = || -> windows::core::Result<()> {
        let xml = ToastNotificationManager::GetTemplateContent(ToastTemplateType::ToastText02)?;
        let texts = xml.GetElementsByTagName(&HSTRING::from("text"))?;
        texts.Item(0)?.AppendChild(&xml.CreateTextNode(&HSTRING::from(title))?)?;
        texts.Item(1)?.AppendChild(&xml.CreateTextNode(&HSTRING::from(body))?)?;
        let toast = ToastNotification::CreateToastNotification(&xml)?;
        let notifier = if packaged() {
            ToastNotificationManager::CreateToastNotifier()?
        } else {
            ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?
        };
        notifier.Show(&toast)
    };
    run().map_err(|e| e.to_string())
}

/// MSIX 패키지로 실행 중인지.
#[cfg(windows)]
fn packaged() -> bool {
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    let mut len = 0u32;
    // 패키지가 없으면 APPMODEL_ERROR_NO_PACKAGE, 있으면 버퍼 부족 오류가 난다.
    unsafe { GetCurrentPackageFullName(&mut len, None) }.0 != 15700
}
