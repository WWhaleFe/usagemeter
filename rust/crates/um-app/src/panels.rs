//! 호버 패널과 메뉴바 정보 패널(계획서 5-3, 5-4, 결정 D1).
//! - 테두리 호버: 커서가 선이 있는 변 근처면 커서 옆에 사용량 글을 띄운다(Swift HoverInfoController).
//! - 메뉴바 아이콘 호버: 같은 글을 아이콘 아래에.
//! - 메뉴바 클릭 설정에 따라 정보 카드 패널 또는 카드가 포함된 전체 메뉴를 연다.
//!
//! 창 조작과 강조 갱신은 두 플랫폼 모두 메인 스레드에서 한다.
//! 패널 좌표는 왼쪽 아래 원점이며 Windows에서는 물리 픽셀과 반전한 y를 쓴다.

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

static MENU_TRACKING: AtomicUsize = AtomicUsize::new(0);

use chrono::{Local, Utc};
use tauri::AppHandle;
use um_core::menu_text::{self, ActiveAi, InfoSection};
use um_core::settings::{AppAppearance, Settings};
use um_platform::InfoPanel;
use um_platform::display::{self, DisplayInfo};
use um_platform::mouse::{MouseKind, MouseMonitor};
use um_render::border::{provider_color, sub_item_color};
use um_render::panel::{self, CardLine, CardSection, ChartSeries, ChartSpec, HoverLine, Theme};

use crate::state::AppState;

/// 호버 패널 최대 폭(pt).
const HOVER_MAX_W: f32 = 520.0;
/// 패널을 커서·아이콘에서 띄우는 거리(Swift HoverInfoController.swift:137-159).
const CURSOR_GAP: f64 = 14.0;
const BAR_GAP: f64 = 6.0;

/// 트레이 아이콘 영역(왼쪽 아래 원점, macOS는 pt, Windows는 물리 픽셀).
#[derive(Clone, Copy)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

struct Panels {
    hover: InfoPanel,
    card: InfoPanel,
    card_open: bool,
    tray: Option<Rect>,
    /// Windows: 트레이 아이콘 신호(Enter/Move)가 알려 준 아이콘 영역. 호버를 띄운 동안만 있다.
    #[cfg(windows)]
    tray_hover_rect: Option<Rect>,
    /// 마지막으로 그린 호버 글(같으면 위치만 옮긴다).
    hover_key: String,
    emphasis: Vec<(String, Option<String>)>,
    emphasis_timer: Option<EmphasisTimer>,
    #[cfg(windows)]
    app: AppHandle,
    emphasis_line: bool,
    /// 열린 정보 패널의 차트 높이(pt, 차트 없으면 0)와 마지막 차트 호버 위치(같으면 다시 그리지 않음).
    chart_h: f32,
    chart_hover: Option<i32>,
    displays: Vec<DisplayInfo>,
    _mouse: MouseMonitor,
    #[cfg(target_os = "macos")]
    _menu_observers: Vec<objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_foundation::NSObjectProtocol>>>,
}

thread_local! {
    static PANELS: RefCell<Option<Panels>> = const { RefCell::new(None) };
}

/// 앱 시작 때 한 번(메인 스레드).
pub fn init(app: &AppHandle) {
    let (Ok(hover), Ok(card)) = (InfoPanel::new(7.0), InfoPanel::new(10.0)) else {
        tracing::warn!("정보 패널을 만들지 못했습니다");
        return;
    };
    #[cfg(target_os = "macos")]
    let observers = {
        let center = objc2_foundation::NSNotificationCenter::defaultCenter();
        let mut observers = vec![];
        for (name, tracking) in unsafe { [(objc2_app_kit::NSMenuDidBeginTrackingNotification, true), (objc2_app_kit::NSMenuDidEndTrackingNotification, false)] } {
            let h = app.clone();
            let block = block2::RcBlock::new(move |note: std::ptr::NonNull<objc2_foundation::NSNotification>| {
                if tracking { MENU_TRACKING.fetch_add(1, Ordering::Relaxed); }
                else if MENU_TRACKING.load(Ordering::Relaxed) > 0 { MENU_TRACKING.fetch_sub(1, Ordering::Relaxed); }
                if tracking {
                    menu_opening();
                    let note = unsafe { note.as_ref() };
                    if let Some(object) = note.object()
                        && let Some(menu) = object.downcast_ref::<objc2_app_kit::NSMenu>()
                    {
                        install_menu_card(&h, menu);
                    }
                } else if MENU_TRACKING.load(Ordering::Relaxed) == 0 {
                    MENU_CARD.with(|c| c.borrow_mut().take());
                }
            });
            observers.push(unsafe { center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block) });
        }
        observers
    };
    let h = app.clone();
    let mouse = MouseMonitor::start(move |x, y, kind| {
        #[cfg(target_os = "macos")]
        on_mouse(&h, x, y, kind);
        #[cfg(windows)]
        {
            let app = h.clone();
            let _ = h.run_on_main_thread(move || on_mouse(&app, x, native_y(y), kind));
        }
    });
    PANELS.with(|p| {
        *p.borrow_mut() = Some(Panels { hover, card, card_open: false, tray: None, hover_key: String::new(), emphasis: vec![], emphasis_timer: None, emphasis_line: false, chart_h: 0.0, chart_hover: None, displays: display::displays(), _mouse: mouse,
            #[cfg(windows)] app: app.clone(),
            #[cfg(windows)] tray_hover_rect: None,
            #[cfg(target_os = "macos")] _menu_observers: observers })
    });
}

/// 화면 구성이 바뀌면 다시 읽는다.
pub fn refresh_displays() {
    PANELS.with(|p| {
        if let Some(p) = p.borrow_mut().as_mut() {
            p.displays = display::displays();
        }
    });
}

fn theme(s: &Settings) -> Theme {
    Theme {
        dark: match s.appearance {
            AppAppearance::Dark => true,
            AppAppearance::Light => false,
            AppAppearance::System => um_platform::system_dark(),
        },
    }
}

fn sections(st: &AppState) -> Vec<InfoSection> {
    let names: Vec<(String, &'static str)> =
        st.snapshots.keys().map(|id| (id.clone(), um_providers::spec::spec(id).map(|s| s.name).unwrap_or("AI"))).collect();
    let active: Vec<ActiveAi> = names.iter().filter_map(|(id, name)| st.snapshots.get(id).map(|s| (id.as_str(), *name, s))).collect();
    let now = Utc::now();
    let ts = now.timestamp_millis() as f64 / 1000.0;
    menu_text::info_sections(&st.main, &active, |id| st.history.as_ref().and_then(|db| db.pace(id, ts).ok().flatten()), now, &Local)
}

fn dot_color(st: &AppState, provider: &str, item: &Option<String>) -> Option<um_core::types::Rgba> {
    let item = item.as_ref()?;
    let items = st.snapshots.get(provider)?.sub_items();
    Some(sub_item_color(&st.main, provider, item, &items))
}

/// 호버 글(Swift 호버 문자열과 같은 구성: AI 이름 줄, 정보 줄, AI 사이 빈 줄).
fn hover_lines(st: &AppState, border: bool, emphasis: &[(String, Option<String>)]) -> Vec<HoverLine> {
    let secs: Vec<_> = sections(st).into_iter().filter(|s| !border || !st.main.border_hidden_ids.contains(&s.id)).collect();
    if secs.is_empty() {
        return vec![HoverLine { emphasized: false, text: menu_text::no_ai_text(st.main.language), dot: None }];
    }
    let mut out = vec![];
    for (i, s) in secs.iter().enumerate() {
        if i > 0 {
            out.push(HoverLine { emphasized: false, text: String::new(), dot: None });
        }
        out.push(HoverLine { emphasized: emphasis.contains(&(s.id.clone(), None)), text: s.name.clone(), dot: None });
        for l in &s.lines {
            let dot = dot_color(st, &s.id, &l.dot_item);
            let text = if dot.is_some() { format!("● {}", l.text) } else { l.text.clone() };
            out.push(HoverLine { text, dot, emphasized: emphasis.contains(&(s.id.clone(), l.dot_item.clone())) && l.dot_item.is_some() });
        }
    }
    out
}

/// 정보 카드 + (켜져 있으면) 미니 차트를 한 장으로. 반환 = (그림, 차트 높이 pt). `hover_x` = 차트 왼쪽 기준 커서 x.
fn card_image(st: &AppState, scale: f32, hover_x: Option<f32>) -> (panel::Rendered, f32) {
    let th = theme(&st.main);
    let secs = sections(st);
    let cards: Vec<CardSection> = secs
        .iter()
        .map(|s| CardSection {
            name: s.name.clone(),
            color: provider_color(&st.main, &s.id),
            plan: s.plan.clone().filter(|p| !p.is_empty()),
            lines: s.lines.iter().map(|l| CardLine { text: l.text.clone(), dot: dot_color(st, &s.id, &l.dot_item) }).collect(),
        })
        .collect();
    let empty = um_core::i18n::tr("menu.noAI", st.main.language);
    let card = panel::render_card(&cards, Some(&empty), th, scale);
    if !st.main.menu_show_chart {
        return (backdrop(card, th, scale, 10.0), 0.0);
    }
    let hours = st.main.chart_hours as f64;
    let now = Utc::now().timestamp_millis() as f64 / 1000.0;
    let since = now - hours * 3600.0;
    let line = |name: String, color, pts: &[um_core::analytics::Point]| {
        ChartSeries { name, color, points: pts.iter().map(|p| (p.t, p.r)).collect() }.displayed(st.main.bar_fill_mode)
    };
    let series: Vec<ChartSeries> = um_providers::spec::ALL.iter()
        .map(|sp| sp.id)
        .filter(|id| st.snapshots.contains_key(*id))
        .flat_map(|id| {
            let ai_name = um_providers::spec::spec(id).map(|s| s.name).unwrap_or("AI");
            // AI 기본 선(AI 선 색)은 언제나 그린다.
            let base = st.history.as_ref().and_then(|db| db.series(id, since).ok()).filter(|pts| pts.len() >= 2).unwrap_or_default();
            let mut out: Vec<ChartSeries> = if base.is_empty() { vec![] } else { vec![line(ai_name.to_string(), provider_color(&st.main, id), &base)] };
            // 세부 항목 탭에서 "미니차트"를 켠 항목을 더한다. 기본 선과 똑같은 선(대개 5시간)은 겹치므로 뺀다.
            if st.main.sub_item_chart_shown.iter().any(|k| k.starts_with(&format!("{id}/"))) {
                out.extend(
                    crate::analytics_ui::item_series(st, id, since)
                        .into_iter()
                        .filter(|s| s.item.as_ref().is_some_and(|i| st.main.sub_item_chart_shown.contains(&format!("{id}/{i}"))))
                        .filter(|s| s.series.points.len() >= 2 && !um_core::analytics::same_series(&s.series.points, &base))
                        .map(|s| line(format!("{ai_name} {}", s.name), s.rgba, &s.series.points)),
                );
            }
            out
        })
        .collect();
    if series.is_empty() {
        return (backdrop(card, th, scale, 10.0), 0.0);
    }
    // x 눈금 3개: 기간을 4등분한 지점의 정시(로컬 "HH").
    let labels: Vec<(f64, String)> = (1..=3)
        .filter_map(|k| {
            let t = now - hours * 3600.0 * (4 - k) as f64 / 4.0;
            let lt = chrono::DateTime::from_timestamp(t as i64, 0)?.with_timezone(&Local);
            Some((t, lt.format("%H").to_string()))
        })
        .collect();
    let title = um_core::i18n::tn(st.main.bar_fill_mode.text_key("chart.titleFmt"), st.main.language, st.main.chart_hours as i64);
    let chart = panel::render_chart(
        &ChartSpec { title: &title, series: &series, width: card.width, now, hours, x_labels: &labels, hover_x },
        th,
        scale,
    );
    let ch = chart.height;
    (backdrop(stack(card, chart, scale, th), th, scale, 10.0), ch)
}

/// 두 그림을 위아래로 붙인다.
fn stack(a: panel::Rendered, b: panel::Rendered, scale: f32, theme: Theme) -> panel::Rendered {
    let w = a.pixmap.width().max(b.pixmap.width());
    let h = a.pixmap.height() + b.pixmap.height();
    let mut pm = tiny_skia::Pixmap::new(w, h).expect("panel size");
    let paint = tiny_skia::PixmapPaint::default();
    pm.draw_pixmap(0, 0, a.pixmap.as_ref(), &paint, tiny_skia::Transform::identity(), None);
    pm.draw_pixmap(0, a.pixmap.height() as i32, b.pixmap.as_ref(), &paint, tiny_skia::Transform::identity(), None);
    let c = theme.secondary();
    let mut line = tiny_skia::Paint::default();
    line.set_color(tiny_skia::Color::from_rgba(c[0] as f32, c[1] as f32, c[2] as f32, 0.25).unwrap());
    if let Some(rect) = tiny_skia::Rect::from_xywh(10.0 * scale, a.pixmap.height() as f32, (w as f32 - 20.0 * scale).max(1.0), scale) {
        pm.fill_rect(rect, &line, tiny_skia::Transform::identity(), None);
    }
    panel::Rendered { pixmap: pm, width: w as f32 / scale, height: h as f32 / scale }
}

fn screen_of(ds: &[DisplayInfo], x: f64, y: f64) -> Option<(&DisplayInfo, f64, f64)> {
    ds.iter().find_map(|d| d.to_local(x, native_y(y)).map(|(lx, ly)| (d, lx, ly)))
}

/// 패널을 화면 안에 들어가게 놓을 위치(전역 좌표, 왼쪽 아래 기준).
fn clamp_to_screen(d: &DisplayInfo, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
    let (left, top) = screen_top(d);
    let (right, bottom) = (left + extent(d.width_pt, d.scale), top - extent(d.height_pt, d.scale));
    clamp_in_screen(Rect { x: left, y: bottom, w: right - left, h: top - bottom }, x, y, w, h)
}

fn clamp_in_screen(bounds: Rect, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
    (x.clamp(bounds.x + 4.0, (bounds.x + bounds.w - w - 4.0).max(bounds.x + 4.0)),
     y.clamp(bounds.y + 4.0, (bounds.y + bounds.h - h).max(bounds.y + 4.0)))
}

/// 커서 옆(+14, 위로). 화면을 넘으면 반대쪽으로. (Swift HoverInfoController.swift:148-159)
fn near_cursor(d: &DisplayInfo, cx: f64, cy: f64, w: f64, h: f64) -> (f64, f64) {
    let (left, top) = screen_top(d);
    let right = left + extent(d.width_pt, d.scale);
    let gap = extent(CURSOR_GAP, d.scale);
    let mut x = cx + gap;
    let mut y = cy + gap;
    if x + w > right {
        x = cx - w - gap;
    }
    if y + h > top {
        y = cy - h - gap;
    }
    #[cfg(windows)]
    { clamp_to_screen(d, x, y, w, h) }
    #[cfg(target_os = "macos")]
    { (x, y) }
}

fn show_hover_at(p: &mut Panels, st: &AppState, scale: f32, border: bool, place: impl Fn(f64, f64) -> (f64, f64)) {
    if !border { set_emphasis(p, None, false); }
    let lines = hover_lines(st, border, if border && st.main.hover_text_emphasis { &p.emphasis } else { &[] });
    let key = format!("{scale}:{:?}:{:?}:{}", st.main.appearance, p.emphasis, lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n"));
    if key != p.hover_key || !p.hover.is_visible() {
        let r = backdrop(panel::render_hover(&lines, HOVER_MAX_W, theme(&st.main), scale), theme(&st.main), scale, 7.0);
        let (x, y) = place(extent(r.width as f64, scale as f64), extent(r.height as f64, scale as f64));
        if p.hover.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, x, y).is_ok() {
            p.hover_key = key;
        }
    } else {
        let (w, h) = p.hover.size();
        let (x, y) = place(w, h);
        p.hover.move_to(x, y);
    }
}

/// `line` = 호버한 선을 띠에 강조(선 강조 옵션). 글 강조만 켜져 있으면 후보만 골라 호버 글에 쓴다.
fn set_emphasis(p: &mut Panels, hit: Option<(&str, f64, f64)>, line: bool) {
    let emphasis = crate::OVERLAY.with(|o| o.borrow_mut().hover_at(hit, line));
    if p.emphasis == emphasis && p.emphasis_line == line { return; }
    p.emphasis = emphasis;
    p.emphasis_line = line;
    p.emphasis_timer.take();
    // 글은 후보를 모두 강조하고 선만 번갈아 그린다.
    if line && p.emphasis.len() > 1 {
        #[cfg(target_os = "macos")]
        {
            let block = block2::RcBlock::new(move |_: std::ptr::NonNull<objc2_foundation::NSTimer>| {
                crate::OVERLAY.with(|o| o.borrow_mut().cycle_hover());
            });
            p.emphasis_timer = Some(EmphasisTimer(unsafe {
                objc2_foundation::NSTimer::scheduledTimerWithTimeInterval_repeats_block(1.5, true, &block)
            }));
        }
        #[cfg(windows)]
        { p.emphasis_timer = Some(EmphasisTimer::start(&p.app)); }
    }
}

/// 설정·사용량·화면 구성이 바뀌면 이전 장면의 호버와 타이머를 버린다.
pub fn reset_hover() -> bool {
    with(|p| {
        let active = !p.emphasis.is_empty();
        if active { hide_hover(p); }
        active
    }).unwrap_or(false)
}

/// 커서가 멈춰 있어도 사용량 갱신 후 새 장면에 맞춰 강조를 복원한다.
pub fn refresh_hover(app: &AppHandle) {
    if let Some((x, y)) = um_platform::mouse::cursor_position() {
        on_mouse(app, x, native_y(y), MouseKind::Move);
    }
}

fn hide_hover(p: &mut Panels) {
    set_emphasis(p, None, false);
    if p.hover.is_visible() {
        p.hover.hide();
    }
}

fn with<R>(f: impl FnOnce(&mut Panels) -> R) -> Option<R> {
    PANELS.with(|p| p.borrow_mut().as_mut().map(f))
}

fn on_mouse(app: &AppHandle, x: f64, y: f64, kind: MouseKind) {
    let st = crate::shared(app);
    let Ok(st) = st.try_lock() else { return };
    let tray = app.tray_by_id(crate::TRAY_ID).and_then(|t| t.rect().ok().flatten()).map(|r| crate::tray_rect(app, &r));
    with(|p| {
        if let Some(rect) = tray { p.tray = Some(rect); }
        if MENU_TRACKING.load(Ordering::Relaxed) > 0 {
            hide_hover(p);
            #[cfg(windows)]
            if p.card_open { position_beside_menu(p); card_chart_hover(p, &st, x, y); }
            return;
        }
        if kind == MouseKind::Down {
            let (fx, fy, fw, fh) = p.card.frame();
            let in_card = Rect { x: fx, y: fy, w: fw, h: fh }.contains(x, y);
            let in_tray = p.tray.is_some_and(|r| r.contains(x, y));
            if p.card_open && !in_card && !in_tray {
                p.card.hide();
                p.card_open = false;
            }
            return;
        }
        if p.card_open {
            hide_hover(p);
            card_chart_hover(p, &st, x, y);
            return;
        }
        // Windows: 아이콘이 숨김 메뉴(^) 안에 있으면 트레이 영역이 ^ 버튼으로 잡혀, 영역 추정으로 띄우면
        // ^에 올리기만 해도 카드가 떠 숨김 메뉴를 가렸다. 띄우기는 아이콘 신호로만 하고, 여기서는
        // 신호가 알려 준 아이콘 영역을 벗어나면 Leave를 기다리지 않고 닫는다(숨김 메뉴가 닫히면 Leave가 안 온다).
        #[cfg(windows)]
        if let Some(r) = p.tray_hover_rect {
            if r.contains(x, y) { return; }
            p.tray_hover_rect = None;
        }
        #[cfg(target_os = "macos")]
        if st.main.menu_bar_hover_info_enabled
            && let Some(r) = p.tray
            && let Some((d, _, _)) = screen_of(&p.displays, r.x + r.w / 2.0, r.y + r.h / 2.0)
        {
            let pad = (d.width_pt * 0.006).clamp(8.0, 24.0);
            let top = screen_top(d).1;
            let bar_h = (top - r.y).max(r.h);
            let below = (bar_h * 0.25).max(4.0);
            let zone = Rect { x: r.x - pad, y: r.y - below, w: r.w + pad * 2.0, h: bar_h + below + 1.0 };
            if zone.contains(x, y) {
                let d = d.clone();
                return show_hover_at(p, &st, d.scale as f32, false, |w, h| clamp_to_screen(&d, x - w / 2.0, r.y - h - BAR_GAP, w, h));
            }
        }

        let Some((d, lx, ly)) = screen_of(&p.displays, x, y) else { return hide_hover(p) };
        let enabled = !st.disabled_displays.contains(&d.id);
        let mut ds = st.settings_for_display(&d.id);
        // 실제 띠가 없는 AI는 로그아웃·숨김 모두 호버 영역에서 제외한다.
        ds.border_hidden_ids.extend(um_core::settings::PROVIDER_IDS.iter().filter(|id| !st.snapshots.contains_key(**id)).map(|id| id.to_string()));
        let side = crate::overlay::taskbar_settings(&mut ds, d.taskbar, cfg!(windows));
        if enabled && um_core::hover::border_hover_hit_positioned(&ds, &ds, d.width_pt, d.height_pt, lx, ly, side.edge()) {
            let d = d.clone();
            let (line, text) = (st.main.hover_emphasis_enabled, st.main.hover_text_emphasis);
            set_emphasis(p, ((line || text) && !st.hide_overlay).then_some((d.id.as_str(), lx, ly)), line);
            show_hover_at(p, &st, d.scale as f32, true, |w, h| near_cursor(&d, x, y, w, h));
        } else {
            hide_hover(p);
        }
    });
}

/// 열린 정보 패널의 차트 위에 커서가 있으면 그 위치의 값을 보여준다(Swift MiniChartView 호버, 설정으로 끔).
fn card_chart_hover(p: &mut Panels, st: &AppState, x: f64, y: f64) {
    if p.chart_h <= 0.0 || !st.main.chart_hover_enabled {
        return;
    }
    let (fx, fy, fw, _) = p.card.frame();
    // 차트는 패널 아래쪽(전역 좌표는 아래 원점이라 fy ~ fy + 차트 높이).
    let inside = x >= fx && x <= fx + fw && y >= fy && y <= fy + extent(p.chart_h as f64, screen_of(&p.displays, x, y).map(|(d, _, _)| d.scale).unwrap_or(1.0));
    let scale = screen_of(&p.displays, fx + fw / 2.0, fy + 1.0).map(|(d, _, _)| d.scale).unwrap_or(1.0);
    let hx = inside.then_some(((x - fx) / extent(1.0, scale)) as i32);
    if hx == p.chart_hover {
        return;
    }
    p.chart_hover = hx;
    let scale = screen_of(&p.displays, fx + fw / 2.0, fy + 1.0).map(|(d, _, _)| d.scale).unwrap_or(2.0) as f32;
    let (r, _) = card_image(st, scale, hx.map(|v| v as f32));
    let _ = p.card.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, fx, fy);
}

/// 트레이 아이콘에 커서가 들어오거나 나감. `rect`는 패널 전역 좌표.
pub fn tray_hover(app: &AppHandle, rect: Rect, inside: bool) {
    let st = crate::shared(app);
    let Ok(st) = st.try_lock() else { return };
    with(|p| {
        p.tray = Some(rect);
        #[cfg(windows)]
        { p.tray_hover_rect = inside.then_some(rect); }
        if MENU_TRACKING.load(Ordering::Relaxed) > 0 || !inside || p.card_open || !st.main.menu_bar_hover_info_enabled {
            return hide_hover(p);
        }
        let ds = p.displays.clone();
        let cx = rect.x + rect.w / 2.0;
        show_hover_at(p, &st, screen_of(&ds, cx, rect.y + rect.h / 2.0).map(|(d, _, _)| d.scale).unwrap_or(2.0) as f32, false, |w, h| {
            let y = rect.y - h - BAR_GAP;
            match screen_of(&ds, cx, rect.y + rect.h / 2.0) {
                Some((d, _, _)) => tray_position(d, rect, w, h),
                None => (cx - w / 2.0, y),
            }
        });
    });
}

/// 클릭으로 메뉴가 열릴 때: 호버·정보 패널을 숨긴다(Swift는 드롭다운이 열려 있는 동안 호버를 막음).
pub fn menu_opening() {
    with(|p| {
        hide_hover(p);
        if p.card_open {
            p.card.hide();
            p.card_open = false;
        }
    });
}

/// 정보 카드를 연다(이미 열려 있으면 그대로). 트레이 메뉴의 새로고침 결과를 보여 줄 때 쓴다.
#[cfg(windows)]
pub fn open_card(app: &AppHandle) {
    let Some(rect) = app.tray_by_id(crate::TRAY_ID).and_then(|t| t.rect().ok().flatten()).map(|r| crate::tray_rect(app, &r)) else { return };
    if with(|p| p.card_open).unwrap_or(true) { return; }
    toggle_card(app, rect);
}

/// 메뉴바 클릭 설정의 카드 버튼: 정보 패널 열기/닫기.
pub fn toggle_card(app: &AppHandle, rect: Rect) {
    let st = crate::shared(app);
    let Ok(st) = st.lock() else { return };
    with(|p| {
        p.tray = Some(rect);
        if p.card_open {
            p.card.hide();
            p.card_open = false;
            return;
        }
        hide_hover(p);
        let scale = screen_of(&p.displays, rect.x + rect.w / 2.0, rect.y + rect.h / 2.0).map(|(d, _, _)| d.scale).unwrap_or(2.0) as f32;
        let (r, ch) = card_image(&st, scale, None);
        p.chart_h = ch;
        p.chart_hover = None;
        let (w, h) = (extent(r.width as f64, scale as f64), extent(r.height as f64, scale as f64));
        let cx = rect.x + rect.w / 2.0;
        let (mut x, mut y) = (cx - w / 2.0, rect.y - h - BAR_GAP);
        if let Some((d, _, _)) = screen_of(&p.displays, cx, rect.y + rect.h / 2.0) {
            (x, y) = card_position(d, rect, w, h);
        }
        if p.card.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, x, y).is_ok() {
            p.card_open = true;
        }
    });
}


/// 네이티브 메뉴의 카드 자리 식별자. 표시 전 사용자 뷰로 바꾼다.
#[cfg(target_os = "macos")]
pub const MENU_CARD_TITLE: &str = "UsageMeter";

#[cfg(target_os = "macos")]
thread_local! {
    static MENU_CARD: RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSImageView>>> = const { RefCell::new(None) };
}

/// Swift의 NSMenuItem.view와 같이 카드·차트를 메뉴 안에 넣는다.
#[cfg(target_os = "macos")]
fn install_menu_card(app: &AppHandle, menu: &objc2_app_kit::NSMenu) {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::NSImageView;
    let st = crate::shared(app);
    let Ok(st) = st.try_lock() else { return };
    if st.main.tray_click_mode != um_core::settings::TrayClickMode::Together { return; }
    let Some(item) = menu.itemAtIndex(0) else { return };
    if item.title().to_string() != MENU_CARD_TITLE { return; }
    let Some(mtm) = MainThreadMarker::new() else { return };
    let view = NSImageView::initWithFrame(NSImageView::alloc(mtm), objc2_foundation::NSRect::ZERO);
    paint_menu_card(&view, &st);
    item.setView(Some(&view));
    MENU_CARD.with(|c| *c.borrow_mut() = Some(view));
}

#[cfg(target_os = "macos")]
fn paint_menu_card(view: &objc2_app_kit::NSImageView, st: &AppState) {
    use objc2::AllocAnyThread;
    use objc2_app_kit::NSImage;
    use objc2_foundation::{NSData, NSPoint, NSRect, NSSize};
    let scale = view.window().map(|w| w.backingScaleFactor() as f32).unwrap_or(2.0);
    let (r, _) = card_image(st, scale, None);
    let Ok(png) = r.pixmap.encode_png() else { return };
    let data = NSData::with_bytes(&png);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else { return };
    let size = NSSize::new(r.width as f64, r.height as f64);
    image.setSize(size);
    view.setFrame(NSRect::new(NSPoint::ZERO, size));
    view.setImage(Some(&image));
}

/// 데이터·설정 갱신을 열린 정보 패널과 메뉴 카드에 바로 반영한다.
pub fn refresh_cards(st: &AppState) {
    // 클릭으로 연 정보 패널도 커서를 움직이지 않아도 즉시 갱신한다.
    with(|p| {
        if !p.card_open { return; }
        let Some(rect) = p.tray else { return };
        let cx = rect.x + rect.w / 2.0;
        let screen = screen_of(&p.displays, cx, rect.y + rect.h / 2.0);
        let scale = screen.map(|(d, _, _)| d.scale).unwrap_or(2.0) as f32;
        let (r, ch) = card_image(st, scale, p.chart_hover.map(|x| x as f32));
        p.chart_h = ch;
        let (w, h) = (extent(r.width as f64, scale as f64), extent(r.height as f64, scale as f64));
        let (mut x, mut y) = (cx - w / 2.0, rect.y - h - BAR_GAP);
        if let Some((d, _, _)) = screen { (x, y) = card_position(d, rect, w, h); }
        let _ = p.card.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, x, y);
        #[cfg(windows)]
        if menu_tracking() { position_beside_menu(p); }
    });
    #[cfg(target_os = "macos")]
    MENU_CARD.with(|c| {
        if let Some(view) = c.borrow().as_ref() { paint_menu_card(view, st); }
    });
}

// Windows 전역 좌표는 물리 픽셀을 유지하고 y축만 반전한다. DPI는 길이에만 적용한다.
fn native_y(y: f64) -> f64 {
    if cfg!(windows) { -y } else { y }
}

fn extent(value: f64, scale: f64) -> f64 {
    if cfg!(windows) { value * scale } else { value }
}

fn screen_top(d: &DisplayInfo) -> (f64, f64) {
    let (x, y) = d.to_global(0.0, 0.0);
    (x, native_y(y))
}

fn tray_position(d: &DisplayInfo, rect: Rect, w: f64, h: f64) -> (f64, f64) {
    #[cfg(target_os = "macos")]
    { clamp_to_screen(d, rect.x + rect.w / 2.0 - w / 2.0, rect.y - h - BAR_GAP, w, h) }
    #[cfg(windows)]
    {
        let (left, top) = screen_top(d);
        let right = left + extent(d.width_pt, d.scale);
        let bottom = top - extent(d.height_pt, d.scale);
        windows_tray_position(Rect { x: left, y: bottom, w: right - left, h: top - bottom }, rect, w, h, BAR_GAP * d.scale)
    }
}

fn card_position(d: &DisplayInfo, rect: Rect, w: f64, h: f64) -> (f64, f64) {
    #[cfg(target_os = "macos")]
    {
        let (x, y) = (rect.x + rect.w / 2.0 - w / 2.0, rect.y - h - BAR_GAP);
        (clamp_to_screen(d, x, y, w, h).0, y)
    }
    #[cfg(windows)]
    { tray_position(d, rect, w, h) }
}

#[cfg(any(windows, test))]
fn windows_tray_position(bounds: Rect, rect: Rect, w: f64, h: f64, gap: f64) -> (f64, f64) {
    let cx = rect.x + rect.w / 2.0;
    let cy = rect.y + rect.h / 2.0;
    let distances = [bounds.y + bounds.h - cy, cy - bounds.y, cx - bounds.x, bounds.x + bounds.w - cx];
    let edge = distances.iter().enumerate().min_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i).unwrap_or(0);
    let (x, y) = match edge {
        0 => (cx - w / 2.0, rect.y - h - gap),
        1 => (cx - w / 2.0, rect.y + rect.h + gap),
        2 => (rect.x + rect.w + gap, cy - h / 2.0),
        _ => (rect.x - w - gap, cy - h / 2.0),
    };
    clamp_in_screen(bounds, x, y, w, h)
}

fn backdrop(r: panel::Rendered, theme: Theme, scale: f32, corner: f32) -> panel::Rendered {
    #[cfg(windows)]
    { panel::with_background(r, theme, scale, corner) }
    #[cfg(target_os = "macos")]
    { let _ = (theme, scale, corner); r }
}

#[cfg(target_os = "macos")]
struct EmphasisTimer(objc2::rc::Retained<objc2_foundation::NSTimer>);
#[cfg(target_os = "macos")]
impl Drop for EmphasisTimer {
    fn drop(&mut self) { self.0.invalidate(); }
}

#[cfg(windows)]
struct EmphasisTimer(std::sync::Arc<std::sync::atomic::AtomicBool>);
#[cfg(windows)]
impl EmphasisTimer {
    fn start(app: &AppHandle) -> Self {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled = stop.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            // 짧게 나눠 기다려 호버가 끝난 뒤 스레드도 곧바로 종료한다.
            while !cancelled.load(Ordering::Relaxed) {
                for _ in 0..30 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if cancelled.load(Ordering::Relaxed) { return; }
                }
                let cancelled = cancelled.clone();
                let _ = app.run_on_main_thread(move || {
                    if !cancelled.load(Ordering::Relaxed) {
                        crate::OVERLAY.with(|o| o.borrow_mut().cycle_hover());
                    }
                });
            }
        });
        Self(stop)
    }
}
#[cfg(windows)]
impl Drop for EmphasisTimer {
    fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); }
}

/// Windows 메뉴는 별도 창이다. 메뉴의 실제 영역 옆에 카드를 놓고 메뉴가 닫힐 때 함께 닫는다.
#[cfg(windows)]
pub fn show_menu(app: &AppHandle, rect: Rect, together: bool) {
    if menu_tracking() { return; }
    menu_opening();
    if together { toggle_card(app, rect); }
    MENU_TRACKING.store(1, Ordering::Relaxed);
    if let Some(tray) = app.tray_by_id(crate::TRAY_ID) {
        let _ = tray.with_inner_tray_icon(|t| t.show_menu());
    }
    MENU_TRACKING.store(0, Ordering::Relaxed);
    menu_opening();
    // 고른 항목은 메뉴가 닫힌 뒤 WM_COMMAND로 따로 온다. 여기서 바로 메뉴를 바꾸면 새 메뉴에서 옛 항목 번호를
    // 못 찾아 선택이 버려졌다(설정·종료 등 무반응). 그 메시지가 처리된 뒤에 바꾼다.
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            if menu_tracking() { return; }
            crate::menu::rebuild(&h);
            crate::redraw(&h);
        });
    });
}

#[cfg(windows)]
fn position_beside_menu(p: &mut Panels) {
    let Some((mx, my, mw, mh)) = um_platform::mouse::menu_frame() else { return };
    let Some((d, _, _)) = screen_of(&p.displays, mx + mw / 2.0, -my - mh / 2.0) else { return };
    let (w, h) = p.card.size();
    let (left, _) = screen_top(d);
    let gap = BAR_GAP * d.scale;
    let x = if mx - gap - w >= left { mx - gap - w } else { mx + mw + gap };
    let (x, y) = clamp_to_screen(d, x, -my - h, w, h);
    p.card.move_to(x, y);
}

/// 네이티브 메뉴가 트레이 내부를 빌린 동안에는 메뉴 교체를 닫힌 뒤로 미룬다.
#[cfg(windows)]
pub fn menu_tracking() -> bool { MENU_TRACKING.load(Ordering::Relaxed) > 0 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 작업표시줄_네_방향과_혼합_dpi_음수_원점에서_카드를_화면_안에_놓는다() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let bounds = Rect { x: -1920.0, y: -1080.0, w: 1920.0, h: 1080.0 };
            let size = 24.0 * scale;
            let (w, h) = (320.0 * scale, 240.0 * scale);
            let trays = [
                Rect { x: -300.0, y: -size, w: size, h: size },
                Rect { x: -300.0, y: -1080.0, w: size, h: size },
                Rect { x: -1920.0, y: -500.0, w: size, h: size },
                Rect { x: -size, y: -500.0, w: size, h: size },
            ];
            for (edge, tray) in trays.into_iter().enumerate() {
                let (x, y) = windows_tray_position(bounds, tray, w, h, 6.0 * scale);
                assert!(x >= bounds.x && x + w <= bounds.x + bounds.w);
                assert!(y >= bounds.y && y + h <= bounds.y + bounds.h);
                match edge {
                    0 => assert!(y + h < tray.y),
                    1 => assert!(y > tray.y + tray.h),
                    2 => assert!(x > tray.x + tray.w),
                    _ => assert!(x + w < tray.x),
                }
            }
        }
    }
}
