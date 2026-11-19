//! 메뉴바 링 옆 퍼센트(Swift StatusBarController.swift:859-877): 선택한 AI의 표시 %를 각자 색으로 나란히 붙인다.
//! Tauri `set_title`은 색을 못 넣어서 NSStatusItem 버튼에 서식 있는 제목을 직접 건다.

use objc2::{AllocAnyThread, MainThreadMarker};
use objc2::runtime::AnyObject;
use objc2_app_kit::{
    NSCellImagePosition, NSColor, NSFont, NSFontAttributeName, NSFontWeightSemibold,
    NSForegroundColorAttributeName, NSStatusItem,
};
use objc2_foundation::{NSAttributedString, NSDictionary, NSMutableAttributedString, NSString};

/// `shown` = (표시 %, 색) 목록, 'AI 표시 순서'대로. 비어 있으면 링만 보인다.
pub fn apply(item: &NSStatusItem, shown: &[(i64, [f64; 4])]) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let Some(button) = item.button(mtm) else { return };
    if shown.is_empty() {
        button.setImagePosition(NSCellImagePosition::ImageOnly);
        button.setAttributedTitle(&NSAttributedString::new());
        button.setTitle(&NSString::new());
        return;
    }
    button.setImagePosition(NSCellImagePosition::ImageLeading);
    let font = NSFont::systemFontOfSize_weight(11.0, unsafe { NSFontWeightSemibold });
    let out = NSMutableAttributedString::from_nsstring(&NSString::from_str(" "));
    for (i, (pct, c)) in shown.iter().enumerate() {
        if i > 0 {
            out.appendAttributedString(&NSAttributedString::from_nsstring(&NSString::from_str(" ")));
        }
        let color = NSColor::colorWithSRGBRed_green_blue_alpha(c[0], c[1], c[2], c[3]);
        let keys: [&NSString; 2] = unsafe { [NSForegroundColorAttributeName, NSFontAttributeName] };
        let vals: [&AnyObject; 2] = [color.as_ref(), font.as_ref()];
        let attrs = NSDictionary::from_slices(&keys, &vals);
        let s = unsafe {
            NSAttributedString::initWithString_attributes(
                NSAttributedString::alloc(),
                &NSString::from_str(&format!("{pct}%")),
                Some(&attrs),
            )
        };
        out.appendAttributedString(&s);
    }
    button.setAttributedTitle(&out);
}
