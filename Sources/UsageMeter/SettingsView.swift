import SwiftUI

/// 전체 설정을 한 화면에서 보고 조절하는 설정 창 내용.
/// 각 값은 슬라이더 + 숫자 입력칸 + 화살표(Stepper)로 조절할 수 있고,
/// 공유 `OverlaySettings`에 바로 바인딩되어 오버레이가 실시간으로 바뀐다.
struct SettingsView: View {
    @ObservedObject var settings: OverlaySettings
    @State private var newPresetName: String = ""

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
                section(settings.t("sec.login")) {
                    Toggle(settings.t("login.keep"), isOn: $settings.keepLoggedIn)
                }
            }
            .padding(.horizontal, 18)
            .padding(.top, 14)

            TabView {
                tabScroll { presetTab }.tabItem { Label(settings.t("tab.presets"), systemImage: "square.stack.3d.up") }
                tabScroll { displayTab }.tabItem { Label(settings.t("tab.display"), systemImage: "menubar.rectangle") }
                tabScroll { lineTab }.tabItem { Label(settings.t("tab.line"), systemImage: "paintbrush") }
                tabScroll { layoutTab }.tabItem { Label(settings.t("tab.layout"), systemImage: "square.dashed") }
                tabScroll { cornerTab }.tabItem { Label(settings.t("tab.corner"), systemImage: "rectangle.roundedtop") }
                tabScroll { notchTab }.tabItem { Label(settings.t("tab.notch"), systemImage: "rectangle.tophalf.inset.filled") }
            }
            .padding(.horizontal, 10)
            .padding(.bottom, 10)
        }
        .frame(width: 640, height: 740)
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
            Toggle(settings.t("info.5h"), isOn: $settings.menuShow5h)
            Toggle(settings.t("info.weekly"), isOn: $settings.menuShowWeekly)
            Toggle(settings.t("info.reset"), isOn: $settings.menuShowReset)
            Toggle(settings.t("info.updated"), isOn: $settings.menuShowUpdated)
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
            Text(settings.tf("color.lineColor", spec.name)).font(.subheadline)
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

    @ViewBuilder private var layoutTab: some View {
        section(settings.t("sec.layout")) {
            Toggle(settings.t("lay.separate"), isOn: $settings.separateByAI)
            if settings.separateByAI {
                ForEach(ProviderSpec.all) { spec in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(spec.name).font(.subheadline).bold()
                        layoutColumns {
                            edgeButtons(isOn: { layoutEdgeBinding(spec.id, $0) },
                                        canToggle: { layoutCanToggle(spec.id, $0) })
                            anchorRow(corner: layoutAnchorBinding(spec.id),
                                      corners: OverlaySettings.validAnchors(settings.layout(for: spec.id).edges),
                                      clockwise: layoutClockwiseBinding(spec.id),
                                      isLoop: settings.layout(for: spec.id).edges.count == 4)
                        } right: {
                            endFinish(extendStart: layoutExtendStart(spec.id),
                                      extendEnd: layoutExtendEnd(spec.id),
                                      noCurve: layoutNoCurve(spec.id))
                        }
                    }
                    .padding(8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: Self.boxRadius).fill(Color.primary.opacity(0.05)))
                }
            } else {
                Text(settings.t("lay.overlapDesc"))
                    .font(.caption).foregroundStyle(.secondary)
                layoutColumns {
                    edgeButtons(isOn: { edgeBinding($0) }, canToggle: { settings.canToggleEdge($0) })
                    anchorRow(corner: $settings.anchorCorner, corners: settings.validAnchorCorners,
                              clockwise: $settings.anchorClockwise, isLoop: settings.isLoop)
                } right: {
                    endFinish(extendStart: $settings.extendStartCorner,
                              extendEnd: $settings.extendEndCorner, noCurve: $settings.startCornerAbove)
                }
            }
        }
    }

    @ViewBuilder private var cornerTab: some View {
        section(settings.t("sec.corner")) {
            ForEach(BorderCorner.allCases, id: \.self) { c in
                sliderRow(c.label(settings.language), value: cornerBinding(c), range: 0...80)
            }
        }
    }

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
            .disabled(!settings.notchEnabled)                 // #4: 꺼지면 노치 세부 비활성화
            .opacity(settings.notchEnabled ? 1 : 0.4)
        }
    }

    /// 선 끝 마감 토글 3종(전역·AI별 공용). 오른쪽 열에 세로로 배치.
    @ViewBuilder
    private func endFinish(extendStart: Binding<Bool>, extendEnd: Binding<Bool>, noCurve: Binding<Bool>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(settings.t("lay.endFinish")).font(.caption).foregroundStyle(.secondary)
            Toggle(settings.t("lay.extendStart"), isOn: extendStart)
            Toggle(settings.t("lay.noCurve"), isOn: noCurve)
                .disabled(!extendStart.wrappedValue)
            Toggle(settings.t("lay.extendEnd"), isOn: extendEnd)
        }
    }

    /// 배치 한 세트: 왼쪽(변·방향) + 오른쪽(선 끝 마감) 2열.
    @ViewBuilder
    private func layoutColumns<L: View, R: View>(@ViewBuilder left: () -> L,
                                                 @ViewBuilder right: () -> R) -> some View {
        HStack(alignment: .top, spacing: 16) {
            VStack(alignment: .leading, spacing: 8) { left() }
                .frame(maxWidth: .infinity, alignment: .leading)
            right().frame(width: 250, alignment: .leading)
        }
    }

    /// 버튼 표시 순서: 상 하 좌 우.
    private let edgeOrder: [BorderEdge] = [.top, .bottom, .left, .right]

    /// 상 하 좌 우 변 선택 버튼(모든 곳에서 같은 스타일).
    @ViewBuilder
    private func edgeButtons(isOn: @escaping (BorderEdge) -> Binding<Bool>,
                             canToggle: @escaping (BorderEdge) -> Bool) -> some View {
        HStack(spacing: 6) {
            ForEach(edgeOrder, id: \.self) { e in
                Toggle(e.label(settings.language), isOn: isOn(e)).toggleStyle(.button).disabled(!canToggle(e))
            }
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
    private func edgeBinding(_ e: BorderEdge) -> Binding<Bool> {
        Binding(get: { settings.edges.contains(e) },
                set: { _ in
                    guard settings.canToggleEdge(e) else { return }   // 안 붙는 조합 방지
                    if settings.edges.contains(e) { settings.edges.remove(e) }
                    else { settings.edges.insert(e) }
                })
    }
    private func cornerBinding(_ c: BorderCorner) -> Binding<CGFloat> {
        Binding(get: { settings.cornerRadii[c] ?? 0 }, set: { settings.cornerRadii[c] = $0 })
    }
    private func notchBinding(_ c: NotchCorner) -> Binding<CGFloat> {
        Binding(get: { settings.notchRadii[c] ?? 0 }, set: { settings.notchRadii[c] = $0 })
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
        // 시작 꼭짓점 유효성 보정.
        if !OverlaySettings.validAnchors(l.edges).contains(l.anchorCorner) {
            l.anchorCorner = OverlaySettings.validAnchors(l.edges).first ?? .topLeft
        }
        settings.providerLayouts[id] = l
    }
    private func layoutEdgeBinding(_ id: String, _ e: BorderEdge) -> Binding<Bool> {
        Binding(get: { settings.layout(for: id).edges.contains(e) },
                set: { on in
                    var next = settings.layout(for: id).edges
                    if on { next.insert(e) } else if next.count > 1 { next.remove(e) }
                    guard settings.canUseLayout(id, next) else { return }
                    mutateLayout(id) { $0.edges = next }
                })
    }
    private func layoutAnchorBinding(_ id: String) -> Binding<BorderCorner> {
        Binding(get: { settings.layout(for: id).anchorCorner },
                set: { v in mutateLayout(id) { $0.anchorCorner = v } })
    }
    private func layoutClockwiseBinding(_ id: String) -> Binding<Bool> {
        Binding(get: { settings.layout(for: id).clockwise },
                set: { v in mutateLayout(id) { $0.clockwise = v } })
    }
    private func layoutExtendStart(_ id: String) -> Binding<Bool> {
        Binding(get: { settings.layout(for: id).xStart }, set: { v in mutateLayout(id) { $0.extendStart = v } })
    }
    private func layoutExtendEnd(_ id: String) -> Binding<Bool> {
        Binding(get: { settings.layout(for: id).xEnd }, set: { v in mutateLayout(id) { $0.extendEnd = v } })
    }
    private func layoutNoCurve(_ id: String) -> Binding<Bool> {
        Binding(get: { settings.layout(for: id).xNoCurve }, set: { v in mutateLayout(id) { $0.noCurveExtend = v } })
    }
    private func layoutCanToggle(_ id: String, _ e: BorderEdge) -> Bool {
        var s = settings.layout(for: id).edges
        if s.contains(e) { if s.count <= 1 { return false }; s.remove(e) } else { s.insert(e) }
        return settings.canUseLayout(id, s)
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
