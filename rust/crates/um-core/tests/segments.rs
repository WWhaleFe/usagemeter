//! Swift 기준값(`fixtures/segments_ref.jsonl`, 생성기 `rust/tools/swift-ref/segments_ref.swift`)과 전수 비교.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use um_core::segments::*;
use um_core::types::*;

fn lines() -> Vec<Value> {
    let text = include_str!("fixtures/segments_ref.jsonl");
    text.lines().map(|l| serde_json::from_str(l).expect("fixture json")).collect()
}

fn of_kind(kind: &str) -> Vec<Value> {
    lines().into_iter().filter(|v| v["kind"] == kind).collect()
}

fn seg_from(name: &str) -> SegPart {
    SegPart::ALL.into_iter().find(|s| s.raw() == name).unwrap_or_else(|| panic!("seg {name}"))
}
fn segs_from(v: &Value) -> SegSet {
    v.as_array().unwrap().iter().map(|x| seg_from(x.as_str().unwrap())).collect()
}
fn mask_set(m: u64) -> SegSet {
    SegPart::ALL.iter().enumerate().filter(|(i, _)| (m >> i) & 1 == 1).map(|(_, s)| *s).collect()
}
fn zone_from(name: &str) -> ScreenZone {
    ScreenZone::ALL.into_iter().find(|z| format!("{z:?}").to_lowercase() == name.to_lowercase()).unwrap()
}
fn zones_mask(m: u64) -> BTreeSet<ScreenZone> {
    ScreenZone::ALL.iter().enumerate().filter(|(i, _)| (m >> i) & 1 == 1).map(|(_, z)| *z).collect()
}
fn edges_mask(m: u64) -> BTreeSet<BorderEdge> {
    BorderEdge::ALL.iter().enumerate().filter(|(i, _)| (m >> i) & 1 == 1).map(|(_, e)| *e).collect()
}
fn edge_from(name: &str) -> BorderEdge {
    BorderEdge::ALL.into_iter().find(|e| format!("{e:?}").to_lowercase() == name).unwrap()
}
fn cap_from(name: &str) -> SegCap {
    match name {
        "none" => SegCap::None,
        "up" => SegCap::Up,
        "down" => SegCap::Down,
        "include" => SegCap::Include,
        _ => panic!("cap {name}"),
    }
}
fn node_from(v: &Value) -> SegNode {
    SegNode { right: v[0].as_bool().unwrap(), level: v[1].as_u64().unwrap() as u8 }
}
fn radii_from(v: &Value) -> BTreeMap<BorderCorner, f64> {
    BorderCorner::ALL
        .into_iter()
        .map(|c| {
            let key = match c {
                BorderCorner::TopLeft => "topLeft",
                BorderCorner::TopRight => "topRight",
                BorderCorner::BottomRight => "bottomRight",
                BorderCorner::BottomLeft => "bottomLeft",
            };
            (c, v[key].as_f64().unwrap())
        })
        .collect()
}

#[test]
fn valid_and_chain_ends_match_swift_for_all_subsets() {
    let rows = of_kind("valid");
    assert_eq!(rows.len(), 4 * 1024);
    let mut valid_count = 0;
    for r in &rows {
        let (menu, dock) = (r["menuOn"].as_bool().unwrap(), r["dockOn"].as_bool().unwrap());
        let set = mask_set(r["mask"].as_u64().unwrap());
        let ctx = format!("mask={} menu={menu} dock={dock}", r["mask"]);
        let valid = is_valid_segments(&set, menu, dock);
        assert_eq!(valid, r["valid"].as_bool().unwrap(), "valid {ctx}");
        valid_count += valid as usize;
        let got = chain_ends(&set, menu, dock);
        if r["chain"].is_null() {
            assert!(got.is_none(), "chain {ctx}");
        } else {
            let c = got.unwrap_or_else(|| panic!("chain none {ctx}"));
            assert_eq!(c.start.node, node_from(&r["chain"]["start"]["node"]), "start node {ctx}");
            assert_eq!(c.start.seg, seg_from(r["chain"]["start"]["seg"].as_str().unwrap()), "start seg {ctx}");
            assert_eq!(c.end.node, node_from(&r["chain"]["end"]["node"]), "end node {ctx}");
            assert_eq!(c.end.seg, seg_from(r["chain"]["end"]["seg"].as_str().unwrap()), "end seg {ctx}");
        }
        if !r["segEnds"].is_null() {
            for (i, e) in r["segEnds"].as_array().unwrap().iter().enumerate() {
                let s = SegPart::ALL[i];
                let (a, b) = seg_ends(s, menu, dock);
                assert_eq!((a, b), (node_from(&e[0]), node_from(&e[1])), "segEnds {s:?} {ctx}");
                assert_eq!(seg_available(s, menu, dock), e[2].as_bool().unwrap(), "avail {s:?} {ctx}");
            }
        }
    }
    assert!(valid_count > 100, "유효 조합이 너무 적다: {valid_count}");
}

#[test]
fn zone_perimeter_matches_swift() {
    let rows = of_kind("perimeter");
    assert_eq!(rows.len(), 4 * 8);
    for r in &rows {
        let got = zone_perimeter(
            &zones_mask(r["zones"].as_u64().unwrap()),
            r["menuOn"].as_bool().unwrap(),
            r["dockOn"].as_bool().unwrap(),
        );
        assert_eq!(got, segs_from(&r["segs"]), "{r}");
    }
}

#[test]
fn migrate_to_segments_matches_swift() {
    let rows = of_kind("migrate");
    assert_eq!(rows.len(), 4 * 8 * 16);
    for r in &rows {
        let got = migrate_to_segments(
            &edges_mask(r["edges"].as_u64().unwrap()),
            &zones_mask(r["zones"].as_u64().unwrap()),
            r["menuOn"].as_bool().unwrap(),
            r["dockOn"].as_bool().unwrap(),
        );
        assert_eq!(got, segs_from(&r["segs"]), "{r}");
    }
}

fn layout_from(p: &Value) -> Option<ProviderLayout> {
    if !p["present"].as_bool().unwrap() {
        return None;
    }
    let opt_set = |k: &str| (!p[k].is_null()).then(|| p[k].as_array().unwrap().clone());
    Some(ProviderLayout {
        segment_geometry: None,
        edges: opt_set("edges").map(|a| a.iter().map(|x| edge_from(x.as_str().unwrap())).collect()),
        anchor_corner: BorderCorner::TopLeft,
        anchor_side: AnchorSide::Center,
        clockwise: true,
        extend_start: None,
        extend_end: None,
        no_curve_extend: None,
        zones: opt_set("zones").map(|a| a.iter().map(|x| zone_from(x.as_str().unwrap())).collect()),
        segments: (!p["segments"].is_null()).then(|| segs_from(&p["segments"])),
        cap_start: None,
        cap_end: None,
        h_caps: (!p["hCaps"].is_null()).then(|| {
            p["hCaps"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), cap_from(v.as_str().unwrap()))).collect()
        }),
    })
}

#[test]
fn multi_ai_scenarios_match_swift() {
    let rows = of_kind("scenario");
    assert_eq!(rows.len(), 400);
    let nodes: Vec<SegNode> =
        (0..4u8).flat_map(|level| [false, true].map(move |right| SegNode { right, level })).collect();
    let hsegs = [SegPart::HTop, SegPart::HMenu, SegPart::HDock, SegPart::HBottom];
    let vsegs =
        [SegPart::LMenuBar, SegPart::LMain, SegPart::LDock, SegPart::RMenuBar, SegPart::RMain, SegPart::RDock];
    let (mut half_seen, mut auto_seen, mut overlap_blocked, mut enforced_changed) = (0, 0, 0, 0);

    for (n, r) in rows.iter().enumerate() {
        let mut layouts = BTreeMap::new();
        for (i, id) in PROVIDER_IDS.iter().enumerate() {
            if let Some(l) = layout_from(&r["providers"][i]) {
                layouts.insert(id.to_string(), l);
            }
        }
        let (main, menu, dock) = (radii_from(&r["main"]), radii_from(&r["menu"]), radii_from(&r["dock"]));
        let ctx = SegContext {
            side: um_core::types::BorderEdge::Bottom,
            menu_on: r["menuOn"].as_bool().unwrap(),
            dock_on: r["dockOn"].as_bool().unwrap(),
            no_overlap: r["noOverlap"].as_bool().unwrap(),
            layouts: &layouts,
            provider_ids: &PROVIDER_IDS,
            main_radii: &main,
            menu_radii: &menu,
            dock_radii: &dock,
        };

        for (i, id) in PROVIDER_IDS.iter().enumerate() {
            assert_eq!(segs_for(&ctx, id), segs_from(&r["segs"][i]), "segs #{n} {id}");
            // canUseSegments
            for (j, m) in r["cands"].as_array().unwrap().iter().enumerate() {
                let want = r["canUse"][i][j].as_bool().unwrap();
                let got = can_use_segments(&ctx, id, &mask_set(m.as_u64().unwrap()));
                assert_eq!(got, want, "canUse #{n} {id} cand {m}");
                if ctx.no_overlap && !want && is_valid_segments(&mask_set(m.as_u64().unwrap()), ctx.menu_on, ctx.dock_on) {
                    overlap_blocked += 1;
                }
            }
            // endOverride
            let mut k = 0;
            for &nd in &nodes {
                for s in SegPart::ALL {
                    let code = r["endOverride"][i][k].as_u64().unwrap();
                    k += 1;
                    let (half, auto) = end_override(&ctx, id, nd, s);
                    assert_eq!((half, auto), (code & 1 == 1, code & 2 == 2), "endOverride #{n} {id} {nd:?} {s:?}");
                    half_seen += half as usize;
                    auto_seen += auto as usize;
                }
            }
            // bendRadiusEstimate
            let mut k = 0;
            for &h in &hsegs {
                for &v in &vsegs {
                    for &nd in &nodes {
                        let want = r["bend"][i * 4 * 6 * 8 + k].as_f64().unwrap();
                        k += 1;
                        let got = bend_radius_estimate(&ctx, id, h, v, nd);
                        assert_eq!(got, want, "bend #{n} {id} {h:?} {v:?} {nd:?}");
                    }
                }
            }
        }
        // cornerUsed
        let mut k = 0;
        for z in ScreenZone::ALL {
            for c in BorderCorner::ALL {
                assert_eq!(corner_used(&ctx, z, c), r["cornerUsed"][k].as_bool().unwrap(), "cornerUsed #{n} {z:?} {c:?}");
                k += 1;
            }
        }
        // edgeActive
        for (k, e) in BorderEdge::ALL.into_iter().enumerate() {
            assert_eq!(edge_active(&ctx, e), r["edgeActive"][k].as_bool().unwrap(), "edgeActive #{n} {e:?}");
        }
        // enforceNoOverlap
        let enforced = enforce_no_overlap(&ctx);
        for (i, id) in PROVIDER_IDS.iter().enumerate() {
            let want = segs_from(&r["enforced"][i]);
            assert_eq!(enforced[*id].segments.clone().unwrap(), want, "enforce #{n} {id}");
            enforced_changed += (want != segs_for(&ctx, id)) as usize;
        }
    }
    // 시나리오가 실제로 분기를 밟았는지(공허한 통과 방지).
    assert!(half_seen > 50 && auto_seen > 50, "half={half_seen} auto={auto_seen}");
    assert!(overlap_blocked > 20, "overlap_blocked={overlap_blocked}");
    assert!(enforced_changed > 50, "enforced_changed={enforced_changed}");
}

#[test]
fn h_cap_keys_and_lookup() {
    assert_eq!(h_cap_key(SegPart::HTop, false), "hTop.L");
    assert_eq!(h_cap_key(SegPart::HMenu, true), "hMenu.R");
    assert_eq!(h_cap_key(SegPart::HDock, false), "hDock.L");
    assert_eq!(h_cap_key(SegPart::HBottom, true), "hBottom.R");

    let mut layouts = default_layouts();
    layouts.get_mut("claude").unwrap().h_caps =
        Some([("hTop.R".to_string(), SegCap::Down)].into_iter().collect());
    let radii = BTreeMap::new();
    let ctx = SegContext {
        side: um_core::types::BorderEdge::Bottom,
        menu_on: false,
        dock_on: false,
        no_overlap: false,
        layouts: &layouts,
        provider_ids: &PROVIDER_IDS,
        main_radii: &radii,
        menu_radii: &radii,
        dock_radii: &radii,
    };
    assert_eq!(h_cap(&ctx, "claude", SegPart::HTop, true), SegCap::Down);
    assert_eq!(h_cap(&ctx, "claude", SegPart::HTop, false), SegCap::None);
    assert_eq!(h_cap(&ctx, "gemini", SegPart::HTop, true), SegCap::None);
    // 기본 레이아웃: claude=hTop, codex=lMain. claude의 hTop 오른쪽 끝 캡 down + codex 세로선 -> 반쪽.
    let mut layouts2 = layouts.clone();
    layouts2.get_mut("codex").unwrap().segments = Some([SegPart::RMain].into_iter().collect());
    let ctx2 = SegContext { layouts: &layouts2, ..ctx };
    let top_right = SegNode { right: true, level: 0 };
    assert_eq!(end_override(&ctx2, "claude", top_right, SegPart::HTop), (true, false));
    assert_eq!(end_override(&ctx2, "codex", top_right, SegPart::RMain), (true, true));
}

#[test]
fn provider_ids_order() {
    assert_eq!(PROVIDER_IDS, ["claude", "gemini", "codex"]);
}
