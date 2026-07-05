import SwiftUI
import Combine

/// 테두리를 그릴 화면 변(면). 인접한 변끼리 이어붙이면 모서리가 둥글게 연결된다.
enum BorderEdge: String, CaseIterable, Codable {
    case top, right, bottom, left

    var label: String {
        switch self {
        case .top: return "상"
        case .right: return "우"
        case .bottom: return "하"
        case .left: return "좌"
        }
    }
    func label(_ lang: AppLanguage) -> String {
        switch self {
        case .top: return Loc.tr("edge.top", lang)
        case .right: return Loc.tr("edge.right", lang)
        case .bottom: return Loc.tr("edge.bottom", lang)
        case .left: return Loc.tr("edge.left", lang)
        }
    }
}

/// 네 모서리. 각 모서리마다 곡률을 따로 준다(맥 미니 외장 모니터처럼 위만 둥근 경우 대응).
enum BorderCorner: String, CaseIterable, Codable {
    case topLeft, topRight, bottomRight, bottomLeft

    var label: String {
        switch self {
        case .topLeft: return "좌상"
        case .topRight: return "우상"
        case .bottomRight: return "우하"
        case .bottomLeft: return "좌하"
        }
    }
    func label(_ lang: AppLanguage) -> String {
        switch self {
        case .topLeft: return Loc.tr("corner.topLeft", lang)
        case .topRight: return Loc.tr("corner.topRight", lang)
        case .bottomRight: return Loc.tr("corner.bottomRight", lang)
        case .bottomLeft: return Loc.tr("corner.bottomLeft", lang)
        }
    }
}

/// 잔여율 띠가 꼭짓점의 어느 지점에서 시작할지.
enum AnchorSide: String, CaseIterable, Codable {
    case left, center, right
    var label: String {
        switch self {
        case .left: return "곡률 왼쪽"
        case .center: return "꼭짓점"
        case .right: return "곡률 오른쪽"
        }
    }
}

/// 노치 우회 부분의 네 모서리(본체 모서리와 별개로 곡률 조절).
/// outer=상변과 만나는 바깥쪽 위 모서리, inner=노치 아래쪽 안쪽 모서리.
enum NotchCorner: String, CaseIterable {
    case outerLeft, innerLeft, innerRight, outerRight

    var label: String {
        switch self {
        case .outerLeft: return "좌·바깥(위)"
        case .innerLeft: return "좌·안쪽(아래)"
        case .innerRight: return "우·안쪽(아래)"
        case .outerRight: return "우·바깥(위)"
        }
    }
    func label(_ lang: AppLanguage) -> String {
        switch self {
        case .outerLeft: return Loc.tr("notch.outerLeft", lang)
        case .innerLeft: return Loc.tr("notch.innerLeft", lang)
        case .innerRight: return Loc.tr("notch.innerRight", lang)
        case .outerRight: return Loc.tr("notch.outerRight", lang)
        }
    }
}

/// 화면을 가로 경계선으로 나눈 영역(파티션). 메뉴바 띠 / 본문 / Dock 띠.
/// 경계선이 꺼져 있으면 그 영역은 이웃 영역과 자동으로 합쳐진 것과 같다.
enum ScreenZone: String, CaseIterable, Codable {
    case menuBar, main, dock

    func label(_ lang: AppLanguage) -> String {
        switch self {
        case .menuBar: return Loc.tr("zone.menuBar", lang)
        case .main: return Loc.tr("zone.main", lang)
        case .dock: return Loc.tr("zone.dock", lang)
        }
    }
}

/// 화면 골격의 선분(세그먼트). 가로 4개(상단/메뉴바선/Dock선/하단) +
/// 세로 좌·우 각 3구간(메뉴바/본문/Dock). 경계선이 꺼져 있으면 관련 세그먼트는 선택 불가.
enum SegPart: String, CaseIterable, Codable {
    case hTop, hMenu, hDock, hBottom          // 가로선 4개(#요청1)
    case lMenuBar, lMain, lDock               // 좌변 3구간(#요청2)
    case rMenuBar, rMain, rDock               // 우변 3구간

    var isHorizontal: Bool {
        self == .hTop || self == .hMenu || self == .hDock || self == .hBottom
    }
    func label(_ lang: AppLanguage) -> String { Loc.tr("seg.\(rawValue)", lang) }
}

/// 세그먼트 그래프의 노드: (좌/우) × (가로 레벨 0=상단 1=메뉴바선 2=Dock선 3=하단).
struct SegNode: Hashable, Comparable {
    let right: Bool
    let level: Int
    static func < (a: SegNode, b: SegNode) -> Bool {
        (a.level, a.right ? 1 : 0) < (b.level, b.right ? 1 : 0)
    }
}

/// 열린 체인 끝의 모서리 캡 방향.
enum SegCap: String, Codable, CaseIterable {
    case none      // 캡 없음(직각 끝)
    case up        // 가로선 끝: 위로 둥글게
    case down      // 가로선 끝: 아래로 둥글게
    case include   // 세로선 끝: 모서리 포함(가로 방향으로 둥글게)
}

/// AI별 배치: 세그먼트 조합(현행 모델) + 시작 꼭짓점·방향 + 가로선 끝 캡.
/// edges/zones/extend*/cap* 필드는 구버전 저장 데이터 디코딩 호환용 — 로드 시
/// 세그먼트로 1회 변환(materialize)된 뒤로는 쓰지 않는다(#스키마 일원화).
struct ProviderLayout: Codable {
    var edges: Set<BorderEdge>?          // (구버전) 변 선택
    var anchorCorner: BorderCorner
    var anchorSide: AnchorSide
    var clockwise: Bool
    var extendStart: Bool?               // (구버전) 선 끝 마감
    var extendEnd: Bool?
    var noCurveExtend: Bool?
    var zones: Set<ScreenZone>?          // (구버전) 영역 선택 — 변환 소스로만 사용
    /// 이 AI가 그릴 세그먼트 조합(현행 모델).
    var segments: Set<SegPart>?
    var capStart: SegCap?                // (구버전) 체인 끝 캡
    var capEnd: SegCap?
    /// 가로선 좌/우 끝의 위/아래 둥글게. 키 = "hMenu.L" 등. 세로선이 붙어 있거나
    /// 고리여도 유지된다. 값이 없으면 .none(자연스러운 꺾임).
    var hCaps: [String: SegCap]?

    var zoneSet: Set<ScreenZone> { zones ?? Set(ScreenZone.allCases) }
}

/// 저장 프리셋에 담기는 설정 스냅샷(잔여율 등 실시간 값은 제외). 영속화용 Codable.
struct SettingsSnapshot: Codable {
    var thickness: Double
    var colorRGBA: [Double]
    var edges: [String]
    var cornerRadii: [String: Double]
    var showTrack: Bool
    var notchEnabled: Bool
    var notchWidth: Double
    var notchHeight: Double
    var notchRadii: [String: Double]
    var anchorCorner: String
    var anchorSide: String
    var anchorClockwise: Bool
    var lineOpacity: Double?
    var extendStartCorner: Bool?
    var extendEndCorner: Bool?
    var startCornerAbove: Bool?
    var fadeEnabled: Bool?
    var fadeFraction: Double?
    var keepLoggedIn: Bool?
    var providerColors: [String: [Double]]?
    var separateByAI: Bool?
    var providerLayouts: [String: ProviderLayout]?
    var showPercentInMenuBar: Bool?          // 구버전 호환(단일 토글)
    var menuBarPercentIDs: [String]?
    var menuBarIconID: String?
    var menuBarPercentFirstID: String?
    var menuShow5h: Bool?
    var menuShowWeekly: Bool?
    var menuShowReset: Bool?
    var menuShowUpdated: Bool?
    var language: String?
    var refreshPresetMinutes: Int?
    var refreshUseCustom: Bool?
    var refreshCustomMinutes: Int?
    var menuShowOpus: Bool?
    var menuShowCountdown: Bool?
    var menuShowPace: Bool?
    var menuShowChart: Bool?
    var notifyEnabled: Bool?
    var notifyThresholds: [Int]?
    // 화면 분할(파티션) — 가로 경계선.
    var menuLineEnabled: Bool?
    var menuLineHeight: Double?
    var dockLineEnabled: Bool?
    var dockLineHeight: Double?
    var globalZones: [String]?
    // 영역별 모서리 곡률(본문은 기존 cornerRadii 재사용).
    var menuZoneRadii: [String: Double]?
    var dockZoneRadii: [String: Double]?
    // 세그먼트 신모델(전역).
    var globalSegments: [String]?
    var segCapStart: String?
    var segCapEnd: String?
    var noOverlapLines: Bool?
    var splitOverlapLines: Bool?
    var chartHours: Int?
    var linkMenuMainRadii: Bool?
    var linkMainDockRadii: Bool?
}

/// 이름이 붙은 저장 프리셋.
struct SavedPreset: Codable, Identifiable {
    var id: UUID
    var name: String
    var snapshot: SettingsSnapshot
}

/// 오버레이 테두리의 사용자 설정(굵기·색·모서리 곡률·노치 등). 메뉴바에서 바꾸면
/// `@Published`를 통해 오버레이가 실시간으로 다시 그려진다. 앱 전역에서 하나 공유.
final class OverlaySettings: ObservableObject {

    /// 선 굵기(pt).
    @Published var thickness: CGFloat = 2

    /// 선 색.
    @Published var color: Color = Color(red: 0.30, green: 0.82, blue: 0.45)

    /// 선 전체 투명도(0~1). 색 자체의 알파와 곱해진다.
    @Published var lineOpacity: CGFloat = 1.0

    // MARK: - 부분 선택 시 모서리 곡률 추가(#1)

    /// 부분 선택한 변의 '시작' 끝에 곡률 모서리를 추가할지. 켜면 그 끝점이 띠 시작점이 된다.
    @Published var extendStartCorner: Bool = false
    /// 부분 선택한 변의 '끝' 끝에 곡률 모서리를 추가할지.
    @Published var extendEndCorner: Bool = false
    /// 추가한 시작 곡률 모서리에서 시작점을 곡률 위(true)/아래(false) 접점으로(#2).
    @Published var startCornerAbove: Bool = false

    /// (구버전) 전역 변 선택 — 세그먼트 모델로 대체됨. 저장 호환용으로만 유지.
    @Published var edges: Set<BorderEdge> = Set(BorderEdge.allCases)

    /// 본문(main) 영역의 모서리별 곡률 반경(pt). 0이면 직각. 기본: 위(좌상·우상)만 22.
    /// 파티션이 없으면 화면 전체가 본문 영역이므로 이것이 전체 곡률이다.
    @Published var cornerRadii: [BorderCorner: CGFloat] = [
        .topLeft: 22, .topRight: 22, .bottomRight: 0, .bottomLeft: 0,
    ]
    /// 메뉴바 영역의 모서리별 곡률(영역별 개별 설정 — #요청3,4).
    @Published var menuZoneRadii: [BorderCorner: CGFloat] = [
        .topLeft: 22, .topRight: 22, .bottomRight: 0, .bottomLeft: 0,
    ]
    /// Dock 영역의 모서리별 곡률.
    @Published var dockZoneRadii: [BorderCorner: CGFloat] = [
        .topLeft: 12, .topRight: 12, .bottomRight: 0, .bottomLeft: 0,
    ]

    /// 해당 영역의 곡률 사전.
    func zoneRadiiDict(_ z: ScreenZone) -> [BorderCorner: CGFloat] {
        switch z {
        case .menuBar: return menuZoneRadii
        case .main: return cornerRadii
        case .dock: return dockZoneRadii
        }
    }
    /// 곡률 연결(#연결): 켜면 경계선을 공유하는 두 영역의 맞닿은 꼭짓점이 함께 움직인다.
    /// (예: 메뉴바 좌하 ↔ 본문 좌상). 꺼져 있으면 각각 독립적으로 설정.
    @Published var linkMenuMainRadii: Bool = true
    @Published var linkMainDockRadii: Bool = true

    /// 영역 곡률 쓰기(+연결 미러링).
    func setZoneRadius(_ z: ScreenZone, _ c: BorderCorner, _ v: CGFloat) {
        switch z {
        case .menuBar: menuZoneRadii[c] = v
        case .main: cornerRadii[c] = v
        case .dock: dockZoneRadii[c] = v
        }
        // 메뉴바선 경계: 메뉴바 아래 꼭짓점 ↔ 본문 위 꼭짓점.
        if linkMenuMainRadii {
            if z == .menuBar, c == .bottomLeft { cornerRadii[.topLeft] = v }
            if z == .menuBar, c == .bottomRight { cornerRadii[.topRight] = v }
            if z == .main, c == .topLeft { menuZoneRadii[.bottomLeft] = v }
            if z == .main, c == .topRight { menuZoneRadii[.bottomRight] = v }
        }
        // Dock선 경계: 본문 아래 꼭짓점 ↔ Dock 위 꼭짓점.
        if linkMainDockRadii {
            if z == .main, c == .bottomLeft { dockZoneRadii[.topLeft] = v }
            if z == .main, c == .bottomRight { dockZoneRadii[.topRight] = v }
            if z == .dock, c == .topLeft { cornerRadii[.bottomLeft] = v }
            if z == .dock, c == .topRight { cornerRadii[.bottomRight] = v }
        }
    }

    /// 겹치는 선 없음(#요청2): 켜면 서로 다른 AI가 같은 세그먼트를 고를 수 없다.
    @Published var noOverlapLines: Bool = false

    /// 겹침 구간 굵기 분할(#겹침 규칙3): 완전히 같은 모양끼리 겹칠 때도
    /// 굵기를 나눠 나란히 표시한다. (부분 겹침은 옵션과 무관하게 항상 분할 — 규칙2.)
    @Published var splitOverlapLines: Bool = false

    /// '겹치는 선 없음'을 켜는 순간 기존 겹침을 정리한다(#요청3,4).
    /// 앞선 AI(스펙 순서)가 세그먼트를 갖고, 뒤 AI에서 겹치는 것을 제거한다.
    /// 제거로 선이 끊어지면(한 줄 아님) 전부 해제하고 비어 있는 임의의 세그먼트 하나를 준다.
    func enforceNoOverlap() {
        let menuOn = menuLineEnabled, dockOn = dockLineEnabled
        var used: Set<SegPart> = []
        for spec in ProviderSpec.all {
            var mine = segs(for: spec.id)
            mine.subtract(used)
            if mine.isEmpty || !Self.isValidSegments(mine, menuOn: menuOn, dockOn: dockOn) {
                let free = SegPart.allCases.first {
                    Self.segAvailable($0, menuOn: menuOn, dockOn: dockOn) && !used.contains($0)
                }
                mine = free.map { [$0] } ?? []
            }
            var l = layout(for: spec.id)
            l.segments = mine
            providerLayouts[spec.id] = l
            used.formUnion(mine)
        }
    }

    /// 잔여율 0.0~1.0. 이 비율만큼만 색 띠가 채워진다. 로그인 전엔 1(가득).
    @Published var ratio: Double = 1

    /// 옅은 트랙(전체 길이)을 함께 그릴지.
    @Published var showTrack: Bool = true

    /// 소모 끝부분 투명 그라데이션 켜기/끄기.
    @Published var fadeEnabled: Bool = true
    /// 그라데이션 길이(경로 대비 비율 0~0.2). 슬라이더는 0~20(%)로 노출.
    @Published var fadeFraction: CGFloat = 0.012

    // MARK: - 앵커(잔여율 띠가 시작하는 지점)

    /// 시작 꼭짓점.
    @Published var anchorCorner: BorderCorner = .topLeft
    /// 그 꼭짓점의 어느 지점에서 시작할지(곡률 왼쪽/꼭짓점/곡률 오른쪽).
    @Published var anchorSide: AnchorSide = .center
    /// 시계방향으로 줄어들지(끄면 반시계).
    @Published var anchorClockwise: Bool = true

    // MARK: - 노치

    /// 상단 노치를 감싸도록 테두리를 그릴지. 기본 켜짐(맥북 Air 15 기준).
    @Published var notchEnabled: Bool = true

    /// 노치 너비(pt). 기본 186(맥북 Air 15).
    @Published var notchWidth: CGFloat = 186

    /// 노치 높이(pt, 아래로 감싸는 깊이). 기본 33.
    @Published var notchHeight: CGFloat = 33

    /// 노치 우회 부분 모서리별 곡률. 기본: 위(바깥) 5, 아래(안쪽) 12.
    @Published var notchRadii: [NotchCorner: CGFloat] = [
        .outerLeft: 5, .outerRight: 5, .innerLeft: 12, .innerRight: 12,
    ]

    /// AI별 띠 색(브랜드 색 기본).
    @Published var providerColors: [String: Color] = Dictionary(
        uniqueKeysWithValues: ProviderSpec.all.map { ($0.id, $0.defaultColor) })

    /// 한 번 로그인하면 명시적으로 로그아웃하기 전까지 로그인 유지(재실행해도).
    @Published var keepLoggedIn: Bool = true

    /// UI 언어(한국어/영어/일본어). 바뀌면 설정 창·메뉴 문자열이 즉시 갱신.
    @Published var language: AppLanguage = .english   // 처음 설치 시 영문 기본
    /// 현재 언어로 문자열 조회.
    func t(_ key: String) -> String { Loc.tr(key, language) }
    /// %@ 를 인자로 치환.
    func tf(_ key: String, _ arg: String) -> String { t(key).replacingOccurrences(of: "%@", with: arg) }
    /// %d 를 숫자로 치환.
    func tn(_ key: String, _ n: Int) -> String { t(key).replacingOccurrences(of: "%d", with: String(n)) }

    // MARK: - 자동 갱신 주기

    /// 선택 가능한 프리셋 주기(분).
    static let refreshPresets: [Int] = [1, 2, 3, 4, 5, 10, 15, 20, 30, 60, 120]
    /// 프리셋 주기(분). 기본 5분.
    @Published var refreshPresetMinutes: Int = 5
    /// 사용자 설정(직접 분 입력) 사용 여부.
    @Published var refreshUseCustom: Bool = false
    /// 사용자 설정 주기(분).
    @Published var refreshCustomMinutes: Int = 5
    /// 실제 적용되는 갱신 주기(분).
    var effectiveRefreshMinutes: Int { max(1, refreshUseCustom ? refreshCustomMinutes : refreshPresetMinutes) }

    /// 설정 창에서 열 탭 요청(메뉴에서 트리거). 처리 후 nil로 되돌린다. 영속화 안 함.
    @Published var requestedTab: String? = nil

    // MARK: - 메뉴바/드롭다운 표시

    /// 메뉴바 링 아이콘이 어느 AI 기준으로 표시될지(#2). 로그인 안 됐으면 자동 폴백.
    @Published var menuBarIconID: String = ProviderSpec.all.first?.id ?? "claude"
    /// 메뉴바 아이콘 옆에 % 표시할 AI들. 비어 있으면 표시 안 함. 여러 개면 나란히 표시.
    @Published var menuBarPercentIDs: Set<String> = []
    /// % 표시에서 먼저(앞) 둘 AI.
    @Published var menuBarPercentFirstID: String = ProviderSpec.all.first?.id ?? "claude"
    /// 드롭다운에 표시할 정보(#2).
    @Published var menuShow5h: Bool = true
    @Published var menuShowWeekly: Bool = true
    @Published var menuShowReset: Bool = true
    @Published var menuShowUpdated: Bool = false
    /// Opus 주간 잔여 표시(Claude).
    @Published var menuShowOpus: Bool = true
    /// 리셋까지 남은 시간(카운트다운) 표시.
    @Published var menuShowCountdown: Bool = true
    /// 소진 예측(pace) 표시.
    @Published var menuShowPace: Bool = false
    /// 드롭다운에 미니 차트 표시.
    @Published var menuShowChart: Bool = false
    /// 미니 차트가 보여줄 기간(시간): 6 / 12 / 24.
    @Published var chartHours: Int = 24

    // MARK: - 임계치 알림

    /// 임계치 알림 켜기.
    @Published var notifyEnabled: Bool = false
    /// 알림을 보낼 사용률(%) 임계치(잔여 = 100 - 이 값).
    @Published var notifyThresholds: Set<Int> = [75, 90, 95]

    // MARK: - 화면 분할(파티션) — 가로 경계선

    /// 메뉴바 아래 가로 경계선 켜기. 켜면 화면이 '메뉴바 띠'와 '본문'으로 나뉜다.
    @Published var menuLineEnabled: Bool = false
    /// 메뉴바 경계선의 위치(화면 위에서부터 pt).
    @Published var menuLineHeight: CGFloat = 25

    /// Dock 위 가로 경계선 켜기. 켜면 화면 하단이 'Dock 띠'로 나뉜다.
    @Published var dockLineEnabled: Bool = false
    /// Dock 경계선의 높이(화면 아래에서부터 pt) — Dock 크기에 맞게 조절.
    @Published var dockLineHeight: CGFloat = 70

    /// 전역(겹침 모드)에서 띠가 감쌀 영역(파티션). 기본 = 전체.
    @Published var zones: Set<ScreenZone> = Set(ScreenZone.allCases)

    /// 전역(겹침 모드)에서 그릴 세그먼트 조합(신모델). 기본 = 화면 전체 둘레.
    @Published var segParts: Set<SegPart> = [.hTop, .hBottom, .lMain, .rMain]
    /// 전역 열린 체인 끝 모서리 캡(정렬 첫/마지막 끝).
    @Published var segCapStart: SegCap = .none
    @Published var segCapEnd: SegCap = .none

    /// 이 AI의 세그먼트 조합(없으면 구버전 변+영역 설정에서 변환).
    func segs(for id: String) -> Set<SegPart> {
        let l = layout(for: id)
        if let s = l.segments { return s }
        return Self.migrateToSegments(edges: l.edges ?? Set(BorderEdge.allCases), zones: l.zoneSet,
                                      menuOn: menuLineEnabled, dockOn: dockLineEnabled)
    }

    /// 가로선 캡 딕셔너리 키: "hMenu.L" / "hMenu.R" 형태.
    static func hCapKey(_ s: SegPart, right: Bool) -> String { "\(s.rawValue).\(right ? "R" : "L")" }

    /// 이 AI의 특정 가로선 좌/우 끝 캡.
    func hCap(_ id: String, _ s: SegPart, right: Bool) -> SegCap {
        layout(for: id).hCaps?[Self.hCapKey(s, right: right)] ?? SegCap.none
    }

    /// 체인 끝(노드+세그먼트)의 교차 AI 보정(#모서리 반씩 양분).
    /// - 내 가로선 끝에 캡이 있고, 다른 AI의 세로선이 캡 방향으로 그 노드에 붙어 있으면 → 내 캡은 반쪽 아크.
    /// - 내 세로선 끝에서, 다른 AI 가로선의 캡이 내 쪽을 향하면 → 나도 반쪽 아크를 자동으로 그린다.
    /// 반환: (half: 반쪽 여부, autoArc: 세로 끝 자동 아크 여부)
    func endOverride(for id: String, node: SegNode, seg: SegPart) -> (half: Bool, autoArc: Bool) {
        let menuOn = menuLineEnabled, dockOn = dockLineEnabled
        if seg.isHorizontal {
            let cap = hCap(id, seg, right: node.right)
            guard cap == .up || cap == .down else { return (false, false) }
            for spec in ProviderSpec.all where spec.id != id {
                for s in segs(for: spec.id) where !s.isHorizontal {
                    let (a, b) = Self.segEnds(s, menuOn: menuOn, dockOn: dockOn)
                    guard a == node || b == node else { continue }
                    let otherLevel = (a == node) ? b.level : a.level
                    if (cap == .down && otherLevel > node.level) ||
                       (cap == .up && otherLevel < node.level) { return (true, false) }
                }
            }
            return (false, false)
        } else {
            let (a, b) = Self.segEnds(seg, menuOn: menuOn, dockOn: dockOn)
            guard a == node || b == node else { return (false, false) }
            let otherEnd = (a == node) ? b : a
            let myDown = otherEnd.level > node.level     // 내 세로선이 이 노드에서 아래로 뻗는가
            for spec in ProviderSpec.all where spec.id != id {
                for s in segs(for: spec.id) where s.isHorizontal {
                    let (ha, hb) = Self.segEnds(s, menuOn: menuOn, dockOn: dockOn)
                    guard ha == node || hb == node else { continue }
                    let cap = hCap(spec.id, s, right: node.right)
                    if (cap == .down && myDown) || (cap == .up && !myDown) { return (true, true) }
                }
            }
            return (false, false)
        }
    }

    /// 이 AI가 세그먼트 조합을 쓸 수 있는지: 한 줄로 이어져야 하고(#요청4),
    /// '겹치는 선 없음'이 켜져 있으면 다른 AI가 쓰는 세그먼트와 겹칠 수 없다(#요청2).
    /// (꺼져 있으면 완전히 같은 조합도 허용 — 선 겹침 OK #요청1.)
    func canUseSegments(_ id: String, _ set: Set<SegPart>) -> Bool {
        guard Self.isValidSegments(set, menuOn: menuLineEnabled, dockOn: dockLineEnabled) else { return false }
        if noOverlapLines {
            for spec in ProviderSpec.all where spec.id != id {
                if !segs(for: spec.id).isDisjoint(with: set) { return false }
            }
        }
        return true
    }

    /// 이 영역 꼭짓점 곡률이 현재 어느 선택에서든 실제로 쓰이는지(#요청6 — 안 쓰면 비활성화).
    /// 쓰임 = 그 꼭짓점에서 가로+세로 세그먼트가 꺾이거나, 체인 끝 캡이 그 꼭짓점을 향함.
    func cornerUsed(_ zone: ScreenZone, _ corner: BorderCorner) -> Bool {
        let menuOn = menuLineEnabled, dockOn = dockLineEnabled
        func topLevel(_ z: ScreenZone) -> Int {
            switch z { case .menuBar: return 0; case .main: return menuOn ? 1 : 0; case .dock: return 2 }
        }
        func bottomLevel(_ z: ScreenZone) -> Int {
            switch z { case .menuBar: return 1; case .main: return dockOn ? 2 : 3; case .dock: return 3 }
        }
        // 영역이 병합돼 없으면(경계선 꺼짐) 그 영역 곡률은 안 쓰인다(본문 것만 쓰임).
        if zone == .menuBar && !menuOn { return false }
        if zone == .dock && !dockOn { return false }
        let isTop = (corner == .topLeft || corner == .topRight)
        let level = isTop ? topLevel(zone) : bottomLevel(zone)
        let right = (corner == .topRight || corner == .bottomRight)
        let vSeg: SegPart = {
            switch zone {
            case .menuBar: return right ? .rMenuBar : .lMenuBar
            case .main: return right ? .rMain : .lMain
            case .dock: return right ? .rDock : .lDock
            }
        }()
        let hSeg: SegPart = {
            switch level { case 0: return .hTop; case 1: return .hMenu; case 2: return .hDock; default: return .hBottom }
        }()
        // 모든 AI(항상 AI별 선택)에서: 이 노드의 꺾임 또는 이 꼭짓점을 향하는 가로선 캡이 있으면 사용 중.
        for spec in ProviderSpec.all {
            let set = segs(for: spec.id)
            // 꺾임: 이 노드에서 가로+세로가 모두 선택됨.
            if set.contains(vSeg) && set.contains(hSeg) { return true }
            // 가로선 캡: 이 가로선의 이 쪽 끝 캡이 이 영역 꼭짓점을 향함.
            if set.contains(hSeg) {
                let cap = hCap(spec.id, hSeg, right: right)
                if cap == .up && !isTop && bottomLevel(zone) == level { return true }
                if cap == .down && isTop && topLevel(zone) == level { return true }
            }
        }
        return false
    }

    /// 영역 조합이 유효한지: 비어있지 않고, 서로 붙어 있어야 한다(메뉴바+Dock만은 불가).
    static func isValidZones(_ z: Set<ScreenZone>) -> Bool {
        guard !z.isEmpty else { return false }
        if z.contains(.menuBar) && z.contains(.dock) && !z.contains(.main) { return false }
        return true
    }

    /// 영역 z를 토글해도 되는지(전역 zones 기준).
    func canToggleZone(_ z: ScreenZone) -> Bool {
        var s = zones
        if s.contains(z) { s.remove(z) } else { s.insert(z) }
        return Self.isValidZones(s)
    }

    // MARK: - 세그먼트 그래프(#요청1,2,4)

    /// 이 세그먼트가 현재 경계선 설정에서 존재하는지.
    static func segAvailable(_ s: SegPart, menuOn: Bool, dockOn: Bool) -> Bool {
        switch s {
        case .hMenu, .lMenuBar, .rMenuBar: return menuOn
        case .hDock, .lDock, .rDock: return dockOn
        default: return true
        }
    }

    /// 세그먼트의 양 끝 노드. (본문 세로선은 경계선 유무에 따라 늘어난다.)
    static func segEnds(_ s: SegPart, menuOn: Bool, dockOn: Bool) -> (SegNode, SegNode) {
        let mainTop = menuOn ? 1 : 0
        let mainBot = dockOn ? 2 : 3
        switch s {
        case .hTop:    return (SegNode(right: false, level: 0), SegNode(right: true, level: 0))
        case .hMenu:   return (SegNode(right: false, level: 1), SegNode(right: true, level: 1))
        case .hDock:   return (SegNode(right: false, level: 2), SegNode(right: true, level: 2))
        case .hBottom: return (SegNode(right: false, level: 3), SegNode(right: true, level: 3))
        case .lMenuBar: return (SegNode(right: false, level: 0), SegNode(right: false, level: 1))
        case .lMain:    return (SegNode(right: false, level: mainTop), SegNode(right: false, level: mainBot))
        case .lDock:    return (SegNode(right: false, level: 2), SegNode(right: false, level: 3))
        case .rMenuBar: return (SegNode(right: true, level: 0), SegNode(right: true, level: 1))
        case .rMain:    return (SegNode(right: true, level: mainTop), SegNode(right: true, level: mainBot))
        case .rDock:    return (SegNode(right: true, level: 2), SegNode(right: true, level: 3))
        }
    }

    /// 노드 → 인접 세그먼트 목록.
    static func segAdjacency(_ set: Set<SegPart>, menuOn: Bool, dockOn: Bool) -> [SegNode: [SegPart]] {
        var adj: [SegNode: [SegPart]] = [:]
        for s in set {
            let (a, b) = segEnds(s, menuOn: menuOn, dockOn: dockOn)
            adj[a, default: []].append(s)
            adj[b, default: []].append(s)
        }
        return adj
    }

    /// 선택이 '한 줄로 이어진 선(체인 또는 고리)'인지(#요청4,10).
    /// 조건: 전부 존재하는 세그먼트, 노드 차수 ≤ 2, 전체 연결, 끝점 0개(고리) 또는 2개(체인).
    static func isValidSegments(_ set: Set<SegPart>, menuOn: Bool, dockOn: Bool) -> Bool {
        guard !set.isEmpty else { return false }
        guard set.allSatisfy({ segAvailable($0, menuOn: menuOn, dockOn: dockOn) }) else { return false }
        let adj = segAdjacency(set, menuOn: menuOn, dockOn: dockOn)
        guard adj.values.allSatisfy({ $0.count <= 2 }) else { return false }
        let ends = adj.values.filter { $0.count == 1 }.count
        guard ends == 0 || ends == 2 else { return false }
        // 연결성: 아무 세그먼트에서 BFS로 전부 도달해야 함.
        var visited: Set<SegPart> = []
        var queue: [SegPart] = [set.first!]
        while let s = queue.popLast() {
            guard visited.insert(s).inserted else { continue }
            let (a, b) = segEnds(s, menuOn: menuOn, dockOn: dockOn)
            for n in [a, b] { queue.append(contentsOf: (adj[n] ?? []).filter { !visited.contains($0) }) }
        }
        return visited.count == set.count
    }

    /// 열린 체인의 두 끝 (노드, 그 끝에 붙은 세그먼트). 정렬 순서 고정(위→아래, 좌→우).
    /// 고리이거나 유효하지 않으면 nil.
    static func chainEnds(_ set: Set<SegPart>, menuOn: Bool, dockOn: Bool)
        -> (start: (node: SegNode, seg: SegPart), end: (node: SegNode, seg: SegPart))? {
        guard isValidSegments(set, menuOn: menuOn, dockOn: dockOn) else { return nil }
        let adj = segAdjacency(set, menuOn: menuOn, dockOn: dockOn)
        let eps = adj.filter { $0.value.count == 1 }.keys.sorted()
        guard eps.count == 2 else { return nil }
        return ((eps[0], adj[eps[0]]![0]), (eps[1], adj[eps[1]]![0]))
    }

    /// 영역 조합의 둘레를 이루는 세그먼트(#요청3 — 영역 빠른 선택).
    static func zonePerimeter(_ z: Set<ScreenZone>, menuOn: Bool, dockOn: Bool) -> Set<SegPart> {
        // 꺼진 경계선의 영역은 본문으로 병합.
        var eff = Set(z.map { zone -> ScreenZone in
            if zone == .menuBar && !menuOn { return .main }
            if zone == .dock && !dockOn { return .main }
            return zone
        })
        if eff.isEmpty { eff = [.main] }
        func topLevel(_ zone: ScreenZone) -> Int {
            switch zone { case .menuBar: return 0; case .main: return menuOn ? 1 : 0; case .dock: return 2 }
        }
        func bottomLevel(_ zone: ScreenZone) -> Int {
            switch zone { case .menuBar: return 1; case .main: return dockOn ? 2 : 3; case .dock: return 3 }
        }
        let order: [ScreenZone] = [.menuBar, .main, .dock]
        let top = order.first { eff.contains($0) } ?? .main
        let bottom = order.last { eff.contains($0) } ?? .main
        func hSeg(_ level: Int) -> SegPart {
            switch level { case 0: return .hTop; case 1: return .hMenu; case 2: return .hDock; default: return .hBottom }
        }
        var out: Set<SegPart> = [hSeg(topLevel(top)), hSeg(bottomLevel(bottom))]
        for zone in order where eff.contains(zone) {
            switch zone {
            case .menuBar: out.insert(.lMenuBar); out.insert(.rMenuBar)
            case .main: out.insert(.lMain); out.insert(.rMain)
            case .dock: out.insert(.lDock); out.insert(.rDock)
            }
        }
        return out
    }

    /// 구버전 (변 + 영역) 설정 → 세그먼트 조합으로 변환.
    static func migrateToSegments(edges: Set<BorderEdge>, zones z: Set<ScreenZone>,
                                  menuOn: Bool, dockOn: Bool) -> Set<SegPart> {
        let peri = zonePerimeter(z, menuOn: menuOn, dockOn: dockOn)
        // 영역 둘레의 위/아래 가로선과 좌/우 세로줄을 변 선택에 맞춰 채운다.
        let lv: [SegPart: Int] = [.hTop: 0, .hMenu: 1, .hDock: 2, .hBottom: 3]
        let hs = peri.filter { $0.isHorizontal }.sorted { (lv[$0] ?? 0) < (lv[$1] ?? 0) }
        var out: Set<SegPart> = []
        if edges.contains(.top), let t = hs.first { out.insert(t) }
        if edges.contains(.bottom), let b = hs.last { out.insert(b) }
        if edges.contains(.left) { out.formUnion(peri.filter { [SegPart.lMenuBar, .lMain, .lDock].contains($0) }) }
        if edges.contains(.right) { out.formUnion(peri.filter { [SegPart.rMenuBar, .rMain, .rDock].contains($0) }) }
        if out.isEmpty { out = [.hTop] }
        return out
    }

    // MARK: - AI별 겹치지 않게 보기

    /// 켜면 각 AI가 자기 변·방향으로 따로 그린다(겹치지 않음). 끄면 같은 레일에 겹침.
    @Published var separateByAI: Bool = false
    /// AI별 레이아웃(변·시작지점·방향).
    @Published var providerLayouts: [String: ProviderLayout] = OverlaySettings.defaultLayouts()

    static func defaultLayouts() -> [String: ProviderLayout] {
        var m: [String: ProviderLayout] = [:]
        // 기본 분배(세그먼트 모델): 첫 AI=상단, 둘째=하단, 셋째=좌·본문, 넷째=우·본문.
        let segByIndex: [SegPart] = [.hTop, .hBottom, .lMain, .rMain]
        let anchorBySeg: [SegPart: BorderCorner] = [.hTop: .topLeft, .hBottom: .bottomLeft,
                                                    .lMain: .bottomLeft, .rMain: .topRight]
        for (i, spec) in ProviderSpec.all.enumerated() {
            let s = segByIndex[i % 4]
            m[spec.id] = ProviderLayout(edges: nil, anchorCorner: anchorBySeg[s] ?? .topLeft,
                                        anchorSide: .center, clockwise: true, segments: [s])
        }
        return m
    }

    func layout(for id: String) -> ProviderLayout {
        providerLayouts[id] ?? Self.defaultLayouts()[id]
            ?? ProviderLayout(edges: nil, anchorCorner: .topLeft, anchorSide: .center,
                              clockwise: true, segments: [.hTop])
    }

    /// 이 AI가 노드에서 (가로 h + 세로 v) 꺾일 때 쓰는 곡률 — 겹침 분할 클립 경계 계산용.
    /// SegmentChainShape.bendVertices와 동일한 규칙(캡 우선, 아니면 세로선 영역 꼭짓점).
    func bendRadiusEstimate(for id: String, h: SegPart, v: SegPart, node: SegNode) -> CGFloat {
        let menuOn = menuLineEnabled, dockOn = dockLineEnabled
        func zoneOf(_ s: SegPart) -> ScreenZone {
            switch s {
            case .lMenuBar, .rMenuBar: return .menuBar
            case .lDock, .rDock: return .dock
            default: return .main
            }
        }
        func bottomLevel(_ z: ScreenZone) -> Int {
            switch z { case .menuBar: return 1; case .main: return dockOn ? 2 : 3; case .dock: return 3 }
        }
        func zoneAbove(_ level: Int) -> ScreenZone? {
            switch level { case 1: return menuOn ? .menuBar : nil; case 2: return .main
                           case 3: return dockOn ? .dock : .main; default: return nil }
        }
        func zoneBelow(_ level: Int) -> ScreenZone? {
            switch level { case 0: return menuOn ? .menuBar : .main; case 1: return .main
                           case 2: return dockOn ? .dock : nil; default: return nil }
        }
        let cap = hCap(id, h, right: node.right)
        if cap == .up || cap == .down {
            if let z = (cap == .up) ? zoneAbove(node.level) : zoneBelow(node.level) {
                let corner: BorderCorner = (cap == .up)
                    ? (node.right ? .bottomRight : .bottomLeft)
                    : (node.right ? .topRight : .topLeft)
                return max(0, zoneRadiiDict(z)[corner] ?? 0)
            }
        }
        let z = zoneOf(v)
        let isBottom = bottomLevel(z) == node.level
        let corner: BorderCorner = isBottom
            ? (node.right ? .bottomRight : .bottomLeft)
            : (node.right ? .topRight : .topLeft)
        return max(0, zoneRadiiDict(z)[corner] ?? 0)
    }

    /// 커서 호버 판정용: 화면의 이 변(상/하/좌/우)에 그려진 세그먼트가 하나라도 있는지.
    func edgeActive(_ e: BorderEdge) -> Bool {
        for spec in ProviderSpec.all {
            let s = segs(for: spec.id)
            switch e {
            case .top: if s.contains(.hTop) { return true }
            case .bottom: if s.contains(.hBottom) { return true }
            case .left: if !s.isDisjoint(with: [.lMenuBar, .lMain, .lDock]) { return true }
            case .right: if !s.isDisjoint(with: [.rMenuBar, .rMain, .rDock]) { return true }
            }
        }
        return false
    }

    /// provider 색 조회(없으면 스펙 기본색).
    func color(forProvider id: String) -> Color {
        providerColors[id] ?? ProviderSpec.spec(id)?.defaultColor ?? color
    }

    /// 저장된 사용자 프리셋(최대 5개, 재시작해도 유지).
    @Published var savedPresets: [SavedPreset] = []
    static let maxPresets = 10
    private static let presetsKey = "usagemeter.savedPresets"
    private static let currentKey = "usagemeter.currentSettings"
    private static let defaultKey = "usagemeter.defaultSettings"
    private var autosave: AnyCancellable?

    /// 사용자가 지정한 기본값이 저장돼 있는지(있으면 '기본값으로 초기화' 가능).
    var hasCustomDefault: Bool { UserDefaults.standard.data(forKey: Self.defaultKey) != nil }

    /// 현재 설정을 '기본 상태'로 저장한다.
    func saveAsDefault() {
        if let data = try? JSONEncoder().encode(snapshot()) {
            UserDefaults.standard.set(data, forKey: Self.defaultKey)
        }
        objectWillChange.send()
    }

    /// 저장된 기본 상태로 되돌린다(없으면 아무 것도 안 함).
    func resetToDefault() {
        guard let data = UserDefaults.standard.data(forKey: Self.defaultKey),
              let snap = try? JSONDecoder().decode(SettingsSnapshot.self, from: data) else { return }
        apply(snap)
    }

    init() {
        loadSavedPresets()
        // 설정 스키마 버전이 오르면 저장된 라이브 설정을 1회 초기화(프리셋은 유지).
        let version = 11
        if UserDefaults.standard.integer(forKey: "usagemeter.settingsVersion") < version {
            UserDefaults.standard.removeObject(forKey: Self.currentKey)
            UserDefaults.standard.set(version, forKey: "usagemeter.settingsVersion")
        }
        // 지난번 설정을 불러와 적용(재시작 후 유지).
        var loadedStored = false
        if let data = UserDefaults.standard.data(forKey: Self.currentKey),
           let snap = try? JSONDecoder().decode(SettingsSnapshot.self, from: data) {
            apply(snap)
            loadedStored = true
        }
        // 노치 자동 감지는 **최초 실행(저장 설정 없음)일 때만** 기본값으로 반영한다.
        // 이후에는 노치 탭에서 수동으로 조절한 값이 유지된다(탭에 자동 감지 버튼 별도 제공).
        if !loadedStored {
            if let n = Self.detectNotch() {
                notchEnabled = true
                notchWidth = n.width
                notchHeight = n.height
            } else {
                notchEnabled = false
            }
        }
        // 이후 설정이 바뀌면 자동 저장(변경 직후 값으로, 300ms 디바운스).
        autosave = objectWillChange
            .debounce(for: .milliseconds(300), scheduler: RunLoop.main)
            .sink { [weak self] in self?.persistCurrent() }
    }

    /// 현재 설정을 디스크에 저장(잔여율 등 실시간 값 제외).
    private func persistCurrent() {
        if let data = try? JSONEncoder().encode(snapshot()) {
            UserDefaults.standard.set(data, forKey: Self.currentKey)
        }
    }

    // MARK: - 화면 감지

    /// 현재 주 화면의 노치 크기를 감지한다(없으면 nil). safeAreaInsets/보조영역 이용.
    static func detectNotch() -> (width: CGFloat, height: CGFloat)? {
        guard let screen = NSScreen.main else { return nil }
        let top = screen.safeAreaInsets.top
        guard top > 0 else { return nil }              // 노치 없음
        let full = screen.frame.width
        let left = screen.auxiliaryTopLeftArea?.width ?? 0
        let right = screen.auxiliaryTopRightArea?.width ?? 0
        let width = full - left - right
        guard width > 0, width < full else { return nil }
        return (width, top)
    }

    /// 특정 모서리 곡률 조회(없으면 0).
    func radius(_ corner: BorderCorner) -> CGFloat { cornerRadii[corner] ?? 0 }

    /// 특정 노치 모서리 곡률 조회(없으면 0).
    func notchRadius(_ corner: NotchCorner) -> CGFloat { notchRadii[corner] ?? 0 }

    // MARK: - 저장 프리셋(사용자가 현재 설정을 저장/불러오기)

    /// 현재 설정을 스냅샷으로 캡처(잔여율·실시간 값은 제외).
    func snapshot() -> SettingsSnapshot {
        let ns = NSColor(color).usingColorSpace(.sRGB) ?? .green
        return SettingsSnapshot(
            thickness: Double(thickness),
            colorRGBA: [ns.redComponent, ns.greenComponent, ns.blueComponent, ns.alphaComponent].map(Double.init),
            edges: edges.map { $0.rawValue },
            cornerRadii: Dictionary(uniqueKeysWithValues: cornerRadii.map { ($0.key.rawValue, Double($0.value)) }),
            showTrack: showTrack,
            notchEnabled: notchEnabled, notchWidth: Double(notchWidth), notchHeight: Double(notchHeight),
            notchRadii: Dictionary(uniqueKeysWithValues: notchRadii.map { ($0.key.rawValue, Double($0.value)) }),
            anchorCorner: anchorCorner.rawValue, anchorSide: anchorSide.rawValue, anchorClockwise: anchorClockwise,
            lineOpacity: Double(lineOpacity), extendStartCorner: extendStartCorner,
            extendEndCorner: extendEndCorner, startCornerAbove: startCornerAbove,
            fadeEnabled: fadeEnabled, fadeFraction: Double(fadeFraction),
            keepLoggedIn: keepLoggedIn,
            providerColors: Dictionary(uniqueKeysWithValues: providerColors.map { (id, c) in
                let ns = NSColor(c).usingColorSpace(.sRGB) ?? .green
                return (id, [ns.redComponent, ns.greenComponent, ns.blueComponent, ns.alphaComponent].map(Double.init))
            }),
            separateByAI: separateByAI,
            providerLayouts: providerLayouts,
            showPercentInMenuBar: nil,
            menuBarPercentIDs: Array(menuBarPercentIDs),
            menuBarIconID: menuBarIconID,
            menuBarPercentFirstID: menuBarPercentFirstID,
            menuShow5h: menuShow5h, menuShowWeekly: menuShowWeekly,
            menuShowReset: menuShowReset, menuShowUpdated: menuShowUpdated,
            language: language.rawValue,
            refreshPresetMinutes: refreshPresetMinutes,
            refreshUseCustom: refreshUseCustom,
            refreshCustomMinutes: refreshCustomMinutes,
            menuShowOpus: menuShowOpus,
            menuShowCountdown: menuShowCountdown,
            menuShowPace: menuShowPace,
            menuShowChart: menuShowChart,
            notifyEnabled: notifyEnabled,
            notifyThresholds: Array(notifyThresholds),
            menuLineEnabled: menuLineEnabled,
            menuLineHeight: Double(menuLineHeight),
            dockLineEnabled: dockLineEnabled,
            dockLineHeight: Double(dockLineHeight),
            globalZones: zones.map { $0.rawValue },
            menuZoneRadii: Dictionary(uniqueKeysWithValues: menuZoneRadii.map { ($0.key.rawValue, Double($0.value)) }),
            dockZoneRadii: Dictionary(uniqueKeysWithValues: dockZoneRadii.map { ($0.key.rawValue, Double($0.value)) }),
            globalSegments: segParts.map { $0.rawValue },
            segCapStart: segCapStart.rawValue,
            segCapEnd: segCapEnd.rawValue,
            noOverlapLines: noOverlapLines,
            splitOverlapLines: splitOverlapLines,
            chartHours: chartHours,
            linkMenuMainRadii: linkMenuMainRadii,
            linkMainDockRadii: linkMainDockRadii
        )
    }

    /// 스냅샷을 현재 설정에 적용.
    func apply(_ s: SettingsSnapshot) {
        thickness = CGFloat(s.thickness)
        if s.colorRGBA.count == 4 {
            color = Color(.sRGB, red: s.colorRGBA[0], green: s.colorRGBA[1], blue: s.colorRGBA[2], opacity: s.colorRGBA[3])
        }
        let es = s.edges.compactMap { BorderEdge(rawValue: $0) }
        if !es.isEmpty { edges = Set(es) }
        cornerRadii = Dictionary(uniqueKeysWithValues: s.cornerRadii.compactMap { k, v in
            BorderCorner(rawValue: k).map { ($0, CGFloat(v)) } })
        showTrack = s.showTrack
        notchEnabled = s.notchEnabled; notchWidth = CGFloat(s.notchWidth); notchHeight = CGFloat(s.notchHeight)
        notchRadii = Dictionary(uniqueKeysWithValues: s.notchRadii.compactMap { k, v in
            NotchCorner(rawValue: k).map { ($0, CGFloat(v)) } })
        anchorCorner = BorderCorner(rawValue: s.anchorCorner) ?? .topLeft
        anchorSide = AnchorSide(rawValue: s.anchorSide) ?? .center
        anchorClockwise = s.anchorClockwise
        lineOpacity = CGFloat(s.lineOpacity ?? 1)
        extendStartCorner = s.extendStartCorner ?? false
        extendEndCorner = s.extendEndCorner ?? false
        startCornerAbove = s.startCornerAbove ?? false
        fadeEnabled = s.fadeEnabled ?? true
        fadeFraction = CGFloat(s.fadeFraction ?? 0.012)
        keepLoggedIn = s.keepLoggedIn ?? true
        if let pc = s.providerColors {
            for (id, a) in pc where a.count == 4 {
                providerColors[id] = Color(.sRGB, red: a[0], green: a[1], blue: a[2], opacity: a[3])
            }
        }
        separateByAI = s.separateByAI ?? false
        if let pl = s.providerLayouts { providerLayouts = pl }
        if let ids = s.menuBarPercentIDs {
            menuBarPercentIDs = Set(ids)
        } else if s.showPercentInMenuBar == true {
            menuBarPercentIDs = Set(ProviderSpec.all.map { $0.id })   // 구버전: 전부 표시로 이전
        }
        menuBarIconID = s.menuBarIconID ?? menuBarIconID
        menuBarPercentFirstID = s.menuBarPercentFirstID ?? menuBarPercentFirstID
        menuShow5h = s.menuShow5h ?? true
        menuShowWeekly = s.menuShowWeekly ?? true
        menuShowReset = s.menuShowReset ?? true
        menuShowUpdated = s.menuShowUpdated ?? false
        if let l = s.language, let lang = AppLanguage(rawValue: l) { language = lang }
        refreshPresetMinutes = s.refreshPresetMinutes ?? 5
        refreshUseCustom = s.refreshUseCustom ?? false
        refreshCustomMinutes = s.refreshCustomMinutes ?? 5
        menuShowOpus = s.menuShowOpus ?? true
        menuShowCountdown = s.menuShowCountdown ?? true
        menuShowPace = s.menuShowPace ?? false
        menuShowChart = s.menuShowChart ?? false
        notifyEnabled = s.notifyEnabled ?? false
        if let t = s.notifyThresholds { notifyThresholds = Set(t) }
        menuLineEnabled = s.menuLineEnabled ?? false
        menuLineHeight = CGFloat(s.menuLineHeight ?? 25)
        dockLineEnabled = s.dockLineEnabled ?? false
        dockLineHeight = CGFloat(s.dockLineHeight ?? 70)
        if let gz = s.globalZones {
            let set = Set(gz.compactMap { ScreenZone(rawValue: $0) })
            if Self.isValidZones(set) { zones = set }
        }
        if let mr = s.menuZoneRadii {
            menuZoneRadii = Dictionary(uniqueKeysWithValues: mr.compactMap { k, v in
                BorderCorner(rawValue: k).map { ($0, CGFloat(v)) } })
        }
        if let dr = s.dockZoneRadii {
            dockZoneRadii = Dictionary(uniqueKeysWithValues: dr.compactMap { k, v in
                BorderCorner(rawValue: k).map { ($0, CGFloat(v)) } })
        }
        if let gs = s.globalSegments {
            let set = Set(gs.compactMap { SegPart(rawValue: $0) })
            if Self.isValidSegments(set, menuOn: menuLineEnabled, dockOn: dockLineEnabled) { segParts = set }
        } else {
            // 구버전: 변+영역 설정에서 변환.
            segParts = Self.migrateToSegments(edges: edges, zones: zones,
                                              menuOn: menuLineEnabled, dockOn: dockLineEnabled)
        }
        segCapStart = s.segCapStart.flatMap { SegCap(rawValue: $0) } ?? .none
        segCapEnd = s.segCapEnd.flatMap { SegCap(rawValue: $0) } ?? .none
        noOverlapLines = s.noOverlapLines ?? false
        splitOverlapLines = s.splitOverlapLines ?? false
        chartHours = s.chartHours ?? 24
        linkMenuMainRadii = s.linkMenuMainRadii ?? true
        linkMainDockRadii = s.linkMainDockRadii ?? true
        // 스키마 일원화(#정리): 구버전(변+영역) 저장분은 세그먼트로 1회 변환해 고정한다.
        for spec in ProviderSpec.all {
            if var l = providerLayouts[spec.id], l.segments == nil {
                l.segments = Self.migrateToSegments(edges: l.edges ?? Set(BorderEdge.allCases),
                                                    zones: l.zoneSet,
                                                    menuOn: menuLineEnabled, dockOn: dockLineEnabled)
                providerLayouts[spec.id] = l
            }
        }
    }

    /// 현재 설정을 이름으로 저장(최대 maxPresets).
    @discardableResult
    func saveCurrent(name: String) -> Bool {
        guard savedPresets.count < Self.maxPresets else { return false }
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        let finalName = trimmed.isEmpty ? "프리셋 \(savedPresets.count + 1)" : trimmed
        savedPresets.append(SavedPreset(id: UUID(), name: finalName, snapshot: snapshot()))
        persistPresets()
        return true
    }

    func loadPreset(_ preset: SavedPreset) { apply(preset.snapshot) }

    func deletePreset(_ preset: SavedPreset) {
        savedPresets.removeAll { $0.id == preset.id }
        persistPresets()
    }

    private func persistPresets() {
        if let data = try? JSONEncoder().encode(savedPresets) {
            UserDefaults.standard.set(data, forKey: Self.presetsKey)
        }
    }

    private func loadSavedPresets() {
        guard let data = UserDefaults.standard.data(forKey: Self.presetsKey),
              let list = try? JSONDecoder().decode([SavedPreset].self, from: data) else { return }
        savedPresets = list
    }

    // MARK: - 메뉴 프리셋

    static let presetCornerRadii: [(String, CGFloat)] = [
        ("직각 (0)", 0), ("살짝 (6)", 6), ("보통 (12)", 12),
        ("둥글게 (20)", 20), ("많이 (32)", 32),
    ]

    /// 대표 AI 브랜드 로고 주요 색 + 몇 가지 기본색.
    static let presetColors: [(String, Color)] = [
        ("Claude", Color(red: 0.85, green: 0.47, blue: 0.34)),      // Anthropic 코랄 #D97757
        ("ChatGPT", Color(red: 0.06, green: 0.64, blue: 0.50)),     // OpenAI 틸 #10A37F
        ("Gemini", Color(red: 0.26, green: 0.52, blue: 0.96)),      // Google 블루 #4285F4
        ("Copilot", Color(red: 0.00, green: 0.47, blue: 0.83)),     // MS 블루 #0078D4
        ("Perplexity", Color(red: 0.13, green: 0.50, blue: 0.55)),  // 틸 #20808D
        ("Mistral", Color(red: 0.98, green: 0.31, blue: 0.06)),     // 오렌지 #FA5010
        ("Grok", Color(red: 0.10, green: 0.10, blue: 0.11)),        // 다크 #1A1A1C
    ]

    static let presetThicknesses: [(String, CGFloat)] = [
        ("머리카락 (1)", 1),
        ("아주 얇게 (1.5)", 1.5),
        ("얇게 (2)", 2),
        ("보통 (3)", 3),
        ("두껍게 (5)", 5),
    ]

    static let presetNotchWidths: [(String, CGFloat)] = [
        ("좁게 (160)", 160), ("보통 (200)", 200),
        ("넓게 (230)", 230), ("아주 넓게 (260)", 260),
    ]
}
