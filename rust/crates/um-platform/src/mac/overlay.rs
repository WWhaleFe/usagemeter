//! macOS 테두리 띠 창(ADR 0002). 테두리 없는 투명 NSWindow + CALayer에 CGImage.
//! Swift OverlayFrameController.swift:51-61과 같은 창 속성.

use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{NSBackingStoreType, NSColor, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_core_foundation::CFData;
use objc2_core_graphics::{CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::{PlatformError, StripFrame};

/// NSStatusWindowLevel. 호버 패널은 +1, 선 위치 끌기는 +2(Swift와 같음).
const STATUS_LEVEL: isize = 25;

pub struct OverlayStrip {
    win: Retained<NSWindow>,
}

impl OverlayStrip {
    /// 띠 창을 만든다(아직 보이지 않음). 메인 스레드에서만.
    pub fn new(frame: StripFrame) -> Result<Self, PlatformError> {
        let mtm = MainThreadMarker::new().ok_or(PlatformError::NotMainThread)?;
        let rect = NSRect::new(NSPoint::new(frame.x, frame.y), NSSize::new(frame.w, frame.h));
        let win = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm), rect, NSWindowStyleMask::Borderless, NSBackingStoreType::Buffered, false,
            )
        };
        unsafe { win.setReleasedWhenClosed(false) };
        win.setOpaque(false);
        win.setBackgroundColor(Some(&NSColor::clearColor()));
        win.setHasShadow(false);
        win.setIgnoresMouseEvents(true);
        win.setLevel(STATUS_LEVEL);
        win.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        let view = win.contentView().ok_or_else(|| PlatformError::Os("contentView 없음".into()))?;
        view.setWantsLayer(true);
        Ok(Self { win })
    }

    pub fn set_frame(&self, frame: StripFrame) {
        let rect = NSRect::new(NSPoint::new(frame.x, frame.y), NSSize::new(frame.w, frame.h));
        self.win.setFrame_display(rect, false);
    }

    /// 미리곱한 RGBA 픽셀을 올린다. 픽셀은 CFData로 복사되므로 호출 뒤 버려도 된다(계획서 4-3 ②).
    pub fn present(&self, rgba_premul: &[u8], width_px: u32, height_px: u32, scale: f64) -> Result<(), PlatformError> {
        let expected = width_px as usize * height_px as usize * 4;
        if rgba_premul.len() != expected {
            return Err(PlatformError::BadPixels { got: rgba_premul.len(), expected });
        }
        let data = CFData::from_bytes(rgba_premul);
        let provider = CGDataProvider::with_cf_data(Some(&data)).ok_or_else(|| PlatformError::Os("CGDataProvider".into()))?;
        let cs = CGColorSpace::new_device_rgb().ok_or_else(|| PlatformError::Os("CGColorSpace".into()))?;
        let img = unsafe {
            CGImage::new(
                width_px as usize, height_px as usize, 8, 32, width_px as usize * 4, Some(&cs),
                CGBitmapInfo(CGImageAlphaInfo::PremultipliedLast.0), Some(&provider),
                std::ptr::null(), false, CGColorRenderingIntent::RenderingIntentDefault,
            )
        }
        .ok_or_else(|| PlatformError::Os("CGImage".into()))?;
        let view = self.win.contentView().ok_or_else(|| PlatformError::Os("contentView 없음".into()))?;
        let layer = view.layer().ok_or_else(|| PlatformError::Os("layer 없음".into()))?;
        // CGImage는 CF 타입이라 CALayer.contents(id)에 그대로 넘긴다.
        let obj: &AnyObject = unsafe { &*(objc2_core_foundation::CFRetained::as_ptr(&img).as_ptr() as *const AnyObject) };
        unsafe { layer.setContents(Some(obj)) };
        layer.setContentsScale(scale);
        Ok(())
    }

    pub fn show(&self) {
        self.win.orderFrontRegardless();
    }

    pub fn hide(&self) {
        self.win.orderOut(None);
    }
}
