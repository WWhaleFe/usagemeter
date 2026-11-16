//! 글자 그리기 도우미: cosmic-text + 필요한 글꼴만 늦게 읽는 전역 `FontSystem`.
//! 계획서 4-3 ⑥, ADR 0002(S8): 시스템 글꼴 전체 대신 한·영·일 몇 개만 읽어 메모리를 줄인다.

use std::sync::{Mutex, OnceLock};

use cosmic_text::{Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap, fontdb};
use tiny_skia::Pixmap;
use um_core::types::Rgba;

struct Fonts {
    fs: FontSystem,
    cache: SwashCache,
    /// 읽은 첫 글꼴의 패밀리 이름(없으면 시스템 sans-serif에 맡긴다).
    primary: Option<String>,
}

static FONTS: OnceLock<Mutex<Fonts>> = OnceLock::new();
/// 앱이 정한 글자 로케일(예: "ja-JP"). 같은 한자라도 로케일에 맞는 글꼴 모양을 고른다.
static LOCALE: Mutex<Option<String>> = Mutex::new(None);

#[cfg(target_os = "macos")]
const FONT_FILES: &[&str] = &[
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/AppleSDGothicNeo.ttc",
    "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
    "/System/Library/Fonts/ヒラギノ角ゴシック W6.ttc",
];
#[cfg(target_os = "windows")]
const FONT_FILES: &[&str] = &[
    "C:\\Windows\\Fonts\\SegUIVar.ttf",
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "C:\\Windows\\Fonts\\segoeuib.ttf",
    "C:\\Windows\\Fonts\\malgun.ttf",
    "C:\\Windows\\Fonts\\malgunbd.ttf",
    "C:\\Windows\\Fonts\\YuGothM.ttc",
    "C:\\Windows\\Fonts\\YuGothB.ttc",
];
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const FONT_FILES: &[&str] = &[];

/// 앱 언어에 맞춘 로케일을 정한다(앱 언어 = ko/en/ja). 이미 글꼴을 읽었으면 다시 만든다.
/// macOS GUI 앱은 `LANG`이 비어 있는 경우가 많아 앱 설정 언어를 넘겨받는다.
pub fn set_locale(code: &str) {
    let changed = {
        let mut l = LOCALE.lock().unwrap();
        let new = Some(code.to_string());
        let changed = *l != new;
        *l = new;
        changed
    };
    if changed && let Some(f) = FONTS.get() {
        *f.lock().unwrap() = build();
    }
}

fn locale() -> String {
    if let Some(l) = LOCALE.lock().unwrap().clone() {
        return l;
    }
    // LANG 형식 "ko_KR.UTF-8" -> "ko-KR". 없으면 한국어(주 사용자층).
    std::env::var("LANG")
        .ok()
        .and_then(|l| l.split('.').next().map(|s| s.replace('_', "-")))
        .filter(|l| l.len() >= 2 && l != "C" && l != "POSIX")
        .unwrap_or_else(|| "ko-KR".into())
}

fn build() -> Fonts {
    let mut db = fontdb::Database::new();
    for p in FONT_FILES {
        let _ = db.load_font_file(p);
    }
    if db.faces().next().is_none() {
        db.load_system_fonts();
    }
    let primary = db.faces().next().and_then(|f| f.families.first().map(|(n, _)| n.clone()));
    Fonts { fs: FontSystem::new_with_locale_and_db(locale(), db), cache: SwashCache::new(), primary }
}

fn fonts() -> &'static Mutex<Fonts> {
    FONTS.get_or_init(|| Mutex::new(build()))
}

/// 한 줄 높이(pt). Swift NSTextField/SwiftUI 기본 줄 간격에 가깝게 글자 크기의 1.2배를 올림한다.
pub fn line_height(size: f32) -> f32 {
    (size * 1.2).ceil()
}

/// 이어 붙여 그릴 글자 조각 하나(색은 조각마다).
pub struct Span<'a> {
    pub text: &'a str,
    pub color: Rgba,
}

/// 배치 결과(pt).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSize {
    pub width: f32,
    pub height: f32,
}

fn to_color(c: Rgba) -> Color {
    let b = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgba(b(c[0]), b(c[1]), b(c[2]), b(c[3]))
}

/// 조각들을 배치하고, `target`이 있으면 (x, y)pt 위치에 그린다. `wrap`이 있으면 그 폭(pt)에서 줄바꿈,
/// 없으면 줄바꿈 없이 한 줄. 줄 높이는 `line_height`.
fn run(
    target: Option<(&mut Pixmap, f32, f32)>,
    spans: &[Span],
    size: f32,
    weight: u16,
    scale: f32,
    wrap: Option<f32>,
) -> TextSize {
    let mut guard = fonts().lock().unwrap_or_else(|e| e.into_inner());
    let Fonts { fs, cache, primary } = &mut *guard;
    let lh = line_height(size);
    let mut buf = Buffer::new(fs, Metrics::new(size * scale, lh * scale));
    buf.set_wrap(if wrap.is_some() { Wrap::WordOrGlyph } else { Wrap::None });
    buf.set_size(wrap.map(|w| w * scale), None);
    let base = {
        let a = Attrs::new().weight(Weight(weight)).color(Color::rgb(0, 0, 0));
        match primary.as_deref() {
            Some(n) => a.family(Family::Name(n)),
            None => a.family(Family::SansSerif),
        }
    };
    buf.set_rich_text(
        spans.iter().map(|s| (s.text, base.clone().color(to_color(s.color)))),
        &base,
        Shaping::Advanced,
        None,
    );
    buf.shape_until_scroll(fs, false);
    let (mut w, mut lines) = (0.0f32, 0usize);
    for r in buf.layout_runs() {
        w = w.max(r.line_w);
        lines += 1;
    }
    let out = TextSize { width: w / scale, height: lines.max(1) as f32 * lh };
    if let Some((pm, x, y)) = target {
        let (ox, oy) = ((x * scale).round() as i32, (y * scale).round() as i32);
        buf.draw(fs, cache, Color::rgb(0, 0, 0), |gx, gy, gw, gh, c| {
            blend_rect(pm, ox + gx, oy + gy, gw, gh, c);
        });
    }
    out
}

/// 글자 크기만 잰다(pt).
pub fn measure(spans: &[Span], size: f32, weight: u16, wrap: Option<f32>) -> TextSize {
    run(None, spans, size, weight, 1.0, wrap)
}

/// 글자를 (x, y)pt(왼쪽 위)에 그린다. 반환은 pt 크기.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    pm: &mut Pixmap,
    x: f32,
    y: f32,
    spans: &[Span],
    size: f32,
    weight: u16,
    scale: f32,
    wrap: Option<f32>,
) -> TextSize {
    run(Some((pm, x, y)), spans, size, weight, scale, wrap)
}

/// 비곱셈(straight) 알파 색을 곱셈(premultiplied) 픽스맵에 source-over로 섞는다.
fn blend_rect(pm: &mut Pixmap, x: i32, y: i32, w: u32, h: u32, c: Color) {
    let a = c.a() as u32;
    if a == 0 {
        return;
    }
    let (pw, ph) = (pm.width() as i32, pm.height() as i32);
    let data = pm.pixels_mut();
    let (sr, sg, sb) = (c.r() as u32 * a, c.g() as u32 * a, c.b() as u32 * a);
    for yy in y.max(0)..(y + h as i32).min(ph) {
        for xx in x.max(0)..(x + w as i32).min(pw) {
            let d = &mut data[(yy * pw + xx) as usize];
            let inv = 255 - a;
            let mix = |s: u32, dv: u8| -> u8 { ((s + 127) / 255 + (dv as u32 * inv + 127) / 255).min(255) as u8 };
            let na = (a + (d.alpha() as u32 * inv + 127) / 255).min(255) as u8;
            let (nr, ng, nb) = (mix(sr, d.red()), mix(sg, d.green()), mix(sb, d.blue()));
            *d = tiny_skia::PremultipliedColorU8::from_rgba(nr.min(na), ng.min(na), nb.min(na), na)
                .unwrap_or(*d);
        }
    }
}
