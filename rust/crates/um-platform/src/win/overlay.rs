//! Windows 테두리 띠 창. 클릭 통과·최상위·작업표시줄에 안 보이는 레이어드 창에
//! `UpdateLayeredWindow`로 픽셀별 알파를 올린다(계획서 5-2, 6장).
//! 그린 뒤 DIB는 바로 지운다: DWM이 사본을 가진다(계획서 4-3 ②).

use std::sync::Once;

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

    pub fn show(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
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
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
