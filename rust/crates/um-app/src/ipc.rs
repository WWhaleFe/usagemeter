//! 설정·분석 창(웹뷰)과 앱 사이의 명령(계획서 5-5). 계산은 Rust가 하고 화면은 결과만 그린다.
//! - `ui_state`: 화면이 그릴 모든 값(설정, 화면 목록, 로그인 상태, 세부 항목, 프리셋, 업데이트, 문자열).
//! - `ui_set`: 설정 일부(스냅샷 JSON 키)를 바꾼다. 범위 = "main"(전역) 또는 "border"(편집 중인 화면의 테두리).
//! - `ui_action`: 그 밖의 동작(로그인, 프리셋, 화면 선택, 노치 감지, 자동 실행 …).
//!
//! 상태가 바뀌면 `ui-state` 이벤트로 모든 창에 알린다.

use serde_json::{Map, Value, json};
use tauri::{AppHandle, Emitter, Manager};
use um_core::hover::seg_ctx;
use um_core::i18n::tr;
use um_core::segments::{can_use_segments, is_valid_segments, seg_available, segs_for};
use um_core::settings::{BRAND_COLORS, MAX_PRESETS, REFRESH_PRESETS, ScreenInfo, Settings, SettingsSnapshot, add_preset, device_fit};
use um_core::types::{SegPart, AppLanguage};
use um_providers::spec;
use um_render::border::sub_item_color;

use crate::state::AppState;

/// 후원 탭 카카오페이 송금 링크(Swift SupportView.swift).
const KAKAO_URL: &str = "https://qr.kakaopay.com/Fa89McYNg";

/// 편집 중인 화면(설정 창 "디스플레이" 선택). None = 주 화면.
#[derive(Default)]
pub struct Editing(pub std::sync::Mutex<Option<String>>);

fn rgba_hex(c: [f64; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", (c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8)
}

/// 편집 대상 화면 id가 주 화면이면 None.
fn editing_id(app: &AppHandle, st: &AppState) -> Option<String> {
    let e = app.state::<Editing>().0.lock().unwrap().clone();
    e.filter(|id| st.main_display_id.as_deref() != Some(id.as_str()))
}

/// 편집 중인 화면의 테두리 설정(주 화면이면 주 설정).
fn border_of<'a>(st: &'a AppState, id: &Option<String>) -> &'a Settings {
    id.as_ref().and_then(|i| st.displays.get(i)).unwrap_or(&st.main)
}

fn snapshot_value(s: &Settings) -> Value {
    serde_json::to_value(s.to_snapshot()).unwrap_or(Value::Null)
}

/// 화면이 그릴 값 전부.
#[tauri::command]
pub fn ui_state(app: AppHandle) -> Value {
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
                    let kind = tr(if it.is_five_hour { "menu.line5h" } else { "menu.lineWeekly" }, lang);
                    let name = it.model_label.as_ref().map(|l| format!("{l} {kind}")).unwrap_or(kind);
                    json!({
                        "id": it.id, "name": name, "ratio": it.remaining_ratio,
                        "color": rgba_hex(sub_item_color(&st.main, id, &it.id, &items)),
                        "key": um_core::model::UsageSubItem::settings_key(id, &it.id),
                    })
                })
                .collect();
            Some(json!({
                "id": id, "name": sp.name, "loggedIn": ps.logged_in, "experimental": sp.id == "gemini",
                "color": rgba_hex(um_render::border::provider_color(&st.main, id)),
                "items": items_v, "itemsLive": live,
                "segOptions": seg_options(border, id),
            }))
        })
        .collect();
    let strings: Map<String, Value> = um_core::i18n::keys().map(|k| (k.to_string(), Value::String(tr(k, lang)))).collect();
    json!({
        "platform": if cfg!(target_os = "macos") { "mac" } else { "win" },
        "store": cfg!(feature = "store"),
        "version": crate::update::current_version(),
        "lang": lang.code(),
        "strings": strings,
        "main": snapshot_value(&st.main),
        "border": snapshot_value(border),
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
    json!({"state": state, "version": version, "download": dl})
}

fn autostart_value(app: &AppHandle) -> Value {
    use tauri_plugin_autostart::ManagerExt;
    // 번들(.app·설치본)에서만 동작한다(Swift: 번들 id가 없으면 비활성).
    let available = std::env::current_exe().map(|p| p.to_string_lossy().contains(".app/") || cfg!(windows)).unwrap_or(false);
    json!({"available": available, "enabled": available && app.autolaunch().is_enabled().unwrap_or(false)})
}

/// 레이아웃 탭 다이어그램용: 세그먼트마다 (쓸 수 있는지, 눌렀을 때 결과가 유효한지, 다른 AI가 쓰는지).
fn seg_options(s: &Settings, id: &str) -> Value {
    let ctx = seg_ctx(s);
    let cur = segs_for(&ctx, id);
    let mut out = Map::new();
    for seg in SegPart::ALL {
        let available = seg_available(seg, s.menu_line_enabled, s.dock_line_enabled);
        let mut toggled = cur.clone();
        if !toggled.remove(&seg) {
            toggled.insert(seg);
        }
        let valid = available && is_valid_segments(&toggled, s.menu_line_enabled, s.dock_line_enabled);
        let allowed = valid && can_use_segments(&ctx, id, &toggled);
        let used_by_other = um_core::segments::PROVIDER_IDS.iter().any(|o| *o != id && segs_for(&ctx, o).contains(&seg));
        out.insert(seg.raw().into(), json!({"on": cur.contains(&seg), "available": available, "toggleValid": valid, "toggleAllowed": allowed, "usedByOther": used_by_other}));
    }
    Value::Object(out)
}

/// 상태가 바뀌었음을 모든 창에 알리고 메뉴·테두리를 갱신한다.
pub fn changed(app: &AppHandle) {
    let _ = app.emit("ui-state", ());
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
    {
        let st = crate::shared(&app);
        let mut st = st.lock().unwrap();
        let edit = editing_id(&app, &st);
        let prev_lang = st.main.language;
        match (scope.as_str(), edit) {
            ("border", Some(id)) => {
                let base = st.main.clone();
                let s = st.displays.entry(id).or_insert(base);
                patch(s, &patch_v)?;
            }
            _ => patch(&mut st.main, &patch_v)?,
        }
        if st.main.language != prev_lang {
            crate::apply_text_locale(st.main.language);
        }
        st.rebuild_snapshots();
        st.save();
    }
    changed(&app);
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
                crate::toggle_account(&app, id);
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
                        // Swift detectNotch(:1104-1114): 노치 크기만 다시 읽는다.
                        if let Some(n) = info.notch {
                            s.notch_width = n.width;
                            s.notch_height = n.height;
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
    {
        let st = crate::shared(app);
        let mut st = st.lock().unwrap();
        f(&mut st);
        st.rebuild_snapshots();
        st.save();
    }
    changed(app);
}
