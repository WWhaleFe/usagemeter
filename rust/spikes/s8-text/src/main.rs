// S8: cosmic-text로 한·영·일 12pt 글자를 그려 PNG로 저장하고, 시스템 글꼴 전체 대신 필요한 글꼴만 읽을 때의 메모리를 본다.
use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, fontdb};

fn main() {
    let all = std::env::args().any(|a| a == "--all");
    let mut db = fontdb::Database::new();
    if all {
        db.load_system_fonts();
    } else {
        for p in ["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/AppleSDGothicNeo.ttc", "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"] {
            let _ = db.load_font_file(p);
        }
    }
    let mut fs = FontSystem::new_with_locale_and_db("ko-KR".into(), db);
    let mut cache = SwashCache::new();
    let scale = 2.0f32;
    let mut buf = Buffer::new(&mut fs, Metrics::new(12.0 * scale, 16.0 * scale));
    buf.set_size(Some(420.0 * scale), Some(90.0 * scale));
    let text = "Claude  5시간 잔여 58% · 리셋 11/16 15:30\nGemini  週間残量 97% · Weekly remaining\n분석 보기  使用量の分析  Usage Analytics";
    buf.set_text(text, &Attrs::new().family(Family::SansSerif), Shaping::Advanced, None);
    buf.shape_until_scroll(&mut fs, false);
    let (w, h) = ((420.0 * scale) as u32, (90.0 * scale) as u32);
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    pm.fill(tiny_skia::Color::from_rgba8(30, 30, 32, 255));
    buf.draw(&mut fs, &mut cache, Color::rgb(235, 235, 240), |x, y, gw, gh, c| {
        let mut p = tiny_skia::Paint::default();
        p.set_color_rgba8(c.r(), c.g(), c.b(), c.a());
        if let Some(r) = tiny_skia::Rect::from_xywh(x as f32, y as f32, gw as f32, gh as f32) {
            pm.fill_rect(r, &p, tiny_skia::Transform::identity(), None);
        }
    });
    let out = std::env::args().nth(1).filter(|a| a != "--all").unwrap_or("s8.png".into());
    pm.save_png(&out).unwrap();
    eprintln!("[S8] {} 글꼴 얼굴 {}개 → {out}", if all { "시스템 전체" } else { "필요한 것만" }, fs.db().faces().count());
    std::thread::sleep(std::time::Duration::from_secs(3));
}
