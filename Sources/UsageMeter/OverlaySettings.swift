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

/// AI별 "겹치지 않게 보기"에서 각 AI가 쓸 변·시작지점·방향·선 끝 마감.
struct ProviderLayout: Codable {
    var edges: Set<BorderEdge>
    var anchorCorner: BorderCorner
    var anchorSide: AnchorSide
    var clockwise: Bool
    // 선 끝 마감(구버전 호환 위해 옵셔널). nil = 기본(false).
    var extendStart: Bool?
    var extendEnd: Bool?
    var noCurveExtend: Bool?    // 곡선을 이웃 변으로 안 뻗기(= 기존 startCornerAbove)

    var xStart: Bool { extendStart ?? false }
    var xEnd: Bool { extendEnd ?? false }
    var xNoCurve: Bool { noCurveExtend ?? false }
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

    /// 그릴 변(면)들. 기본값은 4변 전체. 바뀌면 시작 꼭짓점 유효성 보정.
    @Published var edges: Set<BorderEdge> = Set(BorderEdge.allCases) {
        didSet { clampAnchorCorner() }
    }

    /// 모서리별 곡률 반경(pt). 0이면 직각. 기본: 위(좌상·우상)만 22, 아래는 0.
    @Published var cornerRadii: [BorderCorner: CGFloat] = [
        .topLeft: 22, .topRight: 22, .bottomRight: 0, .bottomLeft: 0,
    ]

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
    @Published var language: AppLanguage = .korean
    /// 현재 언어로 문자열 조회.
    func t(_ key: String) -> String { Loc.tr(key, language) }
    /// %@ 를 인자로 치환.
    func tf(_ key: String, _ arg: String) -> String { t(key).replacingOccurrences(of: "%@", with: arg) }
    /// %d 를 숫자로 치환.
    func tn(_ key: String, _ n: Int) -> String { t(key).replacingOccurrences(of: "%d", with: String(n)) }

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

    // MARK: - AI별 겹치지 않게 보기

    /// 켜면 각 AI가 자기 변·방향으로 따로 그린다(겹치지 않음). 끄면 같은 레일에 겹침.
    @Published var separateByAI: Bool = false
    /// AI별 레이아웃(변·시작지점·방향).
    @Published var providerLayouts: [String: ProviderLayout] = OverlaySettings.defaultLayouts()

    static func defaultLayouts() -> [String: ProviderLayout] {
        var m: [String: ProviderLayout] = [:]
        // 기본 분배: 첫 AI=상변, 둘째=하변, 셋째=좌변, 넷째=우변.
        let edgeByIndex: [BorderEdge] = [.top, .bottom, .left, .right]
        let anchorByEdge: [BorderEdge: BorderCorner] = [.top: .topLeft, .bottom: .bottomLeft, .left: .bottomLeft, .right: .topRight]
        for (i, spec) in ProviderSpec.all.enumerated() {
            let e = edgeByIndex[i % 4]
            m[spec.id] = ProviderLayout(edges: [e], anchorCorner: anchorByEdge[e] ?? .topLeft, anchorSide: .center, clockwise: true)
        }
        return m
    }

    func layout(for id: String) -> ProviderLayout {
        providerLayouts[id] ?? Self.defaultLayouts()[id] ?? ProviderLayout(edges: [.top], anchorCorner: .topLeft, anchorSide: .center, clockwise: true)
    }

    // 이 AI가 주어진 변 조합을 쓸 수 있는지: 연속이고, 다른 AI와 **완전히 같은 조합**만 아니면 OK.
    /// (겹치는 건 허용 — 예: 상우하 vs 하. 단 상 vs 상 처럼 동일 조합은 금지.)
    func canUseLayout(_ id: String, _ edges: Set<BorderEdge>) -> Bool {
        guard !edges.isEmpty, Self.isContiguous(edges) else { return false }
        for spec in ProviderSpec.all where spec.id != id {
            if layout(for: spec.id).edges == edges { return false }
        }
        return true
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
        let version = 10
        if UserDefaults.standard.integer(forKey: "usagemeter.settingsVersion") < version {
            UserDefaults.standard.removeObject(forKey: Self.currentKey)
            UserDefaults.standard.set(version, forKey: "usagemeter.settingsVersion")
        }
        // 지난번 설정을 불러와 적용(재시작 후 유지).
        if let data = UserDefaults.standard.data(forKey: Self.currentKey),
           let snap = try? JSONDecoder().decode(SettingsSnapshot.self, from: data) {
            apply(snap)
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

    // MARK: - 시작 꼭짓점 유효성(#2)

    /// 꼭짓점에 인접한 두 변.
    static func cornerEdges(_ c: BorderCorner) -> (BorderEdge, BorderEdge) {
        switch c {
        case .topLeft: return (.top, .left)
        case .topRight: return (.top, .right)
        case .bottomRight: return (.bottom, .right)
        case .bottomLeft: return (.bottom, .left)
        }
    }

    /// 주어진 변 집합에서 시작 꼭짓점으로 쓸 수 있는 것들.
    static func validAnchors(_ edges: Set<BorderEdge>) -> [BorderCorner] {
        if edges.count >= 4 { return BorderCorner.allCases }
        return BorderCorner.allCases.filter {
            let (e1, e2) = cornerEdges($0)
            return edges.contains(e1) != edges.contains(e2)
        }
    }

    /// 시작 꼭짓점으로 고를 수 있는 것들(현재 전역 edges 기준).
    var validAnchorCorners: [BorderCorner] { Self.validAnchors(edges) }

    /// 현재 시작 꼭짓점이 유효하지 않으면 유효한 것으로 보정.
    func clampAnchorCorner() {
        if !validAnchorCorners.contains(anchorCorner) {
            anchorCorner = validAnchorCorners.first ?? .topLeft
        }
    }

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

    /// 변 집합이 '이어붙는(연속)' 형태인지. 서로 안 붙는 조합(상+하, 좌+우)은 불가.
    static func isContiguous(_ set: Set<BorderEdge>) -> Bool {
        if set.count <= 1 || set.count >= 4 { return true }
        if set.count == 2 { return set != [.top, .bottom] && set != [.left, .right] }
        return true   // 3개는 항상 연속
    }

    /// 변 e를 토글해도 되는지(최소 1변 유지 + 연속 형태 유지).
    func canToggleEdge(_ e: BorderEdge) -> Bool {
        var s = edges
        if s.contains(e) {
            if s.count <= 1 { return false }
            s.remove(e)
        } else {
            s.insert(e)
        }
        return Self.isContiguous(s)
    }

    /// 4변 전체가 선택돼 루프(닫힌 고리)를 이루는지.
    var isLoop: Bool { edges.count == 4 }

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
            language: language.rawValue
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

    static let presetEdgeSets: [(String, Set<BorderEdge>)] = [
        ("전체 (4변)", Set(BorderEdge.allCases)),
        ("좌변만", [.left]),
        ("우변만", [.right]),
        ("상변만", [.top]),
        ("하변만", [.bottom]),
        ("ㄷ자 (좌·하·우)", [.left, .bottom, .right]),
        ("ㄴ자 (좌·하)", [.left, .bottom]),
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
