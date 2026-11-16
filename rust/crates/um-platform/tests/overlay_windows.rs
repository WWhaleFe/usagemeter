//! Windows CI에서 띠 창을 실제로 만들고 픽셀을 올려 본다(레이어드 창 생성·UpdateLayeredWindow 성공 여부).
//! macOS는 NSWindow가 메인 스레드를 요구해 테스트 하네스에서 돌릴 수 없으므로 스파이크(S2)로 확인했다.
#![cfg(windows)]

use um_platform::{OverlayStrip, StripFrame};

#[test]
fn 띠_창을_만들고_픽셀을_올린다() {
    let frame = StripFrame { x: 0.0, y: 0.0, w: 40.0, h: 300.0 };
    let strip = OverlayStrip::new(frame).expect("창 생성");
    let px: Vec<u8> = [200u8, 100, 80, 255].repeat(40 * 300);
    strip.present(&px, 40, 300, 1.0).expect("UpdateLayeredWindow");
    strip.show();
    strip.hide();
    assert!(strip.present(&px[..10], 40, 300, 1.0).is_err(), "크기가 틀린 픽셀은 거부");
}
