//! Swift ColorPicker와 같은 시스템 색 패널. AI 선 색은 알파를 지원하고 세부 항목은 불투명하게 저장한다.

use std::cell::RefCell;

use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained, sel};
use objc2_app_kit::{NSColor, NSColorPanel, NSColorSpace};
use objc2_foundation::{NSObject, NSObjectProtocol};

struct ColorIvars {
    changed: Box<dyn Fn([f64; 4])>,
    alpha: bool,
}

define_class!(
    // NSObject에 별도 서브클래스 제약이 없고 AppKit 콜백은 메인 스레드에서만 실행된다.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ColorIvars]
    struct ColorTarget;

    unsafe impl NSObjectProtocol for ColorTarget {}

    impl ColorTarget {
        #[unsafe(method(colorChanged:))]
        fn color_changed(&self, panel: &NSColorPanel) {
            if let Some(c) = panel.color().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()) {
                let alpha = if self.ivars().alpha { c.alphaComponent() } else { 1.0 };
                (self.ivars().changed)([c.redComponent(), c.greenComponent(), c.blueComponent(), alpha]);
            }
        }
    }
);

thread_local! {
    // NSColorPanel의 target은 소유되지 않으므로 패널이 열려 있는 동안 직접 보관한다.
    static TARGET: RefCell<Option<Retained<ColorTarget>>> = const { RefCell::new(None) };
}

pub fn open(color: [f64; 4], alpha: bool, changed: impl Fn([f64; 4]) + 'static) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let target = ColorTarget::alloc(mtm).set_ivars(ColorIvars {
        changed: Box::new(changed),
        alpha,
    });
    // NSObject의 기본 초기화와 위에서 선언한 콜백 시그니처를 사용한다.
    let target: Retained<ColorTarget> = unsafe { msg_send![super(target), init] };
    let panel = NSColorPanel::sharedColorPanel(mtm);
    // 이전 행으로 초기 색 변경이 전달되지 않도록 target을 먼저 비운다.
    unsafe { panel.setTarget(None) };
    panel.setShowsAlpha(alpha);
    panel.setContinuous(true);
    panel.setColor(&NSColor::colorWithSRGBRed_green_blue_alpha(
        color[0],
        color[1],
        color[2],
        if alpha { color[3] } else { 1.0 },
    ));
    unsafe {
        panel.setTarget(Some(&target));
        panel.setAction(Some(sel!(colorChanged:)));
    }
    TARGET.with(|t| *t.borrow_mut() = Some(target));
    panel.orderFront(None);
}

pub fn close() {
    if let Some(mtm) = MainThreadMarker::new()
        && NSColorPanel::sharedColorPanelExists(mtm)
    {
        let panel = NSColorPanel::sharedColorPanel(mtm);
        unsafe {
            panel.setTarget(None);
            panel.setAction(None);
        }
        panel.orderOut(None);
        TARGET.with(|t| *t.borrow_mut() = None);
    }
}
