//! 디스플레이 정보와 좌표 변환(계획서 5-2 "디스플레이 관리").
//! 테두리 계산은 화면마다 "왼쪽 위 원점, pt" 좌표로 하고, 띠 창을 놓을 때만 OS 좌표로 바꾼다.

use crate::StripFrame;

/// 노치(macOS 내장 화면). 폭·높이 pt.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Notch {
    pub width: f64,
    pub height: f64,
}

/// Dock·작업 표시줄이 차지한 화면 변과 논리 두께.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarSide { Bottom, Top, Left, Right }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taskbar {
    pub side: TaskbarSide,
    pub thickness: f64,
}

impl Taskbar {
    /// AppKit 좌표의 프레임 차이. 위쪽 차이는 메뉴바이므로 후보에서 제외한다.
    pub fn from_visible_frame(frame: [f64; 4], visible: [f64; 4], fallback: TaskbarSide) -> Self {
        let [x, y, w, _] = frame;
        let [vx, vy, vw, _] = visible;
        [(TaskbarSide::Bottom, vy - y), (TaskbarSide::Left, vx - x),
            (TaskbarSide::Right, x + w - vx - vw)]
            .into_iter().filter(|(_, n)| *n > 1.0)
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map_or(Self { side: fallback, thickness: 0.0 }, |(side, thickness)| Self { side, thickness })
    }

    /// 자동 숨김 시에도 셸 창의 방향을 유지하고 화면 안에 보이는 두께만 사용한다.
    pub fn from_window_rect(monitor: [i32; 4], window: [i32; 4], scale: f64) -> Self {
        let [l, t, r, b] = monitor;
        let [wl, wt, wr, wb] = window;
        let side = if wr - wl > wb - wt {
            if (wt + wb - 2 * t).abs() < (2 * b - wt - wb).abs() { TaskbarSide::Top } else { TaskbarSide::Bottom }
        } else if (wl + wr - 2 * l).abs() < (2 * r - wl - wr).abs() { TaskbarSide::Left } else { TaskbarSide::Right };
        let pixels = match side {
            TaskbarSide::Top | TaskbarSide::Bottom => (wb.min(b) - wt.max(t)).max(0),
            TaskbarSide::Left | TaskbarSide::Right => (wr.min(r) - wl.max(l)).max(0),
        };
        Self { side, thickness: f64::from(pixels) / scale }
    }

    /// 모니터별 작업 영역 차이 중 가장 두꺼운 예약 영역을 쓴다.
    /// 자동 숨김으로 예약 영역이 사라지면 None으로 바뀐다.
    pub fn from_work_area(monitor: [i32; 4], work: [i32; 4], scale: f64) -> Option<Self> {
        let [l, t, r, b] = monitor;
        let [wl, wt, wr, wb] = work;
        [(TaskbarSide::Bottom, b - wb), (TaskbarSide::Top, wt - t),
         (TaskbarSide::Left, wl - l), (TaskbarSide::Right, r - wr)]
            .into_iter().filter(|(_, n)| *n > 0).max_by_key(|(_, n)| *n)
            .map(|(side, n)| Self { side, thickness: f64::from(n) / scale })
    }
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
    pub taskbar: Option<Taskbar>,
    /// OS 좌표의 화면 원점. macOS = 전역 AppKit pt(왼쪽 아래), Windows = 가상 화면 물리 px(왼쪽 위).
    origin: (f64, f64),
}

impl DisplayInfo {
    /// 화면 배치·작업 영역의 변경을 감시하는 서명.
    pub fn configuration_signature(&self) -> String {
        format!("{}:{:?}:{}x{}@{}:{:?}", self.id, self.origin, self.width_pt, self.height_pt, self.scale, self.taskbar)
    }

    /// 화면 안 띠(왼쪽 위 원점, pt)를 띠 창 프레임(OS 좌표)으로 바꾼다.
    pub fn strip_frame(&self, x: f64, y: f64, w: f64, h: f64) -> StripFrame {
        #[cfg(target_os = "macos")]
        {
            StripFrame { x: self.origin.0 + x, y: self.origin.1 + self.height_pt - y - h, w, h }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let s = self.scale;
            StripFrame { x: (self.origin.0 + x * s).round(), y: (self.origin.1 + y * s).round(), w: (w * s).ceil().max(1.0), h: (h * s).ceil().max(1.0) }
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
        ((w * self.scale).ceil().max(1.0) as u32, (h * self.scale).ceil().max(1.0) as u32)
    }

    #[cfg(test)]
    pub(crate) fn for_test(origin: (f64, f64), w: f64, h: f64, scale: f64) -> Self {
        Self { id: "t".into(), name: "t".into(), width_pt: w, height_pt: h, scale, is_builtin: false, is_primary: true, notch: None, taskbar: None, origin }
    }
}

/// 화면 ID를 파일 이름으로 쓸 때 장치 경로와 금지 문자를 제거한다. macOS UUID는 그대로다.
pub fn file_id(id: &str) -> String {
    let id = id.strip_prefix(r"\\.\").unwrap_or(id);
    let name: String = id.chars().map(|c| {
        if c.is_control() || r#"\/:*?"<>|"#.contains(c) { '_' } else { c }
    }).collect();
    let name = name.trim_end_matches(['.', ' ']);
    if name.is_empty() { "display".into() } else { name.to_string() }
}

/// 연결된 화면 목록(첫 항목 = 주 화면). macOS는 메인 스레드에서만.
pub fn displays() -> Vec<DisplayInfo> {
    imp::displays()
}

#[cfg(target_os = "macos")]
mod imp {
    use super::{DisplayInfo, Notch, Taskbar, TaskbarSide};
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
        // 매번 읽어 자동 숨김·다른 화면의 Dock도 방향 변경을 따라간다. 시스템 설정은 쓰지 않는다.
        let orientation = std::process::Command::new("/usr/bin/defaults")
            .args(["read", "com.apple.dock", "orientation"]).output().ok();
        let fallback = match orientation.as_ref().map(|o| String::from_utf8_lossy(&o.stdout)).as_deref().map(str::trim) {
            Some("left") => TaskbarSide::Left,
            Some("right") => TaskbarSide::Right,
            _ => TaskbarSide::Bottom,
        };
        let mut out = Vec::new();
        for (i, sc) in screens.iter().enumerate() {
            let f = sc.frame();
            let v = sc.visibleFrame();
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
                taskbar: Some(Taskbar::from_visible_frame(
                    [f.origin.x, f.origin.y, f.size.width, f.size.height],
                    [v.origin.x, v.origin.y, v.size.width, v.size.height], fallback)),
                origin: (f.origin.x, f.origin.y),
            });
        }
        out
    }
}

#[cfg(windows)]
mod imp {
    use super::{DisplayInfo, Taskbar, file_id};
    use windows::Win32::Foundation::{LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, GetWindowRect};
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::core::BOOL;

    const MONITORINFOF_PRIMARY: u32 = 1;

    unsafe extern "system" fn collect(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let v = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
        v.push(m);
        true.into()
    }

    fn hidden_taskbar(monitor: HMONITOR, r: RECT, scale: f64) -> Option<Taskbar> {
        // 주 화면과 보조 화면의 셸 작업 표시줄을 각각 찾는다.
        for class in [w!("Shell_TrayWnd"), w!("Shell_SecondaryTrayWnd")] {
            let mut after = None;
            while let Ok(hwnd) = unsafe { FindWindowExW(None, after, class, PCWSTR::null()) } {
                after = Some(hwnd);
                if unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) } != monitor { continue; }
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
                    return Some(Taskbar::from_window_rect([r.left, r.top, r.right, r.bottom],
                        [rect.left, rect.top, rect.right, rect.bottom], scale));
                }
            }
        }
        None
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
            let work = mi.monitorInfo.rcWork;
            let reserved = Taskbar::from_work_area([r.left, r.top, r.right, r.bottom],
                [work.left, work.top, work.right, work.bottom], scale);
            // 자동 숨김의 얇은 예약 띠만 남으면 셸 창을 봐서 펼침·숨김도 따라간다.
            let taskbar = match reserved {
                Some(t) if t.thickness * scale > 2.0 => Some(t),
                _ => hidden_taskbar(m, r, scale).or(reserved),
            };
            let name = String::from_utf16_lossy(&mi.szDevice).trim_end_matches('\0').to_string();
            out.push(DisplayInfo {
                id: file_id(&name),
                name,
                width_pt: (r.right - r.left) as f64 / scale,
                height_pt: (r.bottom - r.top) as f64 / scale,
                scale,
                is_builtin: false,
                is_primary: mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                notch: None,
                taskbar,
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
    fn 독은_메뉴바를_제외하고_화면별_여백과_숨김_방향을_감지() {
        let frame = [-1200.0, -800.0, 1200.0, 800.0];
        for (visible, side) in [([-1200.0, -730.0, 1200.0, 706.0], TaskbarSide::Bottom),
            ([-1130.0, -800.0, 1130.0, 776.0], TaskbarSide::Left),
            ([-1200.0, -800.0, 1130.0, 776.0], TaskbarSide::Right)] {
            assert_eq!(Taskbar::from_visible_frame(frame, visible, TaskbarSide::Bottom), Taskbar { side, thickness: 70.0 });
        }
        for side in [TaskbarSide::Bottom, TaskbarSide::Left, TaskbarSide::Right] {
            // 위 메뉴바만 남거나 자동 숨김이면 환경설정의 방향을 쓴다.
            let t = Taskbar::from_visible_frame(frame, [-1200.0, -800.0, 1200.0, 776.0], side);
            assert_eq!(t, Taskbar { side, thickness: 0.0 });
            let mut d = DisplayInfo::for_test((0.0, 0.0), 1200.0, 800.0, 2.0);
            d.taskbar = Some(t);
            let signature = d.configuration_signature();
            d.taskbar.as_mut().unwrap().thickness = 70.0;
            assert_ne!(signature, d.configuration_signature());
        }
    }

    #[test]
    fn 작업_표시줄_네_방향과_자동_숨김() {
        for (work, side) in [([0, 0, 1200, 750], TaskbarSide::Bottom),
            ([0, 50, 1200, 800], TaskbarSide::Top),
            ([50, 0, 1200, 800], TaskbarSide::Left),
            ([0, 0, 1150, 800], TaskbarSide::Right)] {
            assert_eq!(Taskbar::from_work_area([0, 0, 1200, 800], work, 1.25), Some(Taskbar { side, thickness: 40.0 }));
        }
        assert_eq!(Taskbar::from_work_area([-1200, 0, 0, 800], [-1200, 0, 0, 750], 1.0), Some(Taskbar { side: TaskbarSide::Bottom, thickness: 50.0 }));
        assert_eq!(Taskbar::from_work_area([0, 0, 1200, 800], [0, 0, 1200, 800], 1.0), None);
    }

    #[test]
    fn 자동_숨김_작업_표시줄은_방향을_유지() {
        for (window, side) in [([0, 798, 1200, 838], TaskbarSide::Bottom),
            ([0, -38, 1200, 2], TaskbarSide::Top), ([-38, 0, 2, 800], TaskbarSide::Left),
            ([1198, 0, 1238, 800], TaskbarSide::Right)] {
            assert_eq!(Taskbar::from_window_rect([0, 0, 1200, 800], window, 1.0), Taskbar { side, thickness: 2.0 });
        }
    }

    #[test]
    fn 화면_서명은_위치_두께_숨김_변화를_포함() {
        let mut d = DisplayInfo::for_test((0.0, 0.0), 1200.0, 800.0, 1.0);
        let mut signatures = std::collections::BTreeSet::new();
        signatures.insert(d.configuration_signature());
        for side in [TaskbarSide::Bottom, TaskbarSide::Top, TaskbarSide::Left, TaskbarSide::Right] {
            for thickness in [1.0, 40.0, 60.0] {
                d.taskbar = Some(Taskbar { side, thickness });
                assert!(signatures.insert(d.configuration_signature()));
            }
        }
        let old = d.configuration_signature();
        d.origin.0 = -1200.0;
        assert_ne!(old, d.configuration_signature());
    }

    #[test]
    fn 화면_id는_안전한_파일_이름() {
        assert_eq!(file_id(r"\\.\DISPLAY1"), "DISPLAY1");
        assert_eq!(file_id("a/b:c*?\"<>|"), "a_b_c______");
        let uuid = "A1B2C3D4-1234-5678-90AB-CDEF12345678";
        assert_eq!(file_id(uuid), uuid);
    }

    #[test]
    fn 배율별_픽셀_크기는_올림() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let d = DisplayInfo::for_test((0.0, 0.0), 1000.0, 800.0, scale);
            assert_eq!(d.pixel_size(7.0, 29.0), ((7.0 * scale).ceil() as u32, (29.0 * scale).ceil() as u32));
            #[cfg(not(target_os = "macos"))]
            {
                let f = d.strip_frame(0.0, 0.0, 7.0, 29.0);
                assert_eq!((f.w as u32, f.h as u32), d.pixel_size(7.0, 29.0));
            }
            if scale == 1.0 || scale == 2.0 {
                assert_eq!(d.pixel_size(7.0, 29.0), ((7.0 * scale).round() as u32, (29.0 * scale).round() as u32));
            }
        }
    }

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
