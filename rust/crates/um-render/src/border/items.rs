//! AI별 그리기 항목 계산. Swift `BorderView.makeShape`/`drawItems`(BorderView.swift:591-677)와
//! 세부 항목 색·띠 규칙(OverlaySettings.swift:884-946).

use std::collections::{BTreeMap, BTreeSet};

use um_core::model::{UsageSnapshot, UsageSubItem};
use um_core::segments::{PROVIDER_IDS, SegContext, chain_ends_on, end_override, seg_ends_on, segs_for};
use um_core::settings::{Settings, default_provider_color};
use um_core::types::{Rgba, SegNode, SegPart};

use super::shape::ChainShape;
use super::{BandPart, ProviderBand};

/// 한 AI의 그리기 항목: 모양(레인·구간 굵기 포함) + 잔여율 + 색 + 분할 여부.
pub(crate) struct DrawItem {
    pub id: String,
    pub shape: ChainShape,
    pub ratio: f64,
    pub color: Rgba,
    /// 구간 굵기(widthSpans) 사용 여부
    pub split: bool,
    /// 세부 항목별 색 띠. 비어 있으면 ratio·color로 한 줄.
    pub parts: Vec<BandPart>,
}

/// 세그먼트 그래프 함수에 넘길 설정 묶음.
fn seg_ctx(s: &Settings, side: um_core::types::BorderEdge) -> SegContext<'_> {
    SegContext {
        side,
        menu_on: s.menu_line_enabled,
        dock_on: s.dock_line_enabled,
        no_overlap: s.no_overlap_lines,
        layouts: &s.provider_layouts,
        provider_ids: &PROVIDER_IDS,
        main_radii: &s.corner_radii,
        menu_radii: &s.menu_zone_radii,
        dock_radii: &s.dock_zone_radii,
    }
}

/// Swift `makeShape(for:laneOffsets:laneWidths:nodeWidths:notchDetour:)`.
fn make_shape(
    s: &Settings,
    id: &str,
    side: um_core::types::BorderEdge,
    lane_offsets: BTreeMap<SegPart, f64>,
    lane_widths: BTreeMap<SegPart, f64>,
    node_widths: BTreeMap<SegNode, f64>,
    notch_detour: bool,
) -> ChainShape {
    let l = um_core::segments::positioned_layout(&seg_ctx(s, side), id);
    let segs = segs_for(&seg_ctx(s, side), id);
    let (mut a_half, mut a_auto, mut b_half, mut b_auto) = (false, false, false, false);
    if let Some(ends) = chain_ends_on(&segs, s.menu_line_enabled, s.dock_line_enabled, side) {
        let ctx = seg_ctx(s, side);
        (a_half, a_auto) = end_override(&ctx, id, ends.start.node, ends.start.seg);
        (b_half, b_auto) = end_override(&ctx, id, ends.end.node, ends.end.seg);
    }
    ChainShape {
        segments: segs,
        menu_on: s.menu_line_enabled,
        dock_on: s.dock_line_enabled,
        menu_h: s.menu_line_height,
        dock_h: s.dock_line_height,
        inset: s.thickness / 2.0,
        menu_radii: s.menu_zone_radii.clone(),
        main_radii: s.corner_radii.clone(),
        dock_radii: s.dock_zone_radii.clone(),
        notch_enabled: s.notch_enabled,
        notch_width: s.notch_width,
        notch_height: s.notch_height,
        notch_radii: s.notch_radii.clone(),
        anchor_corner: l.anchor_corner,
        anchor_side: l.anchor_side,
        clockwise: l.clockwise,
        h_caps: l.h_caps.unwrap_or_default(),
        end_a_half: a_half,
        end_a_auto: a_auto,
        end_b_half: b_half,
        end_b_auto: b_auto,
        lane_offsets,
        lane_widths,
        full_width: s.thickness,
        notch_detour,
        node_widths,
    }
}

/// 스펙 순서(`ProviderSpec.all`, 모르는 id는 뒤)로 정렬한 켜진 AI 목록.
fn spec_order(bands: &[ProviderBand]) -> Vec<&ProviderBand> {
    let mut v: Vec<&ProviderBand> = bands.iter().collect();
    v.sort_by_key(|b| PROVIDER_IDS.iter().position(|p| *p == b.id).unwrap_or(usize::MAX));
    v
}

/// 그릴 AI 목록(뒤 -> 앞 순서). 겹침 분할 레인/굵기를 여기서 계산한다. (Swift `drawItems`)
pub(crate) fn draw_items(s: &Settings, bands: &[ProviderBand], side: um_core::types::BorderEdge) -> Vec<DrawItem> {
    let t = s.thickness;
    let active = spec_order(bands);
    // 세그먼트별 공유자 목록(활성 AI만, 스펙 순서로 고정 -> 레인 배정 결정적).
    let mut sharers: BTreeMap<SegPart, Vec<&str>> = BTreeMap::new();
    for b in &active {
        for seg in segs_for(&seg_ctx(s, side), &b.id) {
            sharers.entry(seg).or_default().push(&b.id);
        }
    }
    // 상단선이 분할될 때 노치 우회 소유자: 상단선 공유자 중 잔여율 최소(최상단 레이어). 동률이면 앞쪽.
    let mut notch_owner: Option<&ProviderBand> = None;
    for b in active.iter().filter(|b| segs_for(&seg_ctx(s, side), &b.id).contains(&SegPart::HTop)) {
        if notch_owner.is_none_or(|o| b.ratio < o.ratio) {
            notch_owner = Some(b);
        }
    }
    let notch_owner = notch_owner.map(|b| b.id.as_str());

    // 노드(모서리)별로 몇 개의 AI가 지나가는지: 스쿱 오버슛·아크 분할용.
    let (menu_on, dock_on) = (s.menu_line_enabled, s.dock_line_enabled);
    let mut node_ais: BTreeMap<SegNode, BTreeSet<&str>> = BTreeMap::new();
    for b in &active {
        for seg in segs_for(&seg_ctx(s, side), &b.id) {
            let (a, c) = seg_ends_on(seg, menu_on, dock_on, side);
            node_ais.entry(a).or_default().insert(&b.id);
            node_ais.entry(c).or_default().insert(&b.id);
        }
    }
    let node_width_map: BTreeMap<SegNode, f64> =
        node_ais.iter().filter(|(_, ids)| ids.len() > 1).map(|(n, ids)| (*n, t / ids.len() as f64)).collect();

    // 잔여율 높은 띠부터(뒤) -> %가 가장 적은 띠가 최상단. 안정 정렬.
    let mut ordered = active.clone();
    ordered.sort_by(|a, b| b.ratio.partial_cmp(&a.ratio).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = vec![];
    for item in ordered {
        let my_id = item.id.as_str();
        let my_segs = segs_for(&seg_ctx(s, side), my_id);
        let mut lanes: BTreeMap<SegPart, f64> = BTreeMap::new();
        let mut widths: BTreeMap<SegPart, f64> = BTreeMap::new();
        for seg in &my_segs {
            let sh: Vec<&str> = sharers.get(seg).cloned().unwrap_or_else(|| vec![my_id]);
            let k = sh.len();
            // 분할 조건: '겹침 분할' 옵션을 켰을 때만 레인으로 나눈다. 기본은 겹쳐 쌓는다(잔여율 낮은 띠가 위).
            if k > 1
                && s.split_overlap_lines
                && let Some(i) = sh.iter().position(|x| *x == my_id)
            {
                lanes.insert(*seg, (i as f64 - (k as f64 - 1.0) / 2.0) * (t / k as f64));
                widths.insert(*seg, t / k as f64);
            }
        }
        let h_top_split = lanes.contains_key(&SegPart::HTop);
        // 노드 분할 굵기는 겹침 분할이 실제로 동작 중일 때만(내가 분할에 참여할 때) 적용.
        let my_node_widths = if lanes.is_empty() { BTreeMap::new() } else { node_width_map.clone() };
        let split = !lanes.is_empty() || !my_node_widths.is_empty();
        let shape = make_shape(s, my_id, side, lanes, widths, my_node_widths, !h_top_split || Some(my_id) == notch_owner);
        out.push(DrawItem {
            id: my_id.to_string(),
            shape,
            ratio: item.ratio,
            color: item.color,
            split,
            parts: item.parts.clone(),
        });
    }
    out
}

// ---- 색·세부 항목 (OverlaySettings.swift:884-946) ----

/// AI 색. 없으면 기본 색, 그것도 없으면 전역 선 색. (Swift `color(forProvider:)`)
/// 디스플레이별 설정이면 호출 쪽이 주 설정의 값을 넘겨야 한다(Swift는 `parent`를 따라간다).
pub fn provider_color(s: &Settings, id: &str) -> Rgba {
    s.provider_colors.get(id).copied().or_else(|| default_provider_color(id)).unwrap_or(s.color)
}

/// 세부 항목 기본 색. 첫 항목은 AI 색, 이후는 AI마다 따로 둔 4색 묶음.
/// Swift `subItemPalette`는 AI마다 같은 5색을 처음부터 다시 써서 AI끼리 같은 색이 겹쳤다.
/// 12색은 OKLCH 후보에서 서로(그리고 AI 기본 색 3개와) OKLab ΔE가 가장 멀어지게 골랐다(전체 최소 10.8, 묶음 안 최소 16.5).
/// 묶음 안 순서는 앞 항목일수록 그 AI 색과 앞서 쓴 색에서 먼 색(자주 쓰는 앞 항목끼리 가장 잘 구분되게).
const SUB_ITEM_PALETTES: [(&str, [Rgba; 4]); 3] = [
    ("claude", [[0.30, 0.84, 0.46, 1.0], [0.69, 0.50, 0.99, 1.0], [0.22, 0.57, 0.72, 1.0], [0.94, 0.74, 0.98, 1.0]]),
    ("gemini", [[0.85, 0.86, 0.15, 1.0], [1.00, 0.56, 0.70, 1.0], [0.17, 0.93, 0.96, 1.0], [0.64, 0.65, 0.16, 1.0]]),
    ("codex", [[0.75, 0.34, 0.72, 1.0], [0.94, 0.65, 0.27, 1.0], [0.52, 0.74, 0.96, 1.0], [0.63, 0.89, 0.68, 1.0]]),
];

fn sub_item_palette(provider_id: &str) -> [Rgba; 4] {
    SUB_ITEM_PALETTES.iter().find(|(id, _)| *id == provider_id).map_or(SUB_ITEM_PALETTES[0].1, |(_, p)| *p)
}

/// 세부 항목 색: 사용자 지정이 있으면 그것, 없으면 목록 안의 위치로(첫 항목 = AI 색, 이후 그 AI의 묶음 순환).
pub fn sub_item_color(s: &Settings, provider_id: &str, item_id: &str, items: &[UsageSubItem]) -> Rgba {
    let key = UsageSubItem::settings_key(provider_id, item_id);
    s.sub_item_colors.get(&key).copied().unwrap_or_else(|| {
        let i = items.iter().position(|it| it.id == item_id).unwrap_or(0);
        let palette = sub_item_palette(provider_id);
        if i == 0 { provider_color(s, provider_id) } else { palette[(i - 1) % palette.len()] }
    })
}

/// 띠에 겹쳐 그릴 세부 항목들(잔여율 + 색). 겹침 모드가 아니거나 전부 뺐으면 빈 배열 = 대표 한 줄.
/// (Swift `subItemBarParts`)
pub fn sub_item_bar_parts(s: &Settings, provider_id: &str, snap: &UsageSnapshot) -> Vec<BandPart> {
    if !s.sub_item_split_ids.contains(provider_id) {
        return vec![];
    }
    let items = snap.sub_items();
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| {
            !s.sub_item_bar_hidden.contains(&um_core::model::UsageSubItem::settings_key(provider_id, &it.id))
        })
        .map(|(_, it)| BandPart { id: it.id.clone(), ratio: it.remaining_ratio, color: sub_item_color(s, provider_id, &it.id, &items) })
        .collect()
}
