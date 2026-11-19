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
    scenes: BTreeMap<String, (border::BorderScene, border::HoverGeometry, f64)>,
    hover: Option<(String, Vec<usize>, usize)>,
    /// 호버한 선을 띠에 강조해 그리는 중인지(선 강조 옵션). 글 강조만 켜면 후보만 고르고 띠는 그대로.
    hover_draw: bool,
}

fn signature(ds: &[DisplayInfo]) -> String {
    ds.iter()
        .map(DisplayInfo::configuration_signature)
        .collect::<Vec<_>>()
        .join("|")
}

/// 저장된 독 높이는 macOS용으로 유지하고 Windows에서만 실제 예약 두께를 쓴다.
pub(crate) fn taskbar_settings(s: &mut um_core::settings::Settings, taskbar: Option<display::Taskbar>, windows: bool) -> StripSide {
    if !s.dock_line_enabled { return StripSide::Bottom; }
    if windows { s.dock_line_height = taskbar.map_or(0.0, |t| t.thickness); }
    // Windows 예약 영역이 없으면 구역을 접고, macOS는 저장한 경계 거리를 유지한다.
    match taskbar.map(|t| t.side) {
        Some(display::TaskbarSide::Top) => StripSide::Top,
        Some(display::TaskbarSide::Left) => StripSide::Left,
        Some(display::TaskbarSide::Right) => StripSide::Right,
        _ => StripSide::Bottom,
    }
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
        self.scenes.clear();
        self.hover = None;
        let ds = display::displays();
        if self.signature.is_empty() {
            self.signature = signature(&ds);
            tracing::info!("화면 초기화: {}개", ds.len());
        }
        let mut seen = vec![];
        for d in &ds {
            seen.push(d.id.clone());
            let hidden = st.hide_overlay || st.disabled_displays.contains(&d.id);
            let mut s = st.settings_for_display(&d.id);
            let side = taskbar_settings(&mut s, d.taskbar, cfg!(windows));
            let rects = border::strip_layout(&s, d.width_pt, d.height_pt, side, d.scale);
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
            let scene = border::build_positioned_paths(&BorderInput {
                settings: &s,
                screen_w: d.width_pt,
                screen_h: d.height_pt,
                bands,
            }, side);
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
            let full = border::render(&scene, (0.0, 0.0), scene.screen, d.scale);
            for (w, r) in entry.iter() {
                let pm = border::crop_strip(&full, *r, d.scale);
                if let Err(e) = w.present(pm.data(), pm.width(), pm.height(), d.scale) {
                    tracing::warn!("띠 그리기 실패: {e}");
                }
                w.show();
            }
        }
        self.strips.retain(|id, _| seen.contains(id));
    }
}

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
            let full = border::render_emphasized(scene, (0.0, 0.0), scene.screen, *scale, selected);
            for (w, r) in strips {
                let pm = border::crop_strip(&full, *r, *scale);
                if let Err(e) = w.present(pm.data(), pm.width(), pm.height(), *scale) {
                    tracing::warn!("강조 띠 그리기 실패: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_strips(scene: &border::BorderScene, s: &um_core::settings::Settings, scale: f64) -> tiny_skia::Pixmap {
        let (w,h)=scene.screen;
        let full=border::render(scene,(0.0,0.0),scene.screen,scale);
        let l=border::strip_extent(s,StripSide::Left);
        let r=border::strip_extent(s,StripSide::Right);
        let t=border::strip_extent(s,StripSide::Top);
        let b=border::strip_extent(s,StripSide::Bottom);
        let mut before=tiny_skia::Pixmap::new(full.width(),full.height()).unwrap();
        for rect in [[0.0,0.0,l,h],[w-r,0.0,r,h],[l-2.0,0.0,w-l-r+4.0,t],[l-2.0,h-b,w-l-r+4.0,b]] {
            let pm=border::render(scene,(rect[0],rect[1]),(rect[2],rect[3]),scale);
            before.draw_pixmap((rect[0]*scale).round() as i32,(rect[1]*scale).round() as i32,
                pm.as_ref(),&Default::default(),tiny_skia::Transform::identity(),None);
        }
        before
    }

    #[test]
    fn 띠_합성_재현() {
        let s = um_core::settings::Settings { thickness: 3.5, notch_enabled: false,
            line_opacity: 0.7, ..Default::default() };
        let (w, h, scale) = (1512.0, 982.0, 1.25);
        let scene = border::build_paths(&BorderInput { settings: &s, screen_w: w, screen_h: h,
            bands: vec![ProviderBand { id: "claude".into(), ratio: 1.0, color: [1.0, 0.2, 0.0, 1.0], parts: vec![] }] });
        let full = border::render(&scene, (0.0, 0.0), (w, h), scale);
        let mut tiled = tiny_skia::Pixmap::new(full.width(), full.height()).unwrap();
        for r in border::strip_layout(&s, w, h, StripSide::Bottom, scale) {
            let pm = border::crop_strip(&full, r, scale);
            tiled.draw_pixmap((r[0]*scale).round() as i32, (r[1]*scale).round() as i32,
                pm.as_ref(), &Default::default(), tiny_skia::Transform::identity(), None);
        }
        assert_eq!(full.data(),tiled.data(),"공통 픽셀 버퍼 분할은 채널 차 0이어야 한다");
        // 이전 앱과 같은 반올림 전 원점·2pt 중복 합성으로 결함을 재현한다.
        let before=legacy_strips(&scene,&s,scale);
        let differences:Vec<_>=full.pixels().iter().zip(before.pixels()).enumerate()
            .filter(|(_, (a,b))| a!=b).map(|(i,_)|(i as u32%full.width(),i as u32/full.width())).collect();
        let max=full.data().iter().zip(before.data()).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
        assert!(!differences.is_empty(),"이전 띠 합성 결함이 재현돼야 한다");
        println!("이전 합성: {}픽셀, 채널 최대 {max}, 위치 {differences:?}",differences.len());
        if std::env::var_os("CORNER_DUMP").is_some() {
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("corner-check");
            std::fs::create_dir_all(&dir).unwrap();
            for (name,pm) in [("full",&full),("strips-before",&before),("strips-after",&tiled)] {
                pm.save_png(dir.join(format!("{name}.png"))).unwrap();
                for (corner,x) in [("left",0),("right",full.width()-96)] {
                    let small=pm.clone_rect(tiny_skia::IntRect::from_xywh(x as i32,0,96,96).unwrap()).unwrap();
                    let mut zoom=tiny_skia::Pixmap::new(768,768).unwrap();
                    zoom.draw_pixmap(0,0,small.as_ref(),&Default::default(),tiny_skia::Transform::from_scale(8.0,8.0),None);
                    zoom.save_png(dir.join(format!("{name}-{corner}-8x.png"))).unwrap();
                }
            }
        }
    }

    #[test]
    fn 둥근_모서리의_분수_픽셀_원점_재현() {
        let mut s=um_core::settings::Settings { thickness:3.5,notch_enabled:false,line_opacity:0.7,..Default::default() };
        for c in um_core::types::BorderCorner::ALL {s.corner_radii.insert(c,40.0);}
        s.provider_layouts.get_mut("claude").unwrap().segments=Some(um_core::segments::zone_perimeter(
            &[um_core::types::ScreenZone::Main].into(),false,false));
        let scene=border::build_paths(&BorderInput {settings:&s,screen_w:1366.0,screen_h:768.0,
            bands:vec![ProviderBand {id:"claude".into(),ratio:1.0,color:[1.0,0.2,0.0,1.0],parts:vec![]}]});
        let scale=1.5;
        let full=border::render(&scene,(0.0,0.0),scene.screen,scale);
        let before=legacy_strips(&scene,&s,scale);
        let bad=full.pixels().iter().zip(before.pixels()).filter(|(a,b)|a!=b).count();
        let max=full.data().iter().zip(before.data()).map(|(a,b)|a.abs_diff(*b)).max().unwrap();
        assert!(bad>0);
        println!("둥근 모서리 이전 합성: {bad}픽셀, 최대 채널 차 {max}, 오른쪽 창 원점 오차 {}px",
            ((scene.screen.0-border::strip_extent(&s,StripSide::Right))*scale).round()
                -(scene.screen.0-border::strip_extent(&s,StripSide::Right))*scale);
        let mut after=tiny_skia::Pixmap::new(full.width(),full.height()).unwrap();
        for r in border::strip_layout(&s,scene.screen.0,scene.screen.1,StripSide::Bottom,scale) {
            let pm=border::crop_strip(&full,r,scale);
            after.draw_pixmap((r[0]*scale).round() as i32,(r[1]*scale).round() as i32,
                pm.as_ref(),&Default::default(),tiny_skia::Transform::identity(),None);
        }
        assert_eq!(full.data(),after.data());
        if std::env::var_os("CORNER_DUMP").is_some() {
            let dir=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("corner-check");
            std::fs::create_dir_all(&dir).unwrap();
            for (name,pm) in [("curve-before",&before),("curve-after",&after)] {
                let small=pm.clone_rect(tiny_skia::IntRect::from_xywh((full.width()-96) as i32,0,96,96).unwrap()).unwrap();
                let mut zoom=tiny_skia::Pixmap::new(768,768).unwrap();
                zoom.draw_pixmap(0,0,small.as_ref(),&Default::default(),tiny_skia::Transform::from_scale(8.0,8.0),None);
                zoom.save_png(dir.join(format!("{name}-right-8x.png"))).unwrap();
            }
        }
    }

    #[test]
    fn 맥은_감지_두께와_무관하게_저장한_거리와_위_메뉴바_띠를_유지() {
        for side in [display::TaskbarSide::Left, display::TaskbarSide::Right, display::TaskbarSide::Bottom] {
            let mut s = um_core::settings::Settings { dock_line_enabled: true, menu_line_enabled: true,
                menu_line_height: 25.0, dock_line_height: 70.0, ..Default::default() };
            for thickness in [0.0, 50.0, 100.0] {
                let position = taskbar_settings(&mut s, Some(display::Taskbar { side, thickness }), false);
                assert_eq!(s.dock_line_height, 70.0);
                let rects = border::strip_layout(&s, 1200.0, 800.0, position, 1.0);
                assert!(rects.iter().any(|r| r[1] == 0.0 && r[3] > 25.0 && r[2] > 600.0));
                match side {
                    display::TaskbarSide::Left => assert!(rects[0][2] > 70.0),
                    display::TaskbarSide::Right => assert!(rects[1][2] > 70.0),
                    _ => assert!(rects[3][3] > 70.0),
                }
            }
        }
    }

    #[test]
    fn 작업_표시줄_실제_두께는_저장_높이와_독립() {
        for (position, side) in [(display::TaskbarSide::Bottom, StripSide::Bottom),
            (display::TaskbarSide::Top, StripSide::Top), (display::TaskbarSide::Left, StripSide::Left),
            (display::TaskbarSide::Right, StripSide::Right)] {
            let mut s = um_core::settings::Settings { dock_line_enabled: true, dock_line_height: 123.0, ..Default::default() };
            assert_eq!(taskbar_settings(&mut s, Some(display::Taskbar { side: position, thickness: 48.0 }), true), side);
            assert_eq!(s.dock_line_height, 48.0);
            let (cw, ch) = side.canonical_size(1200.0, 800.0);
            let rects = border::strip_layout(&s, cw, ch, StripSide::Bottom, 1.0);
            let dock = side.orient_rect(rects[3], 1200.0, 800.0);
            match side {
                StripSide::Bottom => assert!(dock[1] < 752.0 && dock[1] + dock[3] == 800.0),
                StripSide::Top => assert!(dock[1] == 0.0 && dock[3] > 48.0),
                StripSide::Left => assert!(dock[0] == 0.0 && dock[2] > 48.0),
                StripSide::Right => assert!(dock[0] < 1152.0 && dock[0] + dock[2] == 1200.0),
            }
        }
        let mut s = um_core::settings::Settings::default();
        taskbar_settings(&mut s, None, false);
        assert_eq!(s.dock_line_height, 70.0);
        s.dock_line_enabled = true;
        taskbar_settings(&mut s, None, true);
        assert_eq!(s.dock_line_height, 0.0);
    }
}
