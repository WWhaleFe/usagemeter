//! 설정·분석 창(웹뷰)과 앱 사이의 명령(계획서 5-5). 계산은 Rust가 하고 화면은 결과만 그린다.
//! - `ui_state`: 화면이 그릴 모든 값(설정, 화면 목록, 로그인 상태, 세부 항목, 프리셋, 업데이트, 문자열).
//! - `ui_set`: 설정 일부(스냅샷 JSON 키)를 바꾼다. 범위 = "main"(전역) 또는 "border"(편집 중인 화면의 테두리).
//! - `ui_action`: 그 밖의 동작(로그인, 프리셋, 화면 선택, 노치 감지, 자동 실행 …).
//!
//! 상태가 바뀌면 `ui-state` 이벤트로 모든 창에 알린다.

use serde_json::{Map, Value, json};
use tauri::{AppHandle, Emitter, Manager};
use um_core::types::BorderEdge;

use um_core::i18n::tr;
use um_core::segments::{
    SegContext, bend_radius_estimate, can_use_segments, chain_ends_on, corner_used, enforce_no_overlap, h_cap, is_valid_segments_on, layout_for, seg_adjacency_on,
    seg_available, seg_ends_on, segs_for, zone_perimeter_on,
};
use um_core::settings::{BRAND_COLORS, MAX_PRESETS, REFRESH_PRESETS, ScreenInfo, Settings, SettingsSnapshot, add_preset, device_fit};
use um_core::types::{AppLanguage, BorderCorner, ScreenZone, SegCap, SegNode, SegPart};
use um_providers::spec;
use um_render::border::sub_item_color;

use crate::state::AppState;

fn seg_ctx(s: &Settings, side: BorderEdge) -> SegContext<'_> {
    SegContext { side, ..um_core::hover::seg_ctx(s) }
}


/// 후원 탭 카카오페이 송금 링크(Swift SupportView.swift).
const KAKAO_URL: &str = "https://qr.kakaopay.com/Fa89McYNg";

/// 편집 중인 화면(설정 창 "디스플레이" 선택). None = 주 화면.
#[derive(Default)]
pub struct Editing(pub std::sync::Mutex<Option<String>>);

fn rgba_hex(c: [f64; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", (c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8)
}

/// 편집 대상 화면 id가 주 화면이면 None.
/// 고른 화면이 없거나 연결이 끊겼으면 주 화면, 주 화면도 연결돼 있지 않으면(원격 접속 등) 첫 연결 화면을 고친다(Swift DisplayRegistry.refresh).
fn editing_id(app: &AppHandle, st: &AppState) -> Option<String> {
    let connected = |id: &str| st.connected_displays.is_empty() || st.connected_displays.iter().any(|c| c == id);
    let e = app.state::<Editing>().0.lock().unwrap().clone().filter(|id| connected(id));
    let e = e.or_else(|| {
        let main_on = st.main_display_id.as_deref().is_some_and(connected);
        if main_on { None } else { st.connected_displays.first().cloned() }
    });
    e.filter(|id| st.main_display_id.as_deref() != Some(id.as_str()))
}

/// 편집 중인 화면의 테두리 설정(주 화면이면 주 설정).
fn border_of<'a>(st: &'a AppState, id: &Option<String>) -> &'a Settings {
    id.as_ref().and_then(|i| st.displays.get(i)).unwrap_or(&st.main)
}

fn snapshot_value(s: &Settings) -> Value {
    serde_json::to_value(s.to_snapshot()).unwrap_or(Value::Null)
}

/// Windows에서 작업 표시줄 문구로 바꿔 보여 줄 Dock 관련 키(`win.<키>`가 i18n에 있어야 한다).
const WIN_TERMS: [&str; 25] = [
    "tab.menuBar", "sec.menuIcon", "menuHover.enable", "menuHover.enableDesc", "tray.clickMode", "disp.iconDesc", "disp.iconUsedDesc", "notify.desc", "display.desc",
    "login.autostart", "corner.linkMainDock", "part.dockHeight", "part.dockLine", "preset.dockWrap",
    "seg.hDock", "seg.lDock", "seg.rDock", "zone.dock", "hoverArea.dockLine",
    "part.dockBoundaryHeight", "part.dockWidth", "seg.dockVertical", "seg.dockTop", "seg.dockBottom", "seg.dockOuter",
];

/// 화면이 그릴 값 전부. 창이 늦게 뜨는 원인을 찾을 수 있게 오래 걸리면 기록한다.
#[tauri::command]
pub fn ui_state(app: AppHandle) -> Value {
    let t = std::time::Instant::now();
    let out = state_value(app);
    let ms = t.elapsed().as_millis();
    if ms >= 200 { tracing::info!("ui_state {ms}ms"); }
    out
}

fn state_value(app: AppHandle) -> Value {
    let st = crate::shared(&app);
    let st = st.lock().unwrap();
    let lang = st.main.language;
    let edit = editing_id(&app, &st);
    let border = border_of(&st, &edit);
    let ds = um_platform::display::displays();
    let displays: Vec<Value> = ds
        .iter()
        .map(|d| {
            json!({
                "id": d.id, "name": d.name, "primary": d.is_primary, "builtin": d.is_builtin,
                "enabled": !st.disabled_displays.contains(&d.id),
                "main": st.main_display_id.as_deref() == Some(d.id.as_str()) || (st.main_display_id.is_none() && d.is_primary),
                "notch": d.notch.map(|n| json!({"width": n.width, "height": n.height})),
            })
        })
        .collect();
    let taskbar = ds.iter().find(|d| edit.as_ref().or(st.main_display_id.as_ref()).map_or(d.is_primary, |id| id == &d.id))
        .or_else(|| ds.first()).and_then(|d| d.taskbar);
    let side = taskbar.map_or(BorderEdge::Bottom, |t| match t.side {
        um_platform::display::TaskbarSide::Top => BorderEdge::Top,
        um_platform::display::TaskbarSide::Bottom => BorderEdge::Bottom,
        um_platform::display::TaskbarSide::Left => BorderEdge::Left,
        um_platform::display::TaskbarSide::Right => BorderEdge::Right,
    });
    let providers: Vec<Value> = st
        .main
        .provider_order
        .iter()
        .filter_map(|id| {
            let sp = spec::spec(id)?;
            let ps = st.providers.get(id).cloned().unwrap_or_default();
            let (items, live) = ps.sub_items();
            let items_v: Vec<Value> = items
                .iter()
                .map(|it| {
                    let kind = tr(st.main.bar_fill_mode.text_key(if it.is_five_hour { "menu.line5h" } else { "menu.lineWeekly" }), lang);
                    let name = it.model_label.as_ref().map(|l| format!("{l} {kind}")).unwrap_or(kind);
                    json!({
                        "id": it.id, "name": name, "ratio": st.main.bar_fill_mode.display_ratio(it.remaining_ratio),
                        "color": rgba_hex(sub_item_color(&st.main, id, &it.id, &items)),
                        "key": um_core::model::UsageSubItem::settings_key(id, &it.id),
                    })
                })
                .collect();
            Some(json!({
                "id": id, "name": sp.name, "loggedIn": ps.logged_in, "experimental": sp.id == "gemini",
                "color": rgba_hex(um_render::border::provider_color(&st.main, id)),
                "items": items_v, "itemsLive": live,
                "segOptions": seg_options(border, id, side),
                "layout": layout_value(border, id, side),
                "chain": chain_ends_on(&segs_for(&seg_ctx(border, side), id), border.menu_line_enabled, border.dock_line_enabled, side).is_some(),
                "anchorSideKind": anchor_side_kind(&seg_ctx(border, side), id),
                "shapePresets": shape_presets(border, id, side),
            }))
        })
        .collect();
    // Windows에는 Dock이 없어 같은 구역을 작업 표시줄이라 부른다(win.<키> 문구로 바꿔 보냄).
    let shown_key = |k: &'static str| -> String {
        let k = st.main.bar_fill_mode.text_key(k);
        if cfg!(windows) && WIN_TERMS.contains(&k) { format!("win.{k}") } else { k.to_string() }
    };
    let strings: Map<String, Value> = um_core::i18n::keys().map(|k| (k.to_string(), Value::String(tr(&shown_key(k), lang)))).collect();
    json!({
        "dockSide": ds.iter().find(|d| edit.as_ref().or(st.main_display_id.as_ref()).map_or(d.is_primary, |id| id == &d.id))
            .or_else(|| ds.first()).and_then(|d| d.taskbar).map(|t| match t.side {
                um_platform::display::TaskbarSide::Top => "top", um_platform::display::TaskbarSide::Bottom => "bottom",
                um_platform::display::TaskbarSide::Left => "left", um_platform::display::TaskbarSide::Right => "right",
            }),
        "platform": if cfg!(target_os = "macos") { "mac" } else { "win" },
        "store": cfg!(feature = "store"),
        "version": crate::update::current_version(),
        "lang": lang.code(),
        "strings": strings,
        "main": snapshot_value(&st.main),
        "border": snapshot_value(border),
        "cornerUsed": corner_used_value(border, side),
        "editing": edit,
        "displays": displays,
        "providers": providers,
        "presets": st.presets.iter().map(|p| json!({"id": p.id, "name": p.name})).collect::<Vec<_>>(),
        "maxPresets": MAX_PRESETS,
        "hasDefault": st.default_snapshot.is_some(),
        "refreshPresets": REFRESH_PRESETS,
        "effectiveRefresh": st.main.effective_refresh_minutes(),
        "brandColors": BRAND_COLORS.iter().map(|b| json!({"name": b.name, "hex": rgba_hex([b.rgb[0], b.rgb[1], b.rgb[2], 1.0])})).collect::<Vec<_>>(),
        "update": update_value(&st),
        "autostart": autostart_value(&app),
    })
}

fn update_value(st: &AppState) -> Value {
    use crate::update::{DownloadState, UpdateState};
    let (state, version) = match &st.update {
        UpdateState::Idle => ("idle", None),
        UpdateState::Checking => ("checking", None),
        UpdateState::UpToDate => ("upToDate", None),
        UpdateState::Available { version, .. } => ("available", Some(version.clone())),
        UpdateState::Failed => ("failed", None),
    };
    let dl = match &st.download {
        DownloadState::Idle => "idle",
        DownloadState::Downloading => "downloading",
        DownloadState::Done(_) => "done",
        DownloadState::Failed => "failed",
    };
    json!({"state": state, "version": version, "download": dl, "latest": st.latest_version, "blocked": st.install_blocked})
}

fn autostart_value(app: &AppHandle) -> Value {
    use tauri_plugin_autostart::ManagerExt;
    // 번들(.app·설치본)에서만 동작한다(Swift: 번들 id가 없으면 비활성).
    let available = std::env::current_exe().map(|p| p.to_string_lossy().contains(".app/") || cfg!(windows)).unwrap_or(false);
    json!({"available": available, "enabled": available && app.autolaunch().is_enabled().unwrap_or(false)})
}

/// 레이아웃 탭 다이어그램용: 세그먼트마다 (쓸 수 있는지, 눌렀을 때 결과가 유효한지, 다른 AI가 쓰는지).
fn seg_options(s: &Settings, id: &str, side: BorderEdge) -> Value {
    let ctx = seg_ctx(s, side);
    let cur = segs_for(&ctx, id);
    let mut out = Map::new();
    for seg in SegPart::ALL {
        let available = seg_available(seg, s.menu_line_enabled, s.dock_line_enabled);
        let mut toggled = cur.clone();
        if !toggled.remove(&seg) {
            toggled.insert(seg);
        }
        let valid = available && is_valid_segments_on(&toggled, s.menu_line_enabled, s.dock_line_enabled, side);
        let allowed = valid && can_use_segments(&ctx, id, &toggled);
        let used_by_other = um_core::segments::PROVIDER_IDS.iter().any(|o| *o != id && segs_for(&ctx, o).contains(&seg));
        let (a, b) = seg_ends_on(seg, s.menu_line_enabled, s.dock_line_enabled, side);
        let ends = json!([{"right": a.right, "level": a.level}, {"right": b.right, "level": b.level}]);
        out.insert(seg.raw().into(), json!({"on": cur.contains(&seg), "available": available, "toggleValid": valid, "toggleAllowed": allowed, "usedByOther": used_by_other, "ends": ends}));
    }
    Value::Object(out)
}

/// 이 AI의 실제 배치(저장된 값이 없으면 기본값, segments는 구버전 변환 결과). 화면이 고쳐서 `providerLayouts`에 되돌려 쓴다.
fn layout_value(s: &Settings, id: &str, side: BorderEdge) -> Value {
    let ctx = seg_ctx(s, side);
    let l = um_core::segments::positioned_layout(&ctx, id);
    serde_json::to_value(l).unwrap_or(Value::Null)
}

/// 곡선 모서리에서 차감이 시작될 지점의 종류: "hidden"(곡률 0·고리 아님) / "round"(2지점) / "scoop"(3지점). (Swift anchorSideKind, SettingsView.swift:~960)
fn anchor_side_kind(ctx: &SegContext, id: &str) -> &'static str {
    let (menu_on, dock_on) = (ctx.menu_on, ctx.dock_on);
    let segs = segs_for(ctx, id);
    if !is_valid_segments_on(&segs, menu_on, dock_on, ctx.side) || chain_ends_on(&segs, menu_on, dock_on, ctx.side).is_some() {
        return "hidden";
    }
    let adj = seg_adjacency_on(&segs, menu_on, dock_on, ctx.side);
    if um_core::segments::lateral_menu(menu_on, dock_on, ctx.side) {
        let l = layout_for(ctx, id);
        let target = match l.anchor_corner {
            BorderCorner::TopLeft => (0.0,0.0), BorderCorner::TopRight => (1.0,0.0),
            BorderCorner::BottomLeft => (0.0,1.0), BorderCorner::BottomRight => (1.0,1.0),
        };
        let node = *adj.keys().min_by(|&&a,&&b| {
            let distance = |n| { let p = um_core::segments::side_node_point(n,ctx.side); (p.0-target.0).hypot(p.1-target.1) };
            distance(a).total_cmp(&distance(b))
        }).unwrap();
        let parts = &adj[&node];
        return if bend_radius_estimate(ctx,id,parts[0],parts[1],node) > 0.01 { "round" } else { "hidden" };
    }
    let (Some(tl), Some(bl)) = (adj.keys().map(|n| n.level).min(), adj.keys().map(|n| n.level).max()) else { return "hidden" };
    let l = layout_for(ctx, id);
    let node = match l.anchor_corner {
        BorderCorner::TopLeft => SegNode { right: false, level: tl },
        BorderCorner::TopRight => SegNode { right: true, level: tl },
        BorderCorner::BottomRight => SegNode { right: true, level: bl },
        BorderCorner::BottomLeft => SegNode { right: false, level: bl },
    };
    let Some(incident) = adj.get(&node) else { return "hidden" };
    let (Some(&h), Some(&v)) = (incident.iter().find(|s| s.is_horizontal()), incident.iter().find(|s| !s.is_horizontal())) else { return "hidden" };
    if bend_radius_estimate(ctx, id, h, v, node) <= 0.01 {
        return "hidden";
    }
    // 스쿱 판정: 가로선 캡 방향이 세로선 진행 방향과 반대.
    let cap = h_cap(ctx, id, h, node.right);
    if cap == SegCap::Up || cap == SegCap::Down {
        let (a, b) = seg_ends_on(v, menu_on, dock_on, ctx.side);
        let v_down = (if a == node { b.level } else { a.level }) > node.level;
        let matches = (cap == SegCap::Down && v_down) || (cap == SegCap::Up && !v_down);
        if !matches {
            return "scoop";
        }
    }
    "round"
}

/// 모양 프리셋(전체·메뉴바·본문·Dock): 영역, 둘레 세그먼트, 지금 쓸 수 있는지. (Swift presetButton, SettingsView.swift:~770)
fn shape_presets(s: &Settings, id: &str, side: BorderEdge) -> Value {
    let ctx = seg_ctx(s, side);
    let mut out = Map::new();
    for (key, zones) in [
        ("all", vec![ScreenZone::MenuBar, ScreenZone::Main, ScreenZone::Dock]),
        ("menuBar", vec![ScreenZone::MenuBar]),
        ("main", vec![ScreenZone::Main]),
        ("dock", vec![ScreenZone::Dock]),
    ] {
        let set = zone_perimeter_on(&zones.iter().copied().collect(), s.menu_line_enabled, s.dock_line_enabled, side);
        let allowed = can_use_segments(&ctx, id, &set);
        out.insert(key.into(), json!({"zones": zones, "segments": set, "allowed": allowed}));
    }
    Value::Object(out)
}

/// 영역 x 꼭짓점별 곡률 사용 여부(모서리 탭에서 안 쓰는 꼭짓점을 비활성화). (Swift cornerUsed)
fn corner_used_value(s: &Settings, side: BorderEdge) -> Value {
    let ctx = seg_ctx(s, side);
    let mut out = Map::new();
    for (name, z) in [("menuBar", ScreenZone::MenuBar), ("main", ScreenZone::Main), ("dock", ScreenZone::Dock)] {
        let mut m = Map::new();
        for (cn, c) in [("topLeft", BorderCorner::TopLeft), ("topRight", BorderCorner::TopRight), ("bottomLeft", BorderCorner::BottomLeft), ("bottomRight", BorderCorner::BottomRight)] {
            m.insert(cn.into(), Value::Bool(corner_used(&ctx, z, c)));
        }
        out.insert(name.into(), Value::Object(m));
    }
    Value::Object(out)
}

/// 상태가 바뀌었음을 모든 창에 알리고 메뉴·테두리를 갱신한다.
pub fn changed(app: &AppHandle) {
    let _ = app.emit("ui-state", ());
    #[cfg(target_os = "macos")]
    crate::windows::refresh_titles(app);
    crate::menu::rebuild(app);
    crate::redraw(app);
}

/// 스냅샷 JSON 일부를 덮어써서 설정에 반영한다.
fn patch(s: &mut Settings, patch: &Value) -> Result<(), String> {
    let mut v = snapshot_value(s);
    let (Some(obj), Some(p)) = (v.as_object_mut(), patch.as_object()) else { return Err("bad patch".into()) };
    for (k, val) in p {
        obj.insert(k.clone(), val.clone());
    }
    let snap: SettingsSnapshot = serde_json::from_value(v).map_err(|e| e.to_string())?;
    s.apply_snapshot(&snap);
    s.clamp_all();
    s.apply_platform_rules(cfg!(windows));
    Ok(())
}

/// 설정 바꾸기. `scope` = "main"(전역 + 주 화면 테두리) 또는 "border"(편집 중인 화면의 테두리).
#[tauri::command]
pub fn ui_set(app: AppHandle, scope: String, patch_v: Value) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let appearance_changed;
    let auto_update_enabled;
    {
        let st = crate::shared(&app);
        let mut st = st.lock().unwrap();
        let edit = editing_id(&app, &st);
        let prev_lang = st.main.language;
        let prev_auto = crate::update::wants_check(&st.main);
        #[cfg(target_os = "macos")]
        let prev_appearance = st.main.appearance;
        match (scope.as_str(), edit) {
            ("border", Some(id)) => {
                let base = st.main.clone();
                let s = st.displays.entry(id).or_insert(base);
                patch(s, &patch_v)?;
            }
            _ => patch(&mut st.main, &patch_v)?,
        }
        auto_update_enabled = !prev_auto && crate::update::wants_check(&st.main);
        if st.main.language != prev_lang {
            crate::apply_text_locale(st.main.language);
        }
        st.rebuild_snapshots();
        st.save();
        #[cfg(target_os = "macos")]
        {
            appearance_changed = prev_appearance != st.main.appearance;
        }
    }
    #[cfg(target_os = "macos")]
    if appearance_changed {
        crate::windows::refresh_appearance(&app);
    }
    changed(&app);
    if auto_update_enabled { crate::menu::check_update(&app); }
    Ok(())
}

/// 그 밖의 동작.
#[tauri::command]
pub fn ui_action(app: AppHandle, action: String, arg: Value) -> Result<Value, String> {
    let s = |k: &str| arg.get(k).and_then(Value::as_str).map(String::from);
    match action.as_str() {
        "login" | "logout" => {
            let id = s("id").ok_or("id")?;
            let want_in = action == "login";
            if crate::shared(&app).lock().unwrap().is_logged_in(&id) != want_in {
                crate::toggle_account(&app, id, false);
            }
        }
        "refresh" => {
            let app = app.clone();
            std::thread::spawn(move || crate::refresh_all(&app));
        }
        "selectDisplay" => {
            *app.state::<Editing>().0.lock().unwrap() = s("id");
            let _ = app.emit("ui-state", ());
        }
        "displayEnabled" => {
            let id = s("id").ok_or("id")?;
            let on = arg.get("enabled").and_then(Value::as_bool).unwrap_or(true);
            with_state(&app, |st| {
                st.disabled_displays.retain(|d| *d != id);
                if !on {
                    st.disabled_displays.push(id.clone());
                }
            });
        }
        "copyToOthers" => {
            let edit = editing_id(&app, &crate::shared(&app).lock().unwrap());
            let ids: Vec<String> = um_platform::display::displays().into_iter().map(|d| d.id).collect();
            with_state(&app, |st| {
                let src = border_of(st, &edit).clone();
                for id in ids {
                    if st.main_display_id.as_deref() == Some(id.as_str()) {
                        if edit.is_some() {
                            st.main.copy_border_from(&src);
                        }
                    } else if edit.as_deref() != Some(id.as_str()) {
                        let base = st.main.clone();
                        st.displays.entry(id).or_insert(base).copy_border_from(&src);
                    }
                }
            });
        }
        "presetSave" => {
            let name = s("name").unwrap_or_default();
            let edit = editing_id(&app, &crate::shared(&app).lock().unwrap());
            with_state(&app, |st| {
                // 편집 중인 화면이 있으면 전역 값 + 그 화면 테두리를 함께 저장(Swift :258-293).
                let mut base = st.main.clone();
                base.copy_border_from(border_of(st, &edit));
                let lang = st.main.language;
                add_preset(&mut st.presets, crate::menu::new_id(), &name, base.to_snapshot(), |n| um_core::i18n::tn("preset.defaultName", lang, n as i64));
            });
        }
        "presetLoad" => {
            let id = s("id").ok_or("id")?;
            let edit = editing_id(&app, &crate::shared(&app).lock().unwrap());
            with_state(&app, |st| {
                let Some(p) = st.presets.iter().find(|p| p.id == id).cloned() else { return };
                match &edit {
                    // 다른 화면 편집 중이면 그 화면 테두리에만 적용.
                    Some(d) => {
                        let mut tmp = st.main.clone();
                        tmp.apply_snapshot(&p.snapshot);
                        let base = st.main.clone();
                        st.displays.entry(d.clone()).or_insert(base).copy_border_from(&tmp);
                    }
                    None => st.main.apply_snapshot(&p.snapshot),
                }
                st.main.apply_platform_rules(cfg!(windows));
            });
        }
        "presetDelete" => {
            let id = s("id").ok_or("id")?;
            with_state(&app, |st| st.presets.retain(|p| p.id != id));
        }
        "defaultSave" => with_state(&app, |st| st.default_snapshot = Some(st.main.to_snapshot())),
        "defaultReset" => with_state(&app, |st| {
            if let Some(d) = st.default_snapshot.clone() {
                st.main.apply_snapshot(&d);
                st.main.apply_platform_rules(cfg!(windows));
            }
        }),
        // '겹치는 선 없음'을 켤 때 기존 겹침 정리(Swift :613-619). 편집 중인 화면의 테두리에 쓴다.
        "enforceNoOverlap" => {
            let st = crate::shared(&app);
            let st = st.lock().unwrap();
            let target = editing_id(&app, &st).or(st.main_display_id.clone());
            let ds = um_platform::display::displays();
            let side = ds.iter().find(|d| target.as_ref().map_or(d.is_primary, |id| id == &d.id))
                .or_else(|| ds.first()).and_then(|d| d.taskbar).map_or(BorderEdge::Bottom, |t| match t.side {
                    um_platform::display::TaskbarSide::Top => BorderEdge::Top,
                    um_platform::display::TaskbarSide::Bottom => BorderEdge::Bottom,
                    um_platform::display::TaskbarSide::Left => BorderEdge::Left,
                    um_platform::display::TaskbarSide::Right => BorderEdge::Right,
                });
            drop(st);
            let edit = editing_id(&app, &crate::shared(&app).lock().unwrap());
            with_state(&app, |st| {
                let s = match &edit {
                    Some(id) => {
                        let base = st.main.clone();
                        st.displays.entry(id.clone()).or_insert(base)
                    }
                    None => &mut st.main,
                };
                let fixed = enforce_no_overlap(&seg_ctx(s, side));
                s.provider_layouts.extend(fixed);
            });
        }
        // 화면 위에서 메뉴선·Dock선을 끌어 조정하는 전체 화면 창(macOS 전용, Swift LineDragOverlay).
        "dragLines" => crate::windows::open_line_drag(&app),
        "closeLineDrag" => {
            if let Some(w) = app.get_webview_window("linedrag") {
                let _ = w.close();
            }
        }
        "deviceFit" | "detectNotch" => {
            let edit = editing_id(&app, &crate::shared(&app).lock().unwrap());
            let ds = um_platform::display::displays();
            let target = edit.clone().or_else(|| crate::shared(&app).lock().unwrap().main_display_id.clone());
            let d = ds.iter().find(|d| Some(&d.id) == target.as_ref()).or(ds.first()).cloned();
            if let Some(d) = d {
                let info = ScreenInfo { width_pt: d.width_pt, notch: d.notch.map(|n| um_core::settings::NotchInfo { width: n.width, height: n.height }) };
                let notch_only = action == "detectNotch";
                with_state(&app, |st| {
                    let s = match &edit {
                        Some(id) => {
                            let base = st.main.clone();
                            st.displays.entry(id.clone()).or_insert(base)
                        }
                        None => &mut st.main,
                    };
                    if notch_only {
                        // Swift 노치 탭 감지 버튼(SettingsView.swift:696-704): 노치가 있으면 켜고 크기를 채우고, 없으면 끈다.
                        match info.notch {
                            Some(n) => {
                                s.notch_enabled = true;
                                s.notch_width = n.width;
                                s.notch_height = n.height;
                            }
                            None => s.notch_enabled = false,
                        }
                    } else {
                        device_fit(s, &info);
                    }
                });
            }
        }
        "autostart" => {
            use tauri_plugin_autostart::ManagerExt;
            let on = arg.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            let al = app.autolaunch();
            let r = if on { al.enable() } else { al.disable() };
            r.map_err(|e| e.to_string())?;
            let _ = app.emit("ui-state", ());
        }
        "checkUpdate" => crate::menu::on_event(&app, "update:check"),
        "download" => crate::menu::on_event(&app, "update:download"),
        "installUpdate" => crate::menu::on_event(&app, "update:install"),
        "openReleases" => crate::menu::on_event(&app, "update:page"),
        "openMail" => crate::menu::on_event(&app, "report:mail"),
        "copyEmail" => crate::menu::on_event(&app, "report:copy"),
        "openUrl" => {
            let url = s("url").ok_or("url")?;
            if url.starts_with("https://") || url.starts_with("mailto:") {
                let _ = open::that(url);
            }
        }
        // 후원 탭 카카오페이 QR(SVG 문자열, 오류 정정 M). (Swift SupportView.swift CoreImage QR)
        "kakaoQrSvg" => {
            let code = qrcode::QrCode::with_error_correction_level(KAKAO_URL, qrcode::EcLevel::M).map_err(|e| e.to_string())?;
            let svg = code.render::<qrcode::render::svg::Color>().min_dimensions(130, 130).quiet_zone(true).build();
            return Ok(json!({"svg": svg, "url": KAKAO_URL}));
        }
        "openAnalytics" => crate::windows::open(&app, "analytics"),
        #[cfg(target_os = "macos")]
        "pickColor" => {
            let id = s("id").filter(|id| spec::spec(id).is_some()).ok_or("id")?;
            let item = s("item");
            let color = {
                let st = crate::shared(&app);
                let st = st.lock().unwrap();
                if let Some(key) = &item {
                    let (items, _) = st.providers.get(&id).map(|p| p.sub_items()).unwrap_or_default();
                    let it = items.iter().find(|it| um_core::model::UsageSubItem::settings_key(&id, &it.id) == *key).ok_or("item")?;
                    sub_item_color(&st.main, &id, &it.id, &items)
                } else {
                    um_render::border::provider_color(&st.main, &id)
                }
            };
            let handle = app.clone();
            app.run_on_main_thread(move || {
                let alpha = item.is_none();
                crate::native_color::open(color, alpha, move |color| {
                    with_state(&handle, |st| {
                        if let Some(key) = &item {
                            st.main.sub_item_colors.insert(key.clone(), color);
                        } else {
                            st.main.provider_colors.insert(id.clone(), color);
                        }
                    });
                });
            }).map_err(|e| e.to_string())?;
        }
        "openSettings" => crate::windows::open(&app, &s("tab").map(|t| format!("settings#{t}")).unwrap_or_else(|| "settings".into())),
        "lang" => {
            let l = match s("code").as_deref() {
                Some("ko") => AppLanguage::Korean,
                Some("ja") => AppLanguage::Japanese,
                _ => AppLanguage::English,
            };
            crate::apply_text_locale(l);
            with_state(&app, |st| st.main.language = l);
        }
        _ => return Err(format!("unknown action {action}")),
    }
    Ok(Value::Null)
}

fn with_state(app: &AppHandle, f: impl FnOnce(&mut AppState)) {
    #[cfg(target_os = "macos")]
    let appearance_changed;
    let auto_update_enabled;
    {
        let st = crate::shared(app);
        let mut st = st.lock().unwrap();
        #[cfg(target_os = "macos")]
        let prev_appearance = st.main.appearance;
        let prev_auto = crate::update::wants_check(&st.main);
        f(&mut st);
        auto_update_enabled = !prev_auto && crate::update::wants_check(&st.main);
        st.rebuild_snapshots();
        st.save();
        #[cfg(target_os = "macos")]
        {
            appearance_changed = prev_appearance != st.main.appearance;
        }
    }
    #[cfg(target_os = "macos")]
    if appearance_changed {
        crate::windows::refresh_appearance(app);
    }
    changed(app);
    if auto_update_enabled { crate::menu::check_update(app); }
}
