//! 경로 위 굵기 구간. Swift `widthSpans`(BorderView.swift:58-77)와 `tapered`(:768-815).

use super::shape::{ChainShape, Rec};

/// 경로 길이 비율 [from, to] 구간의 굵기. 노치 구간은 레이어상 가장 뒤에 그린다.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub from: f64,
    pub to: f64,
    pub width: f64,
    pub notch: bool,
}

/// 기록을 비율 구간으로 바꾸고 같은 (굵기, 노치) 인접 구간을 병합한다. 반환: (구간, 전체 길이 pt).
pub fn merge_spans(rec: &[Rec]) -> (Vec<Span>, f64) {
    let total: f64 = rec.iter().map(|r| r.len).sum();
    if total <= 0.0 {
        return (vec![], 0.0);
    }
    let mut out: Vec<Span> = vec![];
    let mut acc = 0.0;
    for r in rec {
        let (a, b) = (acc / total, (acc + r.len) / total);
        match out.last_mut() {
            Some(last) if (last.width - r.width).abs() < 0.01 && last.notch == r.notch => last.to = b,
            _ if r.len > 0.001 => out.push(Span { from: a, to: b, width: r.width, notch: r.notch }),
            _ => {}
        }
        acc += r.len;
    }
    (out, total)
}

impl ChainShape {
    /// 경로를 따라 굵기가 달라지는 구간 목록 + 전체 길이(pt). (Swift `widthSpans`)
    pub fn width_spans(&self, w: f64, h: f64) -> (Vec<Span>, f64) {
        merge_spans(&self.build(w, h).rec)
    }
}

/// 경로 전체를 6pt 균일 슬라이스로 리샘플한 뒤 원형 상자 블러 3회(가우시안 근사)로 굵기를 흐르듯 섞는다.
/// 노치 슬라이스는 매 패스마다 원굵기로 고정해 노치는 정확히 원굵기를 유지하고 그 옆으로 긴 램프가 생긴다.
pub fn tapered(spans: &[Span], total: f64, thickness: f64) -> Vec<Span> {
    if spans.len() < 2 || total <= 0.0 {
        return spans.to_vec();
    }
    let slice_pts = 6.0;
    let n = ((total / slice_pts) as i64).clamp(48, 2048) as usize;
    let mut w = vec![0.0f64; n];
    let mut pin = vec![false; n]; // 노치 = 고정
    let mut is_notch = vec![false; n];
    let mut si = 0;
    for i in 0..n {
        let f = (i as f64 + 0.5) / n as f64;
        while si < spans.len() - 1 && f > spans[si].to {
            si += 1;
        }
        w[i] = spans[si].width;
        is_notch[i] = spans[si].notch;
        pin[i] = spans[si].notch;
    }
    let pinned = w.clone();
    // 반경 = 전환 길이의 절반.
    let taper_pts = f64::max(600.0, thickness * 140.0);
    let radius = (((taper_pts / 2.0) / (total / n as f64)) as i64).min(n as i64 / 2 - 1).max(1) as usize;
    for _ in 0..3 {
        // 3배 패딩 프리픽스(고리 순환): 창이 배열을 벗어나지 않게.
        let mut pre = vec![0.0f64; 3 * n + 1];
        for i in 0..3 * n {
            pre[i + 1] = pre[i] + w[i % n];
        }
        let mut out = w.clone();
        for (i, o) in out.iter_mut().enumerate() {
            let (lo, hi) = (i + n - radius, i + n + radius);
            *o = (pre[hi + 1] - pre[lo]) / (2 * radius + 1) as f64;
        }
        w = out;
        for i in 0..n {
            if pin[i] {
                w[i] = pinned[i]; // 노치 원굵기 고정
            }
        }
    }
    // 비슷한 굵기의 인접 슬라이스 병합.
    let mut out: Vec<Span> = vec![];
    for i in 0..n {
        let (a, b) = (i as f64 / n as f64, (i + 1) as f64 / n as f64);
        match out.last_mut() {
            Some(last) if (last.width - w[i]).abs() < 0.02 && last.notch == is_notch[i] => last.to = b,
            _ => out.push(Span { from: a, to: b, width: w[i], notch: is_notch[i] }),
        }
    }
    if let Some(last) = out.last_mut() {
        last.to = 1.0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_joins_equal_widths_and_skips_tiny() {
        let rec = [
            Rec { len: 10.0, width: 3.0, notch: false },
            Rec { len: 10.0, width: 3.005, notch: false },
            Rec { len: 0.0005, width: 1.0, notch: false }, // 0.001 이하는 새 구간을 만들지 않는다
            Rec { len: 20.0, width: 1.0, notch: true },
        ];
        let (spans, total) = merge_spans(&rec);
        assert!((total - 40.0005).abs() < 1e-9);
        assert_eq!(spans.len(), 2);
        assert!(spans[1].notch && spans[1].width == 1.0);
    }

    #[test]
    fn tapered_pins_notch_and_smooths_neighbours() {
        let spans = [
            Span { from: 0.0, to: 0.4, width: 3.0, notch: false },
            Span { from: 0.4, to: 0.6, width: 3.0, notch: true },
            Span { from: 0.6, to: 1.0, width: 1.0, notch: false },
        ];
        let out = tapered(&spans, 2000.0, 3.0);
        assert!(out.first().unwrap().from == 0.0 && out.last().unwrap().to == 1.0);
        assert!(out.iter().filter(|s| s.notch).all(|s| s.width == 3.0));
        // 노치가 아닌 구간은 블러로 1~3 사이에서 섞인다.
        assert!(out.iter().filter(|s| !s.notch).all(|s| s.width >= 1.0 - 1e-9 && s.width <= 3.0 + 1e-9));
    }
}
