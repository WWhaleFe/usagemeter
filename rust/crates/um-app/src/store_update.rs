//! Windows Store판 업데이트. 새 버전이 있는지는 Microsoft Store에 묻고, 버전 번호는 GitHub
//! `usagemeter-windows`의 최신 공개 릴리스에서 읽는다(Store API는 새 버전 번호를 주지 않는다).
//! 설치는 자동 업데이트와 '지금 업데이트'가 같은 길을 쓴다: Store에 확인하고, 새 패키지를 조용히 받아 설치한 뒤 앱을 다시 연다.
//! '지금 업데이트'만 조용히 안 될 때 Store 확인 창을 띄우고, 그것도 실패하면 Store 상품 페이지를 연다.

use tauri::{AppHandle, Manager};
use um_platform::store::{self, Install};

use crate::update::{DownloadState, UpdateState, current_version};

/// Store 상품 페이지('지금 업데이트'가 앱 안에서 끝나지 못했을 때 연다).
pub const STORE_PAGE: &str = "ms-windows-store://pdp/?productid=9P0K09D9VQNM";

/// Store 확인 창의 주인 창. 열린 창이 없으면 0(창을 띄우지 않는 확인·조용한 설치는 그래도 된다).
fn owner(app: &AppHandle) -> isize {
    #[cfg(windows)]
    {
        let wins = app.webview_windows();
        let pick = wins.get("settings").or_else(|| wins.values().next());
        if let Some(h) = pick.and_then(|w| w.hwnd().ok()) {
            return h.0 as isize;
        }
    }
    let _ = app;
    0
}

/// 확인 결과: (상태, 화면에 보일 최신 버전).
pub fn check(app: &AppHandle) -> (UpdateState, Option<String>) {
    let github = match crate::update::check() {
        UpdateState::Available { version, .. } => Some(version),
        UpdateState::UpToDate => Some(current_version().to_string()),
        _ => None,
    };
    match store::has_update(owner(app)) {
        Ok(true) => {
            // GitHub에 아직 공개 릴리스가 없으면 번호 없이 "새 버전"으로 보인다.
            let version = github.clone().filter(|v| um_core::update::is_newer(v, current_version())).unwrap_or_default();
            // 최신 버전 칸에는 지금보다 새 번호만 둔다(같은 번호면 버튼이 꺼져 보이지 않게).
            let latest = (!version.is_empty()).then(|| version.clone());
            (UpdateState::Available { version, page: STORE_PAGE.into(), asset: None }, latest)
        }
        // Store에 아직 반영되지 않은 GitHub 릴리스는 받을 수 없으니 지금 버전을 최신으로 본다.
        Ok(false) => (UpdateState::UpToDate, Some(current_version().to_string())),
        Err(e) => {
            tracing::info!("Store 업데이트 확인 실패: {e}");
            (UpdateState::Failed, github)
        }
    }
}

/// Store에서 조용히 받아 설치한다(자동 업데이트와 '지금 업데이트' 공통). 설치되면 Store가 앱을 닫고 다시 연다.
/// `manual`('지금 업데이트')이면 조용히 안 될 때 Store 확인 창, 그것도 실패하면 Store 상품 페이지까지 간다.
pub fn install(app: &AppHandle, manual: bool) {
    {
        let st = crate::shared(app);
        let mut st = st.lock().unwrap();
        if st.download == DownloadState::Downloading { return; }
        st.download = DownloadState::Downloading;
        st.install_blocked = false;
    }
    crate::ipc::changed(app);
    let app = app.clone();
    std::thread::spawn(move || {
        store::restart_after_update();
        let r = store::install(owner(&app), manual);
        tracing::info!("Store 업데이트 설치: {r:?}");
        let mut blocked = false;
        let next = match r {
            // 보통은 그 전에 Store가 앱을 닫고, 설치가 끝나면 다시 연다.
            Ok(Install::Completed) => DownloadState::Done(Default::default()),
            Ok(Install::NoUpdate) => {
                crate::shared(&app).lock().unwrap().update = UpdateState::UpToDate;
                DownloadState::Idle
            }
            // Store의 앱 자동 업데이트가 꺼져 있거나 데이터 통신 제한 네트워크라 조용히 설치할 수 없다.
            Ok(Install::NeedsPrompt) => {
                blocked = true;
                DownloadState::Idle
            }
            Ok(Install::Canceled) => DownloadState::Idle,
            // 자동 업데이트는 다음 확인 때 다시 시도한다. '지금 업데이트'는 Store 페이지를 연다.
            Ok(Install::Failed) | Err(_) if manual => {
                let _ = open::that(STORE_PAGE);
                DownloadState::Failed
            }
            Ok(Install::Failed) | Err(_) => DownloadState::Idle,
        };
        {
            let st = crate::shared(&app);
            let mut st = st.lock().unwrap();
            st.download = next;
            st.install_blocked = blocked;
        }
        crate::ipc::changed(&app);
    });
}

/// 확인 결과를 반영한다: 새 버전이면 알림과 자동 설치(각각 설정이 켜져 있을 때).
/// `now`('지금 업데이트')면 자동 업데이트 설정과 관계없이 바로 설치한다.
pub fn after_check(app: &AppHandle, state: &UpdateState, now: bool) {
    let UpdateState::Available { version, .. } = state else { return };
    let (notify, auto) = {
        let st = crate::shared(app);
        let st = st.lock().unwrap();
        (st.main.auto_check_update, st.main.auto_install_update)
    };
    if notify && !now {
        crate::notifications::update(app, version);
    }
    if auto || now {
        install(app, now);
    }
}
