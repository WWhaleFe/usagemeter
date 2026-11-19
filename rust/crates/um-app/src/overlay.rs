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
    #[cfg(target_os = "macos")]
    scenes: BTreeMap<String, (border::BorderScene, border::HoverGeometry, f64)>,
    #[cfg(target_os = "macos")]
    hover: Option<(String, Vec<usize>, usize)>,
    /// 호버한 선을 띠에 강조해 그리는 중인지(선 강조 옵션). 글 강조만 켜면 후보만 고르고 띠는 그대로.
    #[cfg(target_os = "macos")]
    hover_draw: bool,
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
        #[cfg(target_os = "macos")]
        { self.scenes.clear(); self.hover = None; }
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
                .filter(|(id, _)| !st.main.border_hidden_ids.contains(*id))
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
            #[cfg(target_os = "macos")]
            self.scenes.insert(d.id.clone(), (scene.clone(), border::HoverGeometry::new(&scene), d.scale));
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

#[cfg(target_os = "macos")]
impl Overlay {
    /// 후보가 달라질 때만 띠를 다시 그린다. 좌표만 바뀌면 기존 그림을 유지한다.
    pub fn hover_at(&mut self, display: Option<(&str, f64, f64)>, draw: bool) -> Vec<(String, Option<String>)> {
        let next = display.and_then(|(id, x, y)| {
            let (scene, geometry, _) = self.scenes.get(id)?;
            let hits = geometry.candidates(scene, x, y);
            (!hits.is_empty()).then(|| (id.to_string(), hits))
        });
        let same = match (&self.hover, &next) {
            (Some((a, hits, _)), Some((b, next))) => a == b && hits == next,
            (None, None) => true,
            _ => false,
        };
        if !same || draw != self.hover_draw {
            let old = self.hover.take().map(|h| h.0).filter(|_| self.hover_draw);
            self.hover = next.map(|(id, hits)| (id, hits, 0));
            self.hover_draw = draw;
            // 강조를 지우거나 새로 그려야 하는 화면만 다시 그린다.
            if let Some(id) = &old { self.present_hover(id); }
            if draw
                && let Some((id, _, _)) = &self.hover
                && old.as_ref() != Some(id)
            { self.present_hover(id); }
        }
        self.hover.as_ref().and_then(|(id, hits, _)| self.scenes.get(id).map(|(scene, _, _)| {
            hits.iter().map(|&i| {
                let band = &scene.bands[i];
                (scene.items[band.item].id.clone(), band.part.clone())
            }).collect()
        })).unwrap_or_default()
    }

    pub fn cycle_hover(&mut self) {
        if !self.hover_draw { return; }
        let Some((id, hits, index)) = &mut self.hover else { return };
        if hits.len() < 2 { return; }
        *index = (*index + 1) % hits.len();
        let id = id.clone();
        self.present_hover(&id);
    }

    fn present_hover(&self, id: &str) {
        let Some((scene, _, scale)) = self.scenes.get(id) else { return };
        let selected = self.hover.as_ref().filter(|h| self.hover_draw && h.0 == id).map(|h| h.1[h.2]);
        if let Some(strips) = self.strips.get(id) {
            for (w, r) in strips {
                let pm = border::render_emphasized(scene, (r[0], r[1]), (r[2], r[3]), *scale, selected);
                if let Err(e) = w.present(pm.data(), pm.width(), pm.height(), *scale) {
                    tracing::warn!("강조 띠 그리기 실패: {e}");
                }
            }
        }
    }
}
