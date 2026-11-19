//! Windows WebView2 프록시 설정.
//!
//! Windows 기본값인 "설정 자동 감지(WPAD)"만 켜져 있으면 WebView2가 새로 뜰 때마다 프록시를 찾느라
//! 20초 넘게 페이지 로드를 시작하지 않았다(실기: 조회 창·로그인 창이 하얀 빈 창으로 25초 제한을 넘김,
//! `--no-proxy-server`로는 3초). 수동 프록시나 PAC 주소가 설정돼 있으면 시스템 설정을 그대로 따른다.

/// wry가 인자를 따로 주지 않을 때 쓰는 기본값. 환경 변수로 인자를 덮어쓰면 이것도 함께 넣어야 한다.
pub const WRY_DEFAULT_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection";

/// 인터넷 설정(HKCU) 값으로 WebView2에 줄 추가 인자를 정한다. None = 시스템 설정 그대로.
pub fn browser_args(proxy_enabled: bool, proxy_server: &str, pac_url: &str) -> Option<String> {
    if (proxy_enabled && !proxy_server.trim().is_empty()) || !pac_url.trim().is_empty() {
        return None;
    }
    Some(format!("{WRY_DEFAULT_ARGS} --no-proxy-server"))
}

/// 현재 사용자의 인터넷 설정을 읽어 `browser_args`를 돌려준다.
#[cfg(windows)]
pub fn current_browser_args() -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::{HSTRING, w};
    let key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings");
    let dword = |name: &str| -> u32 {
        let mut v: u32 = 0;
        let mut n = size_of::<u32>() as u32;
        let name = HSTRING::from(name);
        let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, key, &name, RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut _), Some(&mut n)) };
        if r.is_ok() { v } else { 0 }
    };
    let string = |name: &str| -> String {
        let mut buf = [0u16; 1024];
        let mut n = (buf.len() * 2) as u32;
        let name = HSTRING::from(name);
        let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, key, &name, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut n)) };
        if !r.is_ok() { return String::new(); }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    };
    browser_args(dword("ProxyEnable") == 1, &string("ProxyServer"), &string("AutoConfigURL"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 자동_감지만_켜져_있으면_프록시_탐색을_건너뛴다() {
        let args = browser_args(false, "", "").unwrap();
        assert!(args.starts_with(WRY_DEFAULT_ARGS) && args.ends_with("--no-proxy-server"));
        // 프록시 주소가 남아 있어도 꺼져 있으면 쓰지 않는 것이다.
        assert!(browser_args(false, "10.0.0.1:8080", "").is_some());
    }

    #[test]
    fn 수동_프록시나_pac가_있으면_시스템_설정을_따른다() {
        assert_eq!(browser_args(true, "10.0.0.1:8080", ""), None);
        assert_eq!(browser_args(false, "", "http://wpad/proxy.pac"), None);
        // 켜져 있어도 주소가 비어 있으면 실제 프록시가 없다.
        assert!(browser_args(true, " ", "").is_some());
    }
}
