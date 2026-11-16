//! macOS 정보 패널(호버·메뉴바 패널). 반투명 HUD 배경(NSVisualEffectView) 위에
//! `um-render`가 그린 투명 배경 그림을 CALayer로 올린다(계획서 6-1 구현 방법).
//! Swift HoverInfoController.swift:33-62: 테두리 없음, 그림자, statusBar+1, 마우스 무시, 모든 Space, 모서리 7.

use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSPanel, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::CFData;
use objc2_core_graphics::{CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::PlatformError;

pub struct InfoPanel {
    panel: Retained<NSPanel>,
    content: Retained<NSView>,
}

impl InfoPanel {
    /// 패널을 만든다(숨김 상태). `corner` = 모서리 반지름(pt). 메인 스레드에서만.
    pub fn new(corner: f64) -> Result<Self, PlatformError> {
        let mtm = MainThreadMarker::new().ok_or(PlatformError::NotMainThread)?;
        let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0));
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            rect,
            NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            NSBackingStoreType::Buffered,
            false,
        );
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(true);
        panel.setIgnoresMouseEvents(true);
        panel.setLevel(26); // NSStatusWindowLevel + 1
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
        let fx = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), rect);
        fx.setMaterial(NSVisualEffectMaterial::HUDWindow);
        fx.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        fx.setState(NSVisualEffectState::Active);
        fx.setWantsLayer(true);
        if let Some(l) = fx.layer() {
            l.setCornerRadius(corner);
            l.setMasksToBounds(true);
        }
        let content = NSView::initWithFrame(NSView::alloc(mtm), rect);
        content.setWantsLayer(true);
        fx.addSubview(&content);
        panel.setContentView(Some(&fx));
        Ok(Self { panel, content })
    }

    /// 그림(미리곱한 RGBA, 투명 배경)을 올리고 전역 좌표 (x, y)(왼쪽 아래 기준, pt)에 w×h로 보인다.
    pub fn show(&self, rgba_premul: &[u8], width_px: u32, height_px: u32, scale: f64, x: f64, y: f64) -> Result<(), PlatformError> {
        let expected = width_px as usize * height_px as usize * 4;
        if rgba_premul.len() != expected {
            return Err(PlatformError::BadPixels { got: rgba_premul.len(), expected });
        }
        let (w, h) = (width_px as f64 / scale, height_px as f64 / scale);
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
        self.panel.setFrame_display(NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)), false);
        self.content.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h)));
        let layer = self.content.layer().ok_or_else(|| PlatformError::Os("layer 없음".into()))?;
        let obj: &AnyObject = unsafe { &*(objc2_core_foundation::CFRetained::as_ptr(&img).as_ptr() as *const AnyObject) };
        unsafe { layer.setContents(Some(obj)) };
        layer.setContentsScale(scale);
        self.panel.orderFrontRegardless();
        Ok(())
    }

    /// 그림은 그대로 두고 위치만 옮긴다(전역 좌표, 왼쪽 아래 기준).
    pub fn move_to(&self, x: f64, y: f64) {
        self.panel.setFrameOrigin(NSPoint::new(x, y));
    }

    /// 현재 크기(pt).
    pub fn size(&self) -> (f64, f64) {
        let f = self.panel.frame();
        (f.size.width, f.size.height)
    }

    /// 현재 위치·크기(전역 좌표, 왼쪽 아래 기준).
    pub fn frame(&self) -> (f64, f64, f64, f64) {
        let f = self.panel.frame();
        (f.origin.x, f.origin.y, f.size.width, f.size.height)
    }

    pub fn hide(&self) {
        self.panel.orderOut(None);
    }

    pub fn is_visible(&self) -> bool {
        self.panel.isVisible()
    }
}
