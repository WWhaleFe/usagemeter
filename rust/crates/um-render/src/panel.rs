//! 호버 글자·정보 카드·미니 차트 그리기(투명 배경 RGBA 픽스맵).
//! 흐린 배경(NSVisualEffectView/Acrylic)은 OS가 그리므로 여기서는 글자·도형만 얹는다.
//! 좌표·크기는 pt, 픽스맵은 `scale`배 픽셀. 크기 반환값(pt)으로 플랫폼 창 크기를 정한다.

use tiny_skia::{
    FillRule, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, Stroke, StrokeDash, Transform,
};
use um_core::types::Rgba;

use crate::text::{self, Span};

/// 글자 굵기(CSS 값). 시스템 글꼴 medium/semibold/bold.
const W_REGULAR: u16 = 400;
const W_MEDIUM: u16 = 500;
const W_SEMIBOLD: u16 = 600;
const W_BOLD: u16 = 700;

/// 라이트/다크에 따른 글자색(macOS `.primary`/`.secondary`에 가깝게).
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub dark: bool,
}

impl Theme {
    pub fn primary(self) -> Rgba {
        if self.dark { [1.0, 1.0, 1.0, 0.92] } else { [0.0, 0.0, 0.0, 0.88] }
    }
    pub fn secondary(self) -> Rgba {
        if self.dark { [1.0, 1.0, 1.0, 0.55] } else { [0.0, 0.0, 0.0, 0.5] }
    }
    /// 차트 툴팁 바탕(Swift `.regularMaterial` 대용: 반투명 단색).
    fn tooltip_bg(self) -> Rgba {
        if self.dark { [0.16, 0.16, 0.18, 0.92] } else { [0.98, 0.98, 0.98, 0.92] }
    }
}

/// 그려진 결과: 픽스맵(premultiplied RGBA, 배경 투명) + pt 크기.
pub struct Rendered {
    pub pixmap: Pixmap,
    pub width: f32,
    pub height: f32,
}

fn alpha(c: Rgba, a: f64) -> Rgba {
    [c[0], c[1], c[2], c[3] * a]
}

fn paint(c: Rgba) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(tiny_skia::Color::from_rgba(c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32).unwrap_or(tiny_skia::Color::BLACK));
    p.anti_alias = true;
    p
}

/// pt 크기에 맞는 투명 픽스맵(최소 1x1).
fn canvas(w: f32, h: f32, scale: f32) -> Pixmap {
    let pw = ((w * scale).ceil() as u32).max(1);
    let ph = ((h * scale).ceil() as u32).max(1);
    Pixmap::new(pw, ph).expect("픽스맵 크기")
}

fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let k = r * 0.552_284_7;
    let mut b = PathBuilder::new();
    b.move_to(x + r, y);
    b.line_to(x + w - r, y);
    b.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    b.line_to(x + w, y + h - r);
    b.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    b.line_to(x + r, y + h);
    b.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    b.line_to(x, y + r);
    b.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    b.close();
    b.finish()
}

#[allow(clippy::too_many_arguments)]
fn fill_rrect(pm: &mut Pixmap, s: f32, x: f32, y: f32, w: f32, h: f32, r: f32, c: Rgba) {
    if let Some(p) = rrect(x, y, w, h, r) {
        pm.fill_path(&p, &paint(c), FillRule::Winding, Transform::from_scale(s, s), None);
    }
}

fn fill_circle(pm: &mut Pixmap, s: f32, cx: f32, cy: f32, d: f32, c: Rgba) {
    if let Some(p) = PathBuilder::from_circle(cx, cy, d / 2.0) {
        pm.fill_path(&p, &paint(c), FillRule::Winding, Transform::from_scale(s, s), None);
    }
}

fn hline(pm: &mut Pixmap, s: f32, x0: f32, x1: f32, y: f32, c: Rgba) {
    let mut b = PathBuilder::new();
    b.move_to(x0, y);
    b.line_to(x1, y);
    if let Some(p) = b.finish() {
        let st = Stroke { width: 1.0, ..Stroke::default() };
        pm.stroke_path(&p, &paint(c), &st, Transform::from_scale(s, s), None);
    }
}

// ---- 1. 호버 패널 (HoverInfoController.swift:33-62) ----

/// 호버 글자 한 줄. `dot`이 있고 `text`가 "● "로 시작하면 그 점만 해당 색으로 그린다.
pub struct HoverLine {
    pub text: String,
    pub dot: Option<Rgba>,
}

/// 줄 -> 조각. 줄 사이는 "\n" 조각.
fn hover_spans<'a>(lines: &'a [HoverLine], theme: Theme) -> Vec<Span<'a>> {
    let mut v = vec![];
    for (i, l) in lines.iter().enumerate() {
        if i > 0 {
            v.push(Span { text: "\n", color: theme.primary() });
        }
        match (l.dot, l.text.strip_prefix("● ")) {
            (Some(dot), Some(rest)) => {
                v.push(Span { text: "● ", color: dot });
                v.push(Span { text: rest, color: theme.primary() });
            }
            _ => v.push(Span { text: &l.text, color: theme.primary() }),
        }
    }
    v
}

const HOVER_PAD_X: f32 = 10.0;
const HOVER_PAD_Y: f32 = 6.0;

/// 호버 패널 크기(pt). `max_width`는 패널 전체 최대 폭(글자는 `max_width - 20`에서 줄바꿈).
pub fn measure_hover(lines: &[HoverLine], max_width: f32, theme: Theme) -> (f32, f32) {
    let spans = hover_spans(lines, theme);
    let t = text::measure(&spans, 12.0, W_MEDIUM, Some((max_width - 2.0 * HOVER_PAD_X).max(1.0)));
    (t.width.ceil() + 2.0 * HOVER_PAD_X, t.height + 2.0 * HOVER_PAD_Y)
}

/// 호버 패널 그리기: 12pt medium, 좌우 10·상하 6 여백, 폭 넘으면 줄바꿈.
pub fn render_hover(lines: &[HoverLine], max_width: f32, theme: Theme, scale: f32) -> Rendered {
    let (w, h) = measure_hover(lines, max_width, theme);
    let mut pm = canvas(w, h, scale);
    let spans = hover_spans(lines, theme);
    text::draw(
        &mut pm,
        HOVER_PAD_X,
        HOVER_PAD_Y,
        &spans,
        12.0,
        W_MEDIUM,
        scale,
        Some((max_width - 2.0 * HOVER_PAD_X).max(1.0)),
    );
    Rendered { pixmap: pm, width: w, height: h }
}

// ---- 2. 정보 카드 (MenuInfoCardView.swift) ----

pub struct CardLine {
    pub text: String,
    pub dot: Option<Rgba>,
}

pub struct CardSection {
    pub name: String,
    pub color: Rgba,
    /// 구독 플랜 뱃지(비었거나 None이면 없음).
    pub plan: Option<String>,
    pub lines: Vec<CardLine>,
}

const CARD_PAD_X: f32 = 16.0; // :52
const CARD_PAD_Y: f32 = 10.0;
const CARD_MIN_W: f32 = 340.0; // :53
const CARD_SPACING: f32 = 9.0; // :34 구획 간격(나눔선 위·아래)
const CARD_LINE_SPACING: f32 = 3.0; // :36 이름 줄과 정보 줄 사이
const DOT: f32 = 8.0; // :52
const DOT_GAP: f32 = 6.0;
const NAME_SIZE: f32 = 13.0; // .headline
const PLAN_SIZE: f32 = 10.0; // .caption
const LINE_SIZE: f32 = 12.0; // .callout

fn plan_of(sec: &CardSection) -> Option<&str> {
    sec.plan.as_deref().filter(|p| !p.is_empty())
}

/// 이름 줄 높이: 이름 글자와 뱃지(글자 + 세로 여백 1+1) 중 큰 쪽.
fn header_h() -> f32 {
    text::line_height(NAME_SIZE).max(text::line_height(PLAN_SIZE) + 2.0)
}

/// 카드 크기(pt). 가장 긴 줄에 맞춰 넓어지고 최소 340.
pub fn measure_card(sections: &[CardSection], empty_text: Option<&str>, theme: Theme) -> (f32, f32) {
    let one = |s: &str, size, wt| text::measure(&[Span { text: s, color: theme.primary() }], size, wt, None);
    let mut content_w = 0.0f32;
    let mut h = 0.0f32;
    for (i, sec) in sections.iter().enumerate() {
        if i > 0 {
            h += CARD_SPACING * 2.0 + 1.0;
        }
        let mut hw = one(&sec.name, NAME_SIZE, W_SEMIBOLD).width;
        if let Some(p) = plan_of(sec) {
            hw += 8.0 + one(p, PLAN_SIZE, W_BOLD).width.ceil() + 12.0;
        }
        content_w = content_w.max(hw);
        h += header_h();
        for l in &sec.lines {
            let lw = one(&l.text, LINE_SIZE, W_REGULAR).width + if l.dot.is_some() { DOT + DOT_GAP } else { 0.0 };
            content_w = content_w.max(lw);
            h += CARD_LINE_SPACING + text::line_height(LINE_SIZE);
        }
    }
    if sections.is_empty()
        && let Some(t) = empty_text
    {
        let s = one(t, LINE_SIZE, W_REGULAR);
        content_w = s.width;
        h = s.height;
    }
    (CARD_MIN_W.max(content_w.ceil() + 2.0 * CARD_PAD_X), h + 2.0 * CARD_PAD_Y)
}

/// 정보 카드 그리기. `sections`가 비면 `empty_text`(보조색)만 그린다.
pub fn render_card(sections: &[CardSection], empty_text: Option<&str>, theme: Theme, scale: f32) -> Rendered {
    let (w, h) = measure_card(sections, empty_text, theme);
    let mut pm = canvas(w, h, scale);
    let mut y = CARD_PAD_Y;
    if sections.is_empty() {
        if let Some(t) = empty_text {
            text::draw(
                &mut pm,
                CARD_PAD_X,
                y,
                &[Span { text: t, color: theme.secondary() }],
                LINE_SIZE,
                W_REGULAR,
                scale,
                None,
            );
        }
        return Rendered { pixmap: pm, width: w, height: h };
    }
    for (i, sec) in sections.iter().enumerate() {
        if i > 0 {
            y += CARD_SPACING;
            hline(&mut pm, scale, CARD_PAD_X, w - CARD_PAD_X, y + 0.5, alpha(theme.secondary(), 0.4));
            y += 1.0 + CARD_SPACING;
        }
        let hh = header_h();
        let name = text::draw(
            &mut pm,
            CARD_PAD_X,
            y + (hh - text::line_height(NAME_SIZE)) / 2.0,
            &[Span { text: &sec.name, color: sec.color }],
            NAME_SIZE,
            W_SEMIBOLD,
            scale,
            None,
        );
        if let Some(p) = plan_of(sec) {
            let tw = text::measure(&[Span { text: p, color: sec.color }], PLAN_SIZE, W_BOLD, None).width.ceil();
            let bx = CARD_PAD_X + name.width + 8.0;
            let bh = text::line_height(PLAN_SIZE) + 2.0;
            let by = y + (hh - bh) / 2.0;
            fill_rrect(&mut pm, scale, bx, by, tw + 12.0, bh, bh / 2.0, alpha(sec.color, 0.16));
            text::draw(&mut pm, bx + 6.0, by + 1.0, &[Span { text: p, color: sec.color }], PLAN_SIZE, W_BOLD, scale, None);
        }
        y += hh;
        let lh = text::line_height(LINE_SIZE);
        for l in &sec.lines {
            y += CARD_LINE_SPACING;
            let mut x = CARD_PAD_X;
            if let Some(d) = l.dot {
                fill_circle(&mut pm, scale, x + DOT / 2.0, y + lh / 2.0, DOT, d);
                x += DOT + DOT_GAP;
            }
            text::draw(&mut pm, x, y, &[Span { text: &l.text, color: theme.primary() }], LINE_SIZE, W_REGULAR, scale, None);
            y += lh;
        }
    }
    Rendered { pixmap: pm, width: w, height: h }
}

// ---- 3. 미니 차트 (MiniChartView.swift) ----

pub struct ChartSeries {
    pub name: String,
    pub color: Rgba,
    /// (유닉스 초, 0..1 잔여율)
    pub points: Vec<(f64, f64)>,
}

/// 차트 입력: 폭(pt), 기준 시각(now, 유닉스 초), 기간(시간), x축 눈금 라벨(시각, 문자열),
/// 호버 x(차트 왼쪽 가장자리 기준 pt)가 있으면 점선과 툴팁.
pub struct ChartSpec<'a> {
    pub title: &'a str,
    pub series: &'a [ChartSeries],
    pub width: f32,
    pub now: f64,
    pub hours: f64,
    pub x_labels: &'a [(f64, String)],
    pub hover_x: Option<f32>,
}

const CH_PAD_X: f32 = 14.0; // :115
const CH_PAD_Y: f32 = 6.0;
const CH_GAP: f32 = 4.0; // VStack spacing
const CH_PLOT_H: f32 = 92.0; // :105 차트 전체 높이(축 라벨 포함)
const CH_LABEL: f32 = 10.0; // 축 라벨·툴팁 caption2

/// 차트 전체 높이(pt): 여백 + 제목 + 간격 + 92.
pub fn measure_chart_height() -> f32 {
    CH_PAD_Y * 2.0 + text::line_height(10.0) + CH_GAP + CH_PLOT_H
}

/// 차분 기반 단조 3차 보간(Fritsch-Carlson)으로 만든 경로. 점은 x 오름차순·중복 없음이어야 한다.
fn monotone_path(pts: &[(f32, f32)]) -> Option<Path> {
    let n = pts.len();
    if n < 2 {
        return None;
    }
    let dx: Vec<f32> = pts.windows(2).map(|w| w[1].0 - w[0].0).collect();
    let d: Vec<f32> = pts.windows(2).zip(&dx).map(|(w, dx)| (w[1].1 - w[0].1) / dx).collect();
    let mut m = vec![0.0f32; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for k in 1..n - 1 {
        m[k] = if d[k - 1] * d[k] <= 0.0 { 0.0 } else { (d[k - 1] + d[k]) / 2.0 };
    }
    for k in 0..n - 1 {
        if d[k] == 0.0 {
            m[k] = 0.0;
            m[k + 1] = 0.0;
            continue;
        }
        let (a, b) = (m[k] / d[k], m[k + 1] / d[k]);
        let s = a * a + b * b;
        if s > 9.0 {
            let t = 3.0 / s.sqrt();
            m[k] = t * a * d[k];
            m[k + 1] = t * b * d[k];
        }
    }
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0, pts[0].1);
    for k in 0..n - 1 {
        let h = dx[k] / 3.0;
        pb.cubic_to(pts[k].0 + h, pts[k].1 + m[k] * h, pts[k + 1].0 - h, pts[k + 1].1 - m[k + 1] * h, pts[k + 1].0, pts[k + 1].1);
    }
    pb.finish()
}

/// 미니 차트 그리기. 반환 크기: (`spec.width`, `measure_chart_height()`).
pub fn render_chart(spec: &ChartSpec, theme: Theme, scale: f32) -> Rendered {
    let (w, h) = (spec.width, measure_chart_height());
    let mut pm = canvas(w, h, scale);
    let tf = Transform::from_scale(scale, scale);
    let sec = theme.secondary();
    let lbl = |s: &str| text::measure(&[Span { text: s, color: sec }], CH_LABEL, W_REGULAR, None);

    text::draw(&mut pm, CH_PAD_X, CH_PAD_Y, &[Span { text: spec.title, color: sec }], 10.0, W_REGULAR, scale, None);

    // 플롯 영역: 왼쪽은 y 라벨 폭, 아래는 x 라벨 줄, 위·오른쪽은 라벨 절반이 잘리지 않을 여유.
    let chart_top = CH_PAD_Y + text::line_height(10.0) + CH_GAP;
    let lh = text::line_height(CH_LABEL);
    let pl = CH_PAD_X + lbl("100").width.ceil() + 4.0;
    let pr = w - CH_PAD_X - 8.0;
    let pt_ = chart_top + lh / 2.0;
    let pb_ = chart_top + CH_PLOT_H - lh - 4.0;
    let (pw, ph) = (pr - pl, pb_ - pt_);
    let span = spec.hours * 3600.0;
    let t0 = spec.now - span;
    let tx = |t: f64| pl + ((t - t0) / span) as f32 * pw;
    let ty = |v: f64| pb_ - (v.clamp(0.0, 100.0) / 100.0) as f32 * ph;
    let grid = alpha(sec, 0.35);

    // 격자와 y 라벨(0/50/100)
    for v in [0.0, 50.0, 100.0] {
        let y = ty(v);
        hline(&mut pm, scale, pl, pr, y, grid);
        let s = format!("{}", v as i32);
        let sz = lbl(&s);
        text::draw(&mut pm, pl - 4.0 - sz.width, y - lh / 2.0, &[Span { text: &s, color: sec }], CH_LABEL, W_REGULAR, scale, None);
    }
    // x 눈금(세로 격자 + 라벨)
    for (t, s) in spec.x_labels {
        if *t < t0 || *t > spec.now {
            continue;
        }
        let x = tx(*t);
        let mut b = PathBuilder::new();
        b.move_to(x, pt_);
        b.line_to(x, pb_);
        if let Some(p) = b.finish() {
            pm.stroke_path(&p, &paint(grid), &Stroke { width: 1.0, ..Stroke::default() }, tf, None);
        }
        let sz = lbl(s);
        text::draw(&mut pm, x - sz.width / 2.0, pb_ + 4.0, &[Span { text: s, color: sec }], CH_LABEL, W_REGULAR, scale, None);
    }

    // 선(플롯 영역으로 자른다)
    let mut mask = Mask::new(pm.width(), pm.height());
    if let Some(m) = mask.as_mut()
        && let Some(r) = tiny_skia::Rect::from_xywh(pl, pt_ - 1.0, pw, ph + 2.0)
    {
        m.fill_path(&PathBuilder::from_rect(r), FillRule::Winding, true, tf);
    }
    for s in spec.series {
        let mut pts: Vec<(f64, f64)> = s.points.clone();
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        pts.dedup_by(|a, b| a.0 == b.0);
        let px: Vec<(f32, f32)> = pts.iter().map(|(t, r)| (tx(*t), ty(r * 100.0))).collect();
        if let Some(p) = monotone_path(&px) {
            let st = Stroke { width: 2.0, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Stroke::default() };
            pm.stroke_path(&p, &paint(s.color), &st, tf, mask.as_ref());
        }
    }

    // 호버: 점선 + 툴팁(Swift는 차트 왼쪽 위에 고정)
    if let Some(hx) = spec.hover_x {
        let x = hx.clamp(pl, pr);
        let mut b = PathBuilder::new();
        b.move_to(x, pt_);
        b.line_to(x, pb_);
        if let Some(p) = b.finish() {
            let st = Stroke { width: 1.0, dash: StrokeDash::new(vec![3.0, 3.0], 0.0), ..Stroke::default() };
            pm.stroke_path(&p, &paint(alpha(sec, 0.9)), &st, tf, None);
        }
        let t = t0 + ((x - pl) / pw) as f64 * span;
        draw_tooltip(&mut pm, spec, theme, scale, t, CH_PAD_X + 4.0, chart_top + 4.0);
    }
    Rendered { pixmap: pm, width: w, height: h }
}

/// 시각 + AI별 "● 이름 NN%"(가장 가까운 점). (MiniChartView.swift:106-127)
fn draw_tooltip(pm: &mut Pixmap, spec: &ChartSpec, theme: Theme, scale: f32, t: f64, x: f32, y: f32) {
    use chrono::{DateTime, Local};
    let when = DateTime::from_timestamp(t as i64, 0)
        .map(|d| d.with_timezone(&Local).format("%-m/%-d %H:%M").to_string())
        .unwrap_or_default();
    let rows: Vec<(String, Rgba)> = spec
        .series
        .iter()
        .filter_map(|s| {
            let n = s.points.iter().min_by(|a, b| (a.0 - t).abs().partial_cmp(&(b.0 - t).abs()).unwrap_or(std::cmp::Ordering::Equal))?;
            Some((format!("{} {}%", s.name, (n.1 * 100.0).round() as i32), s.color))
        })
        .collect();
    let lh = text::line_height(CH_LABEL);
    let measure = |s: &str| text::measure(&[Span { text: s, color: theme.primary() }], CH_LABEL, W_REGULAR, None).width;
    let mut tw = measure(&when);
    for (s, _) in &rows {
        tw = tw.max(7.0 + 5.0 + measure(s));
    }
    let (bw, bh) = (tw.ceil() + 12.0, 12.0 + lh * (1 + rows.len()) as f32 + 2.0 * rows.len() as f32);
    fill_rrect(pm, scale, x, y, bw, bh, 6.0, theme.tooltip_bg());
    if let Some(p) = rrect(x + 0.5, y + 0.5, bw - 1.0, bh - 1.0, 6.0) {
        pm.stroke_path(&p, &paint(alpha(theme.secondary(), 0.3)), &Stroke { width: 1.0, ..Stroke::default() }, Transform::from_scale(scale, scale), None);
    }
    let mut cy = y + 6.0;
    text::draw(pm, x + 6.0, cy, &[Span { text: &when, color: theme.secondary() }], CH_LABEL, W_REGULAR, scale, None);
    cy += lh + 2.0;
    for (s, c) in &rows {
        fill_circle(pm, scale, x + 6.0 + 3.5, cy + lh / 2.0, 7.0, *c);
        text::draw(pm, x + 6.0 + 12.0, cy, &[Span { text: s, color: theme.primary() }], CH_LABEL, W_REGULAR, scale, None);
        cy += lh + 2.0;
    }
}

#[cfg(test)]
mod tests;
