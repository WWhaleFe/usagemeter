//! Windows 테두리 띠 창. 클릭 통과·최상위·작업표시줄에 안 보이는 레이어드 창에
//! `UpdateLayeredWindow`로 픽셀별 알파를 올린다(계획서 5-2, 6장).
//! 그린 뒤 DIB는 바로 지운다: DWM이 사본을 가진다(계획서 4-3 ②).
//!
//! 작업 표시줄도 "항상 위" 창이라, 작업 표시줄을 누르거나 자동 숨김에서 다시 나타나면 그쪽이 맨 앞으로 오고
//! 띠가 그 뒤로 숨는다. `keep_strips_on_top`이 작업 표시줄·앞 창이 바뀌는 순간을 받아 띠를 다시 맨 앞으로 올린다.

use std::sync::{Mutex, Once};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_TOPMOST, RegisterClassW, SW_HIDE, SW_SHOWNOACTIVATE,
    SWP_NOACTIVATE, SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::w;

use crate::{PlatformError, StripFrame};

const CLASS: windows::core::PCWSTR = w!("UsageMeterOverlayStrip");
static REGISTER: Once = Once::new();
/// 살아 있는 띠 창(HWND 값, 정보 패널인지). 작업 표시줄보다 앞으로 다시 올릴 대상.
static STRIPS: Mutex<Vec<(isize, bool)>> = Mutex::new(Vec::new());
static HOOK: Once = Once::new();

/// 보이는 띠 창을 모두 "항상 위" 무리의 맨 앞으로 다시 올린다(포커스·위치·크기는 그대로).
pub fn raise_strips() {
    use windows::Win32::UI::WindowsAndMessaging::{IsWindowVisible, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE};
    // 테두리 띠를 먼저, 정보 패널(호버·카드)을 나중에 올려 패널이 띠보다 앞에 남게 한다.
    let mut list = STRIPS.lock().map(|v| v.clone()).unwrap_or_default();
    list.sort_by_key(|&(_, panel)| panel);
    for (h, _) in list {
        let hwnd = HWND(h as *mut core::ffi::c_void);
        unsafe {
            if IsWindowVisible(hwnd).as_bool() {
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
            }
        }
    }
}

fn is_taskbar(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) } as usize;
    let name = String::from_utf16_lossy(&buf[..n.min(buf.len())]);
    name == "Shell_TrayWnd" || name == "Shell_SecondaryTrayWnd"
}

unsafe extern "system" fn on_win_event(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK, event: u32, hwnd: HWND, id_object: i32, _child: i32, _thread: u32, _time: u32,
) {
    use windows::Win32::UI::WindowsAndMessaging::{EVENT_SYSTEM_FOREGROUND, OBJID_WINDOW};
    if hwnd.is_invalid() || id_object != OBJID_WINDOW.0 { return; }
    // 앞 창이 바뀌면(작업 표시줄을 누른 경우 포함) 언제나, 그 밖의 표시·순서·위치 변화는 작업 표시줄일 때만.
    if event == EVENT_SYSTEM_FOREGROUND || is_taskbar(hwnd) {
        raise_strips();
    }
}

/// 작업 표시줄이 앞으로 오는 순간마다 띠를 다시 맨 앞으로 올리는 훅을 건다. 메시지 루프가 있는
/// 메인 스레드에서 한 번 부른다(여러 번 불러도 한 번만 건다).
pub fn keep_strips_on_top() {
    use windows::Win32::UI::Accessibility::SetWinEventHook;
    use windows::Win32::UI::WindowsAndMessaging::{
        EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW, EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
        WINEVENT_SKIPOWNPROCESS,
    };
    HOOK.call_once(|| unsafe {
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let _ = SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(on_win_event), 0, 0, flags);
        // SHOW(0x8002)부터 LOCATIONCHANGE(0x800B)까지(REORDER 포함): 자동 숨김 작업 표시줄이 나타나거나 앞으로 올라오는 경우.
        let _ = SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_LOCATIONCHANGE, None, Some(on_win_event), 0, 0, flags);
    });
}

unsafe extern "system" fn wndproc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(h, m, w, l) }
}

fn os(e: windows::core::Error) -> PlatformError {
    PlatformError::Os(e.to_string())
}

pub struct OverlayStrip {
    hwnd: HWND,
    frame: StripFrame,
}

impl OverlayStrip {
    /// 띠 창을 만든다(아직 보이지 않음). 프로세스는 Per-Monitor v2 DPI 인식이어야 한다(앱 매니페스트).
    pub fn new(frame: StripFrame) -> Result<Self, PlatformError> {
        unsafe {
            let inst = GetModuleHandleW(None).map_err(os)?;
            REGISTER.call_once(|| {
                let wc = WNDCLASSW { lpfnWndProc: Some(wndproc), hInstance: inst.into(), lpszClassName: CLASS, ..Default::default() };
                RegisterClassW(&wc);
            });
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS,
                w!("UsageMeter"),
                WS_POPUP,
                frame.x as i32,
                frame.y as i32,
                frame.w as i32,
                frame.h as i32,
                None,
                None,
                Some(inst.into()),
                None,
            )
            .map_err(os)?;
            if let Ok(mut v) = STRIPS.lock() { v.push((hwnd.0 as isize, false)); }
            Ok(Self { hwnd, frame })
        }
    }

    pub fn set_frame(&mut self, frame: StripFrame) {
        self.frame = frame;
        unsafe {
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), frame.x as i32, frame.y as i32, frame.w as i32, frame.h as i32, SWP_NOACTIVATE);
        }
    }

    /// 미리곱한 RGBA 픽셀을 올린다(Windows는 BGRA라 순서를 바꿔 복사). `scale`은 Windows에선 쓰지 않는다
    /// (프레임이 이미 물리 픽셀).
    pub fn present(&self, rgba_premul: &[u8], width_px: u32, height_px: u32, _scale: f64) -> Result<(), PlatformError> {
        let expected = width_px as usize * height_px as usize * 4;
        if rgba_premul.len() != expected {
            return Err(PlatformError::BadPixels { got: rgba_premul.len(), expected });
        }
        unsafe {
            let screen = GetDC(None);
            let mem = CreateCompatibleDC(Some(screen));
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width_px as i32,
                    biHeight: -(height_px as i32), // 위에서 아래로
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(Some(mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0).map_err(os)?;
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, expected);
            for (d, s) in dst.as_chunks_mut::<4>().0.iter_mut().zip(rgba_premul.as_chunks::<4>().0) {
                *d = [s[2], s[1], s[0], s[3]];
            }
            let old = SelectObject(mem, HGDIOBJ(dib.0));
            let pos = POINT { x: self.frame.x as i32, y: self.frame.y as i32 };
            let size = SIZE { cx: width_px as i32, cy: height_px as i32 };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let r = UpdateLayeredWindow(self.hwnd, Some(screen), Some(&pos), Some(&size), Some(mem), Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
            SelectObject(mem, old);
            let _ = DeleteObject(HGDIOBJ(dib.0));
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            r.map_err(os)
        }
    }

    /// 정보 패널로 쓰는 창이라고 표시한다(다시 올릴 때 테두리 띠보다 앞에 둔다).
    pub(crate) fn mark_panel(&self) {
        if let Ok(mut v) = STRIPS.lock() {
            for h in v.iter_mut() { if h.0 == self.hwnd.0 as isize { h.1 = true; } }
        }
    }

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0,
                windows::Win32::UI::WindowsAndMessaging::SWP_NOMOVE | windows::Win32::UI::WindowsAndMessaging::SWP_NOSIZE | SWP_NOACTIVATE);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for OverlayStrip {
    fn drop(&mut self) {
        if let Ok(mut v) = STRIPS.lock() { v.retain(|h| h.0 != self.hwnd.0 as isize); }
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
