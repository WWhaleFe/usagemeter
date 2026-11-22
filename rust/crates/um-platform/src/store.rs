//! Microsoft Store 업데이트(Windows.Services.Store). Store에서 설치한 패키지에서만 동작하고,
//! 그 밖(개발 실행·시험 설치·macOS)에서는 `Err`를 돌려준다. 모두 막히는 호출이라 별도 스레드에서 부른다.

/// 설치 결과.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Install {
    /// 설치했다(보통 그 전에 Store가 앱을 닫는다).
    Completed,
    /// 받을 업데이트가 없다.
    NoUpdate,
    /// 사용자가 Store 확인 창에서 취소했다.
    Canceled,
    /// 조용히 설치할 수 없다(Store 자동 업데이트 꺼짐·데이터 통신 제한). 확인 창을 허락하지 않았을 때만 나온다.
    NeedsPrompt,
    Failed,
}

/// Store에 이 앱의 새 패키지가 있는지. Store는 30분에 1번·하루 10번까지만 새로 확인하고 그 밖에는 마지막 결과를 준다.
pub fn has_update(hwnd: isize) -> Result<bool, String> {
    #[cfg(windows)]
    {
        imp::has_update(hwnd).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        Err("Store 업데이트는 Windows에서만 쓴다".into())
    }
}

/// 새 패키지를 받아 설치한다. 먼저 조용히 설치하고, 안 되면 `prompt`일 때만 Store 확인 창을 띄운다.
pub fn install(hwnd: isize, prompt: bool) -> Result<Install, String> {
    #[cfg(windows)]
    {
        imp::install(hwnd, prompt).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, prompt);
        Err("Store 업데이트는 Windows에서만 쓴다".into())
    }
}

/// Store가 업데이트하려고 앱을 닫은 뒤 다시 열리도록 등록한다(충돌·멈춤·재부팅 때는 다시 열지 않음).
pub fn restart_after_update() {
    #[cfg(windows)]
    imp::restart_after_update();
}

#[cfg(windows)]
mod imp {
    use super::Install;
    use windows::Services::Store::{StoreContext, StorePackageUpdateState};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
    use windows::Win32::System::Recovery::{REGISTER_APPLICATION_RESTART_FLAGS, RegisterApplicationRestart};
    use windows::Win32::UI::Shell::IInitializeWithWindow;
    use windows::core::{Interface, PCWSTR, Result};

    /// 데스크톱 앱은 Store 창의 주인 창을 알려 줘야 한다(없으면 ERROR_INVALID_WINDOW_HANDLE).
    fn context(hwnd: isize) -> Result<StoreContext> {
        // 이 스레드에서 WinRT를 쓸 수 있게 한다. 이미 초기화돼 있으면 실패해도 그대로 쓴다.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let ctx = StoreContext::GetDefault()?;
        if hwnd != 0 {
            let init: IInitializeWithWindow = ctx.cast()?;
            unsafe { init.Initialize(HWND(hwnd as *mut core::ffi::c_void))? };
        }
        Ok(ctx)
    }

    pub fn has_update(hwnd: isize) -> Result<bool> {
        let ctx = context(hwnd)?;
        let updates = ctx.GetAppAndOptionalStorePackageUpdatesAsync()?.join()?;
        Ok(updates.Size()? > 0)
    }

    pub fn install(hwnd: isize, prompt: bool) -> Result<Install> {
        let ctx = context(hwnd)?;
        let updates = ctx.GetAppAndOptionalStorePackageUpdatesAsync()?.join()?;
        if updates.Size()? == 0 {
            return Ok(Install::NoUpdate);
        }
        if ctx.CanSilentlyDownloadStorePackageUpdates()? {
            let r = ctx.TrySilentDownloadAndInstallStorePackageUpdatesAsync(&updates)?.join()?;
            if r.OverallState()? == StorePackageUpdateState::Completed {
                return Ok(Install::Completed);
            }
        }
        if !prompt {
            return Ok(Install::NeedsPrompt);
        }
        let r = ctx.RequestDownloadAndInstallStorePackageUpdatesAsync(&updates)?.join()?;
        Ok(match r.OverallState()? {
            StorePackageUpdateState::Completed => Install::Completed,
            StorePackageUpdateState::Canceled => Install::Canceled,
            _ => Install::Failed,
        })
    }

    pub fn restart_after_update() {
        // RESTART_NO_CRASH(1) | RESTART_NO_HANG(2) | RESTART_NO_REBOOT(8): 업데이트로 닫힐 때만 다시 연다.
        unsafe {
            let _ = RegisterApplicationRestart(PCWSTR::null(), REGISTER_APPLICATION_RESTART_FLAGS(1 | 2 | 8));
        }
    }
}
