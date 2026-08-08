import AppKit
import SwiftUI
import Combine

/// 메뉴바 아이콘 + 메뉴. 로그인/사용량 확인/종료/설정(굵기·색)을 여기서 다룬다.
@MainActor
final class StatusBarController: NSObject {

    private let statusItem: NSStatusItem
    private let settings: OverlaySettings
    private let manager: ProviderManager
    private let history: HistoryStore
    private let updater: UpdateChecker

    /// 드롭다운 메뉴(열 때마다 정보 섹션을 다시 채운다).
    private let mainMenu = NSMenu()

    /// 변(면) 토글 메뉴 항목 — 메뉴 열 때 체크 상태를 갱신하기 위해 참조 보관.

    /// 트랙 배경 표시 토글 항목.
    private let trackItem = NSMenuItem(title: "트랙 배경 표시", action: nil, keyEquivalent: "")

    /// 노치 감싸기 켜기/끄기 토글 항목.
    private let notchToggleItem = NSMenuItem(title: "노치 감싸기 켜기", action: nil, keyEquivalent: "")

    /// AI(로그인/로그아웃) 서브메뉴 — 열 때마다 provider별 상태로 다시 채운다.
    private let aiSubmenu = NSMenu()

    /// 설정 창 컨트롤러(지연 생성).
    private lazy var settingsWC = SettingsWindowController(settings: settings, manager: manager, updater: updater)

    /// 분석 대시보드 창(지연 생성).
    private lazy var analyticsWC = AnalyticsWindowController(settings: settings, manager: manager, history: history)

    /// 커서를 테두리에 올리면 현재 정보를 보여주는 컨트롤러.
    private let hoverInfo: HoverInfoController

    /// 프리셋 서브메뉴(저장 목록이 바뀌면 다시 채운다).
    private let presetSubmenu = NSMenu()

    private var cancellables: Set<AnyCancellable> = []

    init(settings: OverlaySettings, manager: ProviderManager, history: HistoryStore, updater: UpdateChecker) {
        self.settings = settings
        self.manager = manager
        self.history = history
        self.updater = updater
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        hoverInfo = HoverInfoController(settings: settings)
        super.init()

        statusItem.button?.toolTip = "UsageMeter"
        updateStatusIcon()
        // 설정(색) 또는 사용량이 바뀌면 링·호버 정보 갱신.
        settings.objectWillChange
            .merge(with: manager.objectWillChange)
            .debounce(for: .milliseconds(150), scheduler: RunLoop.main)
            .sink { [weak self] in self?.updateStatusIcon(); self?.updateHoverInfo() }
            .store(in: &cancellables)
        // 설정·분석 창을 서로 열 수 있게 훅 주입(분석창의 "앱 설정", 설정창의 "분석 보기").
        settings.onOpenAnalytics = { [weak self] in self?.analyticsWC.show() }
        settings.onOpenSettings = { [weak self] in self?.settingsWC.show() }
        mainMenu.autoenablesItems = false
        mainMenu.delegate = self
        populateMenu(mainMenu)
        statusItem.menu = mainMenu
    }

    /// 드롭다운을 처음부터 다시 채운다(정보 섹션은 현재 사용량/표시 옵션 반영).
    private func populateMenu(_ menu: NSMenu) {
        menu.removeAllItems()

        // 1) AI별 정보 섹션 — SwiftUI 카드로 고대비 렌더(#가독성). providerOrder 순서.
        let infos = buildAIInfos()
        if infos.isEmpty {
            let it = NSMenuItem(title: settings.t("menu.noAI"), action: nil, keyEquivalent: "")
            it.isEnabled = false
            menu.addItem(it)
        } else {
            let card = NSHostingView(rootView: MenuInfoCardView(infos: infos))
            card.frame = NSRect(origin: .zero, size: card.fittingSize)
            let item = NSMenuItem()
            item.view = card
            menu.addItem(item)
            // 24시간 미니 차트(#2순위)
            if settings.menuShowChart, let chart = buildChartItem() {
                menu.addItem(.separator())
                menu.addItem(chart)
            }
        }
        menu.addItem(.separator())

        // 분석 대시보드(#분석) — 표시 정보 위.
        menu.addItem(NSMenuItem(title: settings.t("menu.analytics"), action: #selector(openAnalytics), keyEquivalent: ""))

        // 표시 정보 선택(#2)
        let infoItem = NSMenuItem(title: settings.t("menu.showInfo"), action: nil, keyEquivalent: "")
        infoItem.submenu = buildInfoToggleMenu()
        menu.addItem(infoItem)

        // 자동 갱신 주기(#4)
        let intervalItem = NSMenuItem(title: settings.t("menu.interval"), action: nil, keyEquivalent: "")
        intervalItem.submenu = buildIntervalMenu()
        menu.addItem(intervalItem)

        // 언어(#6)
        let langItem = NSMenuItem(title: settings.t("lang.title"), action: nil, keyEquivalent: "")
        langItem.submenu = buildLanguageMenu()
        menu.addItem(langItem)
        menu.addItem(.separator())

        // 2) AI 로그인/로그아웃, 새로고침
        let aiItem = NSMenuItem(title: settings.t("menu.aiLoginout"), action: nil, keyEquivalent: "")
        aiItem.submenu = aiSubmenu
        rebuildAISubmenu()
        menu.addItem(aiItem)
        menu.addItem(NSMenuItem(title: settings.t("menu.refresh"), action: #selector(refreshClicked), keyEquivalent: "r"))
        menu.addItem(.separator())

        // 3) 앱 설정, 프리셋
        menu.addItem(NSMenuItem(title: settings.t("menu.settings"), action: #selector(openSettings), keyEquivalent: ","))
        let presetItem = NSMenuItem(title: settings.t("menu.presets"), action: nil, keyEquivalent: "")
        presetItem.submenu = presetSubmenu
        rebuildPresetSubmenu()
        menu.addItem(presetItem)

        // 테두리 임시 숨기기(체크 시 숨김) + 이 기기에 맞춤(노치/곡률 자동).
        let hideItem = NSMenuItem(title: settings.t("menu.hideBorder"), action: #selector(toggleHideOverlay), keyEquivalent: "")
        hideItem.state = settings.hideOverlay ? .on : .off
        menu.addItem(hideItem)
        let fitItem = NSMenuItem(title: settings.t("menu.deviceFit"), action: #selector(applyDeviceFitClicked), keyEquivalent: "")
        fitItem.image = NSImage(systemSymbolName: "sparkles.rectangle.stack", accessibilityDescription: nil)
        menu.addItem(fitItem)

        // 개발자에게 리포트: 후원 버튼 위 서브메뉴. 첫 줄(안내)+둘째 줄(메일 주소)은 선택 불가 표시.
        menu.addItem(.separator())
        let reportItem = NSMenuItem(title: settings.t("menu.reportSubmenu"), action: nil, keyEquivalent: "")
        reportItem.image = NSImage(systemSymbolName: "envelope", accessibilityDescription: nil)
        let reportSub = NSMenu()
        reportSub.autoenablesItems = false
        // 정보 표시용 헤더: 커스텀 라벨 뷰를 써서 완전 정적으로(선택·하이라이트·클릭 반응 없음)
        // 표시한다. 비활성 항목은 글자가 흐려지고, 활성 항목은 마우스 오버 시 하이라이트되므로
        // 둘 다 피하려면 view 기반이 정답. labelColor라 라이트/다크 모두 또렷하다.
        let reportHeader = NSMenuItem()
        reportHeader.view = Self.makeReportHeaderView(settings)
        reportSub.addItem(reportHeader)
        reportSub.addItem(.separator())
        let mailItem = NSMenuItem(title: settings.t("menu.reportMail"), action: #selector(openReportMail), keyEquivalent: "")
        mailItem.image = NSImage(systemSymbolName: "paperplane", accessibilityDescription: nil)
        mailItem.target = self
        reportSub.addItem(mailItem)
        let copyItem = NSMenuItem(title: settings.t("menu.reportCopy"), action: #selector(copyReportEmail), keyEquivalent: "")
        copyItem.image = NSImage(systemSymbolName: "doc.on.doc", accessibilityDescription: nil)
        copyItem.target = self
        reportSub.addItem(copyItem)
        reportItem.submenu = reportSub
        menu.addItem(reportItem)

        // 후원(☕): 설정 창의 개발자에게 탭으로 이동.
        menu.addItem(.separator())
        let supportItem = NSMenuItem(title: "☕ " + settings.t("menu.support"), action: #selector(openSupport), keyEquivalent: "")
        menu.addItem(supportItem)

        // 버전 · 업데이트 확인.
        menu.addItem(.separator())
        let versionItem = NSMenuItem(title: "UsageMeter " + settings.tf("update.version", updater.displayVersion),
                                     action: nil, keyEquivalent: "")
        versionItem.isEnabled = false
        menu.addItem(versionItem)
        if let stateItem = updateStateItem() { menu.addItem(stateItem) }
        let checkItem = NSMenuItem(title: settings.t("update.check"),
                                   action: #selector(checkUpdateClicked), keyEquivalent: "")
        checkItem.image = NSImage(systemSymbolName: "arrow.down.circle", accessibilityDescription: nil)
        menu.addItem(checkItem)

        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: settings.t("menu.quit"), action: #selector(quit), keyEquivalent: "q"))
        for item in menu.items where item.action != nil { item.target = self }
    }

    /// 업데이트 확인 결과 줄(확인 전이면 표시하지 않는다).
    private func updateStateItem() -> NSMenuItem? {
        switch updater.state {
        case .idle:
            return nil
        case .checking:
            let it = NSMenuItem(title: settings.t("update.checking"), action: nil, keyEquivalent: "")
            it.isEnabled = false
            return it
        case .upToDate:
            let it = NSMenuItem(title: "✓ " + settings.t("update.upToDate"), action: nil, keyEquivalent: "")
            it.isEnabled = false
            return it
        case .available(let version, _):
            let it = NSMenuItem(title: "🔵 " + settings.tf("update.available", "v" + version),
                                action: #selector(openDownloadClicked), keyEquivalent: "")
            return it
        case .failed:
            return NSMenuItem(title: "⚠️ " + settings.t("update.failed"),
                              action: #selector(checkUpdateClicked), keyEquivalent: "")
        }
    }

    @objc private func checkUpdateClicked() { Task { await updater.check(manual: true) } }
    @objc private func openDownloadClicked() { updater.openDownloadPage() }

    /// 표시 순서(providerOrder)대로 정렬된 로그인 AI들.
    private func orderedActive() -> [(spec: ProviderSpec, snap: UsageSnapshot)] {
        manager.active.sorted { settings.orderIndex($0.spec.id) < settings.orderIndex($1.spec.id) }
    }

    /// 드롭다운·호버 공용 AI 정보(같은 내용·같은 순서).
    private func buildAIInfos() -> [MenuAIInfo] {
        orderedActive().map { a in
            MenuAIInfo(id: a.spec.id, name: a.spec.name,
                       color: settings.color(forProvider: a.spec.id),
                       plan: a.snap.plan,       // 구독 플랜(예: Max (5x))
                       lines: infoLines(a))
        }
    }

    /// 드롭다운에 표시할 AI 정보 줄들(표시 옵션에 따라).
    private func infoLines(_ a: (spec: ProviderSpec, snap: UsageSnapshot)) -> [String] {
        let s = a.snap
        var lines: [String] = []
        // 리셋 시각 + 카운트다운을 붙이는 헬퍼.
        func resetSuffix(_ date: Date?) -> String {
            guard let r = date else { return "" }
            var out = ""
            if settings.menuShowReset { out += "   ·   " + settings.t("menu.lineReset") + " " + Self.timeFmt.string(from: r) }
            if settings.menuShowCountdown { out += "   ·   " + countdownString(to: r) }
            return out
        }
        if settings.menuShow5h {
            lines.append(settings.t("menu.line5h") + "  \(Int((s.remainingRatio * 100).rounded()))%" + resetSuffix(s.resetAt))
        }
        if settings.menuShowWeekly, let w = s.secondaryRatio {
            lines.append(settings.t("menu.lineWeekly") + "  \(Int((w * 100).rounded()))%" + resetSuffix(s.secondaryResetAt))
        }
        // 모델별 주간 한도(예: Fable) — 서버가 주는 각 버킷을 표시.
        if settings.menuShowOpus, let buckets = s.modelBuckets {
            for b in buckets {
                lines.append(b.label + " " + settings.t("menu.lineWeekly") + "  \(Int((b.remainingRatio * 100).rounded()))%" + resetSuffix(b.resetAt))
            }
        }
        if settings.menuShowPace, let p = history.pace(for: a.spec.id) {
            lines.append(paceString(p, resetAt: s.resetAt))
        }
        if settings.menuShowUpdated {
            lines.append(settings.t("menu.lineUpdated") + "  " + Self.timeFmt.string(from: s.lastUpdated))
        }
        if lines.isEmpty { lines.append(settings.t("menu.pickInfo")) }
        return lines
    }

    /// "N시간 M분 후" 카운트다운 문자열.
    private func countdownString(to date: Date) -> String {
        let secs = Int(date.timeIntervalSinceNow)
        if secs <= 0 { return settings.t("cd.now") }
        let h = secs / 3600, m = (secs % 3600) / 60
        let hm = h > 0 ? "\(h)\(settings.t("cd.hour")) \(m)\(settings.t("cd.min"))" : "\(m)\(settings.t("cd.min"))"
        return settings.tf("cd.afterFmt", hm)
    }

    /// 소진 예측 문자열.
    private func paceString(_ p: PaceInfo, resetAt: Date?) -> String {
        let inStr = settings.tf("pace.depleteFmt", countdownBare(to: p.depletion))
        var out = settings.t("info.pace") + ": " + inStr
        if let r = resetAt {
            out += " " + (p.depletion < r ? settings.t("pace.warnReset") : settings.t("pace.okReset"))
        }
        return out
    }
    /// 카운트다운에서 "후/in" 없이 시간 부분만.
    private func countdownBare(to date: Date) -> String {
        let secs = max(0, Int(date.timeIntervalSinceNow))
        let h = secs / 3600, m = (secs % 3600) / 60
        return h > 0 ? "\(h)\(settings.t("cd.hour")) \(m)\(settings.t("cd.min"))" : "\(m)\(settings.t("cd.min"))"
    }

    /// 24시간 미니 차트를 담은 메뉴 항목(데이터 있으면).
    private func buildChartItem() -> NSMenuItem? {
        var lines: [MiniChartView.Line] = []
        let hours = Double(settings.chartHours)
        for a in manager.active {
            let pts = history.series(for: a.spec.id, hours: hours)
            if pts.count >= 2 {
                lines.append(.init(id: a.spec.id, name: a.spec.name,
                                   color: settings.color(forProvider: a.spec.id), points: pts))
            }
        }
        guard !lines.isEmpty else { return nil }
        let host = NSHostingView(rootView: MiniChartView(
            title: settings.tn("chart.titleFmt", settings.chartHours), lines: lines, hours: hours,
            hoverEnabled: settings.chartHoverEnabled))
        // 메뉴 폭에 맞춰 늘어나도록(#차트 꽉 채움): 기본 폭 + 가로 자동 리사이즈.
        host.frame = NSRect(x: 0, y: 0, width: 340, height: 118)
        host.autoresizingMask = [.width]
        let item = NSMenuItem(); item.view = host
        return item
    }

    /// 클릭해도 메뉴가 닫히지 않는 체크박스 항목(커스텀 뷰는 자동 dismiss 안 됨 #팝업 유지).
    private static func stickyCheckItem(_ title: String, on: Bool, target: AnyObject, action: Selector) -> NSMenuItem {
        let btn = NSButton(checkboxWithTitle: title, target: target, action: action)
        btn.state = on ? .on : .off
        btn.font = NSFont.menuFont(ofSize: 0)
        let fit = btn.fittingSize
        let leading: CGFloat = 14
        let container = NSView(frame: NSRect(x: 0, y: 0, width: leading + fit.width + 16, height: fit.height + 6))
        btn.frame = NSRect(x: leading, y: 3, width: fit.width, height: fit.height)
        container.addSubview(btn)
        let item = NSMenuItem()
        item.view = container
        return item
    }

    /// '표시 정보' 서브메뉴: 어떤 정보를 드롭다운에 보일지 체크박스로(클릭해도 안 닫힘).
    private func buildInfoToggleMenu() -> NSMenu {
        let m = NSMenu(); m.autoenablesItems = false
        func add(_ title: String, _ on: Bool, _ sel: Selector) {
            m.addItem(Self.stickyCheckItem(title, on: on, target: self, action: sel))
        }
        add(settings.t("info.5h"), settings.menuShow5h, #selector(toggleShow5h))
        add(settings.t("info.weekly"), settings.menuShowWeekly, #selector(toggleShowWeekly))
        add(settings.t("info.opus"), settings.menuShowOpus, #selector(toggleShowOpus))
        add(settings.t("info.reset"), settings.menuShowReset, #selector(toggleShowReset))
        add(settings.t("info.countdown"), settings.menuShowCountdown, #selector(toggleShowCountdown))
        add(settings.t("info.pace"), settings.menuShowPace, #selector(toggleShowPace))
        add(settings.t("info.chart"), settings.menuShowChart, #selector(toggleShowChart))
        add(settings.t("info.updated"), settings.menuShowUpdated, #selector(toggleShowUpdated))
        return m
    }

    @objc private func toggleShow5h() { settings.menuShow5h.toggle() }
    @objc private func toggleShowWeekly() { settings.menuShowWeekly.toggle() }
    @objc private func toggleShowOpus() { settings.menuShowOpus.toggle() }
    @objc private func toggleShowReset() { settings.menuShowReset.toggle() }
    @objc private func toggleShowCountdown() { settings.menuShowCountdown.toggle() }
    @objc private func toggleShowPace() { settings.menuShowPace.toggle() }
    @objc private func toggleShowChart() { settings.menuShowChart.toggle() }
    @objc private func toggleShowUpdated() { settings.menuShowUpdated.toggle() }

    /// '자동 갱신 주기' 서브메뉴: 프리셋 분 + 사용자 설정(#4).
    private func buildIntervalMenu() -> NSMenu {
        let m = NSMenu(); m.autoenablesItems = false
        for mins in OverlaySettings.refreshPresets {
            let it = NSMenuItem(title: settings.tn("interval.minutesFmt", mins), action: #selector(pickInterval(_:)), keyEquivalent: "")
            it.tag = mins; it.target = self
            it.state = (!settings.refreshUseCustom && settings.refreshPresetMinutes == mins) ? .on : .off
            m.addItem(it)
        }
        m.addItem(.separator())
        let custom = NSMenuItem(title: settings.t("menu.intervalCustom"), action: #selector(pickIntervalCustom), keyEquivalent: "")
        custom.target = self
        custom.state = settings.refreshUseCustom ? .on : .off
        m.addItem(custom)
        return m
    }

    @objc private func pickInterval(_ sender: NSMenuItem) {
        settings.refreshUseCustom = false
        settings.refreshPresetMinutes = sender.tag
    }

    /// 사용자 설정 클릭(#5): 설정 창의 '주기' 탭을 열고 사용자 설정 자동 체크.
    @objc private func pickIntervalCustom() {
        settings.refreshUseCustom = true
        settings.requestedTab = "interval"
        openSettings()
    }

    /// '언어' 서브메뉴(#6).
    private func buildLanguageMenu() -> NSMenu {
        let m = NSMenu(); m.autoenablesItems = false
        for lang in AppLanguage.allCases {
            let it = NSMenuItem(title: lang.displayName, action: #selector(pickLanguage(_:)), keyEquivalent: "")
            it.representedObject = lang.rawValue; it.target = self
            it.state = (settings.language == lang) ? .on : .off
            m.addItem(it)
        }
        return m
    }

    @objc private func pickLanguage(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String, let lang = AppLanguage(rawValue: raw) else { return }
        settings.language = lang
    }

    /// 모서리 곡률 서브메뉴: 전체 동일 프리셋 + 모서리별 세부 조정.
    private func buildCornerMenu() -> NSMenu {
        let sub = NSMenu()
        for (i, preset) in OverlaySettings.presetCornerRadii.enumerated() {
            let item = NSMenuItem(title: "전체 " + preset.0, action: #selector(pickAllCorners(_:)), keyEquivalent: "")
            item.tag = i
            item.target = self
            sub.addItem(item)
        }
        sub.addItem(.separator())
        for corner in BorderCorner.allCases {
            let cm = NSMenuItem(title: corner.label + " 모서리", action: nil, keyEquivalent: "")
            cm.submenu = buildOneCornerMenu(corner)
            sub.addItem(cm)
        }
        return sub
    }

    /// 한 모서리의 곡률 서브메뉴(프리셋 + 직접 입력).
    private func buildOneCornerMenu(_ corner: BorderCorner) -> NSMenu {
        let sub = NSMenu()
        for (i, preset) in OverlaySettings.presetCornerRadii.enumerated() {
            let item = NSMenuItem(title: preset.0, action: #selector(pickCornerRadius(_:)), keyEquivalent: "")
            item.tag = i
            item.representedObject = corner.rawValue
            item.target = self
            sub.addItem(item)
        }
        let custom = NSMenuItem(title: "직접 입력…", action: #selector(pickCornerCustom(_:)), keyEquivalent: "")
        custom.representedObject = corner.rawValue
        custom.target = self
        sub.addItem(custom)
        return sub
    }

    /// 노치 감싸기 서브메뉴: 켜기/끄기 + 너비 프리셋/직접입력 + 높이 직접입력.
    private func buildNotchMenu() -> NSMenu {
        let sub = NSMenu()
        notchToggleItem.action = #selector(toggleNotch)
        notchToggleItem.target = self
        sub.addItem(notchToggleItem)
        sub.addItem(.separator())
        for (i, preset) in OverlaySettings.presetNotchWidths.enumerated() {
            let item = NSMenuItem(title: "너비 " + preset.0, action: #selector(pickNotchWidth(_:)), keyEquivalent: "")
            item.tag = i
            item.target = self
            sub.addItem(item)
        }
        let customW = NSMenuItem(title: "너비 직접 입력…", action: #selector(pickNotchWidthCustom), keyEquivalent: "")
        customW.target = self
        sub.addItem(customW)
        let customH = NSMenuItem(title: "높이(깊이) 직접 입력…", action: #selector(pickNotchHeightCustom), keyEquivalent: "")
        customH.target = self
        sub.addItem(customH)

        sub.addItem(.separator())
        let cornerItem = NSMenuItem(title: "노치 모서리 곡률", action: nil, keyEquivalent: "")
        cornerItem.submenu = buildNotchCornerMenu()
        sub.addItem(cornerItem)
        return sub
    }

    /// 노치 모서리 곡률 서브메뉴: 전체 동일 + 모서리별(바깥좌/안쪽좌/안쪽우/바깥우).
    private func buildNotchCornerMenu() -> NSMenu {
        let sub = NSMenu()
        for (i, preset) in OverlaySettings.presetCornerRadii.enumerated() {
            let item = NSMenuItem(title: "전체 " + preset.0, action: #selector(pickAllNotchCorners(_:)), keyEquivalent: "")
            item.tag = i
            item.target = self
            sub.addItem(item)
        }
        sub.addItem(.separator())
        for corner in NotchCorner.allCases {
            let cm = NSMenuItem(title: corner.label, action: nil, keyEquivalent: "")
            cm.submenu = buildOneNotchCornerMenu(corner)
            sub.addItem(cm)
        }
        return sub
    }

    private func buildOneNotchCornerMenu(_ corner: NotchCorner) -> NSMenu {
        let sub = NSMenu()
        for (i, preset) in OverlaySettings.presetCornerRadii.enumerated() {
            let item = NSMenuItem(title: preset.0, action: #selector(pickNotchCornerRadius(_:)), keyEquivalent: "")
            item.tag = i
            item.representedObject = corner.rawValue
            item.target = self
            sub.addItem(item)
        }
        let custom = NSMenuItem(title: "직접 입력…", action: #selector(pickNotchCornerCustom(_:)), keyEquivalent: "")
        custom.representedObject = corner.rawValue
        custom.target = self
        sub.addItem(custom)
        return sub
    }

    private func buildColorMenu() -> NSMenu {
        let sub = NSMenu()
        for (i, preset) in OverlaySettings.presetColors.enumerated() {
            let item = NSMenuItem(title: preset.0, action: #selector(pickPresetColor(_:)), keyEquivalent: "")
            item.tag = i
            item.target = self
            item.image = Self.swatch(NSColor(preset.1))
            sub.addItem(item)
        }
        sub.addItem(.separator())
        let custom = NSMenuItem(title: "직접 선택…", action: #selector(pickCustomColor), keyEquivalent: "")
        custom.target = self
        sub.addItem(custom)
        return sub
    }

    private func buildThicknessMenu() -> NSMenu {
        let sub = NSMenu()
        for (i, preset) in OverlaySettings.presetThicknesses.enumerated() {
            let item = NSMenuItem(title: preset.0, action: #selector(pickThickness(_:)), keyEquivalent: "")
            item.tag = i
            item.target = self
            sub.addItem(item)
        }
        return sub
    }

    /// 메뉴에 보여줄 작은 색 견본 이미지.
    private static func swatch(_ color: NSColor) -> NSImage {
        let size = NSSize(width: 14, height: 14)
        let img = NSImage(size: size)
        img.lockFocus()
        color.setFill()
        NSBezierPath(roundedRect: NSRect(origin: .zero, size: size), xRadius: 3, yRadius: 3).fill()
        img.unlockFocus()
        return img
    }

    // MARK: - 설정 액션

    @objc private func pickPresetColor(_ sender: NSMenuItem) {
        settings.color = OverlaySettings.presetColors[sender.tag].1
    }

    @objc private func pickThickness(_ sender: NSMenuItem) {
        settings.thickness = OverlaySettings.presetThicknesses[sender.tag].1
    }

    @objc private func pickCustomColor() {
        let panel = NSColorPanel.shared
        panel.setTarget(self)
        panel.setAction(#selector(colorPanelChanged(_:)))
        panel.color = NSColor(settings.color)
        NSApp.activate(ignoringOtherApps: true)
        panel.makeKeyAndOrderFront(nil)
    }

    @objc private func colorPanelChanged(_ sender: NSColorPanel) {
        settings.color = Color(nsColor: sender.color)
    }

    @objc private func pickAllCorners(_ sender: NSMenuItem) {
        let v = OverlaySettings.presetCornerRadii[sender.tag].1
        for c in BorderCorner.allCases { settings.cornerRadii[c] = v }
    }

    @objc private func pickCornerRadius(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String,
              let corner = BorderCorner(rawValue: raw) else { return }
        settings.cornerRadii[corner] = OverlaySettings.presetCornerRadii[sender.tag].1
    }

    @objc private func pickCornerCustom(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String,
              let corner = BorderCorner(rawValue: raw) else { return }
        if let v = promptNumber("\(corner.label) 모서리 곡률(pt)", current: settings.radius(corner)) {
            settings.cornerRadii[corner] = max(0, v)
        }
    }

    @objc private func toggleNotch() {
        settings.notchEnabled.toggle()
    }

    @objc private func pickNotchWidth(_ sender: NSMenuItem) {
        settings.notchWidth = OverlaySettings.presetNotchWidths[sender.tag].1
        settings.notchEnabled = true
    }

    @objc private func pickNotchWidthCustom() {
        if let v = promptNumber("노치 너비(pt)", current: settings.notchWidth) {
            settings.notchWidth = max(0, v)
            settings.notchEnabled = true
        }
    }

    @objc private func pickNotchHeightCustom() {
        if let v = promptNumber("노치 높이·감싸는 깊이(pt)", current: settings.notchHeight) {
            settings.notchHeight = max(0, v)
        }
    }

    @objc private func pickAllNotchCorners(_ sender: NSMenuItem) {
        let v = OverlaySettings.presetCornerRadii[sender.tag].1
        for c in NotchCorner.allCases { settings.notchRadii[c] = v }
    }

    @objc private func pickNotchCornerRadius(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String,
              let corner = NotchCorner(rawValue: raw) else { return }
        settings.notchRadii[corner] = OverlaySettings.presetCornerRadii[sender.tag].1
    }

    @objc private func pickNotchCornerCustom(_ sender: NSMenuItem) {
        guard let raw = sender.representedObject as? String,
              let corner = NotchCorner(rawValue: raw) else { return }
        if let v = promptNumber("노치 \(corner.label) 곡률(pt)", current: settings.notchRadius(corner)) {
            settings.notchRadii[corner] = max(0, v)
        }
    }

    @objc private func toggleTrack() {
        settings.showTrack.toggle()
    }

    /// 테두리(오버레이) 임시 숨김 토글. 저장 안 됨(재시작 시 다시 표시).
    @objc private func toggleHideOverlay() { settings.hideOverlay.toggle() }

    /// "이 기기에 맞춤" — 현재 디바이스/해상도에 맞는 노치·곡률로 재설정.
    @objc private func applyDeviceFitClicked() { settings.applyDeviceFit() }

    /// 숫자 하나를 입력받는 간단한 프롬프트(모서리 곡률·노치 크기 정밀 조정용).
    private func promptNumber(_ title: String, current: CGFloat) -> CGFloat? {
        let alert = NSAlert()
        alert.messageText = title
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 120, height: 24))
        field.stringValue = String(format: "%g", current)
        alert.accessoryView = field
        alert.addButton(withTitle: "확인")
        alert.addButton(withTitle: "취소")
        NSApp.activate(ignoringOtherApps: true)
        guard alert.runModal() == .alertFirstButtonReturn else { return nil }
        return Double(field.stringValue.trimmingCharacters(in: .whitespaces)).map { CGFloat($0) }
    }

    @objc private func refreshClicked() { refresh() }

    @objc private func openSettings() { settingsWC.show() }

    @objc private func openAnalytics() { analyticsWC.show() }

    // MARK: - 개발자에게 리포트

    /// 팝업 리포트 헤더: 첫 줄 안내 + 둘째 줄 메일 주소(두 줄, 선택 불가).
    /// 다른 메뉴 항목과 동일한 양식(메뉴 기본 폰트 + 표준 labelColor)으로 맞추고
    /// 글자 크기만 2단(12/13pt)으로 유지한다. labelColor는 라이트/다크 자동 대응.
    private static func reportHeaderTitle(_ settings: OverlaySettings) -> NSAttributedString {
        let para = NSMutableParagraphStyle()
        para.lineSpacing = 2
        para.paragraphSpacing = 4
        let s = NSMutableAttributedString()
        s.append(NSAttributedString(string: settings.t("menu.reportTitle") + "\n", attributes: [
            .font: NSFont.menuFont(ofSize: 12),
            .foregroundColor: NSColor.labelColor,
            .paragraphStyle: para]))
        s.append(NSAttributedString(string: SupportLinks.reportEmail, attributes: [
            .font: NSFont.menuFont(ofSize: 13),
            .foregroundColor: NSColor.labelColor,
            .paragraphStyle: para]))
        return s
    }

    /// 리포트 헤더를 담는 커스텀 라벨 뷰. NSTextField(label)은 선택·편집·하이라이트가 없어
    /// "마우스를 올리거나 클릭해도 반응 없는 단순 텍스트"로 표시된다.
    private static func makeReportHeaderView(_ settings: OverlaySettings) -> NSView {
        let label = NSTextField(labelWithAttributedString: reportHeaderTitle(settings))
        label.translatesAutoresizingMaskIntoConstraints = false
        label.isSelectable = false
        label.isEditable = false
        label.isBezeled = false
        label.drawsBackground = false
        let container = NSView()
        container.addSubview(label)
        NSLayoutConstraint.activate([
            label.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: 21),
            label.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -14),
            label.topAnchor.constraint(equalTo: container.topAnchor, constant: 5),
            label.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -5),
        ])
        container.frame = NSRect(x: 0, y: 0, width: 240, height: container.fittingSize.height)
        return container
    }

    /// "메일 앱으로" → 기본 메일 앱 작성 창(mailto, 제목 자동 채움).
    @objc private func openReportMail() {
        let subject = settings.t("report.mailSubject")
        let enc = subject.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? subject
        if let url = URL(string: "mailto:\(SupportLinks.reportEmail)?subject=\(enc)") {
            NSWorkspace.shared.open(url)
        }
    }

    /// "메일주소 복사" → 클립보드에 메일 주소 복사.
    @objc private func copyReportEmail() {
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(SupportLinks.reportEmail, forType: .string)
    }

    // MARK: - 후원

    /// 팝업 "후원" 클릭 → 설정 창의 개발자에게 탭을 연다.
    @objc private func openSupport() {
        settings.requestedTab = "support"
        openSettings()
    }

    // (AI 표시 순서는 설정 창에서 조절 — 팝업 서브메뉴는 클릭 시 전체가 닫혀 제거함.)

    /// 프리셋 서브메뉴 채우기: 저장된 게 있으면 불러오기/삭제, 항상 현재 설정 저장(5개 미만일 때).
    private func rebuildPresetSubmenu() {
        presetSubmenu.removeAllItems()
        if settings.savedPresets.isEmpty {
            let empty = NSMenuItem(title: settings.t("menu.noPreset"), action: nil, keyEquivalent: "")
            empty.isEnabled = false
            presetSubmenu.addItem(empty)
        } else {
            for preset in settings.savedPresets {
                let it = NSMenuItem(title: settings.t("menu.loadPrefix") + preset.name, action: #selector(loadPresetItem(_:)), keyEquivalent: "")
                it.representedObject = preset.id.uuidString
                it.target = self
                presetSubmenu.addItem(it)
            }
            presetSubmenu.addItem(.separator())
            for preset in settings.savedPresets {
                let it = NSMenuItem(title: settings.t("menu.deletePrefix") + preset.name, action: #selector(deletePresetItem(_:)), keyEquivalent: "")
                it.representedObject = preset.id.uuidString
                it.target = self
                presetSubmenu.addItem(it)
            }
        }
        presetSubmenu.addItem(.separator())
        let save = NSMenuItem(title: settings.t("menu.saveCurrent"), action: #selector(savePresetPrompt), keyEquivalent: "")
        save.isEnabled = settings.savedPresets.count < OverlaySettings.maxPresets
        save.target = self
        presetSubmenu.addItem(save)
    }

    @objc private func savePresetPrompt() {
        guard settings.savedPresets.count < OverlaySettings.maxPresets else { return }
        let alert = NSAlert()
        alert.messageText = settings.t("menu.savePresetTitle")
        alert.informativeText = settings.tn("menu.savePresetInfo", OverlaySettings.maxPresets)
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 220, height: 24))
        field.stringValue = settings.t("menu.presetDefaultName") + " \(settings.savedPresets.count + 1)"
        alert.accessoryView = field
        alert.addButton(withTitle: settings.t("preset.save"))
        alert.addButton(withTitle: settings.t("menu.cancel"))
        NSApp.activate(ignoringOtherApps: true)
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        settings.saveCurrent(name: field.stringValue)
        rebuildPresetSubmenu()
    }

    @objc private func loadPresetItem(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String,
              let preset = settings.savedPresets.first(where: { $0.id.uuidString == id }) else { return }
        settings.loadPreset(preset)
    }

    @objc private func deletePresetItem(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String,
              let preset = settings.savedPresets.first(where: { $0.id.uuidString == id }) else { return }
        settings.deletePreset(preset)
        rebuildPresetSubmenu()
    }

    // MARK: - AI 로그인/로그아웃

    /// AI 서브메뉴: provider마다 상태 + 로그인/로그아웃.
    private func rebuildAISubmenu() {
        aiSubmenu.removeAllItems()
        for id in manager.order {
            guard let st = manager.states[id] else { continue }
            let mark = st.loggedIn ? "● " : "○ "
            let header = NSMenuItem(title: mark + st.spec.name,
                                    action: nil, keyEquivalent: "")
            header.isEnabled = false
            aiSubmenu.addItem(header)
            if st.loggedIn {
                let out = NSMenuItem(title: settings.t("menu.logout"), action: #selector(logoutProvider(_:)), keyEquivalent: "")
                out.representedObject = id; out.target = self
                aiSubmenu.addItem(out)
            } else {
                let inn = NSMenuItem(title: settings.t("menu.login"), action: #selector(loginProvider(_:)), keyEquivalent: "")
                inn.representedObject = id; inn.target = self
                aiSubmenu.addItem(inn)
            }
        }
    }

    @objc private func loginProvider(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String else { return }
        manager.login(id)
    }

    @objc private func logoutProvider(_ sender: NSMenuItem) {
        guard let id = sender.representedObject as? String else { return }
        manager.logout(id)
    }

    @objc private func refresh() { manager.refreshAll() }

    /// 메뉴바 링 아이콘 갱신: 가장 급한(잔여 최소) AI의 색·잔여율로 표시.
    private func updateStatusIcon() {
        guard let button = statusItem.button else { return }
        let active = manager.active
        // 링 아이콘 기준 AI(#2): 선택 AI가 로그인돼 있으면 그 기준, 아니면 가장 급한 AI로 폴백.
        let iconItem = active.first(where: { $0.spec.id == settings.menuBarIconID })
            ?? active.min(by: { $0.snap.remainingRatio < $1.snap.remainingRatio })
        if let iconItem {
            let color = NSColor(settings.color(forProvider: iconItem.spec.id))
            button.image = Self.ringImage(ratio: iconItem.snap.remainingRatio, color: color, dim: false)
        } else {
            button.image = Self.ringImage(ratio: 1, color: .secondaryLabelColor, dim: true)
        }
        // 메뉴바 % 표시(#2): 선택 AI들의 잔여 %를 각자 색으로 나란히.
        // 순서는 'AI 표시 순서'(providerOrder)를 그대로 따른다(#순서 통일).
        var shown = active.filter { settings.menuBarPercentIDs.contains($0.spec.id) }
        shown.sort { settings.orderIndex($0.spec.id) < settings.orderIndex($1.spec.id) }
        if shown.isEmpty {
            button.imagePosition = .imageOnly
            button.attributedTitle = NSAttributedString(string: "")
            button.title = ""
            return
        }
        button.imagePosition = .imageLeading
        let str = NSMutableAttributedString(string: " ")
        for (i, a) in shown.enumerated() {
            if i > 0 { str.append(NSAttributedString(string: " ")) }
            str.append(NSAttributedString(string: "\(Int((a.snap.remainingRatio * 100).rounded()))%", attributes: [
                .foregroundColor: NSColor(settings.color(forProvider: a.spec.id)),
                .font: NSFont.systemFont(ofSize: 11, weight: .semibold)]))
        }
        button.attributedTitle = str
    }

    /// 호버 정보: 팝업 카드와 동일한 내용·순서(#호버=팝업).
    private func updateHoverInfo() {
        let infos = buildAIInfos()
        hoverInfo.infoText = infos.isEmpty ? settings.t("hover.noAI")
            : infos.map { ([$0.name] + $0.lines).joined(separator: "\n") }.joined(separator: "\n\n")
    }

    /// 잔여율만큼 채워지는 원형 링 아이콘.
    private static func ringImage(ratio: Double, color: NSColor, dim: Bool) -> NSImage {
        let size = NSSize(width: 18, height: 18)
        let img = NSImage(size: size)
        img.lockFocus()
        let inset: CGFloat = 2.5, lineW: CGFloat = 2.4
        let rect = NSRect(x: inset, y: inset, width: size.width - inset * 2, height: size.height - inset * 2)
        let center = NSPoint(x: rect.midX, y: rect.midY)
        let radius = rect.width / 2
        // 트랙(전체 옅은 링).
        let track = NSBezierPath()
        track.appendArc(withCenter: center, radius: radius, startAngle: 0, endAngle: 360)
        track.lineWidth = lineW
        color.withAlphaComponent(dim ? 0.25 : 0.28).setStroke()
        track.stroke()
        // 잔여율 호(위 12시에서 시계방향).
        if !dim, ratio > 0.001 {
            let start: CGFloat = 90
            let end = start - CGFloat(min(1, ratio)) * 360
            let arc = NSBezierPath()
            arc.appendArc(withCenter: center, radius: radius, startAngle: start, endAngle: end, clockwise: true)
            arc.lineWidth = lineW
            arc.lineCapStyle = .round
            color.setStroke()
            arc.stroke()
        }
        img.unlockFocus()
        img.isTemplate = false
        return img
    }

    /// 스냅샷을 사람이 읽는 한 줄로(설정 언어에 맞춤).
    private func describe(_ s: UsageSnapshot) -> String {
        switch s.status {
        case .authExpired: return settings.t("hover.authExpired")
        case .unavailable(let why): return settings.tf("hover.unavailable", Self.friendlyReason(why, settings))
        case .stale, .ok:
            let five = Int((s.remainingRatio * 100).rounded())
            var out = settings.t("menu.line5h") + " \(five)%"
            if let w = s.secondaryRatio { out += " · " + settings.t("menu.lineWeekly") + " \(Int((w * 100).rounded()))%" }
            if let r = s.resetAt { out += " · " + settings.t("menu.lineReset") + " " + Self.timeFmt.string(from: r) }
            return out
        }
    }

    /// unavailable 사유 코드를 사용자 친화적 문구로. 모르는 코드는 원본을 그대로 보여준다.
    private static func friendlyReason(_ code: String, _ settings: OverlaySettings) -> String {
        switch code {
        case "parse_failed", "no_data": return settings.t("reason.parseFailed")
        case "usage_failed":            return settings.t("reason.serverError")
        case "exception", "eval_error", "bad_result", "unknown":
            return settings.t("reason.temporary")
        default:                        return code
        }
    }

    private static let timeFmt: DateFormatter = {
        let f = DateFormatter()
        f.dateFormat = "M/d HH:mm"      // 로컬 시간대로 표시
        return f
    }()

    @objc private func quit() { NSApp.terminate(nil) }
}

extension StatusBarController: NSMenuDelegate {
    /// 메뉴가 열릴 때마다 정보 섹션·상태를 현재 값으로 다시 채운다.
    func menuNeedsUpdate(_ menu: NSMenu) {
        guard menu == mainMenu else { return }
        populateMenu(menu)
        updateStatusIcon()
    }
}
