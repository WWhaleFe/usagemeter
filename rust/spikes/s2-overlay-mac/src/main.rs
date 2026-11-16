// S2(macOS): 네이티브 테두리 띠 창 시제품.
// - 띠 창 4개(좌·우·상·하), 크기는 실제 그리는 폭만큼(inset + 두께 + 곡률 + 여유)
// - 화면 전체 경로(kurbo)를 띠 원점만큼 옮겨 tiny-skia로 그리고, CGImage로 CALayer에 올린 뒤 픽셀 버퍼는 버린다
// - 클릭 통과, statusBar 레벨, 모든 Space, 전체화면 보조
use kurbo::{BezPath, ParamCurve, ParamCurveArclen, PathSeg, Point};
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSScreen, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::CFData;
use objc2_core_graphics::{CGBitmapInfo, CGColorRenderingIntent, CGColorSpace, CGDataProvider, CGImage, CGImageAlphaInfo};
use objc2_foundation::{NSPoint, NSRect, NSSize};

const THICK: f64 = 3.0;
const RADIUS: f64 = 22.0;
const RATIO: f64 = 0.62;

/// 화면 전체 테두리 경로(왼쪽 위에서 시계 방향, 위쪽 모서리만 둥글게). 좌표: 왼쪽 위 원점, pt.
fn border_path(w: f64, h: f64) -> BezPath {
    let i = THICK / 2.0;
    let mut p = BezPath::new();
    p.move_to((i, h - i));
    p.line_to((i, i + RADIUS));
    p.quad_to((i, i), (i + RADIUS, i));
    p.line_to((w - i - RADIUS, i));
    p.quad_to((w - i, i), (w - i, i + RADIUS));
    p.line_to((w - i, h - i));
    p.line_to((i, h - i));
    p
}

/// 경로를 길이 기준으로 0~ratio만큼 자른다.
fn trim(path: &BezPath, ratio: f64) -> BezPath {
    let segs: Vec<PathSeg> = path.segments().collect();
    let total: f64 = segs.iter().map(|s| s.arclen(1e-3)).sum();
    let mut left = total * ratio;
    let mut out = BezPath::new();
    for (k, s) in segs.iter().enumerate() {
        let len = s.arclen(1e-3);
        if k == 0 { out.move_to(s.start()); }
        if left >= len { out.push(s.as_path_el()); left -= len; continue; }
        if left > 0.0 { let t = s.inv_arclen(left, 1e-3); out.push(s.subsegment(0.0..t).as_path_el()); }
        break;
    }
    out
}

fn to_skia(p: &BezPath, dx: f64, dy: f64, scale: f64) -> Option<tiny_skia::Path> {
    let mut b = tiny_skia::PathBuilder::new();
    let f = |pt: Point| (((pt.x - dx) * scale) as f32, ((pt.y - dy) * scale) as f32);
    for el in p.elements() {
        match *el {
            kurbo::PathEl::MoveTo(a) => { let (x, y) = f(a); b.move_to(x, y) }
            kurbo::PathEl::LineTo(a) => { let (x, y) = f(a); b.line_to(x, y) }
            kurbo::PathEl::QuadTo(a, c) => { let (x1, y1) = f(a); let (x, y) = f(c); b.quad_to(x1, y1, x, y) }
            kurbo::PathEl::CurveTo(a, c, d) => { let (x1, y1) = f(a); let (x2, y2) = f(c); let (x, y) = f(d); b.cubic_to(x1, y1, x2, y2, x, y) }
            kurbo::PathEl::ClosePath => b.close(),
        }
    }
    b.finish()
}

/// 띠 하나를 그려 CGImage로 만든다. 픽셀 버퍼는 이 함수가 끝나면 사라진다(CFData가 사본을 가짐).
fn render_strip(full: &BezPath, fill: &BezPath, ox: f64, oy: f64, w: f64, h: f64, scale: f64) -> Retained<CGImage> {
    let (pw, ph) = ((w * scale).ceil() as u32, (h * scale).ceil() as u32);
    let mut pm = tiny_skia::Pixmap::new(pw, ph).unwrap();
    let stroke = tiny_skia::Stroke { width: (THICK * scale) as f32, line_cap: tiny_skia::LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
    let mut paint = tiny_skia::Paint { anti_alias: true, ..Default::default() };
    paint.set_color_rgba8(217, 120, 87, 46); // 트랙(0.18)
    if let Some(p) = to_skia(full, ox, oy, scale) { pm.stroke_path(&p, &paint, &stroke, tiny_skia::Transform::identity(), None); }
    paint.set_color_rgba8(217, 120, 87, 255);
    if let Some(p) = to_skia(fill, ox, oy, scale) { pm.stroke_path(&p, &paint, &stroke, tiny_skia::Transform::identity(), None); }
    let data = CFData::from_bytes(pm.data());
    let provider = CGDataProvider::with_cf_data(Some(&data)).unwrap();
    let cs = CGColorSpace::new_device_rgb().unwrap();
    unsafe {
        CGImage::new(pw as usize, ph as usize, 8, 32, (pw * 4) as usize, Some(&cs),
            CGBitmapInfo(CGImageAlphaInfo::PremultipliedLast.0), Some(&provider),
            std::ptr::null(), false, CGColorRenderingIntent::RenderingIntentDefault).unwrap().into()
    }
}

fn main() {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let screen = NSScreen::mainScreen(mtm).unwrap();
    let sf = screen.frame();
    let scale = screen.backingScaleFactor();
    let (sw, sh) = (sf.size.width, sf.size.height);
    let full = border_path(sw, sh);
    let fill = trim(&full, RATIO);
    // 띠 두께 = inset + 두께 + 곡률 + 여유 4pt (계획서 4-3 ②)
    let band = THICK / 2.0 + THICK + RADIUS + 4.0;
    // (원점 x, 원점 y(위 기준), 폭, 높이)
    let strips = [(0.0, 0.0, band, sh), (sw - band, 0.0, band, sh), (band - 2.0, 0.0, sw - 2.0 * band + 4.0, band), (band - 2.0, sh - band, sw - 2.0 * band + 4.0, band)];
    let mut keep = Vec::new();
    for (ox, oy, w, h) in strips {
        let img = render_strip(&full, &fill, ox, oy, w, h, scale);
        // AppKit 좌표(아래 원점)로 변환
        let rect = NSRect::new(NSPoint::new(sf.origin.x + ox, sf.origin.y + sh - oy - h), NSSize::new(w, h));
        let win = unsafe { NSWindow::initWithContentRect_styleMask_backing_defer(NSWindow::alloc(mtm), rect, NSWindowStyleMask::Borderless, NSBackingStoreType::Buffered, false) };
        unsafe { win.setReleasedWhenClosed(false) };
        win.setOpaque(false);
        win.setBackgroundColor(Some(&NSColor::clearColor()));
        win.setHasShadow(false);
        win.setIgnoresMouseEvents(true);
        win.setLevel(25); // NSStatusWindowLevel
        win.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::FullScreenAuxiliary);
        let view = win.contentView().unwrap();
        view.setWantsLayer(true);
        let layer = view.layer().unwrap();
        let obj: &AnyObject = unsafe { &*(Retained::as_ptr(&img) as *const AnyObject) };
        unsafe { layer.setContents(Some(obj)) };
        layer.setContentsScale(scale);
        win.orderFrontRegardless();
        keep.push(win);
    }
    eprintln!("[S2] 화면 {sw}x{sh}pt @{scale}x, 띠 폭 {band}pt");
    app.run();
}
