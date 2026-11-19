//! 패널 그리기 테스트: 크기·잉크·투명 모서리 확인, `PANEL_DUMP=1`이면 target/panel-preview/에 PNG 저장.

use super::*;

const DARK: Theme = Theme { dark: true };
const LIGHT: Theme = Theme { dark: false };
const CLAUDE: Rgba = [0.85, 0.47, 0.34, 1.0];
const GEMINI: Rgba = [0.30, 0.55, 0.95, 1.0];
const CODEX: Rgba = [0.30, 0.75, 0.55, 1.0];

fn dump(name: &str, r: &Rendered, dark: bool) {
    if std::env::var("PANEL_DUMP").is_err() {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("panel-preview");
    std::fs::create_dir_all(&dir).unwrap();
    // 흐린 배경 대용으로 단색 위에 합성해서 저장(투명 PNG는 보기 어렵다).
    let mut bg = Pixmap::new(r.pixmap.width(), r.pixmap.height()).unwrap();
    bg.fill(if dark { tiny_skia::Color::from_rgba8(40, 40, 44, 255) } else { tiny_skia::Color::from_rgba8(235, 235, 238, 255) });
    bg.draw_pixmap(0, 0, r.pixmap.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), None);
    bg.save_png(dir.join(name)).unwrap();
}

fn ink(pm: &Pixmap) -> usize {
    pm.pixels().iter().filter(|p| p.alpha() > 0).count()
}

fn assert_corners_clear(pm: &Pixmap) {
    let (w, h) = (pm.width(), pm.height());
    for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        assert_eq!(pm.pixel(x, y).unwrap().alpha(), 0, "모서리 ({x},{y}) 투명해야 함");
    }
}

fn hover_lines() -> Vec<HoverLine> {
    vec![
        HoverLine { text: "● Claude 5시간 잔여 58% · 리셋 15:30".into(), dot: Some(CLAUDE) },
        HoverLine { text: "● Gemini 週間残量 97% · Weekly remaining".into(), dot: Some(GEMINI) },
        HoverLine { text: "분석 보기 使用量の分析 Usage Analytics".into(), dot: None },
    ]
}

#[test]
fn 호버_크기와_잉크() {
    for scale in [1.0f32, 2.0] {
        let r = render_hover(&hover_lines(), 600.0, DARK, scale);
        assert!(r.width > 20.0 && r.height >= 12.0 + 3.0 * 12.0);
        assert_eq!(r.pixmap.width(), (r.width * scale).ceil() as u32);
        assert_eq!(r.pixmap.height(), (r.height * scale).ceil() as u32);
        assert!(ink(&r.pixmap) > 500);
        assert_corners_clear(&r.pixmap);
        // 좌우 여백 10pt 안쪽(왼쪽 9pt)에는 잉크가 없다.
        let pm = &r.pixmap;
        let px = (9.0 * scale) as u32;
        for y in 0..pm.height() {
            for x in 0..px {
                assert_eq!(pm.pixel(x, y).unwrap().alpha(), 0, "왼쪽 여백에 잉크 ({x},{y})");
            }
        }
        if scale == 2.0 {
            dump("hover.png", &r, true);
        }
    }
    // 점 색이 실제로 칠해진다(주황 계열 픽셀).
    let r = render_hover(&hover_lines(), 600.0, DARK, 2.0);
    assert!(r.pixmap.pixels().iter().any(|p| p.red() > 150 && p.blue() < 120 && p.alpha() > 200));
}

#[test]
fn 호버_줄바꿈() {
    let long = vec![HoverLine { text: "가나다라마바사 ".repeat(20), dot: None }];
    let narrow = render_hover(&long, 200.0, LIGHT, 1.0);
    let wide = render_hover(&long, 2000.0, LIGHT, 1.0);
    assert!(narrow.width <= 200.0 + 0.5);
    assert!(narrow.height > wide.height * 2.0);
    assert_eq!(measure_hover(&long, 200.0, LIGHT), (narrow.width, narrow.height));
}

fn sections() -> Vec<CardSection> {
    vec![
        CardSection {
            name: "Claude".into(),
            color: CLAUDE,
            plan: Some("Max (5x)".into()),
            lines: vec![
                CardLine { text: "5시간 잔여  95% · 34분 후".into(), dot: Some(CLAUDE) },
                CardLine { text: "주간 잔여  71% · 3일 후".into(), dot: Some([0.65, 0.45, 0.95, 1.0]) },
            ],
        },
        CardSection {
            name: "Gemini".into(),
            color: GEMINI,
            plan: None,
            lines: vec![CardLine { text: "週間残量 97% · リセットまで 2日".into(), dot: None }],
        },
        CardSection {
            name: "Codex".into(),
            color: CODEX,
            plan: Some("Plus".into()),
            lines: vec![CardLine { text: "Weekly remaining 40% · resets in 5 days".into(), dot: None }],
        },
    ]
}

#[test]
fn 카드_크기와_잉크() {
    let s = sections();
    let (w, h) = measure_card(&s, None, DARK);
    assert_eq!(w, 340.0, "최소 폭");
    // 헤더 3개 + 줄 4개 + 나눔선 2개(위아래 9 + 1) + 세로 여백 20
    let expect = 3.0 * header_h() + 4.0 * (3.0 + text::line_height(12.0)) + 2.0 * 19.0 + 20.0;
    assert_eq!(h, expect);
    for scale in [1.0f32, 2.0] {
        for (theme, n) in [(DARK, "card_dark.png"), (LIGHT, "card_light.png")] {
            let r = render_card(&s, None, theme, scale);
            assert_eq!((r.width, r.height), (w, h));
            assert!(ink(&r.pixmap) > 1000);
            assert_corners_clear(&r.pixmap);
            if scale == 2.0 {
                dump(n, &r, theme.dark);
            }
        }
    }
}

#[test]
fn 카드_긴_줄이면_넓어진다() {
    let mut s = sections();
    s[1].lines[0].text = "아주 긴 줄 ".repeat(15);
    let (w, _) = measure_card(&s, None, DARK);
    assert!(w > 340.0);
    let r = render_card(&s, None, DARK, 1.0);
    // 오른쪽 16pt 여백에는 잉크가 없다.
    for y in 0..r.pixmap.height() {
        for x in (w as u32 - 16)..w as u32 {
            assert_eq!(r.pixmap.pixel(x, y).unwrap().alpha(), 0);
        }
    }
}

#[test]
fn 카드_빈_상태() {
    let r = render_card(&[], Some("표시할 사용량이 없습니다"), DARK, 2.0);
    assert_eq!(r.width, 340.0);
    assert!(ink(&r.pixmap) > 100);
    assert_corners_clear(&r.pixmap);
    dump("card_empty.png", &r, true);
}

fn chart_series(now: f64) -> Vec<ChartSeries> {
    let mk = |name: &str, color, f: &dyn Fn(f64) -> f64| ChartSeries {
        name: name.into(),
        color,
        points: (0..=24).map(|i| (now - 24.0 * 3600.0 + i as f64 * 3600.0, f(i as f64))).collect(),
    };
    vec![
        mk("Claude", CLAUDE, &|i| (1.0 - i / 30.0).max(0.0)),
        mk("Gemini", GEMINI, &|i| if i < 12.0 { 0.9 } else { 0.97 }),
        mk("Codex", CODEX, &|i| 0.5 + 0.3 * (i / 4.0).sin()),
    ]
}

#[test]
fn 차트_크기와_잉크() {
    let now = 1_780_000_000.0;
    let series = chart_series(now);
    let labels: Vec<(f64, String)> =
        [20.0, 12.0, 4.0].iter().map(|h| (now - h * 3600.0, format!("{:02}", (24.0 - h) as i32))).collect();
    let mut spec =
        ChartSpec { title: "최근 24시간 잔여율", series: &series, width: 340.0, now, hours: 24.0, x_labels: &labels, hover_x: None };
    let a = render_chart(&spec, DARK, 2.0);
    assert_eq!((a.width, a.height), (340.0, measure_chart_height()));
    assert_eq!(a.height, 12.0 + 12.0 + 4.0 + 92.0);
    assert!(ink(&a.pixmap) > 1500);
    assert_corners_clear(&a.pixmap);
    dump("chart.png", &a, true);
    spec.hover_x = Some(150.0);
    let b = render_chart(&spec, DARK, 2.0);
    assert!(ink(&b.pixmap) > ink(&a.pixmap), "호버 점선·툴팁이 추가로 그려진다");
    dump("chart_hover.png", &b, true);
    let c = render_chart(&spec, LIGHT, 1.0);
    dump("chart_hover_light.png", &c, false);
}

#[test]
fn 차트_빈_데이터와_한_점도_안_죽는다() {
    let now = 1_780_000_000.0;
    let one = [ChartSeries { name: "A".into(), color: CLAUDE, points: vec![(now, 0.5)] }];
    let none = [ChartSeries { name: "B".into(), color: CLAUDE, points: vec![] }];
    for s in [&one[..], &none[..], &[][..]] {
        let spec = ChartSpec { title: "t", series: s, width: 300.0, now, hours: 24.0, x_labels: &[], hover_x: Some(100.0) };
        let r = render_chart(&spec, LIGHT, 1.0);
        assert!(ink(&r.pixmap) > 0);
    }
}

#[test]
fn 단조_보간은_넘치지_않는다() {
    let p = monotone_path(&[(0.0, 10.0), (10.0, 0.0), (20.0, 0.0), (30.0, 10.0)]).unwrap();
    let b = p.bounds();
    assert!(b.top() >= -1e-4 && b.bottom() <= 10.0 + 1e-4, "{b:?}");
}

#[test]
fn 세부_항목_색_규칙() {
    use crate::border::{provider_color, sub_item_color};
    use um_core::model::UsageSubItem;
    use um_core::settings::Settings;
    let it = |id: &str| UsageSubItem {
        id: id.into(),
        model_label: None,
        is_five_hour: true,
        remaining_ratio: 0.5,
        reset_at: None,
    };
    let items = vec![it("5h"), it("weekly"), it("model:x:5h")];
    let mut s = Settings::default();
    assert_eq!(sub_item_color(&s, "claude", "5h", &items), provider_color(&s, "claude"));
    let c1 = sub_item_color(&s, "claude", "weekly", &items);
    let c2 = sub_item_color(&s, "claude", "model:x:5h", &items);
    assert_ne!(c1, c2);
    s.sub_item_colors.insert(UsageSubItem::settings_key("claude", "weekly"), [1.0, 0.0, 0.0, 1.0]);
    assert_eq!(sub_item_color(&s, "claude", "weekly", &items), [1.0, 0.0, 0.0, 1.0]);
}

/// 수동 측정용: `cargo test -p um-render --release panel::tests::카드_시간 -- --ignored --nocapture`
#[test]
#[ignore]
fn 카드_시간() {
    let s = sections();
    let t = std::time::Instant::now();
    let _ = render_card(&s, None, DARK, 2.0);
    eprintln!("첫 그리기(글꼴 로드 포함): {:?}", t.elapsed());
    let t = std::time::Instant::now();
    for _ in 0..20 {
        let _ = render_card(&s, None, DARK, 2.0);
    }
    eprintln!("이후 평균: {:?}", t.elapsed() / 20);
}

#[test]
fn 같은_x의_점이_있어도_선을_만든다() {
    // 호버 때마다 기준 시각이 바뀌어 가까운 두 점이 같은 픽셀 x가 되면 선 전체가 사라지던 문제.
    let pts = [(10.0, 50.0), (20.0, 40.0), (20.0, 45.0), (30.0, 30.0), (f32::NAN, 1.0)];
    let p = monotone_path(&pts).expect("선");
    let b = p.bounds();
    assert!(b.left() >= 10.0 && b.right() <= 30.0 && b.top().is_finite() && b.bottom().is_finite());
    assert!(monotone_path(&[(5.0, 1.0), (5.0, 2.0)]).is_none());
}
