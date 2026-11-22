//! 장치 맞춤·해상도 변경 적응. OS 호출 없이 화면 정보를 인자로 받는 순수 함수.
//! (Swift `applyDeviceFit` :1078, `adaptToScreenChange` :1049)

use crate::types::{BorderCorner, NotchCorner};

use super::live::Settings;

/// 화면 노치 크기(pt). Swift `detectNotch`의 결과 `(width, height)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NotchInfo {
    pub width: f64,
    pub height: f64,
}

/// 테두리를 그리는 화면 정보.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenInfo {
    /// 화면 폭(pt).
    pub width_pt: f64,
    pub notch: Option<NotchInfo>,
}

/// 기준 화면 폭(맥북 Air 15의 논리 폭). (Swift :1085)
const REF_WIDTH: f64 = 1710.0;

/// "이 기기에 맞춤": 노치가 있으면 노치 감싸기를 켜고 감지 크기 + 상단 곡률
/// `round(22 x clamp(W/1710, 0.5, 2))`(최대 80)를 적용, 없으면 노치를 끈다.
/// 폭이 0 이하이면 기준 폭(1710)으로 계산한다.
pub fn device_fit(s: &mut Settings, screen: &ScreenInfo) {
    let Some(n) = screen.notch else {
        s.notch_enabled = false;
        return;
    };
    s.notch_enabled = true;
    s.notch_width = n.width;
    s.notch_height = n.height;
    let w = if screen.width_pt > 0.0 { screen.width_pt } else { REF_WIDTH };
    let scale = (w / REF_WIDTH).clamp(0.5, 2.0);
    let r = (22.0 * scale).round().clamp(0.0, 80.0);
    for c in [BorderCorner::TopLeft, BorderCorner::TopRight] {
        s.corner_radii.insert(c, r);
        s.menu_zone_radii.insert(c, r);
    }
}

/// 해상도·모니터 구성이 바뀌었을 때 노치·곡률을 새 화면에 맞춘다.
/// - 노치: 감지값으로 갱신(없으면 끔).
/// - 곡률: `new_w / old_w`(0.5~2.0로 제한)로 비례 스케일. 변화가 1% 이하이면 그대로.
///   구역 곡률은 0~80, 노치 곡률은 0~40으로 제한. `old_w <= 0`이면 곡률은 건드리지 않는다.
///
/// `new_w <= 0`(화면 미연결)이면 아무것도 하지 않는다. 호출 뒤 `new_w`를 다음 `old_w`로 기억할 것.
pub fn adapt_to_screen_change(s: &mut Settings, old_w: f64, new_w: f64, notch: Option<NotchInfo>) {
    if new_w <= 0.0 {
        return;
    }
    match notch {
        Some(n) => {
            s.notch_enabled = true;
            s.notch_width = n.width;
            s.notch_height = n.height;
        }
        None => s.notch_enabled = false,
    }
    if old_w <= 0.0 {
        return;
    }
    let scale = (new_w / old_w).clamp(0.5, 2.0);
    if (scale - 1.0).abs() <= 0.01 {
        return;
    }
    let scale_zone = |v: &mut f64| *v = (*v * scale).round().clamp(0.0, 80.0);
    s.corner_radii.values_mut().for_each(scale_zone);
    s.menu_zone_radii.values_mut().for_each(scale_zone);
    s.dock_zone_radii.values_mut().for_each(scale_zone);
    for c in NotchCorner::ALL {
        if let Some(v) = s.notch_radii.get_mut(&c) {
            *v = (*v * scale).round().clamp(0.0, 40.0);
        }
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    /// macOS 기본 곡률(위 22, 독 12). Windows 기본값은 곡률이 0이라 비례 계산을 볼 수 없다.
    fn mac_radii() -> Settings {
        let mut s = Settings::default();
        for c in [BorderCorner::TopLeft, BorderCorner::TopRight] {
            s.corner_radii.insert(c, 22.0);
            s.menu_zone_radii.insert(c, 22.0);
            s.dock_zone_radii.insert(c, 12.0);
        }
        s
    }

    const NOTCH: NotchInfo = NotchInfo { width: 200.0, height: 32.0 };

    #[test]
    fn 장치_맞춤_노치_1512() {
        let mut s = Settings::default();
        s.notch_enabled = false;
        device_fit(&mut s, &ScreenInfo { width_pt: 1512.0, notch: Some(NOTCH) });
        // 22 x 1512/1710 = 19.45 -> 19
        assert!(s.notch_enabled);
        assert_eq!((s.notch_width, s.notch_height), (200.0, 32.0));
        assert_eq!(s.radius(BorderCorner::TopLeft), 19.0);
        assert_eq!(s.radius(BorderCorner::TopRight), 19.0);
        assert_eq!(s.menu_zone_radii[&BorderCorner::TopLeft], 19.0);
        assert_eq!(s.radius(BorderCorner::BottomLeft), 0.0);
    }

    #[test]
    fn 장치_맞춤_노치없음_3008() {
        let mut s = Settings::default();
        let before = s.corner_radii.clone();
        device_fit(&mut s, &ScreenInfo { width_pt: 3008.0, notch: None });
        assert!(!s.notch_enabled);
        assert_eq!(s.corner_radii, before);
        // 노치 있고 폭이 매우 큰 경우: 22 x 2 = 44, 매우 작으면 22 x 0.5 = 11
        device_fit(&mut s, &ScreenInfo { width_pt: 3008.0, notch: Some(NOTCH) });
        assert_eq!(s.radius(BorderCorner::TopLeft), 39.0); // 22 x 1.759 = 38.7
        device_fit(&mut s, &ScreenInfo { width_pt: 9000.0, notch: Some(NOTCH) });
        assert_eq!(s.radius(BorderCorner::TopLeft), 44.0);
        device_fit(&mut s, &ScreenInfo { width_pt: 100.0, notch: Some(NOTCH) });
        assert_eq!(s.radius(BorderCorner::TopLeft), 11.0);
        device_fit(&mut s, &ScreenInfo { width_pt: 0.0, notch: Some(NOTCH) });
        assert_eq!(s.radius(BorderCorner::TopLeft), 22.0);
    }

    #[test]
    fn 해상도_변경_비례() {
        let mut s = mac_radii();
        adapt_to_screen_change(&mut s, 1000.0, 1500.0, None);
        assert!(!s.notch_enabled);
        assert_eq!(s.radius(BorderCorner::TopLeft), 33.0);
        assert_eq!(s.dock_zone_radii[&BorderCorner::TopLeft], 18.0);
        assert_eq!(s.notch_radius(NotchCorner::InnerLeft), 18.0);
        // 상한: 22 x 2 = 44(구역 80 이내), 노치 12 x 2 = 24
        let mut t = mac_radii();
        adapt_to_screen_change(&mut t, 1000.0, 5000.0, Some(NOTCH));
        assert!(t.notch_enabled);
        assert_eq!(t.radius(BorderCorner::TopLeft), 44.0);
        assert_eq!(t.notch_radius(NotchCorner::InnerRight), 24.0);
        // 노치 곡률 상한 40
        t.notch_radii.insert(NotchCorner::OuterLeft, 30.0);
        adapt_to_screen_change(&mut t, 1000.0, 2000.0, None);
        assert_eq!(t.notch_radius(NotchCorner::OuterLeft), 40.0);
    }

    #[test]
    fn 해상도_변화_1퍼센트_이하는_무시() {
        let mut s = mac_radii();
        adapt_to_screen_change(&mut s, 1000.0, 1010.0, None);
        assert_eq!(s.radius(BorderCorner::TopLeft), 22.0);
        // 이전 폭을 모르면 곡률 유지, 미연결(폭 0)이면 아무것도 안 함.
        adapt_to_screen_change(&mut s, 0.0, 1500.0, None);
        assert_eq!(s.radius(BorderCorner::TopLeft), 22.0);
        let mut u = Settings::default();
        adapt_to_screen_change(&mut u, 1000.0, 0.0, None);
        assert!(u.notch_enabled);
    }
}
