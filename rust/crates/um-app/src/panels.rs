//! 호버 패널과 메뉴바 정보 패널(계획서 5-3, 5-4, 결정 D1).
//! - 테두리 호버: 커서가 선이 있는 변 근처면 커서 옆에 사용량 글을 띄운다(Swift HoverInfoController).
//! - 메뉴바 아이콘 호버: 같은 글을 아이콘 아래에.
//! - 메뉴바 클릭 설정에 따라 정보 카드 패널 또는 카드가 포함된 전체 메뉴를 연다.
//!
//! macOS 전용(메인 스레드). Windows 플라이아웃은 0단계 S9 확인 뒤 붙인다.

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

/// 메뉴바 아이콘 영역(전역 AppKit 좌표: 왼쪽 아래 원점, pt).
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
    /// 마지막으로 그린 호버 글(같으면 위치만 옮긴다).
    hover_key: String,
    emphasis: Vec<(String, Option<String>)>,
    emphasis_timer: Option<objc2::rc::Retained<objc2_foundation::NSTimer>>,
    emphasis_line: bool,
    /// 열린 정보 패널의 차트 높이(pt, 차트 없으면 0)와 마지막 차트 호버 위치(같으면 다시 그리지 않음).
    chart_h: f32,
    chart_hover: Option<i32>,
    displays: Vec<DisplayInfo>,
    _mouse: MouseMonitor,
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
    let h = app.clone();
    let mouse = MouseMonitor::start(move |x, y, kind| on_mouse(&h, x, y, kind));
    PANELS.with(|p| {
        *p.borrow_mut() = Some(Panels { hover, card, card_open: false, tray: None, hover_key: String::new(), emphasis: vec![], emphasis_timer: None, emphasis_line: false, chart_h: 0.0, chart_hover: None, displays: display::displays(), _mouse: mouse, _menu_observers: observers })
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
        return (card, 0.0);
    }
    let hours = st.main.chart_hours as f64;
    let now = Utc::now().timestamp_millis() as f64 / 1000.0;
    let series: Vec<ChartSeries> = um_providers::spec::ALL.iter()
        .map(|sp| sp.id)
        .filter(|id| st.snapshots.contains_key(*id))
        .filter_map(|id| {
            let pts = st.history.as_ref()?.series(id, now - hours * 3600.0).ok()?;
            (pts.len() >= 2).then(|| ChartSeries {
                name: um_providers::spec::spec(id).map(|s| s.name).unwrap_or("AI").to_string(),
                color: provider_color(&st.main, id),
                points: pts.iter().map(|p| (p.t, p.r)).collect(),
            }.displayed(st.main.bar_fill_mode))
        })
        .collect();
    if series.is_empty() {
        return (card, 0.0);
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
    (stack(card, chart, scale, th), ch)
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
    ds.iter().find_map(|d| d.to_local(x, y).map(|(lx, ly)| (d, lx, ly)))
}

/// 패널을 화면 안에 들어가게 놓을 위치(전역 좌표, 왼쪽 아래 기준).
fn clamp_to_screen(d: &DisplayInfo, x: f64, y: f64, w: f64, h: f64) -> (f64, f64) {
    let (left, top) = d.to_global(0.0, 0.0);
    let (right, bottom) = (left + d.width_pt, top - d.height_pt);
    (x.clamp(left + 4.0, (right - w - 4.0).max(left + 4.0)), y.clamp(bottom + 4.0, (top - h).max(bottom + 4.0)))
}

/// 커서 옆(+14, 위로). 화면을 넘으면 반대쪽으로. (Swift HoverInfoController.swift:148-159)
fn near_cursor(d: &DisplayInfo, cx: f64, cy: f64, w: f64, h: f64) -> (f64, f64) {
    let (left, top) = d.to_global(0.0, 0.0);
    let right = left + d.width_pt;
    let mut x = cx + CURSOR_GAP;
    let mut y = cy + CURSOR_GAP;
    if x + w > right {
        x = cx - w - CURSOR_GAP;
    }
    if y + h > top {
        y = cy - h - CURSOR_GAP;
    }
    (x, y)
}

fn show_hover_at(p: &mut Panels, st: &AppState, scale: f32, border: bool, place: impl Fn(f64, f64) -> (f64, f64)) {
    if !border { set_emphasis(p, None, false); }
    let lines = hover_lines(st, border, if border && st.main.hover_text_emphasis { &p.emphasis } else { &[] });
    let key = format!("{scale}:{:?}:{:?}:{}", st.main.appearance, p.emphasis, lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n"));
    if key != p.hover_key || !p.hover.is_visible() {
        let r = panel::render_hover(&lines, HOVER_MAX_W, theme(&st.main), scale);
        let (x, y) = place(r.width as f64, r.height as f64);
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
    if let Some(timer) = p.emphasis_timer.take() { timer.invalidate(); }
    // 번갈아 강조는 선을 그릴 때만 의미가 있다(글은 후보 줄을 모두 강조).
    if line && p.emphasis.len() > 1 {
        let block = block2::RcBlock::new(move |_: std::ptr::NonNull<objc2_foundation::NSTimer>| {
            crate::OVERLAY.with(|o| o.borrow_mut().cycle_hover());
        });
        p.emphasis_timer = Some(unsafe {
            objc2_foundation::NSTimer::scheduledTimerWithTimeInterval_repeats_block(1.5, true, &block)
        });
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
    let point = objc2_app_kit::NSEvent::mouseLocation();
    on_mouse(app, point.x, point.y, MouseKind::Move);
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
        if MENU_TRACKING.load(Ordering::Relaxed) > 0 { return hide_hover(p); }
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
        if st.main.menu_bar_hover_info_enabled
            && let Some(r) = p.tray
            && let Some((d, _, _)) = screen_of(&p.displays, r.x + r.w / 2.0, r.y + r.h / 2.0)
        {
            let pad = (d.width_pt * 0.006).clamp(8.0, 24.0);
            let top = d.to_global(0.0, 0.0).1;
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
        if enabled && um_core::hover::border_hover_hit(&ds, &ds, d.width_pt, d.height_pt, lx, ly) {
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
    let inside = x >= fx && x <= fx + fw && y >= fy && y <= fy + p.chart_h as f64;
    let hx = inside.then_some((x - fx) as i32);
    if hx == p.chart_hover {
        return;
    }
    p.chart_hover = hx;
    let scale = screen_of(&p.displays, fx + fw / 2.0, fy + 1.0).map(|(d, _, _)| d.scale).unwrap_or(2.0) as f32;
    let (r, _) = card_image(st, scale, hx.map(|v| v as f32));
    let _ = p.card.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, fx, fy);
}

/// 메뉴바 아이콘에 커서가 들어오거나 나감. `rect` = 아이콘 영역(전역 AppKit 좌표).
pub fn tray_hover(app: &AppHandle, rect: Rect, inside: bool) {
    let st = crate::shared(app);
    let Ok(st) = st.try_lock() else { return };
    with(|p| {
        p.tray = Some(rect);
        if MENU_TRACKING.load(Ordering::Relaxed) > 0 || !inside || p.card_open || !st.main.menu_bar_hover_info_enabled {
            return hide_hover(p);
        }
        let ds = p.displays.clone();
        let cx = rect.x + rect.w / 2.0;
        show_hover_at(p, &st, screen_of(&ds, cx, rect.y + rect.h / 2.0).map(|(d, _, _)| d.scale).unwrap_or(2.0) as f32, false, |w, h| {
            let y = rect.y - h - BAR_GAP;
            match screen_of(&ds, cx, rect.y + rect.h / 2.0) {
                Some((d, _, _)) => clamp_to_screen(d, cx - w / 2.0, y, w, h),
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
        let (w, h) = (r.width as f64, r.height as f64);
        let cx = rect.x + rect.w / 2.0;
        let (mut x, y) = (cx - w / 2.0, rect.y - h - BAR_GAP);
        if let Some((d, _, _)) = screen_of(&p.displays, cx, rect.y + rect.h / 2.0) {
            x = clamp_to_screen(d, x, y, w, h).0;
        }
        if p.card.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, x, y).is_ok() {
            p.card_open = true;
        }
    });
}


/// 네이티브 메뉴의 카드 자리 식별자. 표시 전 사용자 뷰로 바꾼다.
pub const MENU_CARD_TITLE: &str = "UsageMeter";

thread_local! {
    static MENU_CARD: RefCell<Option<objc2::rc::Retained<objc2_app_kit::NSImageView>>> = const { RefCell::new(None) };
}

/// Swift의 NSMenuItem.view와 같이 카드·차트를 메뉴 안에 넣는다.
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
        let (w, h) = (r.width as f64, r.height as f64);
        let (mut x, y) = (cx - w / 2.0, rect.y - h - BAR_GAP);
        if let Some((d, _, _)) = screen { x = clamp_to_screen(d, x, y, w, h).0; }
        let _ = p.card.show(r.pixmap.data(), r.pixmap.width(), r.pixmap.height(), scale as f64, x, y);
    });
    MENU_CARD.with(|c| {
        if let Some(view) = c.borrow().as_ref() { paint_menu_card(view, st); }
    });
}
