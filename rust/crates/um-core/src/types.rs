//! 설정·세그먼트 그래프가 함께 쓰는 열거형. serde 이름은 Swift Codable rawValue와 같다
//! (Swift 버전의 UserDefaults JSON을 그대로 읽기 위해).

use serde::{Deserialize, Serialize};

/// 화면 변. (Swift `BorderEdge`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BorderEdge {
    Top,
    Right,
    Bottom,
    Left,
}

impl BorderEdge {
    pub const ALL: [BorderEdge; 4] = [Self::Top, Self::Right, Self::Bottom, Self::Left];
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Top => "edge.top",
            Self::Right => "edge.right",
            Self::Bottom => "edge.bottom",
            Self::Left => "edge.left",
        }
    }
}

/// 네 모서리. (Swift `BorderCorner`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BorderCorner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl BorderCorner {
    pub const ALL: [BorderCorner; 4] = [Self::TopLeft, Self::TopRight, Self::BottomRight, Self::BottomLeft];
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::TopLeft => "corner.topLeft",
            Self::TopRight => "corner.topRight",
            Self::BottomRight => "corner.bottomRight",
            Self::BottomLeft => "corner.bottomLeft",
        }
    }
}

/// 잔여율 띠가 꼭짓점의 어느 지점에서 시작할지. (Swift `AnchorSide`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AnchorSide {
    Left,
    #[default]
    Center,
    Right,
}

/// 노치 우회 부분의 네 모서리. 저장 순서 = [outerLeft, innerLeft, innerRight, outerRight]. (Swift `NotchCorner`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotchCorner {
    OuterLeft,
    InnerLeft,
    InnerRight,
    OuterRight,
}

impl NotchCorner {
    pub const ALL: [NotchCorner; 4] = [Self::OuterLeft, Self::InnerLeft, Self::InnerRight, Self::OuterRight];
}

/// 가로 경계선으로 나눈 화면 영역. (Swift `ScreenZone`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScreenZone {
    MenuBar,
    Main,
    Dock,
}

impl ScreenZone {
    pub const ALL: [ScreenZone; 3] = [Self::MenuBar, Self::Main, Self::Dock];
}

/// 화면 골격의 선분. 가로 4개 + 좌·우 각 3구간. (Swift `SegPart`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SegPart {
    #[serde(rename = "hTop")]
    HTop,
    #[serde(rename = "hMenu")]
    HMenu,
    #[serde(rename = "hDock")]
    HDock,
    #[serde(rename = "hBottom")]
    HBottom,
    #[serde(rename = "lMenuBar")]
    LMenuBar,
    #[serde(rename = "lMain")]
    LMain,
    #[serde(rename = "lDock")]
    LDock,
    #[serde(rename = "rMenuBar")]
    RMenuBar,
    #[serde(rename = "rMain")]
    RMain,
    #[serde(rename = "rDock")]
    RDock,
}

impl SegPart {
    /// Swift `SegPart.allCases` 순서.
    pub const ALL: [SegPart; 10] = [
        Self::HTop, Self::HMenu, Self::HDock, Self::HBottom,
        Self::LMenuBar, Self::LMain, Self::LDock,
        Self::RMenuBar, Self::RMain, Self::RDock,
    ];

    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::HTop | Self::HMenu | Self::HDock | Self::HBottom)
    }

    /// Swift rawValue (hCaps 키 "hMenu.L", i18n 키 "seg.hTop" 등에 쓴다).
    pub fn raw(self) -> &'static str {
        match self {
            Self::HTop => "hTop",
            Self::HMenu => "hMenu",
            Self::HDock => "hDock",
            Self::HBottom => "hBottom",
            Self::LMenuBar => "lMenuBar",
            Self::LMain => "lMain",
            Self::LDock => "lDock",
            Self::RMenuBar => "rMenuBar",
            Self::RMain => "rMain",
            Self::RDock => "rDock",
        }
    }
}

/// 세그먼트 그래프 노드: (좌/우) × 가로 레벨(0 = 상단, 1 = 메뉴바선, 2 = Dock선, 3 = 하단).
/// 정렬 = (level, 좌 < 우). (Swift `SegNode`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SegNode {
    pub right: bool,
    pub level: u8,
}

impl Ord for SegNode {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        (self.level, self.right as u8).cmp(&(o.level, o.right as u8))
    }
}
impl PartialOrd for SegNode {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

/// 가로선 끝·열린 체인 끝의 캡 방향. (Swift `SegCap`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SegCap {
    #[default]
    None,
    Up,
    Down,
    Include,
}

/// 앱 UI 언어. (Swift `AppLanguage`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AppLanguage {
    #[serde(rename = "ko")]
    Korean,
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "ja")]
    Japanese,
}

impl AppLanguage {
    pub fn code(self) -> &'static str {
        match self {
            Self::Korean => "ko",
            Self::English => "en",
            Self::Japanese => "ja",
        }
    }
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Korean => "한국어",
            Self::English => "English",
            Self::Japanese => "日本語",
        }
    }
}

/// sRGB 색 [r, g, b, a] (0~1). Swift 저장 형식과 같다.
pub type Rgba = [f64; 4];

/// AI별 배치: 세그먼트 조합 + 시작 꼭짓점·방향 + 가로선 끝 캡. (Swift `ProviderLayout`)
/// `edges`·`zones`·`extend*`·`noCurveExtend`·`cap*`는 구버전 저장 데이터 호환용(읽기 전용, 로드 때 segments로 변환).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges: Option<std::collections::BTreeSet<BorderEdge>>,
    pub anchor_corner: BorderCorner,
    pub anchor_side: AnchorSide,
    pub clockwise: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend_start: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extend_end: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_curve_extend: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zones: Option<std::collections::BTreeSet<ScreenZone>>,
    /// 이 AI가 그릴 세그먼트 조합(현행 모델).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segments: Option<std::collections::BTreeSet<SegPart>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_start: Option<SegCap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap_end: Option<SegCap>,
    /// 가로선 좌/우 끝 캡. 키 = "hMenu.L" 등. 없으면 None(자연스러운 꺾임).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_caps: Option<std::collections::BTreeMap<String, SegCap>>,
}

impl ProviderLayout {
    /// 구버전 `zones`가 없으면 전체 영역. (Swift `zoneSet`)
    pub fn zone_set(&self) -> std::collections::BTreeSet<ScreenZone> {
        self.zones.clone().unwrap_or_else(|| ScreenZone::ALL.into_iter().collect())
    }
}
