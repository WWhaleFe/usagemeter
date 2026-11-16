//! 메뉴바 정보 카드·호버 패널·상태 글. 색은 계산하지 않고 데이터만 낸다.
//! (Swift StatusBarController.swift `buildAIInfos`/`infoLines`/`updateHoverInfo`/`updateStatusIcon`,
//! MenuInfoCardView.swift)

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};

use crate::history::PaceInfo;
use crate::i18n::{tf, tr};
use crate::model::{UsageSnapshot, UsageSubItem};
use crate::settings::Settings;
use crate::types::AppLanguage;

/// 정보 줄 하나. `dot_item`이 있으면 그 세부 항목 색의 점을 앞에 찍는다. (Swift `MenuInfoLine`)
#[derive(Debug, Clone, PartialEq)]
pub struct InfoLine {
    pub text: String,
    pub dot_item: Option<String>,
}

/// AI별 카드 한 섹션. (Swift `MenuAIInfo`, 색은 호출자가 id로 정한다)
#[derive(Debug, Clone, PartialEq)]
pub struct InfoSection {
    pub id: String,
    pub name: String,
    pub plan: Option<String>,
    pub lines: Vec<InfoLine>,
}

/// 활성 AI 한 개: (id, 표시 이름, 스냅샷).
pub type ActiveAi<'a> = (&'a str, &'a str, &'a UsageSnapshot);

/// "M/d HH:mm" 로컬 표기. (Swift `timeFmt` :968)
fn fmt_time<Tz: TimeZone>(t: DateTime<Utc>, tz: &Tz) -> String {
    let l = t.with_timezone(tz);
    format!("{}/{} {:02}:{:02}", l.month(), l.day(), l.hour(), l.minute())
}

/// 반올림한 정수 퍼센트. (Swift `Int((ratio * 100).rounded())`)
fn pct(ratio: f64) -> i64 {
    (ratio * 100.0).round() as i64
}

/// 초 -> "H시간 M분", 시간이 0이면 "M분". (Swift `countdownBare` :331 본문)
fn hm(secs: i64, lang: AppLanguage) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    let min = tr("cd.min", lang);
    if h > 0 {
        format!("{h}{} {m}{min}", tr("cd.hour", lang))
    } else {
        format!("{m}{min}")
    }
}

/// 카운트다운 "N시간 M분 후". 0 이하면 "곧". (Swift `countdownString` :316)
fn countdown(to: DateTime<Utc>, now: DateTime<Utc>, lang: AppLanguage) -> String {
    // Swift Int(timeIntervalSinceNow): 0 쪽으로 버림
    let secs = (to - now).num_milliseconds() / 1000;
    if secs <= 0 {
        return tr("cd.now", lang);
    }
    tf("cd.afterFmt", lang, &hm(secs, lang))
}

/// 소진 예측 줄. 리셋 시각이 있으면 전/후 판정을 덧붙인다. (Swift `paceString` :326)
fn pace_string(p: PaceInfo, reset_at: Option<DateTime<Utc>>, now: DateTime<Utc>, lang: AppLanguage) -> String {
    let now_s = now.timestamp_millis() as f64 / 1000.0;
    // countdownBare: max(0, Int(...))
    let secs = ((p.depletion - now_s) as i64).max(0);
    let mut out = format!("{}: {}", tr("info.pace", lang), tf("pace.depleteFmt", lang, &hm(secs, lang)));
    if let Some(r) = reset_at {
        let r_s = r.timestamp_millis() as f64 / 1000.0;
        out.push(' ');
        out.push_str(&tr(if p.depletion < r_s { "pace.warnReset" } else { "pace.okReset" }, lang));
    }
    out
}

/// AI 하나의 정보 줄들(표시 옵션 반영). (Swift `infoLines` :271)
fn info_lines<Tz: TimeZone>(
    s: &Settings,
    id: &str,
    snap: &UsageSnapshot,
    pace: Option<PaceInfo>,
    now: DateTime<Utc>,
    tz: &Tz,
) -> Vec<InfoLine> {
    let lang = s.language;
    let mut lines = Vec::new();
    // 리셋 시각 + 카운트다운 접미사
    let reset_suffix = |date: Option<DateTime<Utc>>| -> String {
        let Some(r) = date else { return String::new() };
        let mut out = String::new();
        if s.menu_show_reset {
            out += &format!("   ·   {} {}", tr("menu.lineReset", lang), fmt_time(r, tz));
        }
        if s.menu_show_countdown {
            out += &format!("   ·   {}", countdown(r, now, lang));
        }
        out
    };
    // 5시간 -> 주간 -> 모델별 한도 순서
    let colored = s.sub_item_split_ids.contains(id);
    for item in snap.sub_items() {
        if s.sub_item_menu_hidden.contains(&UsageSubItem::settings_key(id, &item.id)) {
            continue;
        }
        let kind_on = if item.model_label.is_some() {
            s.menu_show_opus
        } else if item.is_five_hour {
            s.menu_show_5h
        } else {
            s.menu_show_weekly
        };
        if !kind_on {
            continue;
        }
        // subItemName :919
        let kind = tr(if item.is_five_hour { "menu.line5h" } else { "menu.lineWeekly" }, lang);
        let name = match &item.model_label {
            Some(l) => format!("{l} {kind}"),
            None => kind,
        };
        let text = format!("{name}  {}%{}", pct(item.remaining_ratio), reset_suffix(item.reset_at));
        lines.push(InfoLine { text, dot_item: colored.then(|| item.id.clone()) });
    }
    if s.menu_show_pace
        && let Some(p) = pace
    {
        lines.push(InfoLine { text: pace_string(p, snap.reset_at, now, lang), dot_item: None });
    }
    if s.menu_show_updated {
        let text = format!("{}  {}", tr("menu.lineUpdated", lang), fmt_time(snap.last_updated, tz));
        lines.push(InfoLine { text, dot_item: None });
    }
    if lines.is_empty() {
        lines.push(InfoLine { text: tr("menu.pickInfo", lang), dot_item: None });
    }
    lines
}

/// 드롭다운·호버 공용 섹션(provider_order 순). (Swift `buildAIInfos` :256)
pub fn info_sections<Tz: TimeZone>(
    s: &Settings,
    active: &[ActiveAi],
    pace: impl Fn(&str) -> Option<PaceInfo>,
    now: DateTime<Utc>,
    tz: &Tz,
) -> Vec<InfoSection> {
    let mut v: Vec<&ActiveAi> = active.iter().collect();
    v.sort_by_key(|a| s.order_index(a.0)); // 안정 정렬
    v.into_iter()
        .map(|&(id, name, snap)| InfoSection {
            id: id.to_string(),
            name: name.to_string(),
            plan: snap.plan.clone(), // 빈 문자열 숨김은 그리는 쪽 몫(Swift 뷰가 isEmpty 검사)
            lines: info_lines(s, id, snap, pace(id), now, tz),
        })
        .collect()
}

/// AI가 없을 때 호버 글. (Swift `hover.noAI` :881)
pub fn no_ai_text(lang: AppLanguage) -> String {
    tr("hover.noAI", lang)
}

/// 호버 글. AI 이름, 줄들, AI 사이 빈 줄, 점 있는 줄은 "● " 접두. (Swift `updateHoverInfo` :881)
pub fn hover_text<Tz: TimeZone>(
    s: &Settings,
    active: &[ActiveAi],
    pace: impl Fn(&str) -> Option<PaceInfo>,
    now: DateTime<Utc>,
    tz: &Tz,
) -> String {
    let secs = info_sections(s, active, pace, now, tz);
    if secs.is_empty() {
        return no_ai_text(s.language);
    }
    let mut out = String::new();
    for (i, sec) in secs.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        out.push_str(&sec.name);
        for l in &sec.lines {
            out.push('\n');
            if l.dot_item.is_some() {
                out.push_str("● ");
            }
            out.push_str(&l.text);
        }
    }
    out
}

/// 메뉴바 % 표시: 선택된 활성 AI를 provider_order 순으로 (id, "NN%"). (Swift `updateStatusIcon` :861)
/// 링 아이콘 기준 AI 선택(:850)은 포함하지 않는다.
pub fn status_percents(s: &Settings, active: &[ActiveAi]) -> Vec<(String, String)> {
    let mut v: Vec<&ActiveAi> = active.iter().filter(|a| s.menu_bar_percent_ids.contains(a.0)).collect();
    v.sort_by_key(|a| s.order_index(a.0));
    v.into_iter().map(|a| (a.0.to_string(), format!("{}%", pct(a.2.remaining_ratio)))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{UsageModelBucket, UsageStatus};
    use chrono::FixedOffset;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 5, 3, 0, 0).unwrap()
    }
    fn tz() -> FixedOffset {
        FixedOffset::east_opt(9 * 3600).unwrap()
    }
    fn snap(id: &str, r: f64) -> UsageSnapshot {
        let mut s = UsageSnapshot::failed(id, UsageStatus::Ok, now());
        s.remaining_ratio = r;
        s
    }
    fn st(lang: AppLanguage) -> Settings {
        let mut s = Settings { language: lang, ..Settings::default() };
        s.menu_show_5h = true;
        s.menu_show_weekly = true;
        s.menu_show_opus = true;
        s.menu_show_reset = true;
        s.menu_show_countdown = true;
        s.menu_show_pace = false;
        s.menu_show_updated = false;
        s.sub_item_menu_hidden.clear();
        s.sub_item_split_ids.clear();
        s
    }
    fn no_pace(_: &str) -> Option<PaceInfo> {
        None
    }

    #[test]
    fn countdown_boundaries() {
        let l = AppLanguage::English;
        let at = |sec: i64| countdown(now() + chrono::Duration::seconds(sec), now(), l);
        assert_eq!(at(0), tr("cd.now", l));
        assert_eq!(at(-5), tr("cd.now", l));
        assert_eq!(at(59), tf("cd.afterFmt", l, "0m")); // 59초는 0분
        assert_eq!(at(60), tf("cd.afterFmt", l, "1m"));
        assert_eq!(at(3600), tf("cd.afterFmt", l, "1h 0m"));
        assert_eq!(at(3600 + 34 * 60), tf("cd.afterFmt", l, "1h 34m"));
    }

    #[test]
    fn lines_with_reset_and_countdown_ko_en() {
        let mut sn = snap("claude", 0.955);
        sn.reset_at = Some(now() + chrono::Duration::minutes(34));
        sn.secondary_ratio = Some(0.5);
        sn.secondary_reset_at = Some(now() + chrono::Duration::hours(30));
        sn.plan = Some("Max (5x)".into());
        for lang in [AppLanguage::Korean, AppLanguage::English] {
            let s = st(lang);
            let secs = info_sections(&s, &[("claude", "Claude", &sn)], no_pace, now(), &tz());
            assert_eq!(secs.len(), 1);
            assert_eq!(secs[0].plan.as_deref(), Some("Max (5x)"));
            let k5 = tr("menu.line5h", lang);
            let kw = tr("menu.lineWeekly", lang);
            let rs = tr("menu.lineReset", lang);
            // 03:00Z+34분 = 12:34 KST, 30시간 뒤 = 1/6 18:00 KST
            assert_eq!(
                secs[0].lines[0].text,
                format!("{k5}  96%   ·   {rs} 1/5 12:34   ·   {}", tf("cd.afterFmt", lang, &format!("34{}", tr("cd.min", lang))))
            );
            assert!(secs[0].lines[1].text.starts_with(&format!("{kw}  50%   ·   {rs} 1/6 18:00")));
            assert_eq!(secs[0].lines.len(), 2);
        }
    }

    #[test]
    fn toggles_off_and_empty_hint() {
        let sn = snap("claude", 0.5);
        let mut s = st(AppLanguage::English);
        s.menu_show_reset = false;
        s.menu_show_countdown = false;
        let sec = &info_sections(&s, &[("claude", "Claude", &sn)], no_pace, now(), &tz())[0];
        assert_eq!(sec.lines[0].text, format!("{}  50%", tr("menu.line5h", s.language)));
        s.menu_show_5h = false;
        s.menu_show_weekly = false;
        s.menu_show_opus = false;
        let sec = &info_sections(&s, &[("claude", "Claude", &sn)], no_pace, now(), &tz())[0];
        assert_eq!(sec.lines, vec![InfoLine { text: tr("menu.pickInfo", s.language), dot_item: None }]);
        s.menu_show_updated = true;
        let sec = &info_sections(&s, &[("claude", "Claude", &sn)], no_pace, now(), &tz())[0];
        assert_eq!(sec.lines[0].text, format!("{}  1/5 12:00", tr("menu.lineUpdated", s.language)));
    }

    #[test]
    fn hidden_sub_item_and_model_buckets() {
        let mut sn = snap("claude", 0.9);
        sn.secondary_ratio = Some(0.8);
        sn.model_buckets = Some(vec![UsageModelBucket {
            label: "Fable".into(),
            remaining_ratio: 0.25,
            reset_at: None,
            is_five_hour: false,
        }]);
        let mut s = st(AppLanguage::English);
        s.menu_show_reset = false;
        s.menu_show_countdown = false;
        s.sub_item_menu_hidden.insert("claude/weekly".into());
        s.sub_item_split_ids.insert("claude".into());
        let sec = &info_sections(&s, &[("claude", "Claude", &sn)], no_pace, now(), &tz())[0];
        let w = tr("menu.lineWeekly", s.language);
        assert_eq!(sec.lines.len(), 2);
        assert_eq!(sec.lines[1].text, format!("Fable {w}  25%"));
        assert_eq!(sec.lines[1].dot_item.as_deref(), Some("model:Fable:weekly"));
        assert_eq!(sec.lines[0].dot_item.as_deref(), Some("5h"));
        // 모델 항목은 opus 토글을 따른다
        s.menu_show_opus = false;
        let sec = &info_sections(&s, &[("claude", "Claude", &sn)], no_pace, now(), &tz())[0];
        assert_eq!(sec.lines.len(), 1);
    }

    #[test]
    fn pace_before_and_after_reset() {
        let mut sn = snap("claude", 0.5);
        sn.reset_at = Some(now() + chrono::Duration::hours(2));
        let mut s = st(AppLanguage::English);
        s.menu_show_5h = false;
        s.menu_show_pace = true;
        let l = s.language;
        let now_s = now().timestamp() as f64;
        let mk = |d: f64| move |_: &str| Some(PaceInfo { depletion: d, rate_per_hour: 0.1 });
        let before = info_sections(&s, &[("claude", "C", &sn)], mk(now_s + 3600.0 + 5.0), now(), &tz());
        assert_eq!(
            before[0].lines[0].text,
            format!("{}: {} {}", tr("info.pace", l), tf("pace.depleteFmt", l, "1h 0m"), tr("pace.warnReset", l))
        );
        let after = info_sections(&s, &[("claude", "C", &sn)], mk(now_s + 3.0 * 3600.0), now(), &tz());
        assert!(after[0].lines[0].text.ends_with(&tr("pace.okReset", l)));
        // 리셋 시각이 없으면 접미 없음, 과거 예측은 0m
        sn.reset_at = None;
        let none = info_sections(&s, &[("claude", "C", &sn)], mk(now_s - 100.0), now(), &tz());
        assert_eq!(none[0].lines[0].text, format!("{}: {}", tr("info.pace", l), tf("pace.depleteFmt", l, "0m")));
    }

    #[test]
    fn hover_and_order_and_no_ai() {
        let a = snap("claude", 0.5);
        let b = snap("codex", 0.25);
        let mut s = st(AppLanguage::English);
        s.menu_show_reset = false;
        s.menu_show_countdown = false;
        s.provider_order = vec!["codex".into(), "claude".into()];
        s.sub_item_split_ids.insert("claude".into());
        let l5 = tr("menu.line5h", s.language);
        let t = hover_text(&s, &[("claude", "Claude", &a), ("codex", "Codex", &b)], no_pace, now(), &tz());
        assert_eq!(t, format!("Codex\n{l5}  25%\n\nClaude\n● {l5}  50%"));
        assert_eq!(hover_text(&s, &[], no_pace, now(), &tz()), tr("hover.noAI", s.language));
        assert_eq!(no_ai_text(AppLanguage::Korean), tr("hover.noAI", AppLanguage::Korean));
    }

    #[test]
    fn status_percent_order_and_selection() {
        let a = snap("claude", 0.505);
        let b = snap("codex", 0.25);
        let c = snap("gemini", 0.9);
        let mut s = st(AppLanguage::English);
        s.provider_order = vec!["codex".into(), "gemini".into(), "claude".into()];
        s.menu_bar_percent_ids = ["claude".to_string(), "codex".to_string()].into();
        let act = [("claude", "Claude", &a), ("codex", "Codex", &b), ("gemini", "G", &c)];
        assert_eq!(
            status_percents(&s, &act),
            vec![("codex".into(), "25%".into()), ("claude".into(), "51%".into())]
        );
        s.menu_bar_percent_ids.clear();
        assert!(status_percents(&s, &act).is_empty());
    }
}
