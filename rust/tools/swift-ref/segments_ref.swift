// 세그먼트 그래프 기준값 생성기 (Rust 이식 검증용).
// OverlaySettings.swift의 열거형·세그먼트 함수를 그대로 복사하고(SwiftUI/ObservableObject 부분만 제거),
// 전수·시나리오 입력에 대한 결과를 JSON Lines로 출력한다.
// 실행: swift segments_ref.swift > ../../crates/um-core/tests/fixtures/segments_ref.jsonl
// 복사 구간(OverlaySettings.swift): 97-125 SegPart·SegNode·SegCap, 130-148 ProviderLayout,
// 306-325 곡률 사전, 361-381 enforceNoOverlap, 563-668 segs~cornerUsed, 683-881 세그먼트 그래프~edgeActive.
// (복사본과의 차이: @Published 제거, OverlaySettings.defaultLayouts -> RefSettings.defaultLayouts 한 곳, SegPart.label 제거)
import Foundation

enum BorderEdge: String, CaseIterable, Codable { case top, right, bottom, left }
enum BorderCorner: String, CaseIterable, Codable { case topLeft, topRight, bottomRight, bottomLeft }
enum AnchorSide: String, CaseIterable, Codable { case left, center, right }
enum ScreenZone: String, CaseIterable, Codable { case menuBar, main, dock }

struct ProviderSpec {
    let id: String
    static let all: [ProviderSpec] = [ProviderSpec(id: "claude"), ProviderSpec(id: "gemini"), ProviderSpec(id: "codex")]
}

// ---- 이하 OverlaySettings.swift에서 복사 ----
/// 화면 골격의 선분(세그먼트). 가로 4개(상단/메뉴바선/Dock선/하단) +
/// 세로 좌·우 각 3구간(메뉴바/본문/Dock). 경계선이 꺼져 있으면 관련 세그먼트는 선택 불가.
enum SegPart: String, CaseIterable, Codable {
    case hTop, hMenu, hDock, hBottom          // 가로선 4개(#요청1)
    case lMenuBar, lMain, lDock               // 좌변 3구간(#요청2)
    case rMenuBar, rMain, rDock               // 우변 3구간

    var isHorizontal: Bool {
        self == .hTop || self == .hMenu || self == .hDock || self == .hBottom
    }
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

final class RefSettings {
    var menuLineEnabled = false
    var dockLineEnabled = false
    var noOverlapLines = false
    var cornerRadii: [BorderCorner: CGFloat] = [
        .topLeft: 22, .topRight: 22, .bottomRight: 0, .bottomLeft: 0,
    ]
    /// 메뉴바 영역의 모서리별 곡률(영역별 개별 설정 — #요청3,4).
    var menuZoneRadii: [BorderCorner: CGFloat] = [
        .topLeft: 22, .topRight: 22, .bottomRight: 0, .bottomLeft: 0,
    ]
    /// Dock 영역의 모서리별 곡률.
    var dockZoneRadii: [BorderCorner: CGFloat] = [
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
    var separateByAI: Bool = false
    /// AI별 레이아웃(변·시작지점·방향).
    var providerLayouts: [String: ProviderLayout] = RefSettings.defaultLayouts()

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

}

// ---- 이하 출력 드라이버(복사본 아님) ----

func js(_ s: String) -> String { "\"\(s)\"" }
func jb(_ b: Bool) -> String { b ? "true" : "false" }
func segList(_ s: Set<SegPart>) -> String {
    "[" + SegPart.allCases.filter { s.contains($0) }.map { js($0.rawValue) }.joined(separator: ",") + "]"
}
func maskOf(_ s: Set<SegPart>) -> Int {
    var m = 0
    for (i, p) in SegPart.allCases.enumerated() where s.contains(p) { m |= 1 << i }
    return m
}
func setOf(mask m: Int) -> Set<SegPart> {
    Set(SegPart.allCases.enumerated().filter { (m >> $0.offset) & 1 == 1 }.map { $0.element })
}
func nodeJ(_ n: SegNode) -> String { "[\(jb(n.right)),\(n.level)]" }
func zoneMask(_ m: Int) -> Set<ScreenZone> {
    Set(ScreenZone.allCases.enumerated().filter { (m >> $0.offset) & 1 == 1 }.map { $0.element })
}
func edgeMask(_ m: Int) -> Set<BorderEdge> {
    Set(BorderEdge.allCases.enumerated().filter { (m >> $0.offset) & 1 == 1 }.map { $0.element })
}
func zoneList(_ s: Set<ScreenZone>) -> String {
    "[" + ScreenZone.allCases.filter { s.contains($0) }.map { js($0.rawValue) }.joined(separator: ",") + "]"
}
func edgeList(_ s: Set<BorderEdge>) -> String {
    "[" + BorderEdge.allCases.filter { s.contains($0) }.map { js($0.rawValue) }.joined(separator: ",") + "]"
}

// 1) 전수: 부분집합 1024 x menuOn x dockOn -> 유효성 + chainEnds
for menuOn in [false, true] {
    for dockOn in [false, true] {
        for mask in 0..<1024 {
            let set = setOf(mask: mask)
            let valid = RefSettings.isValidSegments(set, menuOn: menuOn, dockOn: dockOn)
            var chain = "null"
            if let c = RefSettings.chainEnds(set, menuOn: menuOn, dockOn: dockOn) {
                chain = "{\"start\":{\"node\":\(nodeJ(c.start.node)),\"seg\":\(js(c.start.seg.rawValue))},"
                    + "\"end\":{\"node\":\(nodeJ(c.end.node)),\"seg\":\(js(c.end.seg.rawValue))}}"
            }
            // 모든 세그먼트의 양 끝 노드는 마스크와 무관하므로 mask==0 줄에만 싣는다.
            var ends = "null"
            if mask == 0 {
                ends = "[" + SegPart.allCases.map {
                    let (a, b) = RefSettings.segEnds($0, menuOn: menuOn, dockOn: dockOn)
                    return "[\(nodeJ(a)),\(nodeJ(b)),\(jb(RefSettings.segAvailable($0, menuOn: menuOn, dockOn: dockOn)))]"
                }.joined(separator: ",") + "]"
            }
            print("{\"kind\":\"valid\",\"mask\":\(mask),\"menuOn\":\(jb(menuOn)),\"dockOn\":\(jb(dockOn)),"
                + "\"valid\":\(jb(valid)),\"chain\":\(chain),\"segEnds\":\(ends)}")
        }
    }
}

// 2) zonePerimeter: 영역 부분집합(빈 집합 포함) x menuOn x dockOn
for menuOn in [false, true] {
    for dockOn in [false, true] {
        for zm in 0..<8 {
            let p = RefSettings.zonePerimeter(zoneMask(zm), menuOn: menuOn, dockOn: dockOn)
            print("{\"kind\":\"perimeter\",\"zones\":\(zm),\"menuOn\":\(jb(menuOn)),\"dockOn\":\(jb(dockOn)),\"segs\":\(segList(p))}")
        }
    }
}

// 3) migrateToSegments: 변 부분집합 16 x 영역 8 x menuOn x dockOn
for menuOn in [false, true] {
    for dockOn in [false, true] {
        for zm in 0..<8 {
            for em in 0..<16 {
                let r = RefSettings.migrateToSegments(edges: edgeMask(em), zones: zoneMask(zm), menuOn: menuOn, dockOn: dockOn)
                print("{\"kind\":\"migrate\",\"edges\":\(em),\"zones\":\(zm),\"menuOn\":\(jb(menuOn)),\"dockOn\":\(jb(dockOn)),\"segs\":\(segList(r))}")
            }
        }
    }
}

// 4) 다중 AI 시나리오
struct Rng {
    var s: UInt64
    mutating func next() -> UInt64 {
        s = s &* 6364136223846793005 &+ 1442695040888963407
        return s >> 33
    }
    mutating func below(_ n: Int) -> Int { Int(next() % UInt64(n)) }
    mutating func chance(_ pct: Int) -> Bool { below(100) < pct }
}
var rng = Rng(s: 20260101)
let ids = ["claude", "gemini", "codex"]
let hsegs: [SegPart] = [.hTop, .hMenu, .hDock, .hBottom]
let vsegs: [SegPart] = [.lMenuBar, .lMain, .lDock, .rMenuBar, .rMain, .rDock]
let capAll = ["none", "up", "down", "include"]
let radiusPool: [Double] = [-3, 0, 4, 10, 12.5, 22]
func sparseMask(_ r: inout Rng, _ pct: Int) -> Int {
    var m = 0
    for i in 0..<10 where r.chance(pct) { m |= 1 << i }
    return m
}
func radiiDict(_ r: inout Rng) -> [BorderCorner: CGFloat] {
    var d: [BorderCorner: CGFloat] = [:]
    for c in BorderCorner.allCases { d[c] = CGFloat(radiusPool[r.below(radiusPool.count)]) }
    return d
}
func radiiJ(_ d: [BorderCorner: CGFloat]) -> String {
    "{" + BorderCorner.allCases.map { "\(js($0.rawValue)):\(Double(d[$0] ?? 0))" }.joined(separator: ",") + "}"
}

for sc in 0..<400 {
    let st = RefSettings()
    st.menuLineEnabled = rng.chance(50)
    st.dockLineEnabled = rng.chance(50)
    // 시나리오 절반은 '겹치는 선 없음' 켬
    st.noOverlapLines = (sc % 2 == 0)
    st.cornerRadii = radiiDict(&rng)
    st.menuZoneRadii = radiiDict(&rng)
    st.dockZoneRadii = radiiDict(&rng)
    st.providerLayouts = [:]
    var provJ: [String] = []
    for id in ids {
        let kind = rng.below(10)
        if kind == 0 {
            // 저장된 레이아웃 없음 -> 기본 레이아웃으로 대체
            provJ.append("{\"present\":false}")
            continue
        }
        var segsSet: Set<SegPart>? = nil
        var edgesSet: Set<BorderEdge>? = nil
        var zonesSet: Set<ScreenZone>? = nil
        if kind <= 2 {
            // 구버전: 변 + 영역 (edges/zones 각각 없을 수도 있음)
            if rng.chance(80) { edgesSet = edgeMask(rng.below(16)) }
            if rng.chance(80) { zonesSet = zoneMask(rng.below(8)) }
        } else if kind <= 6 {
            // 거의 유효한 한 줄: 영역 둘레의 일부
            let z = zoneMask(1 + rng.below(7))
            let peri = RefSettings.zonePerimeter(z, menuOn: st.menuLineEnabled, dockOn: st.dockLineEnabled)
            segsSet = Set(SegPart.allCases.filter { peri.contains($0) && rng.chance(75) })
        } else {
            segsSet = setOf(mask: sparseMask(&rng, 30))
        }
        var caps: [String: SegCap]? = nil
        if rng.chance(55) {
            var d: [String: SegCap] = [:]
            for h in hsegs {
                for right in [false, true] where rng.chance(50) {
                    d[RefSettings.hCapKey(h, right: right)] = SegCap(rawValue: capAll[rng.below(4)])!
                }
            }
            caps = d
        }
        let l = ProviderLayout(edges: edgesSet, anchorCorner: .topLeft, anchorSide: .center, clockwise: true,
                               zones: zonesSet, segments: segsSet, hCaps: caps)
        st.providerLayouts[id] = l
        let capsJ: String = {
            guard let c = caps else { return "null" }
            return "{" + c.keys.sorted().map { "\(js($0)):\(js(c[$0]!.rawValue))" }.joined(separator: ",") + "}"
        }()
        provJ.append("{\"present\":true,\"segments\":\(segsSet.map(segList) ?? "null"),"
            + "\"edges\":\(edgesSet.map(edgeList) ?? "null"),\"zones\":\(zonesSet.map(zoneList) ?? "null"),\"hCaps\":\(capsJ)}")
    }

    // 후보 조합: 무작위 6개 + 각 AI의 현재 조합
    var cands: [Int] = (0..<6).map { _ in
        // 절반은 유효한 쪽(영역 둘레 일부), 절반은 아무 조합
        if rng.chance(50) {
            let z = zoneMask(1 + rng.below(7))
            return maskOf(RefSettings.zonePerimeter(z, menuOn: st.menuLineEnabled, dockOn: st.dockLineEnabled))
        }
        return sparseMask(&rng, 30)
    }
    var segsOut: [String] = []
    for id in ids {
        let s = st.segs(for: id)
        cands.append(maskOf(s))
        segsOut.append(segList(s))
    }
    let canUse = ids.map { id in "[" + cands.map { jb(st.canUseSegments(id, setOf(mask: $0))) }.joined(separator: ",") + "]" }

    var nodes: [SegNode] = []
    for lv in 0..<4 { for r in [false, true] { nodes.append(SegNode(right: r, level: lv)) } }
    var eo: [String] = []
    for id in ids {
        var codes: [String] = []
        for n in nodes {
            for s in SegPart.allCases {
                let (half, auto) = st.endOverride(for: id, node: n, seg: s)
                codes.append("\((half ? 1 : 0) + (auto ? 2 : 0))")
            }
        }
        eo.append("[" + codes.joined(separator: ",") + "]")
    }
    var cu: [String] = []
    for z in ScreenZone.allCases { for c in BorderCorner.allCases { cu.append(jb(st.cornerUsed(z, c))) } }
    var bend: [String] = []
    for id in ids {
        for h in hsegs { for v in vsegs { for n in nodes {
            bend.append("\(Double(st.bendRadiusEstimate(for: id, h: h, v: v, node: n)))")
        } } }
    }
    let ea = BorderEdge.allCases.map { jb(st.edgeActive($0)) }
    // enforceNoOverlap은 상태를 바꾸므로 마지막에 호출
    st.enforceNoOverlap()
    let enf = ids.map { segList(st.segs(for: $0)) }
    print("{\"kind\":\"scenario\",\"menuOn\":\(jb(st.menuLineEnabled)),\"dockOn\":\(jb(st.dockLineEnabled)),"
        + "\"noOverlap\":\(jb(st.noOverlapLines)),\"main\":\(radiiJ(st.cornerRadii)),\"menu\":\(radiiJ(st.menuZoneRadii)),"
        + "\"dock\":\(radiiJ(st.dockZoneRadii)),\"providers\":[\(provJ.joined(separator: ","))],"
        + "\"segs\":[\(segsOut.joined(separator: ","))],\"cands\":\(cands),"
        + "\"canUse\":[\(canUse.joined(separator: ","))],\"endOverride\":[\(eo.joined(separator: ","))],"
        + "\"cornerUsed\":[\(cu.joined(separator: ","))],\"bend\":[\(bend.joined(separator: ","))],"
        + "\"edgeActive\":[\(ea.joined(separator: ","))],\"enforced\":[\(enf.joined(separator: ","))]}")
}
