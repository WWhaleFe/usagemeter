import AppKit
import Combine

/// 연결된 디스플레이마다 **독립적인 테두리 설정**을 관리한다.
///
/// - 주 설정(`main`, 앱 전역 `OverlaySettings`)은 한 디스플레이(`mainDisplayID`)의 테두리를 겸한다.
///   기존 사용자의 설정이 그대로 그 화면에 남도록, 처음엔 내장 디스플레이(없으면 주 화면)에 묶는다.
/// - 나머지 디스플레이는 각자 `OverlaySettings(displayID:)`를 가지며 UUID 키로 저장된다.
///   처음 연결될 때는 주 설정을 복사해 그 화면에 맞게(노치·곡률) 조정한 값으로 시작한다.
/// - 언어·AI 색·임시 숨김 등 전역 값은 언제나 주 설정을 따른다.
@MainActor
final class DisplayRegistry: ObservableObject {

    struct Display: Identifiable, Equatable {
        let id: String          // 디스플레이 UUID(재부팅·재연결에도 유지)
        let name: String
        let isPrimary: Bool     // 메뉴바가 있는 화면
        let isBuiltin: Bool
    }

    let main: OverlaySettings
    /// 현재 연결된 디스플레이(NSScreen.screens 순서).
    @Published private(set) var displays: [Display] = []
    /// 설정 창에서 테두리를 편집 중인 디스플레이.
    @Published var editingID: String = ""
    /// 테두리를 끈 디스플레이들(저장됨).
    @Published private(set) var disabledIDs: Set<String>

    /// 주 설정이 테두리를 그리는 디스플레이(저장됨).
    private(set) var mainDisplayID: String
    private var instances: [String: OverlaySettings] = [:]
    private var cancellables: Set<AnyCancellable> = []

    private static let mainDisplayKey = "usagemeter.mainDisplayUUID"
    private static let disabledKey = "usagemeter.disabledDisplays"

    init(main: OverlaySettings) {
        self.main = main
        disabledIDs = Set(UserDefaults.standard.stringArray(forKey: Self.disabledKey) ?? [])
        if let saved = UserDefaults.standard.string(forKey: Self.mainDisplayKey) {
            mainDisplayID = saved
        } else {
            let screens = NSScreen.screens
            let pick = screens.first { Self.isBuiltin($0) } ?? screens.first
            mainDisplayID = pick.map(Self.uuid(for:)) ?? ""
            UserDefaults.standard.set(mainDisplayID, forKey: Self.mainDisplayKey)
        }
        let mainID = mainDisplayID
        main.bindScreen { Self.screen(for: mainID) }

        // 디스플레이별 설정은 언어·AI 색을 주 설정에서 읽으므로, 바뀌면 다시 그리게 알린다.
        main.$language.combineLatest(main.$providerColors)
            .dropFirst()
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in self?.instances.values.forEach { $0.objectWillChange.send() } }
            .store(in: &cancellables)

        refresh()
    }

    /// 화면 구성이 바뀌면 호출: 연결 목록 갱신 + 새 디스플레이의 설정 생성.
    func refresh() {
        let primary = NSScreen.screens.first
        displays = NSScreen.screens.map { s in
            Display(id: Self.uuid(for: s), name: s.localizedName,
                    isPrimary: s == primary, isBuiltin: Self.isBuiltin(s))
        }
        for d in displays where d.id != mainDisplayID && instances[d.id] == nil {
            let id = d.id
            instances[id] = OverlaySettings(displayID: id, seed: main) { Self.screen(for: id) }
        }
        if !displays.contains(where: { $0.id == editingID }) {
            editingID = displays.first { $0.id == mainDisplayID }?.id ?? displays.first?.id ?? mainDisplayID
        }
    }

    /// 해상도 변경 등: 연결된 각 디스플레이 설정을 자기 화면에 맞게 조정한다.
    func adaptAll() {
        main.adaptToScreenChange()
        for d in displays where d.id != mainDisplayID { instances[d.id]?.adaptToScreenChange() }
    }

    /// "이 기기에 맞춤": 연결된 모든 디스플레이를 각자 화면에 맞춘다.
    func applyDeviceFitAll() {
        main.applyDeviceFit()
        for d in displays where d.id != mainDisplayID { instances[d.id]?.applyDeviceFit() }
    }

    func settings(for id: String) -> OverlaySettings {
        id == mainDisplayID ? main : (instances[id] ?? main)
    }

    func settings(for screen: NSScreen) -> OverlaySettings { settings(for: Self.uuid(for: screen)) }

    /// 설정 창에서 편집 중인 디스플레이의 설정.
    var editingSettings: OverlaySettings { settings(for: editingID) }

    var editingScreen: NSScreen? { Self.screen(for: editingID) }

    func isEnabled(_ id: String) -> Bool { !disabledIDs.contains(id) }

    func setEnabled(_ id: String, _ on: Bool) {
        if on { disabledIDs.remove(id) } else { disabledIDs.insert(id) }
        UserDefaults.standard.set(Array(disabledIDs), forKey: Self.disabledKey)
    }

    /// 편집 중인 디스플레이의 테두리 설정을 연결된 다른 모든 디스플레이에 복사한다.
    func copyEditingToOthers() {
        let src = editingSettings
        for d in displays where d.id != editingID { settings(for: d.id).copyBorder(from: src) }
    }

    /// 설정 창 피커에 보일 이름. 이름이 같은 모니터가 여럿이면 번호를 붙인다.
    func label(for d: Display) -> String {
        var name = d.name
        if displays.filter({ $0.name == d.name }).count > 1,
           let i = displays.firstIndex(of: d) { name += " (\(i + 1))" }
        if d.isPrimary { name += " — " + main.t("display.primary") }
        return name
    }

    // MARK: - 디스플레이 식별

    static func displayID(of screen: NSScreen) -> CGDirectDisplayID? {
        (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value
    }

    /// 재부팅·재연결에도 바뀌지 않는 디스플레이 UUID(실패 시 표시 번호).
    static func uuid(for screen: NSScreen) -> String {
        guard let did = displayID(of: screen) else { return screen.localizedName }
        if let u = CGDisplayCreateUUIDFromDisplayID(did)?.takeRetainedValue(),
           let str = CFUUIDCreateString(nil, u) as String? {
            return str
        }
        return "display-\(did)"
    }

    static func isBuiltin(_ screen: NSScreen) -> Bool {
        displayID(of: screen).map { CGDisplayIsBuiltin($0) != 0 } ?? false
    }

    static func screen(for id: String) -> NSScreen? {
        NSScreen.screens.first { uuid(for: $0) == id }
    }
}
