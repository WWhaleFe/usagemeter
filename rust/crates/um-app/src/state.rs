//! 앱 상태: 설정(주·화면별), 사용량 스냅샷, 이력 DB. 저장 위치와 Swift 버전 가져오기.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use um_core::history::HistoryDb;
use um_core::model::UsageSnapshot;
use um_core::settings::{SavedPreset, Settings};
use um_providers::state::ProviderState;

/// 정식판은 Swift 폴더에 Rust 파일만 추가한다. Swift 설정·history.json은 읽기만 한다.
#[cfg(feature = "production")]
const DATA_DIR_NAME: &str = "UsageMeter";
/// 개발 중에는 Swift 버전과 섞이지 않게 따로 저장한다.
#[cfg(not(feature = "production"))]
const DATA_DIR_NAME: &str = "UsageMeter-rs";

pub struct AppState {
    pub dir: PathBuf,
    pub sent_notifications: std::collections::BTreeSet<String>,
    pub main: Settings,
    /// 화면 id → 그 화면 테두리 설정(테두리 필드만 의미 있음, 나머지는 `main`을 따른다).
    pub displays: BTreeMap<String, Settings>,
    pub main_display_id: Option<String>,
    display_widths: BTreeMap<String, f64>,
    /// 지금 연결된 화면 id(화면 순서). `sync_displays`가 채운다.
    pub connected_displays: Vec<String>,
    pub disabled_displays: Vec<String>,
    /// 테두리·트레이에 그릴 스냅샷(`rebuild_snapshots`가 만든다).
    pub snapshots: BTreeMap<String, UsageSnapshot>,
    /// AI별 로그인·조회 상태(Swift ProviderManager.states).
    pub providers: BTreeMap<String, ProviderState>,
    /// 이력에서 되살린 마지막 값. 그 AI의 첫 실시간 결과가 오면 지운다.
    pub seeds: BTreeMap<String, UsageSnapshot>,
    pub hide_overlay: bool,
    pub history: Option<HistoryDb>,
    /// 저장 프리셋(최대 10개)과 "기본 상태"(설정 창에서 저장·되돌리기).
    pub presets: Vec<SavedPreset>,
    pub default_snapshot: Option<um_core::settings::SettingsSnapshot>,
    pub update: crate::update::UpdateState,
    pub download: crate::update::DownloadState,
}

impl AppState {
    fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            sent_notifications: Default::default(),
            main: Settings::default(),
            displays: BTreeMap::new(),
            main_display_id: None,
            display_widths: BTreeMap::new(),
            connected_displays: Vec::new(),
            disabled_displays: vec![],
            snapshots: BTreeMap::new(),
            providers: BTreeMap::new(),
            seeds: BTreeMap::new(),
            hide_overlay: false,
            history: None,
            presets: vec![],
            default_snapshot: None,
            update: Default::default(),
            download: Default::default(),
        }
    }

    pub fn load() -> Self {
        let dir = data_dir();
        let _ = std::fs::create_dir_all(dir.join("displays"));
        let first_run = !dir.join("settings.json").exists();
        let mut st = Self::new(dir.clone());
        if first_run {
            st.import_swift();
        } else {
            st.read_files();
        }
        #[cfg(windows)]
        {
            // 이전 버전이 displays.json에 남긴 장치 경로도 새 ID에 맞춘다.
            use um_platform::display::file_id;
            st.main_display_id = st.main_display_id.map(|id| file_id(&id));
            st.disabled_displays = st.disabled_displays.into_iter().map(|id| file_id(&id)).collect();
            st.display_widths = st.display_widths.into_iter().map(|(id, w)| (file_id(&id), w)).collect();
        }
        st.main.apply_platform_rules(cfg!(windows));
        for s in st.displays.values_mut() {
            s.apply_platform_rules(cfg!(windows));
        }
        st.hide_overlay = st.main.hide_overlay_on_launch;
        st.history = HistoryDb::open(dir.join("history.db"))
            .map_err(|e| tracing::warn!("이력 DB 열기 실패: {e}"))
            .ok();
        if first_run {
            st.import_swift_history();
        }
        st.save();
        st
    }

    /// 첫 실행: Swift 버전 설정을 가져온다(계획서 7장). 원본은 건드리지 않는다.
    fn import_swift(&mut self) {
        let Some(path) = um_core::settings::default_swift_plist_path().filter(|p| p.exists())
        else {
            return;
        };
        match um_core::settings::import_from_plist(&path) {
            Ok(imp) => {
                if let Some(cur) = imp.usable_current() {
                    self.main = Settings::from_snapshot(cur);
                }
                for (id, snap) in &imp.displays {
                    self.displays
                        .insert(id.clone(), Settings::from_snapshot(snap));
                }
                self.main_display_id = imp.main_display_uuid.clone();
                self.disabled_displays = imp.disabled_displays.clone();
                self.presets = imp.presets.clone();
                // 세부 항목을 기억해 두면 로그아웃 중에도 설정 탭에서 항목을 고칠 수 있다(Swift knownSubItems).
                for (id, items) in &imp.known_sub_items {
                    self.providers.entry(id.clone()).or_default().known_items =
                        items.iter().map(|k| um_providers::state::KnownItem { id: k.id.clone(), ratio: k.ratio }).collect();
                }
                self.default_snapshot = imp.default_settings.clone();
                for w in &imp.warnings {
                    tracing::warn!("Swift 설정 가져오기: {w}");
                }
            }
            Err(e) => tracing::warn!("Swift 설정 가져오기 실패: {e}"),
        }
    }

    fn import_swift_history(&mut self) {
        let Some(db) = self.history.as_mut() else {
            return;
        };
        let Some(home) = directories::BaseDirs::new() else {
            return;
        };
        let p = home.data_dir().join("UsageMeter").join("history.json");
        if p.exists() {
            match db.import_swift_history_json(&p) {
                Ok(s) => tracing::info!("Swift 이력 가져옴: 표본 {}개", s.samples),
                Err(e) => tracing::warn!("Swift 이력 가져오기 실패: {e}"),
            }
        }
    }

    fn read_files(&mut self) {
        self.sent_notifications = std::fs::read_to_string(self.dir.join("notifications.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        if let Some(s) = read_json(&self.dir.join("settings.json")) {
            self.main = s;
        }
        if let Ok(rd) = std::fs::read_dir(self.dir.join("displays")) {
            for e in rd.flatten() {
                let p = e.path();
                if let (Some(id), Some(s)) = (p.file_stem().and_then(|x| x.to_str()), read_json(&p))
                {
                    self.displays.insert(id.to_string(), s);
                }
            }
        }
        if let Ok(t) = std::fs::read_to_string(self.dir.join("known_items.json")) {
            let known = serde_json::from_str::<BTreeMap<String, Vec<um_providers::state::KnownItem>>>(&t)
                .or_else(|_| serde_json::from_str::<BTreeMap<String, Vec<(String, f64)>>>(&t).map(|v| {
                    v.into_iter().map(|(id, items)| (id, items.into_iter().map(|(id, ratio)| um_providers::state::KnownItem { id, ratio }).collect())).collect()
                }));
            if let Ok(known) = known {
                for (id, items) in known { self.providers.entry(id).or_default().known_items = items; }
            }
        }
        if let Ok(t) = std::fs::read_to_string(self.dir.join("presets.json"))
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&t)
        {
            self.presets = serde_json::from_value(v["presets"].clone()).unwrap_or_default();
            self.default_snapshot = serde_json::from_value(v["default"].clone()).ok().flatten();
        }
        if let Ok(t) = std::fs::read_to_string(self.dir.join("displays.json"))
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&t)
        {
            self.main_display_id = v["main"].as_str().map(String::from);
            self.display_widths = serde_json::from_value(v["widths"].clone()).unwrap_or_default();
            self.disabled_displays = v["disabled"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
        }
    }

    pub fn save(&self) {
        if let Ok(t) = serde_json::to_string(&self.sent_notifications) {
            let _ = std::fs::write(self.dir.join("notifications.json"), t);
        }
        if let Ok(t) = self.main.to_json() {
            let _ = std::fs::write(self.dir.join("settings.json"), t);
        }
        for (id, s) in &self.displays {
            if let Ok(t) = s.to_json() {
                let _ = std::fs::write(self.dir.join("displays").join(format!("{}.json", um_platform::display::file_id(id))), t);
            }
        }
        // Swift는 UserDefaults JSON을 사용한다. 가져온 스냅샷 형식을 그대로 보존한다(OverlaySettings.swift:1348-1357).
        let presets = serde_json::json!({"presets": self.presets, "default": self.default_snapshot});
        let _ = std::fs::write(self.dir.join("presets.json"), presets.to_string());
        let known: BTreeMap<_, _> = self.providers.iter().map(|(id, p)| (id, &p.known_items)).collect();
        if let Ok(t) = serde_json::to_string(&known) {
            let _ = std::fs::write(self.dir.join("known_items.json"), t);
        }
        let v =
            serde_json::json!({ "main": self.main_display_id, "disabled": self.disabled_displays, "widths": self.display_widths });
        let _ = std::fs::write(self.dir.join("displays.json"), v.to_string());
    }

    /// 첫 실행·새 화면 맞춤과 기존 화면 해상도 적응(Swift OverlaySettings.swift:985-1092).
    pub fn sync_displays(&mut self, ds: &[um_platform::display::DisplayInfo]) {
        use um_core::settings::{ScreenInfo, NotchInfo, device_fit, adapt_to_screen_change};
        if self.main_display_id.is_none() {
            self.main_display_id = ds.iter().find(|d| d.is_builtin).or_else(|| ds.first()).map(|d| d.id.clone());
        }
        self.connected_displays = ds.iter().map(|d| d.id.clone()).collect();
        for d in ds {
            let notch = d.notch.map(|n| NotchInfo { width: n.width, height: n.height });
            let old = self.display_widths.get(&d.id).copied();
            let is_main = self.main_display_id.as_deref() == Some(d.id.as_str());
            let known = is_main || self.displays.contains_key(&d.id);
            let s = if is_main { &mut self.main } else { self.displays.entry(d.id.clone()).or_insert_with(|| self.main.clone()) };
            if !known || (old.is_none() && !self.dir.join("display_fit_done").exists()) {
                device_fit(s, &ScreenInfo { width_pt: d.width_pt, notch });
            } else {
                adapt_to_screen_change(s, old.unwrap_or(0.0), d.width_pt, notch);
            }
            s.apply_platform_rules(cfg!(windows));
            self.display_widths.insert(d.id.clone(), d.width_pt);
        }
        if !ds.is_empty() { let _ = std::fs::write(self.dir.join("display_fit_done"), "1"); }
        self.save();
    }

    pub fn apply_snapshot(&mut self, id: &str, snap: UsageSnapshot) {
        self.seeds.remove(id);
        self.providers.entry(id.to_string()).or_default().apply(snap);
        self.rebuild_snapshots();
    }

    /// 그릴 스냅샷 = 로그인돼 정상 조회된 AI + 아직 실시간 결과가 없는 AI의 이력 값.
    pub fn rebuild_snapshots(&mut self) {
        let mut out = BTreeMap::new();
        for id in &self.main.provider_order {
            if let Some(s) = self.providers.get(id).and_then(|p| p.active_snapshot()) {
                out.insert(id.clone(), s.clone());
            } else if let Some(s) = self.seeds.get(id) {
                out.insert(id.clone(), s.clone());
            }
        }
        self.snapshots = out;
    }

    pub fn is_logged_in(&self, id: &str) -> bool {
        self.providers.get(id).is_some_and(|p| p.logged_in)
    }

    /// 이 화면에 쓸 설정: 주 설정(전역 값) + 그 화면의 테두리 값. 주 화면이면 주 설정 그대로.
    pub fn settings_for_display(&self, id: &str) -> Settings {
        let mut s = self.main.clone();
        if self.main_display_id.as_deref() != Some(id)
            && let Some(d) = self.displays.get(id)
        {
            s.copy_border_from(d);
        }
        s
    }
}

fn read_json(p: &Path) -> Option<Settings> {
    Settings::from_json(&std::fs::read_to_string(p).ok()?).ok()
}

pub fn data_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|b| b.data_dir().join(DATA_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(DATA_DIR_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use um_core::model::UsageStatus;
    use um_providers::state::KnownItem;

    #[test]
    fn 프리셋과_기억_항목과_알림키_재시작_복원() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("target").join("parity-tests").join(crate::menu::new_id());
        std::fs::create_dir_all(dir.join("displays")).unwrap();
        let mut st = AppState::new(dir.clone());
        st.presets.push(SavedPreset { id: "preset-1".into(), name: "테스트".into(), snapshot: st.main.to_snapshot() });
        st.default_snapshot = Some(st.main.to_snapshot());
        st.providers.entry("claude".into()).or_default().known_items = vec![KnownItem { id: "5h".into(), ratio: 0.4 }];
        st.sent_notifications.insert("claude|75|na".into());
        st.save();
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("known_items.json")).unwrap()).unwrap();
        assert_eq!(json["claude"][0]["id"], "5h");
        let mut loaded = AppState::new(dir.clone());
        loaded.read_files();
        assert_eq!(loaded.presets, st.presets);
        assert_eq!(loaded.default_snapshot, st.default_snapshot);
        assert_eq!(loaded.providers["claude"].known_items, st.providers["claude"].known_items);
        assert_eq!(loaded.sent_notifications, st.sent_notifications);
        // 이전 Rust 튜플 형식도 계속 읽는다.
        std::fs::write(dir.join("known_items.json"), r#"{"claude":[["weekly",0.2]]}"#).unwrap();
        loaded.read_files();
        assert_eq!(loaded.providers["claude"].known_items[0].id, "weekly");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn 첫_조회_실패면_이력_시드를_버린다() {
        for status in [UsageStatus::AuthExpired, UsageStatus::Unavailable("timeout".into()), UsageStatus::Stale] {
            let mut st = AppState::new(PathBuf::new());
            st.seeds.insert("claude".into(), UsageSnapshot::failed("claude", UsageStatus::Ok, chrono::Utc::now()));
            st.rebuild_snapshots();
            assert!(st.snapshots.contains_key("claude"));
            st.apply_snapshot("claude", UsageSnapshot::failed("claude", status, chrono::Utc::now()));
            assert!(!st.seeds.contains_key("claude"));
            assert!(!st.snapshots.contains_key("claude"));
        }
    }
}
