//! 앱 상태: 설정(주·화면별), 사용량 스냅샷, 이력 DB. 저장 위치와 Swift 버전 가져오기.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use um_core::history::HistoryDb;
use um_core::model::UsageSnapshot;
use um_core::settings::Settings;

/// 개발 중에는 Swift 버전과 섞이지 않게 따로 저장한다(번들 id도 `com.usagemeter.app.rs-dev`).
const DATA_DIR_NAME: &str = "UsageMeter-rs";

pub struct AppState {
    pub dir: PathBuf,
    pub main: Settings,
    /// 화면 id → 그 화면 테두리 설정(테두리 필드만 의미 있음, 나머지는 `main`을 따른다).
    pub displays: BTreeMap<String, Settings>,
    pub main_display_id: Option<String>,
    pub disabled_displays: Vec<String>,
    pub snapshots: BTreeMap<String, UsageSnapshot>,
    /// 이력에서 되살린 값(아직 실시간 조회 전)인 AI.
    pub from_history: Vec<String>,
    pub hide_overlay: bool,
    pub history: Option<HistoryDb>,
}

impl AppState {
    pub fn load() -> Self {
        let dir = data_dir();
        let _ = std::fs::create_dir_all(dir.join("displays"));
        let first_run = !dir.join("settings.json").exists();
        let mut st = AppState {
            dir: dir.clone(),
            main: Settings::default(),
            displays: BTreeMap::new(),
            main_display_id: None,
            disabled_displays: vec![],
            snapshots: BTreeMap::new(),
            from_history: vec![],
            hide_overlay: false,
            history: None,
        };
        if first_run {
            st.import_swift();
        } else {
            st.read_files();
        }
        st.main.apply_platform_rules(cfg!(windows));
        for s in st.displays.values_mut() {
            s.apply_platform_rules(cfg!(windows));
        }
        st.hide_overlay = st.main.hide_overlay_on_launch;
        st.history = HistoryDb::open(dir.join("history.db")).map_err(|e| tracing::warn!("이력 DB 열기 실패: {e}")).ok();
        if first_run {
            st.import_swift_history();
        }
        st.save();
        st
    }

    /// 첫 실행: Swift 버전 설정을 가져온다(계획서 7장). 원본은 건드리지 않는다.
    fn import_swift(&mut self) {
        let Some(path) = um_core::settings::default_swift_plist_path().filter(|p| p.exists()) else { return };
        match um_core::settings::import_from_plist(&path) {
            Ok(imp) => {
                if let Some(cur) = imp.usable_current() {
                    self.main = Settings::from_snapshot(cur);
                }
                for (id, snap) in &imp.displays {
                    self.displays.insert(id.clone(), Settings::from_snapshot(snap));
                }
                self.main_display_id = imp.main_display_uuid.clone();
                self.disabled_displays = imp.disabled_displays.clone();
                for w in &imp.warnings {
                    tracing::warn!("Swift 설정 가져오기: {w}");
                }
            }
            Err(e) => tracing::warn!("Swift 설정 가져오기 실패: {e}"),
        }
    }

    fn import_swift_history(&mut self) {
        let Some(db) = self.history.as_mut() else { return };
        let Some(home) = directories::BaseDirs::new() else { return };
        let p = home.data_dir().join("UsageMeter").join("history.json");
        if p.exists() {
            match db.import_swift_history_json(&p) {
                Ok(s) => tracing::info!("Swift 이력 가져옴: 표본 {}개", s.samples),
                Err(e) => tracing::warn!("Swift 이력 가져오기 실패: {e}"),
            }
        }
    }

    fn read_files(&mut self) {
        if let Some(s) = read_json(&self.dir.join("settings.json")) {
            self.main = s;
        }
        if let Ok(rd) = std::fs::read_dir(self.dir.join("displays")) {
            for e in rd.flatten() {
                let p = e.path();
                if let (Some(id), Some(s)) = (p.file_stem().and_then(|x| x.to_str()), read_json(&p)) {
                    self.displays.insert(id.to_string(), s);
                }
            }
        }
        if let Ok(t) = std::fs::read_to_string(self.dir.join("displays.json"))
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(&t)
        {
            self.main_display_id = v["main"].as_str().map(String::from);
            self.disabled_displays = v["disabled"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        }
    }

    pub fn save(&self) {
        if let Ok(t) = self.main.to_json() {
            let _ = std::fs::write(self.dir.join("settings.json"), t);
        }
        for (id, s) in &self.displays {
            if let Ok(t) = s.to_json() {
                let _ = std::fs::write(self.dir.join("displays").join(format!("{id}.json")), t);
            }
        }
        let v = serde_json::json!({ "main": self.main_display_id, "disabled": self.disabled_displays });
        let _ = std::fs::write(self.dir.join("displays.json"), v.to_string());
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

fn data_dir() -> PathBuf {
    directories::BaseDirs::new().map(|b| b.data_dir().join(DATA_DIR_NAME)).unwrap_or_else(|| PathBuf::from(DATA_DIR_NAME))
}
