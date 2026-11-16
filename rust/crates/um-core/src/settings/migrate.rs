//! 구버전(변+영역) 설정 -> 세그먼트 변환. 그래프 함수는 `crate::segments`의 것을 그대로 쓴다
//! (병합 때 설정 쪽 복사본을 없앰). 설정 로드에만 필요한 `is_valid_zones`는 여기 둔다.

use std::collections::BTreeSet;

use crate::types::ScreenZone;

#[allow(unused_imports)]
pub(crate) use crate::segments::{is_valid_segments, migrate_to_segments, seg_available, seg_ends, zone_perimeter};

/// 영역 조합이 유효한지: 비어있지 않고, 메뉴바+Dock만은 불가. (Swift `isValidZones` :670)
pub(crate) fn is_valid_zones(z: &BTreeSet<ScreenZone>) -> bool {
    if z.is_empty() {
        return false;
    }
    !(z.contains(&ScreenZone::MenuBar) && z.contains(&ScreenZone::Dock) && !z.contains(&ScreenZone::Main))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BorderEdge, SegPart};

    fn all_edges() -> BTreeSet<BorderEdge> {
        BorderEdge::ALL.into_iter().collect()
    }
    fn all_zones() -> BTreeSet<ScreenZone> {
        ScreenZone::ALL.into_iter().collect()
    }

    #[test]
    fn 경계선_꺼지면_전체_둘레() {
        let s = migrate_to_segments(&all_edges(), &all_zones(), false, false);
        let want: BTreeSet<_> = [SegPart::HTop, SegPart::HBottom, SegPart::LMain, SegPart::RMain].into();
        assert_eq!(s, want);
        assert!(is_valid_segments(&s, false, false));
    }

    #[test]
    fn 경계선_켜면_영역_세로선_포함() {
        let s = migrate_to_segments(&all_edges(), &all_zones(), true, true);
        assert!(s.contains(&SegPart::LMenuBar) && s.contains(&SegPart::RDock) && s.contains(&SegPart::HTop));
        assert!(!s.contains(&SegPart::HMenu));
    }

    #[test]
    fn 빈_결과는_상단_가로선() {
        let s = migrate_to_segments(&BTreeSet::new(), &all_zones(), false, false);
        assert_eq!(s, [SegPart::HTop].into());
    }

    #[test]
    fn 유효성() {
        let two: BTreeSet<_> = [SegPart::HTop, SegPart::HBottom].into();
        assert!(!is_valid_segments(&two, false, false));
        let menu: BTreeSet<_> = [SegPart::HMenu].into();
        assert!(!is_valid_segments(&menu, false, false));
        assert!(is_valid_segments(&menu, true, false));
    }
}
