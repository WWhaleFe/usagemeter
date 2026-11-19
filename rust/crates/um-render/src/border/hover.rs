//! 실제 경로에 투영한 위치와 채운 길이로 호버할 띠를 고른다.

use kurbo::{ParamCurve, ParamCurveArclen, ParamCurveNearest, PathSeg, Point};
use super::BorderScene;

// 끝이 수 pt 어긋난 띠도 고를 수 있게 한다. 긴 경로는 화면 크기에 비례해 여유를 준다.
const NEAR_PT: f64 = 6.0;
const NEAR_FRACTION: f64 = 0.015;
const HOVER_SLOP: f64 = 7.0;

/// 장면이 바뀔 때만 길이를 계산한다. 마우스 이동 때는 기존 기하를 재사용한다.
pub struct HoverGeometry {
    paths: Vec<Vec<(PathSeg, f64, f64)>>,
    lengths: Vec<f64>,
}

impl HoverGeometry {
    pub fn new(scene: &BorderScene) -> Self {
        let mut lengths = vec![];
        let paths = scene.items.iter().map(|item| {
            let mut total = 0.0;
            let segs = item.path.segments().filter_map(|seg| {
                let len = seg.arclen(1e-4);
                if !len.is_finite() || len <= 0.0 { return None; }
                let start = total;
                total += len;
                Some((seg, start, len))
            }).collect();
            lengths.push(total);
            segs
        }).collect();
        Self { paths, lengths }
    }

    /// 첫 원소는 현재 보이는 최상단 띠. 뒤에는 길이가 거의 같은 후보만 붙인다.
    pub fn candidates(&self, scene: &BorderScene, x: f64, y: f64) -> Vec<usize> {
        let point = Point::new(x, y);
        let hits: Vec<_> = self.paths.iter().enumerate().map(|(i, segs)| {
            segs.iter().map(|(seg, start, _)| {
                let near = seg.nearest(point, 1e-4);
                let along = start + seg.subsegment(0.0..near.t).arclen(1e-4);
                (near.distance_sq, along)
            }).min_by(|a, b| a.0.total_cmp(&b.0)).and_then(|(dist, along)| {
                let total = self.lengths[i];
                if total <= 0.0 { return None; }
                let fraction = along / total;
                let width = scene.items[i].spans.iter().filter(|s| fraction >= s.from && fraction <= s.to)
                    .map(|s| s.width).fold(0.0, f64::max);
                (width > 0.0 && dist <= (width / 2.0 + HOVER_SLOP).powi(2)).then_some(along)
            })
        }).collect();
        let candidates: Vec<_> = scene.bands.iter().enumerate().rev().filter_map(|(i, band)| {
            let length = self.lengths[band.item] * band.ratio.clamp(0.0, 1.0);
            hits[band.item].filter(|along| length > 0.0 && *along <= length + 1e-4).map(|_| i)
        }).collect();
        let Some(&top) = candidates.first() else { return vec![] };
        let first = &scene.bands[top];
        let first_len = self.lengths[first.item] * first.ratio;
        candidates.into_iter().filter(|&i| {
            let band = &scene.bands[i];
            let tolerance = NEAR_PT.max(NEAR_FRACTION * self.lengths[first.item].max(self.lengths[band.item]));
            (self.lengths[band.item] * band.ratio - first_len).abs() <= tolerance
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::border::{BorderInput, ProviderBand, build_paths};
    use um_core::{settings::Settings, types::{SegPart, BorderCorner, AnchorSide}};

    fn scene(a: f64, b: f64, w: f64) -> BorderScene {
        let mut s = Settings { notch_enabled: false, corner_radii: Default::default(), ..Settings::default() };
        for id in ["claude", "codex"] {
            let l = s.provider_layouts.get_mut(id).unwrap();
            l.segments = Some([SegPart::HTop].into());
            l.anchor_corner = BorderCorner::TopLeft;
            l.anchor_side = AnchorSide::Center;
            l.clockwise = true;
        }
        build_paths(&BorderInput { settings: &s, screen_w: w, screen_h: 200.0,
            bands: [("claude", a), ("codex", b)].into_iter().map(|(id, ratio)| ProviderBand {
                id: id.into(), ratio, color: [1.0; 4], parts: vec![],
            }).collect() })
    }

    #[test]
    fn 짧은_띠_끝을_지나면_긴_띠를_선택() {
        let s = scene(0.2, 0.8, 200.0);
        let h = HoverGeometry::new(&s);
        assert_eq!(h.candidates(&s, 20.0, 1.0), vec![1]);
        assert_eq!(h.candidates(&s, 100.0, 1.0), vec![0]);
        assert!(h.candidates(&s, 190.0, 1.0).is_empty());
        assert!(h.candidates(&s, 20.0, 20.0).is_empty());
    }

    #[test]
    fn 곡선_투영과_구간별_굵기() {
        let mut s = scene(1.0, 0.0, 200.0);
        let mut path = kurbo::BezPath::new();
        path.move_to((10.0, 10.0));
        path.quad_to((100.0, 10.0), (100.0, 100.0));
        s.items[s.bands[0].item].path = path;
        s.items[s.bands[0].item].spans = vec![super::super::Span { from: 0.0, to: 1.0, width: 2.0, notch: false }];
        let h = HoverGeometry::new(&s);
        assert_eq!(h.candidates(&s, 77.5, 32.5), vec![0]);
        assert!(h.candidates(&s, 10.0, 90.0).is_empty());
        s.bands[0].ratio = 0.2;
        assert!(h.candidates(&s, 77.5, 32.5).is_empty());
        assert_eq!(h.candidates(&s, 10.0, 10.0), vec![0]);
    }

    #[test]
    fn 세부_항목의_id와_최상단_선택() {
        let mut s = scene(0.8, 0.0, 200.0);
        let mut part = s.bands[0].clone();
        part.ratio = 0.2;
        part.part = Some("weekly".into());
        s.bands.push(part);
        let h = HoverGeometry::new(&s);
        assert_eq!(h.candidates(&s, 20.0, 1.0), vec![2]);
        assert_eq!(s.bands[2].part.as_deref(), Some("weekly"));
        assert_eq!(h.candidates(&s, 100.0, 1.0), vec![0]);
    }

    #[test]
    fn 비슷한_길이는_절대값과_경로_비율_중_큰_여유() {
        for (a, b, w, n) in [(0.5, 0.52, 200.0, 2), (0.5, 0.54, 200.0, 1),
            (0.5, 0.51, 1000.0, 2), (0.5, 0.52, 1000.0, 1), (0.5, 0.5, 200.0, 2)] {
            let s = scene(a, b, w);
            assert_eq!(HoverGeometry::new(&s).candidates(&s, 20.0, 1.0).len(), n);
        }
    }
}
