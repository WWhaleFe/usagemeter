//! 화면마다 테두리 띠 창을 두고, 스냅샷·설정이 바뀌면 다시 그린다(계획서 5-2, 4-3 ②).
//! 메인 스레드 전용(NSWindow·HWND). 그린 픽셀은 OS에 넘긴 뒤 버린다.

use std::collections::BTreeMap;

use um_platform::OverlayStrip;
use um_platform::display::{self, DisplayInfo};
use um_render::border::{self, BorderInput, ProviderBand, StripSide};

use crate::state::AppState;

#[derive(Default)]
pub struct Overlay {
    /// 화면 id → (띠 창, 화면 안 위치 x, y, w, h(pt))
    strips: BTreeMap<String, Vec<(OverlayStrip, [f64; 4])>>,
    /// 마지막으로 본 화면 구성(바뀌면 띠 창을 다시 만든다).
    signature: String,
}

fn signature(ds: &[DisplayInfo]) -> String {
    ds.iter()
        .map(|d| format!("{}:{}x{}@{}", d.id, d.width_pt, d.height_pt, d.scale))
        .collect::<Vec<_>>()
        .join("|")
}

/// 화면 안 띠 4개(왼쪽 위 원점, pt). 메뉴선·Dock선(macOS)이 켜져 있으면 위·아래 띠를 그 높이까지 넓힌다.
fn layout(s: &um_core::settings::Settings, w: f64, h: f64) -> Vec<[f64; 4]> {
    let l = border::strip_extent(s, StripSide::Left);
    let r = border::strip_extent(s, StripSide::Right);
    let mut t = border::strip_extent(s, StripSide::Top);
    let mut b = border::strip_extent(s, StripSide::Bottom);
    if s.menu_line_enabled {
        t = t.max(s.menu_line_height + border::strip_extent(s, StripSide::Top));
    }
    if s.dock_line_enabled {
        b = b.max(s.dock_line_height + border::strip_extent(s, StripSide::Bottom));
    }
    let seam = 2.0; // 띠 경계 이음매가 안 보이게 살짝 겹침(Swift OverlayFrameController.swift:42)
    vec![
        [0.0, 0.0, l, h],
        [w - r, 0.0, r, h],
        [l - seam, 0.0, w - l - r + 2.0 * seam, t],
        [l - seam, h - b, w - l - r + 2.0 * seam, b],
    ]
}

impl Overlay {
    /// 화면 구성을 확인하고 바뀌었으면 true(띠 창을 버리고 다음 `redraw`에서 새로 만든다).
    pub fn check_displays(&mut self) -> bool {
        let sig = signature(&display::displays());
        if sig != self.signature {
            self.signature = sig;
            self.strips.clear();
            true
        } else {
            false
        }
    }

    pub fn redraw(&mut self, st: &AppState) {
        let ds = display::displays();
        if self.signature.is_empty() {
            self.signature = signature(&ds);
            tracing::info!("화면 초기화: {}개", ds.len());
        }
        let mut seen = vec![];
        for d in &ds {
            seen.push(d.id.clone());
            let hidden = st.hide_overlay || st.disabled_displays.contains(&d.id);
            let s = st.settings_for_display(&d.id);
            let rects = layout(&s, d.width_pt, d.height_pt);
            let entry = self.strips.entry(d.id.clone()).or_default();
            if hidden {
                for (w, _) in entry.iter() {
                    w.hide();
                }
                continue;
            }
            let bands: Vec<ProviderBand> = st
                .snapshots
                .iter()
                .map(|(id, snap)| ProviderBand {
                    id: id.clone(),
                    ratio: snap.remaining_ratio,
                    color: border::provider_color(&st.main, id),
                    parts: border::sub_item_bar_parts(&st.main, id, snap),
                })
                .collect();
            let scene = border::build_paths(&BorderInput {
                settings: &s,
                screen_w: d.width_pt,
                screen_h: d.height_pt,
                bands,
            });
            // 띠 개수·위치가 바뀌었으면 새로 만든다.
            if entry.len() != rects.len() || entry.iter().zip(&rects).any(|((_, a), b)| a != b) {
                tracing::info!("띠 창 재생성: {}개, AI 띠 {}개", rects.len(), st.snapshots.len());
                entry.clear();
                for r in &rects {
                    match OverlayStrip::new(d.strip_frame(r[0], r[1], r[2], r[3])) {
                        Ok(w) => entry.push((w, *r)),
                        Err(e) => tracing::warn!("띠 창 생성 실패: {e}"),
                    }
                }
            }
            for (w, r) in entry.iter() {
                let pm = border::render(&scene, (r[0], r[1]), (r[2], r[3]), d.scale);
                if let Err(e) = w.present(pm.data(), pm.width(), pm.height(), d.scale) {
                    tracing::warn!("띠 그리기 실패: {e}");
                }
                w.show();
            }
        }
        self.strips.retain(|id, _| seen.contains(id));
    }
}
