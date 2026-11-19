//! Windows 정보 패널. 전역 물리 픽셀의 왼쪽 아래 좌표(y는 위로 증가)를 받는다.
//! 배경·둥근 모서리는 렌더러가 그리고 창은 포커스를 받지 않고 마우스를 통과시킨다.

use std::cell::{Cell, RefCell};
use crate::{PlatformError, StripFrame};
use super::overlay::OverlayStrip;

pub struct InfoPanel {
    strip: RefCell<OverlayStrip>,
    frame: Cell<(f64, f64, f64, f64)>,
    visible: Cell<bool>,
}

impl InfoPanel {
    pub fn new(_corner: f64) -> Result<Self, PlatformError> {
        Ok(Self {
            strip: RefCell::new(OverlayStrip::new(StripFrame { x: 0.0, y: 0.0, w: 1.0, h: 1.0 })?),
            frame: Cell::new((0.0, 0.0, 1.0, 1.0)), visible: Cell::new(false),
        })
    }

    pub fn show(&self, rgba: &[u8], width: u32, height: u32, scale: f64, x: f64, y: f64) -> Result<(), PlatformError> {
        let mut strip = self.strip.borrow_mut();
        strip.set_frame(StripFrame { x, y: -y - height as f64, w: width as f64, h: height as f64 });
        strip.present(rgba, width, height, scale)?;
        strip.show();
        self.frame.set((x, y, width as f64, height as f64));
        self.visible.set(true);
        Ok(())
    }

    pub fn move_to(&self, x: f64, y: f64) {
        let (_, _, w, h) = self.frame.get();
        self.strip.borrow_mut().set_frame(StripFrame { x, y: -y - h, w, h });
        self.frame.set((x, y, w, h));
    }

    pub fn size(&self) -> (f64, f64) {
        let (_, _, w, h) = self.frame.get();
        (w, h)
    }

    pub fn frame(&self) -> (f64, f64, f64, f64) { self.frame.get() }
    pub fn hide(&self) { self.strip.borrow().hide(); self.visible.set(false); }
    pub fn is_visible(&self) -> bool { self.visible.get() }
}
