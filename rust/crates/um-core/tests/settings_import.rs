//! Swift UserDefaults plist 가져오기와 설정 왕복 검증. 고정 자료: tests/fixtures/swift_defaults.plist
//! (화면 UUID는 익명화됨).

use std::path::PathBuf;

use um_core::settings::{
    NotchInfo, ScreenInfo, Settings, SettingsSnapshot, adapt_to_screen_change, device_fit, import_from_plist,
    snapshot_from_json, snapshot_to_json,
};
use um_core::types::{BorderCorner, BorderEdge, NotchCorner, SegCap, SegPart};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/swift_defaults.plist")
}

fn load() -> um_core::settings::SwiftImport {
    import_from_plist(&fixture()).expect("가져오기")
}

/// 숫자는 f64로 정규화해 비교(Swift의 `9`와 Rust의 `9.0` 차이 무시), 문자열 배열(Set)은 정렬해 비교.
fn norm(v: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Number(n) => Value::from(n.as_f64().unwrap()),
        Value::Array(a) => {
            let mut items: Vec<Value> = a.iter().map(norm).collect();
            if items.iter().all(Value::is_string) {
                items.sort_by_key(|x| x.to_string());
            }
            Value::Array(items)
        }
        Value::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), norm(v))).collect()),
        other => other.clone(),
    }
}

#[test]
fn 가져오기_최상위_값() {
    let imp = load();
    assert_eq!(imp.settings_version, Some(11));
    assert_eq!(imp.main_display_uuid.as_deref(), Some("722DFB6C-40D6-4779-B007-1D5B00798468"));
    assert_eq!(imp.last_notified_version.as_deref(), Some("1.2.8"));
    assert!(imp.disabled_displays.is_empty());
    assert!(imp.presets.is_empty());
    assert!(imp.default_settings.is_none());
    assert!(imp.warnings.is_empty(), "{:?}", imp.warnings);
    assert!(imp.usable_current().is_some());
}

#[test]
fn 현재_설정_값() {
    let imp = load();
    let cur = imp.current.expect("currentSettings");
    assert_eq!(cur.thickness, 3.5);
    assert_eq!(cur.fade_fraction, Some(0.03));
    assert_eq!(cur.fade_enabled, Some(false));
    assert_eq!(cur.hover_corner_size, Some(50.0));
    assert_eq!(cur.language.as_deref(), Some("ko"));
    for v in cur.notch_radii.values() {
        assert_eq!(*v, 9.0);
    }
    assert_eq!(cur.notch_radii.len(), 4);

    let s = Settings::from_snapshot(&cur);
    assert_eq!(s.thickness, 3.5);
    assert_eq!(s.fade_fraction, 0.03);
    assert!(!s.fade_enabled);
    assert!(!s.notch_enabled);
    for c in NotchCorner::ALL {
        assert_eq!(s.notch_radius(c), 9.0);
    }
    assert_eq!(s.hover_corners, [BorderCorner::BottomRight, BorderCorner::BottomLeft].into());
    assert_eq!(s.hover_edges, [BorderEdge::Right, BorderEdge::Left, BorderEdge::Bottom].into());
    assert_eq!(s.hover_corner_size, 50.0);
    assert_eq!(s.menu_line_height, 33.0);
    assert_eq!(s.dock_line_height, 51.0);
    assert_eq!(s.notify_thresholds, [75, 90, 95].into());
    assert_eq!(s.menu_bar_percent_ids.len(), 3);
    assert_eq!(s.sub_item_split_ids.len(), 3);
    assert_eq!(s.seg_parts, [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain].into());
    assert_eq!(s.seg_cap_start, SegCap::None);
    assert_eq!(s.provider_order, vec!["claude", "gemini", "codex"]);
    assert!(!s.provider_layouts["claude"].clockwise);
    assert_eq!(s.provider_layouts["claude"].zones.as_ref().map(|z| z.len()), Some(3));
    assert_eq!(s.provider_layouts["claude"].h_caps.as_ref().map(|m| m.len()), Some(0));
    assert_eq!(s.provider_colors["codex"], [0.06000000238418579, 0.6399999856948853, 0.5, 1.0]);
}

#[test]
fn 디스플레이_4개_모두_해석() {
    let imp = load();
    assert_eq!(imp.displays.len(), 4);
    for (uuid, snap) in &imp.displays {
        assert!(uuid.starts_with("00000000-0000-0000-0000-00000000000"), "{uuid}");
        let s = Settings::from_snapshot(snap);
        assert!(s.thickness > 0.0);
        assert_eq!(s.provider_order.len(), 3);
        assert!(!s.seg_parts.is_empty());
    }
}

#[test]
fn 세부_항목_기억() {
    let imp = load();
    let k = &imp.known_sub_items;
    assert_eq!(k.len(), 3);
    assert_eq!(k["codex"].len(), 2);
    assert_eq!(k["codex"][0].id, "5h");
    assert_eq!(k["codex"][0].ratio, 0.96);
    assert!(k["gemini"].iter().any(|i| i.id == "model:Claude\u{b7}GPT (agy):weekly"));
}

/// 스냅샷을 JSON으로 다시 쓰면 Swift가 쓴 키가 하나도 빠지거나 늘지 않아야 한다.
#[test]
fn 스냅샷_키_이름_일치() {
    use plist::Value;
    let dict = Value::from_file(fixture()).unwrap().into_dictionary().unwrap();
    let mut checked = 0;
    for (key, v) in &dict {
        if !(key.starts_with("usagemeter.display.") || key == "usagemeter.currentSettings") {
            continue;
        }
        let raw: serde_json::Value = serde_json::from_slice(v.as_data().unwrap()).unwrap();
        let snap: SettingsSnapshot = serde_json::from_value(raw.clone()).unwrap();
        let back = serde_json::to_value(&snap).unwrap();
        assert_eq!(norm(&raw), norm(&back), "{key}");
        checked += 1;
    }
    assert_eq!(checked, 5);
}

#[test]
fn 왕복_안정() {
    let imp = load();
    let all = imp.displays.values().chain(imp.current.iter());
    for snap in all {
        let s1 = Settings::from_snapshot(snap);
        let snap2 = s1.to_snapshot();
        let json = snapshot_to_json(&snap2).unwrap();
        let snap3 = snapshot_from_json(&json).unwrap();
        assert_eq!(snap2, snap3);
        let s2 = Settings::from_snapshot(&snap3);
        assert_eq!(s1, s2);
        assert_eq!(s2.to_snapshot(), snap2);
        // Settings 자체의 JSON 왕복
        assert_eq!(Settings::from_json(&s1.to_json().unwrap()).unwrap(), s1);
    }
}

#[test]
fn 윈도우_규칙은_메뉴_dock_세그먼트를_없앤다() {
    let imp = load();
    let mut s = Settings::from_snapshot(imp.current.as_ref().unwrap());
    s.menu_line_enabled = true;
    s.dock_line_enabled = true;
    s.notch_enabled = true;
    s.provider_layouts.get_mut("claude").unwrap().segments =
        Some([SegPart::LMenuBar, SegPart::HMenu, SegPart::RMenuBar, SegPart::HTop].into());
    s.provider_layouts.get_mut("codex").unwrap().segments = Some([SegPart::LDock].into());
    s.apply_platform_rules(true);
    assert!(!s.menu_line_enabled && !s.dock_line_enabled && !s.notch_enabled);
    assert_eq!(s.segs_for("claude"), [SegPart::HTop].into());
    // codex는 세 번째 AI이므로 기본 lMain으로 복귀
    assert_eq!(s.segs_for("codex"), [SegPart::LMain].into());
    // gemini는 이미 4개 둘레라 그대로
    assert_eq!(s.segs_for("gemini").len(), 4);
    let allowed = [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain];
    for l in s.provider_layouts.values() {
        assert!(l.segments.as_ref().unwrap().iter().all(|p| allowed.contains(p)));
    }
}

#[test]
fn 장치_맞춤_공식() {
    let imp = load();
    let base = Settings::from_snapshot(imp.current.as_ref().unwrap());
    let notch = NotchInfo { width: 185.0, height: 32.0 };

    let mut s = base.clone();
    device_fit(&mut s, &ScreenInfo { width_pt: 1512.0, notch: Some(notch) });
    assert!(s.notch_enabled);
    assert_eq!((s.notch_width, s.notch_height), (185.0, 32.0));
    assert_eq!(s.radius(BorderCorner::TopLeft), 19.0);
    assert_eq!(s.radius(BorderCorner::TopRight), 19.0);
    assert_eq!(s.menu_zone_radii[&BorderCorner::TopLeft], 19.0);
    // 아래 모서리는 그대로
    assert_eq!(s.radius(BorderCorner::BottomLeft), base.radius(BorderCorner::BottomLeft));

    let mut t = Settings::from_snapshot(imp.current.as_ref().unwrap());
    t.notch_enabled = true;
    device_fit(&mut t, &ScreenInfo { width_pt: 3008.0, notch: None });
    assert!(!t.notch_enabled);
    assert_eq!(t.corner_radii, base.corner_radii);

    let mut u = base.clone();
    adapt_to_screen_change(&mut u, 1512.0, 3008.0, None);
    // 3008/1512 = 1.99 -> 0.5~2 안, 본문 곡률 0은 그대로 0, 노치 곡률 9 x 1.99 = 17.9 -> 18
    assert_eq!(u.radius(BorderCorner::TopLeft), 0.0);
    assert_eq!(u.dock_zone_radii[&BorderCorner::TopLeft], 18.0);
    assert_eq!(u.notch_radius(NotchCorner::OuterLeft), 18.0);
}
