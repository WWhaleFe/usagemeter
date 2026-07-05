import SwiftUI
import ServiceManagement

/// 전체 설정을 한 화면에서 보고 조절하는 설정 창 내용.
/// 각 값은 슬라이더 + 숫자 입력칸 + 화살표(Stepper)로 조절할 수 있고,
/// 공유 `OverlaySettings`에 바로 바인딩되어 오버레이가 실시간으로 바뀐다.
struct SettingsView: View {
    @ObservedObject var settings: OverlaySettings
    /// 로그인 탭에서 각 AI의 로그인/로그아웃을 다루기 위한 세션 관리자.
    @ObservedObject var manager: ProviderManager
    @State private var newPresetName: String = ""
    @State private var tab: String = "login"
    /// 다이어그램 겹침 경고: (AI id, 메시지). 잠깐 표시 후 자동 해제.
    @State private var segWarning: (String, String)? = nil
    /// Mac 로그인 시 자동 실행 상태(SMAppService가 진실의 원천 — 시스템이 관리).
    @State private var launchAtLogin: Bool = SettingsView.isLaunchAtLoginEnabled()
    /// 자동 실행은 번들 앱(.app)에서만 가능(swift run은 번들 ID 없음).
    private let launchAtLoginAvailable = Bundle.main.bundleIdentifier != nil

    static func isLaunchAtLoginEnabled() -> Bool {
        guard Bundle.main.bundleIdentifier != nil else { return false }
        return SMAppService.mainApp.status == .enabled
    }

    private static let fmt: NumberFormatter = {
        let f = NumberFormatter()
        f.minimumFractionDigits = 0
        f.maximumFractionDigits = 1
        return f
    }()

    var body: some View {
        VStack(spacing: 10) {
            // 탭과 무관하게 항상 보이는 상단.
            VStack(alignment: .leading, spacing: 10) {
                baseStateSection
            }
            .padding(.horizontal, 18)
            .padding(.top, 14)

            TabView(selection: $tab) {
                tabScroll { loginTab }.tabItem { Label(settings.t("tab.login"), systemImage: "person.circle") }.tag("login")
                tabScroll { presetTab }.tabItem { Label(settings.t("tab.presets"), systemImage: "square.stack.3d.up") }.tag("presets")
                tabScroll { displayTab }.tabItem { Label(settings.t("tab.display"), systemImage: "menubar.rectangle") }.tag("display")
                tabScroll { intervalTab }.tabItem { Label(settings.t("tab.interval"), systemImage: "arrow.clockwise") }.tag("interval")
                tabScroll { lineTab }.tabItem { Label(settings.t("tab.line"), systemImage: "paintbrush") }.tag("line")
                tabScroll { partitionTab }.tabItem { Label(settings.t("tab.partition"), systemImage: "square.split.1x2") }.tag("partition")
                tabScroll { layoutTab }.tabItem { Label(settings.t("tab.layout"), systemImage: "square.dashed") }.tag("layout")
                tabScroll { cornerTab }.tabItem { Label(settings.t("tab.corner"), systemImage: "rectangle.roundedtop") }.tag("corner")
                tabScroll { notchTab }.tabItem { Label(settings.t("tab.notch"), systemImage: "rectangle.tophalf.inset.filled") }.tag("notch")
            }
            .padding(.horizontal, 10)
            .padding(.bottom, 10)
        }
        .frame(width: 720, height: 800)
        .onAppear { consumeRequestedTab() }
        .onChange(of: settings.requestedTab) { _, _ in consumeRequestedTab() }
    }

    /// 메뉴에서 특정 탭 열기 요청이 있으면 반영(#5).
    private func consumeRequestedTab() {
        if let r = settings.requestedTab {
            tab = r
            settings.requestedTab = nil
        }
    }

    /// 각 탭 내용을 스크롤 + 여백으로 감싼다(대개는 스크롤 없이 다 보이는 높이).
    @ViewBuilder
    private func tabScroll<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) { content() }
                .padding(14)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    // MARK: - 항상 보이는 상단

    @ViewBuilder private var baseStateSection: some View {
        section(settings.t("sec.baseState")) {
            HStack {
                Button(settings.t("base.save")) { settings.saveAsDefault() }
                Button(settings.t("base.reset")) { settings.resetToDefault() }
                    .disabled(!settings.hasCustomDefault)
            }
        }
    }

    // MARK: - 로그인 탭

    /// AI 계정별 로그인/로그아웃 + 로그인 옵션(로그인 유지·자동 실행).
    @ViewBuilder private var loginTab: some View {
        section(settings.t("sec.accounts")) {
            ForEach(ProviderSpec.all) { spec in
                accountRow(spec)
                if spec.id != ProviderSpec.all.last?.id { Divider().padding(.vertical, 2) }
            }
        }
        section(settings.t("sec.loginOpts")) {
            Toggle(settings.t("login.keep"), isOn: $settings.keepLoggedIn)
            Toggle(settings.t("login.autostart"), isOn: launchAtLoginBinding)
                .disabled(!launchAtLoginAvailable)
                .help(launchAtLoginAvailable ? "" : settings.t("login.autostartNote"))
            if !launchAtLoginAvailable {
                Text(settings.t("login.autostartNote"))
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
    }

    /// 한 AI의 계정 행: [상태 점] [이름] [상태 텍스트] ---- [로그인/로그아웃 버튼]
    @ViewBuilder
    private func accountRow(_ spec: ProviderSpec) -> some View {
        let loggedIn = manager.isLoggedIn(spec.id)
        HStack(spacing: 10) {
            Circle()
                .fill(loggedIn ? Color.green : Color.secondary.opacity(0.35))
                .frame(width: 9, height: 9)
            Text(spec.name + (spec.id == "gemini" ? settings.t("menu.experimental") : ""))
                .font(.subheadline).bold()
            Text(loggedIn ? settings.t("acct.loggedIn") : settings.t("acct.loggedOut"))
                .font(.caption).foregroundStyle(.secondary)
            Spacer()
            if loggedIn {
                Button(settings.t("acct.logout")) { manager.logout(spec.id) }
            } else {
                Button(settings.t("acct.login")) { manager.login(spec.id) }
            }
        }
    }

    @ViewBuilder private var presetTab: some View {
        section(settings.tn("sec.presets", OverlaySettings.maxPresets)) {
            ForEach(settings.savedPresets) { preset in
                HStack {
                    Button(preset.name) { settings.loadPreset(preset) }
                    Spacer()
                    Button(role: .destructive) { settings.deletePreset(preset) } label: {
                        Image(systemName: "trash")
                    }.buttonStyle(.borderless)
                }
            }
            HStack {
                TextField(settings.t("preset.newName"), text: $newPresetName).textFieldStyle(.roundedBorder)
                Button(settings.t("preset.save")) {
                    settings.saveCurrent(name: newPresetName); newPresetName = ""
                }.disabled(settings.savedPresets.count >= OverlaySettings.maxPresets)
            }
        }
    }

    // MARK: - 탭들

    @ViewBuilder private var displayTab: some View {
        section(settings.t("lang.title")) {
            Picker("", selection: $settings.language) {
                ForEach(AppLanguage.allCases, id: \.self) { Text($0.displayName).tag($0) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
        }
        section(settings.t("sec.menuIcon")) {
            Picker(settings.t("disp.iconBaseAI"), selection: $settings.menuBarIconID) {
                ForEach(ProviderSpec.all) { Text($0.name).tag($0.id) }
            }
            Text(settings.t("disp.iconDesc"))
                .font(.caption).foregroundStyle(.secondary)
        }
        section(settings.t("sec.menuPercent")) {
            ForEach(ProviderSpec.all) { spec in
                Toggle(settings.tf("disp.showPercent", spec.name), isOn: menuBarPercentBinding(spec.id))
            }
            let bothOn = settings.menuBarPercentIDs.count >= 2
            Picker(settings.t("disp.firstAI"), selection: $settings.menuBarPercentFirstID) {
                ForEach(ProviderSpec.all) { Text($0.name).tag($0.id) }
            }.disabled(!bothOn)
            Text(settings.t("disp.percentDesc"))
                .font(.caption).foregroundStyle(.secondary)
        }
        section(settings.t("sec.dropdownInfo")) {
            checkGrid([
                ("info.5h", $settings.menuShow5h),
                ("info.weekly", $settings.menuShowWeekly),
                ("info.opus", $settings.menuShowOpus),
                ("info.reset", $settings.menuShowReset),
                ("info.countdown", $settings.menuShowCountdown),
                ("info.pace", $settings.menuShowPace),
                ("info.chart", $settings.menuShowChart),
                ("info.updated", $settings.menuShowUpdated),
            ])
        }
        section(settings.t("sec.notify")) {
            Toggle(settings.t("notify.enable"), isOn: $settings.notifyEnabled)
            HStack(spacing: 20) {
                ForEach([75, 90, 95], id: \.self) { th in
                    Toggle(settings.tn("notify.thFmt", th), isOn: notifyThresholdBinding(th))
                        .disabled(!settings.notifyEnabled)
                }
            }
            Text(settings.t("notify.desc"))
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    /// 체크박스 항목들을 2열 그리드로 배치(세로로만 길어지는 것 방지).
    @ViewBuilder
    private func checkGrid(_ items: [(String, Binding<Bool>)]) -> some View {
        let cols = [GridItem(.flexible(), alignment: .leading),
                    GridItem(.flexible(), alignment: .leading)]
        LazyVGrid(columns: cols, alignment: .leading, spacing: 6) {
            ForEach(items.indices, id: \.self) { i in
                Toggle(settings.t(items[i].0), isOn: items[i].1)
            }
        }
    }

    /// Mac 로그인 시 자동 실행 토글 — SMAppService 등록/해제(시스템 로그인 항목).
    private var launchAtLoginBinding: Binding<Bool> {
        Binding(get: { launchAtLogin },
                set: { on in
                    guard launchAtLoginAvailable else { return }
                    do {
                        if on { try SMAppService.mainApp.register() }
                        else { try SMAppService.mainApp.unregister() }
                    } catch {
                        // 등록 실패(권한 등) 시 실제 상태로 되돌림.
                    }
                    launchAtLogin = Self.isLaunchAtLoginEnabled()
                })
    }

    /// 임계치 알림 대상에 이 사용률이 포함되는지 토글.
    private func notifyThresholdBinding(_ th: Int) -> Binding<Bool> {
        Binding(get: { settings.notifyThresholds.contains(th) },
                set: { on in
                    if on { settings.notifyThresholds.insert(th) }
                    else { settings.notifyThresholds.remove(th) }
                })
    }

    @ViewBuilder private var intervalTab: some View {
        section(settings.t("sec.interval")) {
            Picker(settings.t("interval.preset"), selection: $settings.refreshPresetMinutes) {
                ForEach(OverlaySettings.refreshPresets, id: \.self) { m in
                    Text(settings.tn("interval.minutesFmt", m)).tag(m)
                }
            }
            .disabled(settings.refreshUseCustom)
            Divider().padding(.vertical, 2)
            Toggle(settings.t("interval.useCustom"), isOn: $settings.refreshUseCustom)
            sliderRow(settings.t("interval.customMin"), value: customMinutesBinding, range: 1...180, step: 1)
                .disabled(!settings.refreshUseCustom)
            Text(settings.tn("interval.currentFmt", settings.effectiveRefreshMinutes))
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    @ViewBuilder private var lineTab: some View {
        section(settings.t("sec.line")) {
            sliderRow(settings.t("line.thickness"), value: $settings.thickness, range: 1...8, step: 0.5)
            sliderRow(settings.t("line.opacity"), value: transparencyBinding, range: 0...100, step: 1)
            Toggle(settings.t("line.track"), isOn: $settings.showTrack)
            Toggle(settings.t("line.fade"), isOn: $settings.fadeEnabled)
            sliderRow(settings.t("line.fadeLen"), value: fadeBinding, range: 0...20, step: 1)
                .disabled(!settings.fadeEnabled)
        }
        // 겹침 표시(#겹침 규칙): 레이어 순서 + 굵기 분할 옵션.
        section(settings.t("sec.overlap")) {
            Text(settings.t("overlap.layerDesc"))
                .font(.caption).foregroundStyle(.secondary)
            Toggle(settings.t("lay.splitOverlap"), isOn: $settings.splitOverlapLines)
            Text(settings.t("overlap.splitDesc"))
                .font(.caption).foregroundStyle(.secondary)
        }
        section(settings.t("sec.aiColor")) {
            ForEach(ProviderSpec.all) { spec in
                providerColorRow(spec)
                if spec.id != ProviderSpec.all.last?.id { Divider().padding(.vertical, 2) }
            }
        }
    }

    /// 한 AI의 선 색: [이름] [현재색(직접 선택)] [프리셋 팔레트]를 한 줄로.
    @ViewBuilder
    private func providerColorRow(_ spec: ProviderSpec) -> some View {
        HStack(spacing: 8) {
            // 이름 길이가 달라도 색 스와치 열이 어긋나지 않게 라벨 폭 고정.
            Text(settings.tf("color.lineColor", spec.name)).font(.subheadline)
                .frame(width: 120, alignment: .leading)
            ColorPicker("", selection: providerColorBinding(spec.id), supportsOpacity: true)
                .labelsHidden()                                  // 현재 설정된 색 블럭(클릭 시 직접 선택)
                .help("직접 선택")
            ForEach(OverlaySettings.presetColors.indices, id: \.self) { i in
                let c = OverlaySettings.presetColors[i]
                let sel = isProviderColor(spec.id, c.1)
                Circle().fill(c.1).frame(width: 22, height: 22)
                    .overlay(Circle().stroke(sel ? Color.accentColor : .secondary.opacity(0.4),
                                             lineWidth: sel ? 2.5 : 1))
                    .contentShape(Circle())
                    .onTapGesture { settings.providerColors[spec.id] = c.1 }
                    .help(c.0)                                   // 호버 시 테마 색 이름
            }
            Spacer(minLength: 0)
        }
    }

    /// 화면 분할 탭(분리): 가로 경계선 켜기/위치 + 드래그 조정.
    @ViewBuilder private var partitionTab: some View {
        section(settings.t("sec.partition")) {
            Toggle(settings.t("part.menuLine"), isOn: $settings.menuLineEnabled)
            sliderRow(settings.t("part.menuHeight"), value: $settings.menuLineHeight, range: 5...200)
                .disabled(!settings.menuLineEnabled)
                .opacity(settings.menuLineEnabled ? 1 : 0.4)
            Divider().padding(.vertical, 2)
            Toggle(settings.t("part.dockLine"), isOn: $settings.dockLineEnabled)
            sliderRow(settings.t("part.dockHeight"), value: $settings.dockLineHeight, range: 5...400)
                .disabled(!settings.dockLineEnabled)
                .opacity(settings.dockLineEnabled ? 1 : 0.4)
            // 화면 위에서 경계선을 직접 드래그로 조정(#드래그 조정).
            Button(settings.t("part.dragAdjust")) {
                LineDragOverlay.shared.show(settings: settings)
            }
            .disabled(!settings.menuLineEnabled && !settings.dockLineEnabled)
            Text(settings.t("part.desc"))
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    @ViewBuilder private var layoutTab: some View {
        section(settings.t("sec.layout")) {
            // 겹치는 선 없음: 켜는 순간 기존 겹침을 정리하고(#요청3,4) 이후 겹침을 차단한다.
            Toggle(settings.t("lay.noOverlap"), isOn: Binding(
                get: { settings.noOverlapLines },
                set: { on in
                    settings.noOverlapLines = on
                    if on { settings.enforceNoOverlap() }
                }))
            // 항상 AI별 선택(#요청1) — 겹쳐도 된다.
            ForEach(ProviderSpec.all) { spec in
                VStack(alignment: .leading, spacing: 6) {
                    Text(spec.name).font(.subheadline).bold()
                    segmentEditor(id: spec.id)
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(RoundedRectangle(cornerRadius: Self.boxRadius).fill(Color.primary.opacity(0.05)))
            }
        }
    }

    /// 모서리 곡률 탭(분리 복원): 영역별 꼭짓점 곡률.
    /// 파티션이 꺼져 있으면 본문(=화면 전체)만 보인다.
    @ViewBuilder private var cornerTab: some View {
        section(settings.t("sec.zoneRadii")) {
            ForEach(zoneOrder, id: \.self) { z in
                if z == .main || zoneAvailable(z) {
                    VStack(alignment: .leading, spacing: 4) {
                        if settings.menuLineEnabled || settings.dockLineEnabled {
                            Text(z.label(settings.language)).font(.subheadline).bold()
                        }
                        // 직관적 배치: 왼쪽 열 = 좌상 위·좌하 아래, 오른쪽 열 = 우상 위·우하 아래.
                        let cols = [GridItem(.flexible()), GridItem(.flexible())]
                        LazyVGrid(columns: cols, alignment: .leading, spacing: 4) {
                            ForEach([BorderCorner.topLeft, .topRight, .bottomLeft, .bottomRight], id: \.self) { c in
                                // 현재 선택에서 실제로 쓰이는 꼭짓점만 활성화.
                                let used = settings.cornerUsed(z, c)
                                sliderRow(c.label(settings.language),
                                          value: zoneRadiusBinding(z, c), range: 0...80)
                                    .disabled(!used)
                                    .opacity(used ? 1 : 0.35)
                            }
                        }
                    }
                    // 곡률 연결(#연결): 경계선을 공유하는 이웃 영역과 맞닿은 꼭짓점을 함께 조절.
                    if z == .menuBar {
                        Toggle(settings.t("corner.linkMenuMain"), isOn: $settings.linkMenuMainRadii)
                            .padding(.vertical, 2)
                    }
                    if z == .main && zoneAvailable(.dock) {
                        Toggle(settings.t("corner.linkMainDock"), isOn: $settings.linkMainDockRadii)
                            .padding(.vertical, 2)
                    }
                    if z != zoneOrder.last(where: { $0 == .main || zoneAvailable($0) }) {
                        Divider().padding(.vertical, 2)
                    }
                }
            }
        }
    }

    /// 노치 탭(부활): 감싸기 켜기/끄기 + 크기 + 노치 모서리 곡률 + 자동 감지.
    @ViewBuilder private var notchTab: some View {
        section(settings.t("sec.notch")) {
            Toggle(settings.t("notch.enable"), isOn: $settings.notchEnabled)
            Group {
                sliderRow(settings.t("notch.width"), value: $settings.notchWidth, range: 0...500)
                sliderRow(settings.t("notch.height"), value: $settings.notchHeight, range: 0...100)
                Divider().padding(.vertical, 2)
                Text(settings.t("notch.cornerRadius")).font(.caption).foregroundStyle(.secondary)
                ForEach(NotchCorner.allCases, id: \.self) { c in
                    sliderRow(c.label(settings.language), value: notchBinding(c), range: 0...40)
                }
            }
            .disabled(!settings.notchEnabled)                 // 꺼지면 노치 세부 비활성화
            .opacity(settings.notchEnabled ? 1 : 0.4)
            // 이 화면의 노치를 자동 감지해 크기를 채운다(노치 없으면 끔).
            Button(settings.t("notch.detect")) {
                if let n = OverlaySettings.detectNotch() {
                    settings.notchEnabled = true
                    settings.notchWidth = n.width
                    settings.notchHeight = n.height
                } else {
                    settings.notchEnabled = false
                }
            }
        }
    }

    private func notchBinding(_ c: NotchCorner) -> Binding<CGFloat> {
        Binding(get: { settings.notchRadii[c] ?? 0 }, set: { settings.notchRadii[c] = $0 })
    }

    // MARK: - 세그먼트 편집기 (#배치 수정 1: 좌=옵션, 우=세그먼트 그리드)

    @ViewBuilder
    private func segmentEditor(id: String?) -> some View {
        let menuOn = settings.menuLineEnabled, dockOn = settings.dockLineEnabled
        let segs = currentSegs(id)
        HStack(alignment: .top, spacing: 14) {
            // 좌측: 모양 프리셋 + 끝 모서리 + 차감 방향 반전.
            VStack(alignment: .leading, spacing: 10) {
                // 모양 프리셋(빠른 선택 통합 #요청6): 영역 둘레 + 어울리는 캡을 한 번에 적용.
                VStack(alignment: .leading, spacing: 4) {
                    Text(settings.t("lay.presets")).font(.caption).foregroundStyle(.secondary)
                    HStack(spacing: 5) {
                        presetButton(id, "lay.zoneAll", zones: Set(ScreenZone.allCases), caps: [:])
                        presetButton(id, "zone.menuBar", zones: [.menuBar],
                                     caps: [OverlaySettings.hCapKey(.hMenu, right: false): .down,
                                            OverlaySettings.hCapKey(.hMenu, right: true): .down])
                            .disabled(!settings.menuLineEnabled)
                        presetButton(id, "zone.main", zones: [.main], caps: [:])
                        presetButton(id, "zone.dock", zones: [.dock],
                                     caps: [OverlaySettings.hCapKey(.hDock, right: false): .up,
                                            OverlaySettings.hCapKey(.hDock, right: true): .up])
                            .disabled(!settings.dockLineEnabled)
                    }
                }
                // 가로선 끝 위/아래 둥글게 — 세로선이 붙어 있어도, 고리여도 항상 설정 가능.
                let hsSel = [SegPart.hTop, .hMenu, .hDock, .hBottom].filter { segs.contains($0) }
                if !hsSel.isEmpty {
                    VStack(alignment: .leading, spacing: 4) {
                        // 무엇을 설정하는지 명시(#라벨 개선): 가로선 양 끝 모서리의 꺾임 방향.
                        Text(settings.t("lay.hCapsTitle")).font(.caption).foregroundStyle(.secondary)
                        ForEach(hsSel, id: \.self) { h in
                            hCapRow(id: id, h: h)
                        }
                    }
                }
                if OverlaySettings.chainEnds(segs, menuOn: menuOn, dockOn: dockOn) != nil {
                    Toggle(settings.t("lay.reverse"), isOn: cwBinding(id))
                } else {
                    anchorRow(corner: anchorCornerBinding(id), corners: BorderCorner.allCases,
                              clockwise: cwBinding(id), isLoop: true)
                    switch anchorSideKind(id) {
                    case .hidden: EmptyView()
                    case .round: anchorSideRow(side: anchorSideBinding(id), scoop: false)
                    case .scoop: anchorSideRow(side: anchorSideBinding(id), scoop: true)
                    }
                }
                Spacer(minLength: 0)
                // 더블 클릭 안내(#요청3).
                Text(settings.t("lay.dblHint"))
                    .font(.caption2).foregroundStyle(.secondary)
            }
            .frame(width: 380, alignment: .leading)
            Spacer(minLength: 8)
            // 우측: 인터랙티브 화면 다이어그램 — 선분을 직접 클릭해 토글(세로 가운데 정렬 #요청7).
            VStack {
                Spacer(minLength: 0)
                segDiagram(id: id)
                Spacer(minLength: 0)
            }
            .padding(.trailing, 6)
        }
        // 옵션 수가 변해도 박스 크기 고정(#요청4).
        .frame(height: 268, alignment: .top)
    }

    /// 모양 프리셋 버튼: 영역 둘레 + 끝 캡 묶음을 한 번에 적용.
    private func presetButton(_ id: String?, _ key: String,
                              zones: Set<ScreenZone>, caps: [String: SegCap]) -> some View {
        Button {
            guard let id else { return }
            let menuOn = settings.menuLineEnabled, dockOn = settings.dockLineEnabled
            let segs = OverlaySettings.zonePerimeter(zones, menuOn: menuOn, dockOn: dockOn)
            guard settings.canUseSegments(id, segs) else { return }
            mutateLayout(id) { $0.zones = zones; $0.segments = segs; $0.hCaps = caps }
        } label: { Text(settings.t(key)).frame(width: 52) }
    }

    /// 한 가로선의 좌/우 끝 위/아래 둥글게 행. 불가능한 방향(상단↑, 하단↓)만 비활성화.
    /// 모든 요소 고정폭 + 줄바꿈 금지로 열이 밀리지 않게 정렬(#배치 가독성).
    @ViewBuilder
    private func hCapRow(id: String?, h: SegPart) -> some View {
        HStack(spacing: 5) {
            Text(h.label(settings.language)).font(.caption)
                .frame(width: 62, alignment: .leading).fixedSize(horizontal: false, vertical: true)
            Text(settings.t("cap.leftEnd")).font(.caption).foregroundStyle(.secondary)
                .frame(width: 38, alignment: .trailing)
            Toggle(settings.t("cap.upShort"), isOn: hCapFlag(id, h, right: false, dir: .up))
                .disabled(h == .hTop).fixedSize()
                .help(settings.t("cap.upHelp"))
            Toggle(settings.t("cap.downShort"), isOn: hCapFlag(id, h, right: false, dir: .down))
                .disabled(h == .hBottom).fixedSize()
                .help(settings.t("cap.downHelp"))
            Divider().frame(height: 14)
            Text(settings.t("cap.rightEnd")).font(.caption).foregroundStyle(.secondary)
                .frame(width: 48, alignment: .trailing)
            Toggle(settings.t("cap.upShort"), isOn: hCapFlag(id, h, right: true, dir: .up))
                .disabled(h == .hTop).fixedSize()
                .help(settings.t("cap.upHelp"))
            Toggle(settings.t("cap.downShort"), isOn: hCapFlag(id, h, right: true, dir: .down))
                .disabled(h == .hBottom).fixedSize()
                .help(settings.t("cap.downHelp"))
        }
    }
    /// 특정 가로선 끝의 특정 방향 체크박스(같은 끝의 반대 방향은 자동 해제).
    private func hCapFlag(_ id: String?, _ h: SegPart, right: Bool, dir: SegCap) -> Binding<Bool> {
        Binding(get: {
            guard let id else { return false }
            return settings.hCap(id, h, right: right) == dir
        }, set: { on in
            guard let id else { return }
            mutateLayout(id) { l in
                var d = l.hCaps ?? [:]
                d[OverlaySettings.hCapKey(h, right: right)] = on ? dir : SegCap.none
                l.hCaps = d
            }
        })
    }

    /// 인터랙티브 화면 다이어그램(#제안1): 화면 모양 위에 세그먼트를 선분으로 그리고
    /// 직접 클릭해 토글한다. 선택 = AI 색, 미선택 = 회색, 선택 불가 = 흐리게.
    @ViewBuilder
    private func segDiagram(id: String?) -> some View {
        let menuOn = settings.menuLineEnabled, dockOn = settings.dockLineEnabled
        let W: CGFloat = 250, H: CGFloat = 160
        let color = id.map { settings.color(forProvider: $0) } ?? .accentColor
        let frames = diagramFrames(menuOn: menuOn, dockOn: dockOn, W: W, H: H)
        ZStack(alignment: .topLeading) {
            RoundedRectangle(cornerRadius: 10).fill(Color.secondary.opacity(0.07))
            RoundedRectangle(cornerRadius: 10).strokeBorder(Color.secondary.opacity(0.2), lineWidth: 1)
            ForEach(frames, id: \.0) { item in
                let (s, f) = item
                let on = currentSegs(id).contains(s)
                let can = segCanToggle(id, s)
                // 선택/선택 가능/불가 3단계를 확실히 구분(#요청1):
                // 선택 = AI 색 굵은 선, 가능 = 진한 회색 + 테두리, 불가 = 아주 흐린 점선 느낌.
                Capsule()
                    .fill(on ? color : (can ? Color.secondary.opacity(0.6) : Color.secondary.opacity(0.12)))
                    .overlay(Capsule().strokeBorder(
                        on ? color.opacity(0.9) : (can ? Color.primary.opacity(0.35) : .clear), lineWidth: 1))
                    .frame(width: s.isHorizontal ? f.width : 7,
                           height: s.isHorizontal ? 7 : f.height)
                    .frame(width: f.width, height: f.height)     // 클릭 영역은 더 두껍게
                    .contentShape(Rectangle())
                    .position(x: f.midX, y: f.midY)
                    .onTapGesture(count: 2) { exclusiveSelect(id, s) }   // 더블 클릭 = 그 선만(#요청2)
                    .onTapGesture { if can { segBinding(id, s).wrappedValue = !on } }
                    .help(s.label(settings.language))
            }
            // 겹치는 선 경고(잠깐 표시 후 사라짐 #요청2).
            if let w = segWarning, w.0 == (id ?? "") {
                Text(w.1)
                    .font(.caption).foregroundStyle(.white)
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(Color.red.opacity(0.88), in: RoundedRectangle(cornerRadius: 7))
                    .position(x: W / 2, y: H / 2)
                    .transition(.opacity)
            }
        }
        .frame(width: W, height: H)
        .animation(.easeInOut(duration: 0.2), value: segWarning?.0)
    }

    /// 더블 클릭: 기존 선택을 모두 해제하고 그 선만 선택(#요청2).
    /// '겹치는 선 없음' 상태에서 다른 AI가 쓰는 선이면 경고만 잠깐 띄우고 진행하지 않는다.
    private func exclusiveSelect(_ id: String?, _ s: SegPart) {
        guard let id else { return }
        guard OverlaySettings.segAvailable(s, menuOn: settings.menuLineEnabled,
                                           dockOn: settings.dockLineEnabled) else { return }
        if settings.noOverlapLines {
            for spec in ProviderSpec.all where spec.id != id {
                if settings.segs(for: spec.id).contains(s) {
                    showSegWarning(id, settings.t("lay.overlapWarn"))
                    return
                }
            }
        }
        mutateLayout(id) { $0.segments = [s] }
    }
    private func showSegWarning(_ id: String, _ msg: String) {
        segWarning = (id, msg)
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.6) {
            if segWarning?.0 == id { segWarning = nil }
        }
    }

    /// 다이어그램에서 각 세그먼트의 (선분, 클릭 영역) 위치. 레벨은 보기 좋게 균등 배치.
    private func diagramFrames(menuOn: Bool, dockOn: Bool, W: CGFloat, H: CGFloat) -> [(SegPart, CGRect)] {
        var lv: [Int] = [0]
        if menuOn { lv.append(1) }
        if dockOn { lv.append(2) }
        lv.append(3)
        let inset: CGFloat = 12
        func y(_ level: Int) -> CGFloat {
            let idx = lv.firstIndex(of: level) ?? 0
            return inset + (H - 2 * inset) * CGFloat(idx) / CGFloat(max(1, lv.count - 1))
        }
        let thick: CGFloat = 16
        var out: [(SegPart, CGRect)] = []
        for (s, level) in [(SegPart.hTop, 0), (.hMenu, 1), (.hDock, 2), (.hBottom, 3)]
        where OverlaySettings.segAvailable(s, menuOn: menuOn, dockOn: dockOn) {
            out.append((s, CGRect(x: 26, y: y(level) - thick / 2, width: W - 52, height: thick)))
        }
        for s in [SegPart.lMenuBar, .lMain, .lDock, .rMenuBar, .rMain, .rDock]
        where OverlaySettings.segAvailable(s, menuOn: menuOn, dockOn: dockOn) {
            let (a, b) = OverlaySettings.segEnds(s, menuOn: menuOn, dockOn: dockOn)
            let x: CGFloat = a.right ? W - 14 : 14
            let yA = y(a.level) + 9, yB = y(b.level) - 9
            guard yB > yA else { continue }
            out.append((s, CGRect(x: x - thick / 2, y: yA, width: thick, height: yB - yA)))
        }
        return out
    }

    // MARK: - 세그먼트 바인딩

    private func currentSegs(_ id: String?) -> Set<SegPart> {
        if let id { return settings.segs(for: id) }
        return settings.segParts
    }
    private func setSegs(_ id: String?, _ s: Set<SegPart>) {
        guard OverlaySettings.isValidSegments(s, menuOn: settings.menuLineEnabled,
                                              dockOn: settings.dockLineEnabled) else { return }
        if let id {
            guard settings.canUseSegments(id, s) else { return }
            mutateLayout(id) { $0.segments = s }
        } else {
            settings.segParts = s
        }
    }
    private func segBinding(_ id: String?, _ s: SegPart) -> Binding<Bool> {
        Binding(get: { currentSegs(id).contains(s) },
                set: { on in
                    var next = currentSegs(id)
                    if on { next.insert(s) } else { next.remove(s) }
                    setSegs(id, next)
                })
    }
    /// 한 줄로 이어지는 선택만 허용(#요청4): 토글 결과가 유효해야 누를 수 있다.
    private func segCanToggle(_ id: String?, _ s: SegPart) -> Bool {
        let menuOn = settings.menuLineEnabled, dockOn = settings.dockLineEnabled
        guard OverlaySettings.segAvailable(s, menuOn: menuOn, dockOn: dockOn) else { return false }
        var next = currentSegs(id)
        if next.contains(s) { next.remove(s) } else { next.insert(s) }
        if let id { return settings.canUseSegments(id, next) }
        return OverlaySettings.isValidSegments(next, menuOn: menuOn, dockOn: dockOn)
    }
    private func cwBinding(_ id: String?) -> Binding<Bool> {
        if let id { return layoutClockwiseBinding(id) }
        return $settings.anchorClockwise
    }
    private func anchorCornerBinding(_ id: String?) -> Binding<BorderCorner> {
        if let id { return layoutAnchorBinding(id) }
        return $settings.anchorCorner
    }

    /// 영역(파티션) 버튼 순서: 메뉴바 · 본문 · Dock.
    private let zoneOrder: [ScreenZone] = [.menuBar, .main, .dock]

    /// 이 영역을 선택할 수 있는지(해당 경계선이 켜져 있어야 의미가 있다).
    private func zoneAvailable(_ z: ScreenZone) -> Bool {
        switch z {
        case .menuBar: return settings.menuLineEnabled
        case .dock: return settings.dockLineEnabled
        case .main: return true
        }
    }

    /// % 차감 시작 위치 + 방향.
    @ViewBuilder
    private func anchorRow(corner: Binding<BorderCorner>, corners: [BorderCorner],
                           clockwise: Binding<Bool>, isLoop: Bool) -> some View {
        Picker(settings.t("lay.anchor"), selection: corner) {
            ForEach(corners, id: \.self) { Text($0.label(settings.language)).tag($0) }
        }
        Toggle(settings.t("lay.clockwise"), isOn: clockwise).disabled(!isLoop)
        if !isLoop {
            Text(settings.t("lay.partialDir"))
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    /// 곡선 모서리에서 차감이 시작될 지점(#차감 시작 지점).
    /// 일반 곡률: 위/아래 2지점. 스쿱: 위(접점)/모서리/아래(오버슛 끝) 3지점(#모서리 시작 수정5).
    /// (저장은 기존 AnchorSide 재사용 — .left=위, .center=모서리, .right=아래.)
    @ViewBuilder
    private func anchorSideRow(side: Binding<AnchorSide>, scoop: Bool) -> some View {
        if scoop {
            Picker(settings.t("lay.anchorSide"), selection: side) {
                Text(settings.t("side.up")).tag(AnchorSide.left)
                Text(settings.t("side.corner")).tag(AnchorSide.center)
                Text(settings.t("side.down")).tag(AnchorSide.right)
            }
        } else {
            Picker(settings.t("lay.anchorSide"), selection: Binding(
                get: { side.wrappedValue == .right ? AnchorSide.right : .left },
                set: { side.wrappedValue = $0 })) {
                Text(settings.t("side.up")).tag(AnchorSide.left)
                Text(settings.t("side.down")).tag(AnchorSide.right)
            }
        }
    }
    private func anchorSideBinding(_ id: String?) -> Binding<AnchorSide> {
        if let id {
            return Binding(get: { settings.layout(for: id).anchorSide },
                           set: { v in mutateLayout(id) { $0.anchorSide = v } })
        }
        return Binding(get: { settings.anchorSide }, set: { settings.anchorSide = $0 })
    }

    private enum AnchorSideKind { case hidden, round, scoop }

    /// 시작 모서리의 종류: 곡률 0 → 숨김, 스쿱 → 3지점, 일반 곡률 → 2지점.
    private func anchorSideKind(_ id: String?) -> AnchorSideKind {
        guard let id else { return .hidden }
        let menuOn = settings.menuLineEnabled, dockOn = settings.dockLineEnabled
        let segs = settings.segs(for: id)
        guard OverlaySettings.isValidSegments(segs, menuOn: menuOn, dockOn: dockOn),
              OverlaySettings.chainEnds(segs, menuOn: menuOn, dockOn: dockOn) == nil else { return .hidden }
        let adj = OverlaySettings.segAdjacency(segs, menuOn: menuOn, dockOn: dockOn)
        let levels = adj.keys.map(\.level)
        guard let tl = levels.min(), let bl = levels.max() else { return .hidden }
        let l = settings.layout(for: id)
        let node: SegNode = {
            switch l.anchorCorner {
            case .topLeft: return SegNode(right: false, level: tl)
            case .topRight: return SegNode(right: true, level: tl)
            case .bottomRight: return SegNode(right: true, level: bl)
            case .bottomLeft: return SegNode(right: false, level: bl)
            }
        }()
        guard let incident = adj[node],
              let h = incident.first(where: { $0.isHorizontal }),
              let v = incident.first(where: { !$0.isHorizontal }) else { return .hidden }
        guard settings.bendRadiusEstimate(for: id, h: h, v: v, node: node) > 0.01 else { return .hidden }
        // 스쿱 판정: 가로선 캡 방향이 세로선 진행 방향과 반대.
        let cap = settings.hCap(id, h, right: node.right)
        if cap == .up || cap == .down {
            let (a, b) = OverlaySettings.segEnds(v, menuOn: menuOn, dockOn: dockOn)
            let vDown = ((a == node) ? b.level : a.level) > node.level
            let matches = (cap == .down && vDown) || (cap == .up && !vDown)
            if !matches { return .scoop }
        }
        return .round
    }

    // MARK: - 구성 요소

    /// 모든 그룹 박스·카드 공통 모서리 곡률(탭 패널과 통일).
    static let boxRadius: CGFloat = 8

    @ViewBuilder
    private func section<Content: View>(_ title: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title).font(.headline)
            content()
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: Self.boxRadius).fill(Color(nsColor: .controlBackgroundColor)))
    }

    /// 슬라이더 + 숫자 입력칸 + 화살표. 행을 포커스(Tab/클릭)하면 ←→↑↓ 키로 조절.
    private func sliderRow(_ label: String, value: Binding<CGFloat>,
                           range: ClosedRange<CGFloat>, step: CGFloat = 1) -> some View {
        LabeledSlider(label: label, value: value, range: range, step: step)
    }

    private var colorPresets: some View {
        LazyVGrid(columns: Array(repeating: GridItem(.fixed(22), spacing: 6), count: 6), spacing: 6) {
            ForEach(OverlaySettings.presetColors.indices, id: \.self) { i in
                let c = OverlaySettings.presetColors[i]
                Circle().fill(c.1).frame(width: 20, height: 20)
                    .overlay(Circle().stroke(.secondary.opacity(0.4), lineWidth: 1))
                    .onTapGesture { settings.color = c.1 }
                    .help(c.0)
            }
        }
    }

    // MARK: - 바인딩

    private func doubleBridge(_ v: Binding<CGFloat>) -> Binding<Double> {
        Binding(get: { Double(v.wrappedValue) }, set: { v.wrappedValue = CGFloat($0) })
    }
    /// 직접 입력값을 슬라이더 범위로 clamp(화살표 한계 초과 방지).
    private func clampedDouble(_ v: Binding<CGFloat>, _ range: ClosedRange<CGFloat>) -> Binding<Double> {
        Binding(get: { Double(v.wrappedValue) },
                set: { v.wrappedValue = min(range.upperBound, max(range.lowerBound, CGFloat($0))) })
    }
    /// 사용자 설정 주기(분) Int ↔ 슬라이더용 CGFloat.
    private var customMinutesBinding: Binding<CGFloat> {
        Binding(get: { CGFloat(settings.refreshCustomMinutes) },
                set: { settings.refreshCustomMinutes = min(180, max(1, Int($0.rounded()))) })
    }
    /// 0~20(%) 그라데이션 길이 ↔ fadeFraction(0~0.2).
    private var fadeBinding: Binding<CGFloat> {
        Binding(get: { settings.fadeFraction * 100 },
                set: { settings.fadeFraction = max(0, min(0.2, $0 / 100)) })
    }
    /// 0~100 투명도(클수록 투명) ↔ lineOpacity(1=불투명).
    private var transparencyBinding: Binding<CGFloat> {
        Binding(get: { (1 - settings.lineOpacity) * 100 },
                set: { settings.lineOpacity = max(0, min(1, 1 - $0 / 100)) })
    }
    /// 영역별 꼭짓점 곡률(#요청3,4).
    private func zoneRadiusBinding(_ z: ScreenZone, _ c: BorderCorner) -> Binding<CGFloat> {
        Binding(get: { settings.zoneRadiiDict(z)[c] ?? 0 },
                set: { settings.setZoneRadius(z, c, $0) })
    }
    private func providerColorBinding(_ id: String) -> Binding<Color> {
        Binding(get: { settings.color(forProvider: id) }, set: { settings.providerColors[id] = $0 })
    }
    /// 메뉴바 % 표시 대상에 이 AI가 포함되는지 토글.
    private func menuBarPercentBinding(_ id: String) -> Binding<Bool> {
        Binding(get: { settings.menuBarPercentIDs.contains(id) },
                set: { on in
                    if on { settings.menuBarPercentIDs.insert(id) }
                    else { settings.menuBarPercentIDs.remove(id) }
                })
    }
    /// 현재 provider 색이 이 프리셋 색과 (거의) 같은지 — 선택 표시용.
    private func isProviderColor(_ id: String, _ c: Color) -> Bool {
        guard let a = NSColor(settings.color(forProvider: id)).usingColorSpace(.sRGB),
              let b = NSColor(c).usingColorSpace(.sRGB) else { return false }
        return abs(a.redComponent - b.redComponent) < 0.02
            && abs(a.greenComponent - b.greenComponent) < 0.02
            && abs(a.blueComponent - b.blueComponent) < 0.02
    }

    // MARK: - AI별 레이아웃 바인딩

    private func mutateLayout(_ id: String, _ f: (inout ProviderLayout) -> Void) {
        var l = settings.layout(for: id)
        f(&l)
        settings.providerLayouts[id] = l
    }
    private func layoutAnchorBinding(_ id: String) -> Binding<BorderCorner> {
        Binding(get: { settings.layout(for: id).anchorCorner },
                set: { v in mutateLayout(id) { $0.anchorCorner = v } })
    }
    private func layoutClockwiseBinding(_ id: String) -> Binding<Bool> {
        Binding(get: { settings.layout(for: id).clockwise },
                set: { v in mutateLayout(id) { $0.clockwise = v } })
    }
}

/// 슬라이더 + 숫자입력 + 스테퍼 한 줄. 행이 포커스되면 화살표키로도 값이 조절된다.
private struct LabeledSlider: View {
    let label: String
    @Binding var value: CGFloat
    let range: ClosedRange<CGFloat>
    let step: CGFloat
    @FocusState private var focused: Bool

    private static let fmt: NumberFormatter = {
        let f = NumberFormatter(); f.minimumFractionDigits = 0; f.maximumFractionDigits = 1; return f
    }()
    private var clamped: Binding<Double> {
        Binding(get: { Double(value) },
                set: { value = min(range.upperBound, max(range.lowerBound, CGFloat($0))) })
    }
    private func adjust(_ d: CGFloat) {
        value = min(range.upperBound, max(range.lowerBound, value + d))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(label).font(.subheadline)
            HStack(spacing: 8) {
                Slider(value: $value, in: range, step: step)
                TextField("", value: clamped, formatter: Self.fmt)
                    .frame(width: 48).multilineTextAlignment(.trailing).textFieldStyle(.roundedBorder)
                Stepper("", value: $value, in: range, step: step).labelsHidden()
            }
        }
        .padding(4)
        .contentShape(Rectangle())
        .focusable()
        .focused($focused)
        .onTapGesture { focused = true }
        .overlay(RoundedRectangle(cornerRadius: 6)
            .stroke(Color.accentColor, lineWidth: focused ? 2 : 0))
        .onKeyPress(.upArrow) { adjust(step); return .handled }
        .onKeyPress(.downArrow) { adjust(-step); return .handled }
        .onKeyPress(.rightArrow) { adjust(step); return .handled }
        .onKeyPress(.leftArrow) { adjust(-step); return .handled }
    }
}
