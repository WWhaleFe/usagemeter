//! 전역 마우스 위치 감시(호버 판정용, 계획서 5-4).
//! - macOS: NSEvent 전역·로컬 mouseMoved 감시(Swift HoverInfoController.swift:73-78와 같음, 권한 불필요). 콜백은 메인 스레드.
//! - Windows: 지금은 30Hz `GetCursorPos` 폴링(대체 경로). 저수준 훅(WH_MOUSE_LL)은 7단계(Windows)에서. 콜백은 별도 스레드.
//!
//! 좌표는 OS 전역 좌표(`display::DisplayInfo::to_local`로 화면 안 좌표로 바꾼다).

/// 마우스 사건 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    Move,
    /// 아무 버튼이나 눌림(패널 바깥 클릭으로 닫기용).
    Down,
}

pub struct MouseMonitor {
    #[cfg(target_os = "macos")]
    monitors: Vec<objc2::rc::Retained<objc2::runtime::AnyObject>>,
    #[cfg(windows)]
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl MouseMonitor {
    /// 마우스가 움직이거나 눌릴 때마다 `f(x, y, 종류)`를 부른다. 돌려받은 값을 버리면 감시가 멈춘다.
    #[cfg(target_os = "macos")]
    pub fn start(f: impl Fn(f64, f64, MouseKind) + Send + Sync + 'static) -> Self {
        use block2::RcBlock;
        use objc2_app_kit::{NSEvent, NSEventMask, NSEventType};
        use std::ptr::NonNull;
        use std::sync::Arc;
        let f = Arc::new(f);
        let mask = NSEventMask::MouseMoved
            | NSEventMask::LeftMouseDragged
            | NSEventMask::LeftMouseDown
            | NSEventMask::RightMouseDown;
        let kind = |e: NonNull<NSEvent>| {
            let t = unsafe { e.as_ref() }.r#type();
            if t == NSEventType::LeftMouseDown || t == NSEventType::RightMouseDown { MouseKind::Down } else { MouseKind::Move }
        };
        let g = f.clone();
        let global = RcBlock::new(move |e: NonNull<NSEvent>| {
            let p = NSEvent::mouseLocation();
            g(p.x, p.y, kind(e));
        });
        let l = f.clone();
        let local = RcBlock::new(move |e: NonNull<NSEvent>| -> *mut NSEvent {
            let p = NSEvent::mouseLocation();
            l(p.x, p.y, kind(e));
            e.as_ptr()
        });
        let mut monitors = vec![];
        if let Some(m) = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(mask, &global) {
            monitors.push(m);
        }
        if let Some(m) = unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &local) } {
            monitors.push(m);
        }
        Self { monitors }
    }

    #[cfg(windows)]
    pub fn start(f: impl Fn(f64, f64, MouseKind) + Send + Sync + 'static) -> Self {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let stop = Arc::new(AtomicBool::new(false));
        let s = stop.clone();
        std::thread::spawn(move || {
            let mut last = (i32::MIN, i32::MIN);
            while !s.load(Ordering::Relaxed) {
                let mut p = POINT::default();
                if unsafe { GetCursorPos(&mut p) }.is_ok() && (p.x, p.y) != last {
                    last = (p.x, p.y);
                    f(p.x as f64, p.y as f64, MouseKind::Move);
                }
                std::thread::sleep(std::time::Duration::from_millis(33));
            }
        });
        Self { stop }
    }
}

impl Drop for MouseMonitor {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        for m in &self.monitors {
            unsafe { objc2_app_kit::NSEvent::removeMonitor(m) };
        }
        #[cfg(windows)]
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
