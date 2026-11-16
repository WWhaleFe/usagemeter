//! 디스플레이 정보와 좌표 변환(계획서 5-2 "디스플레이 관리").
//! 테두리 계산은 화면마다 "왼쪽 위 원점, pt" 좌표로 하고, 띠 창을 놓을 때만 OS 좌표로 바꾼다.

use crate::StripFrame;

/// 노치(macOS 내장 화면). 폭·높이 pt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Notch {
    pub width: f64,
    pub height: f64,
}

/// 화면 하나.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayInfo {
    /// 안정적인 id. macOS = `CGDisplayCreateUUIDFromDisplayID` 문자열(Swift 버전과 같아 화면별 설정이 이어진다),
    /// Windows = 모니터 장치 이름(TODO: DisplayConfig 경로로 바꿔 재연결에도 유지).
    pub id: String,
    pub name: String,
    /// 화면 크기(pt).
    pub width_pt: f64,
    pub height_pt: f64,
    /// pt → px 배율(macOS backingScaleFactor, Windows DPI/96).
    pub scale: f64,
    pub is_builtin: bool,
    pub is_primary: bool,
    pub notch: Option<Notch>,
    /// OS 좌표의 화면 원점. macOS = 전역 AppKit pt(왼쪽 아래), Windows = 가상 화면 물리 px(왼쪽 위).
    origin: (f64, f64),
}

impl DisplayInfo {
    /// 화면 안 띠(왼쪽 위 원점, pt)를 띠 창 프레임(OS 좌표)으로 바꾼다.
    pub fn strip_frame(&self, x: f64, y: f64, w: f64, h: f64) -> StripFrame {
        #[cfg(target_os = "macos")]
        {
            StripFrame { x: self.origin.0 + x, y: self.origin.1 + self.height_pt - y - h, w, h }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let s = self.scale;
            StripFrame { x: (self.origin.0 + x * s).round(), y: (self.origin.1 + y * s).round(), w: (w * s).round(), h: (h * s).round() }
        }
    }

    /// OS 전역 좌표(마우스 위치)가 이 화면 안이면 화면 안 좌표(왼쪽 위 원점, pt)로 바꾼다.
    pub fn to_local(&self, gx: f64, gy: f64) -> Option<(f64, f64)> {
        #[cfg(target_os = "macos")]
        let (x, y) = (gx - self.origin.0, self.origin.1 + self.height_pt - gy);
        #[cfg(not(target_os = "macos"))]
        let (x, y) = ((gx - self.origin.0) / self.scale, (gy - self.origin.1) / self.scale);
        (x >= 0.0 && y >= 0.0 && x < self.width_pt && y < self.height_pt).then_some((x, y))
    }

    /// 화면 안 좌표(왼쪽 위 원점, pt)의 점을 OS 전역 좌표로.
    pub fn to_global(&self, x: f64, y: f64) -> (f64, f64) {
        let f = self.strip_frame(x, y, 0.0, 0.0);
        (f.x, f.y)
    }

    /// 띠의 픽셀 크기.
    pub fn pixel_size(&self, w: f64, h: f64) -> (u32, u32) {
        ((w * self.scale).round().max(1.0) as u32, (h * self.scale).round().max(1.0) as u32)
    }

    #[cfg(test)]
    pub(crate) fn for_test(origin: (f64, f64), w: f64, h: f64, scale: f64) -> Self {
        Self { id: "t".into(), name: "t".into(), width_pt: w, height_pt: h, scale, is_builtin: false, is_primary: true, notch: None, origin }
    }
}

/// 연결된 화면 목록(첫 항목 = 주 화면). macOS는 메인 스레드에서만.
pub fn displays() -> Vec<DisplayInfo> {
    imp::displays()
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{DisplayInfo, Notch};
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;
    use objc2_core_foundation::{CFRetained, CFString, CFUUID};
    use objc2_foundation::{NSNumber, NSString};
    use std::ffi::c_void;
    use std::ptr::NonNull;

    #[link(name = "ColorSync", kind = "framework")]
    unsafe extern "C" {
        fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *mut c_void;
    }

    fn display_uuid(id: u32) -> Option<String> {
        let raw = unsafe { CGDisplayCreateUUIDFromDisplayID(id) };
        let uuid: CFRetained<CFUUID> = unsafe { CFRetained::from_raw(NonNull::new(raw.cast())?) };
        let s: CFRetained<CFString> = CFUUID::new_string(None, Some(&uuid))?;
        Some(s.to_string())
    }

    pub fn displays() -> Vec<DisplayInfo> {
        let Some(mtm) = MainThreadMarker::new() else { return vec![] };
        let screens = NSScreen::screens(mtm);
        let mut out = Vec::new();
        for (i, sc) in screens.iter().enumerate() {
            let f = sc.frame();
            let num = sc
                .deviceDescription()
                .objectForKey(&NSString::from_str("NSScreenNumber"))
                .and_then(|o| o.downcast::<NSNumber>().ok())
                .map(|n| n.unsignedIntValue());
            let id = num.and_then(display_uuid).or_else(|| num.map(|n| format!("display-{n}"))).unwrap_or_else(|| sc.localizedName().to_string());
            let top = sc.safeAreaInsets().top;
            // Swift OverlaySettings.swift:1104-1114: 높이 = safeAreaInsets.top, 폭 = 화면폭 - 양옆 보조 영역.
            let notch = (top > 0.0).then(|| Notch {
                width: f.size.width - sc.auxiliaryTopLeftArea().size.width - sc.auxiliaryTopRightArea().size.width,
                height: top,
            });
            out.push(DisplayInfo {
                id,
                name: sc.localizedName().to_string(),
                width_pt: f.size.width,
                height_pt: f.size.height,
                scale: sc.backingScaleFactor(),
                is_builtin: num.is_some_and(|n| objc2_core_graphics::CGDisplayIsBuiltin(n)),
                is_primary: i == 0,
                notch,
                origin: (f.origin.x, f.origin.y),
            });
        }
        out
    }
}

#[cfg(windows)]
mod imp {
    use super::DisplayInfo;
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::core::BOOL;

    const MONITORINFOF_PRIMARY: u32 = 1;

    unsafe extern "system" fn collect(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let v = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
        v.push(m);
        true.into()
    }

    pub fn displays() -> Vec<DisplayInfo> {
        let mut mons: Vec<HMONITOR> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut mons as *mut _ as isize));
        }
        let mut out = Vec::new();
        for m in mons {
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            if !unsafe { GetMonitorInfoW(m, &mut mi.monitorInfo) }.as_bool() {
                continue;
            }
            let (mut dx, mut dy) = (96u32, 96u32);
            let _ = unsafe { GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
            let scale = dx as f64 / 96.0;
            let r = mi.monitorInfo.rcMonitor;
            let name = String::from_utf16_lossy(&mi.szDevice).trim_end_matches('\0').to_string();
            out.push(DisplayInfo {
                id: name.clone(),
                name,
                width_pt: (r.right - r.left) as f64 / scale,
                height_pt: (r.bottom - r.top) as f64 / scale,
                scale,
                is_builtin: false,
                is_primary: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                notch: None,
                origin: (r.left as f64, r.top as f64),
            });
        }
        // 주 화면을 맨 앞으로(macOS NSScreen.screens 순서와 맞춤).
        out.sort_by_key(|d| !d.is_primary);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 띠_좌표_변환() {
        let d = DisplayInfo::for_test((100.0, 50.0), 1000.0, 800.0, 2.0);
        let f = d.strip_frame(0.0, 0.0, 30.0, 800.0);
        #[cfg(target_os = "macos")]
        assert_eq!(f, StripFrame { x: 100.0, y: 50.0, w: 30.0, h: 800.0 });
        #[cfg(not(target_os = "macos"))]
        assert_eq!(f, StripFrame { x: 100.0, y: 50.0, w: 60.0, h: 1600.0 });
        #[cfg(target_os = "macos")]
        assert_eq!(d.strip_frame(30.0, 0.0, 940.0, 40.0).y, 50.0 + 800.0 - 40.0);
        assert_eq!(d.pixel_size(30.0, 800.0), (60, 1600));
    }
}
