// S1: 트레이만 띄운 Tauri 앱의 평상시 메모리, 웹뷰 창 열고 닫은 뒤 회수 확인.
// 시간표(초): 0 시작 → 60 창 열기 → 90 창 닫기(파기) → 180 창 열기 → 210 닫기 → 계속 대기.
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, tray::TrayIconBuilder};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("S1")
                .build(app)?;
            let h = app.handle().clone();
            std::thread::spawn(move || {
                let step = |at: u64, open: bool, h: &tauri::AppHandle| {
                    std::thread::sleep(Duration::from_secs(at));
                    let h2 = h.clone();
                    h.run_on_main_thread(move || {
                        if open {
                            let _ = WebviewWindowBuilder::new(&h2, "settings", WebviewUrl::App("index.html".into()))
                                .title("S1").inner_size(760.0, 880.0).build();
                            eprintln!("[S1] open");
                        } else if let Some(w) = h2.get_webview_window("settings") {
                            let _ = w.destroy();
                            eprintln!("[S1] destroy");
                        }
                    }).unwrap();
                };
                step(60, true, &h); step(30, false, &h); step(90, true, &h); step(30, false, &h);
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("tauri")
        .run(|_, e| if let tauri::RunEvent::ExitRequested { api, .. } = e { api.prevent_exit() });
}
