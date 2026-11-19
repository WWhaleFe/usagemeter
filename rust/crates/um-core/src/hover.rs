//! 테두리 호버 판정(Swift HoverInfoController.swift:91-116 `handleMove`의 테두리 부분).
//! 좌표: 화면 안, 왼쪽 위 원점, pt. 메뉴바(트레이) 아이콘 호버는 앱이 트레이 이벤트로 따로 처리한다.

use crate::segments::{PROVIDER_IDS, SegContext, edge_active, segs_for};
use crate::settings::Settings;
use crate::types::{BorderCorner, BorderEdge, SegPart};

/// 테두리 판정용 세그먼트 문맥(그 화면의 테두리 설정 기준).
pub fn seg_ctx(s: &Settings) -> SegContext<'_> {
    SegContext {
        side: crate::types::BorderEdge::Bottom,
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

/// 커서가 호버 패널을 띄울 테두리 위인지.
/// - `display`: 그 화면의 테두리 설정(굵기·그려진 변), `main`: 전역 호버 설정(켜짐·변·모서리·크기).
/// - 그 변에 어느 AI든 선이 있어야 하고(`edgeActive`), 변 전체가 호버 대상이거나 지정한 모서리에서 `hoverCornerSize` 이내.
pub fn border_hover_hit(display: &Settings, main: &Settings, w: f64, h: f64, x: f64, y: f64) -> bool {
    if !main.hover_info_enabled {
        return false;
    }
    let th = display.thickness.max(6.0) + 7.0;
    let cs = main.hover_corner_size;
    let inside_x = (0.0..=w).contains(&x);
    let inside_y = (0.0..=h).contains(&y);
    // (변, 그 변 위인지, 시작 꼭짓점, 끝 꼭짓점, 시작에서 거리, 끝에서 거리). Swift는 아래 원점이라 좌·우 변의 시작이 아래쪽.
    let checks = [
        (BorderEdge::Top, y.abs() <= th && inside_x, BorderCorner::TopLeft, BorderCorner::TopRight, x, w - x),
        (BorderEdge::Bottom, (y - h).abs() <= th && inside_x, BorderCorner::BottomLeft, BorderCorner::BottomRight, x, w - x),
        (BorderEdge::Left, x.abs() <= th && inside_y, BorderCorner::BottomLeft, BorderCorner::TopLeft, h - y, y),
        (BorderEdge::Right, (x - w).abs() <= th && inside_y, BorderCorner::BottomRight, BorderCorner::TopRight, h - y, y),
    ];
    let ids: Vec<&str> = PROVIDER_IDS.iter().copied().filter(|id| !main.border_hidden_ids.contains(*id)).collect();
    let mut ctx = seg_ctx(display);
    ctx.provider_ids = &ids;
    let on_edge = checks.iter().any(|&(edge, on, c1, c2, d1, d2)| {
        if !on || !edge_active(&ctx, edge) {
            return false;
        }
        main.hover_edges.contains(&edge)
            || (main.hover_corners.contains(&c1) && d1 <= cs)
            || (main.hover_corners.contains(&c2) && d2 <= cs)
    });
    // 화면 분할의 메뉴바 아래 선·독 위 선은 화면 가장자리가 아니어서 변 판정에 걸리지 않는다(옵션, 기본 꺼짐).
    let seg_on = |part: SegPart| ctx.provider_ids.iter().any(|&id| segs_for(&ctx, id).contains(&part));
    let on_menu = display.menu_line_enabled && main.hover_menu_line && inside_x
        && (y - display.menu_line_height).abs() <= th && seg_on(SegPart::HMenu);
    let on_dock = display.dock_line_enabled && main.hover_dock_line && inside_x
        && (y - (h - display.dock_line_height)).abs() <= th && seg_on(SegPart::HDock);
    on_edge || on_menu || on_dock
}

/// 방향 변환 뒤 화면 좌표에서 옵션을 적용한다. 메뉴바는 위에 고정하고 Dock 경계는 해당 축으로 검사한다.
#[allow(clippy::too_many_arguments)]
pub fn border_hover_hit_positioned(display: &Settings, main: &Settings, w: f64, h: f64, x: f64, y: f64, side: BorderEdge) -> bool {
    if side == BorderEdge::Bottom { return border_hover_hit(display, main, w, h, x, y); }
    if !main.hover_info_enabled { return false; }
    let th = display.thickness.max(6.0) + 7.0;
    let i = display.thickness / 2.0;
    let near = |a: (f64, f64), b: (f64, f64)| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        (x - a.0 - t * dx).hypot(y - a.1 - t * dy) <= th
    };
    let allowed = |edge, a, b, da, db| main.hover_edges.contains(&edge)
        || (main.hover_corners.contains(&a) && da <= main.hover_corner_size)
        || (main.hover_corners.contains(&b) && db <= main.hover_corner_size);
    let mut ctx = seg_ctx(display);
    ctx.side = side;
    PROVIDER_IDS.iter().filter(|id| !main.border_hidden_ids.contains(**id)).any(|id| {
        crate::dock::positioned_lines(display, &segs_for(&ctx, id), w, h, side).iter().any(|line| {
            if !near(line.a, line.b) { return false; }
            let (a, b) = (line.a, line.b);
            let on = |v: f64, p: f64, q: f64| (p - v).abs() < 0.01 && (q - v).abs() < 0.01;
            if on(i, a.1, b.1) { return allowed(BorderEdge::Top, BorderCorner::TopLeft, BorderCorner::TopRight, x, w - x); }
            if on(h - i, a.1, b.1) { return allowed(BorderEdge::Bottom, BorderCorner::BottomLeft, BorderCorner::BottomRight, x, w - x); }
            if on(i, a.0, b.0) { return allowed(BorderEdge::Left, BorderCorner::TopLeft, BorderCorner::BottomLeft, y, h - y); }
            if on(w - i, a.0, b.0) { return allowed(BorderEdge::Right, BorderCorner::TopRight, BorderCorner::BottomRight, y, h - y); }
            ((line.part == SegPart::HMenu || (display.menu_line_enabled && matches!(side, BorderEdge::Left | BorderEdge::Right) && line.part == SegPart::LDock)) && main.hover_menu_line)
                || (line.part == SegPart::HDock && main.hover_dock_line)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 방향별_내부_경계선과_실제_화면_변_옵션() {
        for menu_on in [false, true] {
            let mut s = Settings { menu_line_enabled: menu_on, menu_line_height: 25.0,
                dock_line_enabled: true, dock_line_height: 70.0, ..Default::default() };
            s.provider_layouts.get_mut("claude").unwrap().segments = Some([SegPart::HDock].into());
            s.border_hidden_ids.extend(["gemini".into(), "codex".into()]);
            s.hover_edges.clear();
            s.hover_corners.clear();
            for side in [BorderEdge::Bottom, BorderEdge::Top, BorderEdge::Left, BorderEdge::Right] {
                let (x, y) = match side { BorderEdge::Bottom => (600.0, 730.0), BorderEdge::Top => (600.0, 70.0),
                    BorderEdge::Left => (70.0, 400.0), BorderEdge::Right => (1130.0, 400.0) };
                s.hover_dock_line = false;
                assert!(!border_hover_hit_positioned(&s, &s, 1200.0, 800.0, x, y, side));
                s.hover_dock_line = true;
                assert!(border_hover_hit_positioned(&s, &s, 1200.0, 800.0, x, y, side));
            }
            if menu_on {
                s.provider_layouts.get_mut("claude").unwrap().segments = Some([SegPart::HMenu].into());
                s.hover_menu_line = true;
                for side in [BorderEdge::Left, BorderEdge::Right] {
                    assert!(border_hover_hit_positioned(&s, &s, 1200.0, 800.0, 600.0, 25.0, side));
                }
            }
        }
    }

    #[test]
    fn 화면_분할의_메뉴바_선과_독_선_호버는_옵션() {
        let mut s = Settings { menu_line_enabled: true, dock_line_enabled: true, menu_line_height: 30.0, dock_line_height: 90.0, ..Settings::default() };
        s.provider_layouts.get_mut("claude").unwrap().segments = Some([SegPart::HMenu].into());
        s.provider_layouts.get_mut("gemini").unwrap().segments = Some([SegPart::HDock].into());
        // 기본은 꺼짐.
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 31.0));
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 711.0));
        s.hover_menu_line = true;
        s.hover_dock_line = true;
        // 위 변을 꺼도 메뉴바 선은 따로 켜고 끈다.
        s.hover_edges.remove(&BorderEdge::Top);
        assert!(border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 31.0));
        assert!(border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 711.0));
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 300.0));
    }

    #[test]
    fn 숨긴_ai만_있는_변은_호버하지_않음() {
        let mut s = Settings::default();
        s.border_hidden_ids.insert("claude".into());
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 3.0));
        assert!(border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 797.0));
        s.border_hidden_ids = PROVIDER_IDS.iter().map(|id| id.to_string()).collect();
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 797.0));
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 2.0, 400.0));
    }

    #[test]
    fn 기본값은_선이_있는_변_전체() {
        // 기본 레이아웃: claude=hTop, gemini=hBottom, codex=lMain → 오른쪽 변은 선이 없다.
        let s = Settings::default();
        let (w, h) = (1000.0, 800.0);
        assert!(border_hover_hit(&s, &s, w, h, 500.0, 3.0), "위");
        assert!(border_hover_hit(&s, &s, w, h, 500.0, 797.0), "아래");
        assert!(border_hover_hit(&s, &s, w, h, 2.0, 400.0), "왼쪽");
        assert!(!border_hover_hit(&s, &s, w, h, 998.0, 400.0), "오른쪽은 선 없음");
        assert!(!border_hover_hit(&s, &s, w, h, 500.0, 400.0), "가운데");
    }

    #[test]
    fn 모서리_구간만() {
        let mut s = Settings::default();
        s.hover_edges.clear();
        s.hover_corners = [BorderCorner::BottomLeft].into();
        s.hover_corner_size = 50.0;
        let (w, h) = (1000.0, 800.0);
        assert!(border_hover_hit(&s, &s, w, h, 30.0, 797.0), "아래 변, 왼쪽 아래에서 30");
        assert!(!border_hover_hit(&s, &s, w, h, 300.0, 797.0), "아래 변, 너무 멂");
        assert!(border_hover_hit(&s, &s, w, h, 2.0, 780.0), "왼쪽 변, 아래에서 20");
        assert!(!border_hover_hit(&s, &s, w, h, 2.0, 100.0), "왼쪽 변, 위쪽");
    }

    #[test]
    fn 꺼짐과_두께() {
        let s = Settings { hover_info_enabled: false, ..Settings::default() };
        assert!(!border_hover_hit(&s, &s, 1000.0, 800.0, 500.0, 3.0));
        let d = Settings { thickness: 15.0, ..Settings::default() };
        let m = Settings::default();
        assert!(border_hover_hit(&d, &m, 1000.0, 800.0, 500.0, 21.0), "두께 15면 22pt까지");
        assert!(!border_hover_hit(&d, &m, 1000.0, 800.0, 500.0, 23.0));
    }
}
