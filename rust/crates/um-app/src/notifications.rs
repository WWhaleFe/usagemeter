//! 사용량·업데이트 알림(Swift NotificationManager.swift:18-51, UpdateChecker.swift:228-257).

use tauri::AppHandle;
use um_core::i18n::{tf, tr};
use um_core::model::{UsageSnapshot, UsageStatus};

pub fn usage(app: &AppHandle, id: &str, snap: &UsageSnapshot) {
    let st = crate::shared(app);
    let mut st = st.lock().unwrap();
    if !st.main.notify_enabled || snap.status != UsageStatus::Ok { return; }
    let thresholds = st.main.notify_thresholds.iter().map(|t| *t as i32).collect();
    for (threshold, key) in um_core::notify_rules::due_alerts(id, snap.remaining_ratio, snap.reset_at, &thresholds, &st.sent_notifications) {
        let name = um_providers::spec::spec(id).map(|s| s.name).unwrap_or(id);
        let title = tr("notify.title", st.main.language);
        let body = tf("notify.body", st.main.language, name).replace("%d", &threshold.to_string());
        if fire(app, &key, &title, &body) { st.sent_notifications.insert(key); }
    }
    st.save();
}

pub fn update(app: &AppHandle, version: &str) {
    if cfg!(feature = "store") { return; }
    let st = crate::shared(app);
    let mut st = st.lock().unwrap();
    let key = format!("update|{version}");
    if st.sent_notifications.contains(&key) { return; }
    let body = tf("update.available", st.main.language, &format!("v{version}"));
    if fire(app, &key, "UsageMeter", &body) {
        st.sent_notifications.insert(key);
        st.save();
    }
}

#[cfg(target_os = "macos")]
pub fn init() {
    use objc2_user_notifications::{UNAuthorizationOptions, UNUserNotificationCenter};
    if objc2_foundation::NSBundle::mainBundle().bundleIdentifier().is_none() { return; }
    let completion = block2::RcBlock::new(|_, _: *mut objc2_foundation::NSError| {});
    UNUserNotificationCenter::currentNotificationCenter().requestAuthorizationWithOptions_completionHandler(
        UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound, &completion);
}

#[cfg(target_os = "macos")]
fn fire(_app: &AppHandle, key: &str, title: &str, body: &str) -> bool {
    use objc2_foundation::{NSBundle, NSString};
    use objc2_user_notifications::{UNMutableNotificationContent, UNNotificationRequest, UNNotificationSound, UNUserNotificationCenter};
    if NSBundle::mainBundle().bundleIdentifier().is_none() { return false; }
    let content = UNMutableNotificationContent::new();
    content.setTitle(&NSString::from_str(title));
    content.setBody(&NSString::from_str(body));
    content.setSound(Some(&UNNotificationSound::defaultSound()));
    let request = UNNotificationRequest::requestWithIdentifier_content_trigger(&NSString::from_str(key), &content, None);
    UNUserNotificationCenter::currentNotificationCenter().addNotificationRequest_withCompletionHandler(&request, None);
    true
}

#[cfg(windows)]
fn fire(app: &AppHandle, _key: &str, title: &str, body: &str) -> bool {
    um_platform::toast::show(&app.config().identifier, title, body)
        .map_err(|e| tracing::warn!("알림 보내기 실패: {e}"))
        .is_ok()
}

#[cfg(not(any(target_os = "macos", windows)))]
fn fire(_: &AppHandle, _: &str, _: &str, _: &str) -> bool { false }
