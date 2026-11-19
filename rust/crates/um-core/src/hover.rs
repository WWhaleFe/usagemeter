//! 테두리 호버 판정(Swift HoverInfoController.swift:91-116 `handleMove`의 테두리 부분).
//! 좌표: 화면 안, 왼쪽 위 원점, pt. 메뉴바(트레이) 아이콘 호버는 앱이 트레이 이벤트로 따로 처리한다.

use crate::segments::{PROVIDER_IDS, SegContext, edge_active};
use crate::settings::Settings;
use crate::types::{BorderCorner, BorderEdge};

/// 테두리 판정용 세그먼트 문맥(그 화면의 테두리 설정 기준).
pub fn seg_ctx(s: &Settings) -> SegContext<'_> {
    SegContext {
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
    checks.iter().any(|&(edge, on, c1, c2, d1, d2)| {
        if !on || !edge_active(&ctx, edge) {
            return false;
        }
        main.hover_edges.contains(&edge)
            || (main.hover_corners.contains(&c1) && d1 <= cs)
            || (main.hover_corners.contains(&c2) && d2 <= cs)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
