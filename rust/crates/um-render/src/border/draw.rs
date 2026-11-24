//! 장면(경로 + 굵기 구간) 그리기. Swift `BorderView.body`의 Canvas 부분(BorderView.swift:681-762)을
//! kurbo 길이 기준 자르기 + tiny-skia 스트로크로 옮긴다.

use kurbo::{ParamCurve, ParamCurveArclen, PathSeg};
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};
use um_core::types::Rgba;

use super::BorderScene;

/// 길이 계산 정확도(pt). 기준 이미지와의 차이가 눈에 안 띄는 수준.
const ACC: f64 = 1e-4;

/// 페이드 조각 수. (BorderView.swift:679)
const FADE_STEPS: usize = 12;

/// 경로의 누적 길이 표. `trimmedPath(from:to:)`(전체 길이 비율 기준 자르기)를 대신한다.
struct Measure {
    segs: Vec<PathSeg>,
    /// 각 선분의 길이
    lens: Vec<f64>,
    /// 각 선분 시작까지의 누적 길이
    starts: Vec<f64>,
    total: f64,
}

impl Measure {
    fn new(path: &kurbo::BezPath) -> Self {
        let mut segs = vec![];
        let mut lens = vec![];
        let mut starts = vec![];
        let mut total = 0.0;
        for s in path.segments() {
            let len = s.arclen(ACC);
            if len.is_nan() || len <= 0.0 {
                continue; // 길이 0 선분은 자르기에 영향이 없다. 퇴화한 2차 곡선은 NaN이 나오므로 부정 비교로 거른다.
            }
            segs.push(s);
            lens.push(len);
            starts.push(total);
            total += len;
        }
        Self { segs, lens, starts, total }
    }

    /// 전체 길이의 비율 [a, b] 구간만 남긴 열린 경로(pt 좌표).
    fn trim(&self, a: f64, b: f64) -> Option<tiny_skia::Path> {
        if self.segs.is_empty() || b <= a {
            return None;
        }
        let (la, lb) = (a.clamp(0.0, 1.0) * self.total, b.clamp(0.0, 1.0) * self.total);
        // la가 속한 첫 선분을 이진 탐색.
        let first = self.starts.partition_point(|&s| s <= la).saturating_sub(1);
        let mut pb = PathBuilder::new();
        let mut started = false;
        for i in first..self.segs.len() {
            let (s0, len) = (self.starts[i], self.lens[i]);
            if s0 >= lb {
                break;
            }
            let (ls, le) = ((la - s0).max(0.0), (lb - s0).min(len));
            if le <= ls {
                continue;
            }
            let seg = self.segs[i];
            let t0 = if ls <= 0.0 { 0.0 } else { seg.inv_arclen(ls, ACC) };
            let t1 = if le >= len { 1.0 } else { seg.inv_arclen(le, ACC) };
            let sub = if t0 == 0.0 && t1 == 1.0 { seg } else { seg.subsegment(t0..t1) };
            if !started {
                let p = sub.start();
                pb.move_to(p.x as f32, p.y as f32);
                started = true;
            }
            match sub {
                PathSeg::Line(l) => pb.line_to(l.p1.x as f32, l.p1.y as f32),
                PathSeg::Quad(q) => pb.quad_to(q.p1.x as f32, q.p1.y as f32, q.p2.x as f32, q.p2.y as f32),
                PathSeg::Cubic(c) => pb.cubic_to(
                    c.p1.x as f32,
                    c.p1.y as f32,
                    c.p2.x as f32,
                    c.p2.y as f32,
                    c.p3.x as f32,
                    c.p3.y as f32,
                ),
            }
        }
        if started { pb.finish() } else { None }
    }
}

/// 한 번의 스트로크 모양: 색, 불투명도 배율, 모든 조각 끝을 둥글게 할지.
#[derive(Clone, Copy)]
struct Ink {
    color: Rgba,
    alpha_mul: f64,
    round_all: bool,
}

struct Painter<'a> {
    scene: &'a BorderScene,
    measures: Vec<Measure>,
    pm: Pixmap,
    transform: Transform,
    emphasis: bool,
    width_scale: f64,
}

impl Painter<'_> {
    /// 경로 [a, b] 구간을 굵기 구간별로 잘라 스트로크한다. `round_all`이면 모든 조각 끝을 둥글게,
    /// 아니면 평평한 끝(반투명 겹침 얼룩 방지). (Swift `strokeRange`)
    fn stroke_range(&mut self, idx: usize, a: f64, b: f64, ink: Ink, notch_layer: bool) {
        if b <= a {
            return;
        }
        let Ink { color, alpha_mul, round_all } = ink;
        let alpha = color[3] * alpha_mul * if self.emphasis { 1.0 } else { self.scene.line_opacity };
        if alpha <= 0.0 {
            return;
        }
        let mut paint = Paint { anti_alias: true, ..Paint::default() };
        let c = tiny_skia::Color::from_rgba(
            color[0].clamp(0.0, 1.0) as f32,
            color[1].clamp(0.0, 1.0) as f32,
            color[2].clamp(0.0, 1.0) as f32,
            alpha.clamp(0.0, 1.0) as f32,
        );
        let Some(c) = c else { return };
        paint.set_color(c);
        let scene = self.scene;
        for sp in scene.items[idx].spans.iter().filter(|sp| sp.notch == notch_layer) {
            let (lo, hi) = (a.max(sp.from), b.min(sp.to));
            // 테이퍼 슬라이스도 그려야 하므로 아주 미세한 것만 거른다.
            if hi <= lo + 0.000001 {
                continue;
            }
            let Some(path) = self.measures[idx].trim(lo, hi) else { continue };
            let stroke = Stroke {
                width: (sp.width * self.width_scale) as f32,
                line_cap: if round_all { LineCap::Round } else { LineCap::Butt },
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            self.pm.stroke_path(&path, &paint, &stroke, self.transform, None);
        }
    }

    /// 경로 시작부터 표시 비율만큼 채운다. 끝부분 min(fadeFraction, r)는 12조각으로 불투명도를 낮춘다.
    fn fill(&mut self, idx: usize, ratio: f64, color: Rgba, notch_layer: bool) {
        let r = ratio.clamp(0.0, 1.0);
        let fade = if self.scene.fade_enabled { self.scene.fade_fraction.min(r) } else { 0.0 };
        let solid = (r - fade).max(0.0);
        self.stroke_range(idx, 0.0, solid, Ink { color, alpha_mul: 1.0, round_all: true }, notch_layer);
        if fade > 0.0 {
            for i in 0..FADE_STEPS {
                let a = solid + fade * i as f64 / FADE_STEPS as f64;
                let b = solid + fade * (i + 1) as f64 / FADE_STEPS as f64;
                let op_step = 1.0 - (i + 1) as f64 / FADE_STEPS as f64;
                self.stroke_range(idx, a, b, Ink { color, alpha_mul: op_step, round_all: false }, notch_layer);
            }
        }
    }
}

/// 화면 전체 장면을 `origin`만큼 옮겨 `size_pt` 크기 픽셀 버퍼에 그린다(띠 창 하나의 내용).
/// 결과는 premultiplied RGBA. `scale` = 화면 배율(Retina 2.0).
/// 여러 창을 이어 붙일 때는 전체 화면을 한 번 그린 뒤 `crop_strip`으로 나눠 AA와 픽셀 격자를 공유한다.
pub fn render(scene: &BorderScene, origin: (f64, f64), size_pt: (f64, f64), scale: f64) -> Pixmap {
    render_emphasized(scene, origin, size_pt, scale, None)
}

/// 강조 띠는 모든 레이어 뒤에 불투명하게 다시 그리고 같은 두께 안에 밝은 중심선을 얹는다.
/// 굵기를 늘리지 않아 화면 가장자리와 띠 창 경계에서도 잘리지 않는다.
pub fn render_emphasized(scene: &BorderScene, origin: (f64, f64), size_pt: (f64, f64), scale: f64, emphasis: Option<usize>) -> Pixmap {
    let pw = ((size_pt.0 * scale).ceil() as u32).max(1);
    let ph = ((size_pt.1 * scale).ceil() as u32).max(1);
    render_px(scene, (origin.0 * scale, origin.1 * scale), (pw, ph), scale, emphasis)
}

/// 띠 창 하나(`rect` = 화면 안 pt 사각형)만 그린다. 화면 전체를 그린 뒤 `crop_strip`으로 자른 것과
/// 같은 픽셀 격자(정수 픽셀 원점)를 써서 이웃 띠와 이음매가 맞는다. 전체 화면 버퍼(5K면 62.5 MB)를
/// 잡지 않아 다시 그릴 때마다 생기던 큰 임시 메모리를 없앤다.
pub fn render_strip(scene: &BorderScene, rect: [f64; 4], scale: f64, emphasis: Option<usize>) -> Pixmap {
    let [x, y, w, h] = rect.map(|v| (v * scale).round());
    render_px(scene, (x, y), ((w as u32).max(1), (h as u32).max(1)), scale, emphasis)
}

/// `origin_px`(픽셀)만큼 옮긴 장면을 `pw`×`ph` 픽셀 버퍼에 그린다.
fn render_px(scene: &BorderScene, origin_px: (f64, f64), (pw, ph): (u32, u32), scale: f64, emphasis: Option<usize>) -> Pixmap {
    let pm = Pixmap::new(pw, ph).expect("pixmap size");
    let transform = Transform::from_row(
        scale as f32,
        0.0,
        0.0,
        scale as f32,
        (-origin_px.0) as f32,
        (-origin_px.1) as f32,
    );
    let measures = scene.items.iter().map(|it| Measure::new(&it.path)).collect();
    let mut p = Painter { scene, measures, pm, transform, emphasis: false, width_scale: 1.0 };
    // 노치 우회(메뉴바선 등과 겹치면 뒤에 깔린다) -> 본체 순서. 트랙은 AI당 한 번, 모든 띠 아래.
    for notch_layer in [true, false] {
        if scene.show_track {
            for (idx, it) in scene.items.iter().enumerate() {
                p.stroke_range(idx, 0.0, 1.0, Ink { color: it.color, alpha_mul: 0.18, round_all: false }, notch_layer);
            }
        }
        for band in &scene.bands {
            p.fill(band.item, band.ratio, band.color, notch_layer);
        }
    }
    if let Some(band) = emphasis.and_then(|i| scene.bands.get(i)) {
        p.emphasis = true;
        let color = [band.color[0], band.color[1], band.color[2], 1.0];
        for notch in [true, false] {
            p.stroke_range(band.item, 0.0, band.ratio, Ink { color, alpha_mul: 1.0, round_all: true }, notch);
        }
        p.width_scale = 0.4;
        for notch in [true, false] {
            p.stroke_range(band.item, 0.0, band.ratio, Ink { color: [1.0, 1.0, 1.0, 0.35], alpha_mul: 1.0, round_all: true }, notch);
        }
    }
    p.pm
}
