import SwiftUI
import Charts

/// 분석 대시보드 기간.
enum AnalyticsRange: String, CaseIterable, Identifiable {
    case day1, day3, week, month
    var id: String { rawValue }
    /// 조회 범위(시간).
    var hours: Double {
        switch self { case .day1: return 24; case .day3: return 72; case .week: return 168; case .month: return 720 }
    }
    /// 시계열 버킷 크기(초).
    var bucketSeconds: Double {
        switch self { case .day1: return 3600; case .day3: return 3 * 3600; case .week: return 24 * 3600; case .month: return 24 * 3600 }
    }
    var labelKey: String {
        switch self { case .day1: return "range.day1"; case .day3: return "range.day3"; case .week: return "range.week"; case .month: return "range.month" }
    }
}

/// AI 사용량 분석 대시보드.
///
/// - AI 필터: "전체"면 AI끼리 비교(대표 잔여율 기준), AI 하나를 고르면 그 AI의 세부 항목
///   (5시간·주간·모델별 한도)별로 모든 차트가 다시 그려진다. 세부 항목 칩으로 일부만 볼 수도 있다.
/// - 차트: 소비 비중/항목별 소비 · 시간대별 패턴 · 잔여량 추이 · 기간별 소비 추이 · 요일×시간대 · 세부 항목 상세 표.
/// 데이터 = HistoryStore의 잔여율 하락(=소비)을 집계. AI 대표는 5시간 한도 기준, 세부 항목은 자기 한도 기준(%p).
struct AnalyticsView: View {
    @ObservedObject var settings: OverlaySettings
    @ObservedObject var manager: ProviderManager
    @ObservedObject var history: HistoryStore
    @State private var range: AnalyticsRange = .day1
    /// 자세히 볼 AI(nil = 전체 AI 비교).
    @State private var scope: String? = nil
    /// 고른 AI에서 보이게 할 세부 항목(비어 있으면 전체).
    @State private var selectedItems: Set<String> = []
    /// 데이터 없는 AI 칩을 눌렀을 때 잠깐 뜨는 안내.
    @State private var noDataFlash: String? = nil

    /// 상단 2단 차트 박스의 공통 높이(박스 높이 통일).
    static let rowHeight: CGFloat = 250

    private var since: Date { Date().addingTimeInterval(-range.hours * 3600) }

    // MARK: - 데이터 모델(본문을 그릴 때 한 번만 계산)

    private struct Model {
        /// 데이터가 있는 AI 대표 계열 전부(필터 칩 원천).
        let aiAll: [AnalyticsSeries]
        /// KPI·히트맵에 쓰는 AI 대표 계열(전체 = 전부, AI 선택 = 그 AI).
        let ai: [AnalyticsSeries]
        /// AI별 세부 항목 계열(상세 표). AI 선택 시 그 AI의 고른 항목만.
        let groups: [(spec: ProviderSpec, items: [AnalyticsSeries])]
        /// 고른 AI의 세부 항목 전부(항목 칩 원천).
        let scopedItems: [AnalyticsSeries]
        /// 차트에 그리는 계열(전체 = AI 대표들, AI 선택 = 고른 세부 항목들).
        let shown: [AnalyticsSeries]
        /// 세부 항목 단위로 보고 있는지(단위가 %p, 이름 = 항목 이름).
        let itemMode: Bool

        var styleDomain: [String] { shown.map(\.name) }
        var styleRange: [Color] { shown.map(\.color) }
        var isEmpty: Bool { aiAll.isEmpty && groups.allSatisfy { $0.items.isEmpty } }
    }

    private func buildModel() -> Model {
        let since = self.since
        let aiAll: [AnalyticsSeries] = settings.providerOrder.compactMap { id in
            guard let spec = ProviderSpec.spec(id) else { return nil }
            let pts = history.series(for: id, hours: range.hours).map { (t: $0.0, r: $0.1) }
            guard pts.count >= 2 else { return nil }
            return AnalyticsSeries(id: id, providerID: id, itemID: nil, name: spec.name,
                                   color: settings.color(forProvider: id), points: pts,
                                   events: history.consumptionEvents(for: id, since: since),
                                   resetAt: manager.states[id]?.snapshot?.resetAt)
        }
        let specs = settings.providerOrder.compactMap { ProviderSpec.spec($0) }
        let allGroups = specs.map { ($0, itemSeries(for: $0.id, since: since)) }

        if let scope, let spec = ProviderSpec.spec(scope) {
            let items = allGroups.first { $0.0.id == scope }?.1 ?? []
            let picked = selectedItems.isEmpty ? items : items.filter { selectedItems.contains($0.itemID ?? "") }
            return Model(aiAll: aiAll, ai: aiAll.filter { $0.id == scope },
                         groups: [(spec, picked)], scopedItems: items, shown: picked, itemMode: true)
        }
        return Model(aiAll: aiAll, ai: aiAll, groups: allGroups.filter { !$0.1.isEmpty },
                     scopedItems: [], shown: aiAll, itemMode: false)
    }

    /// 한 AI의 세부 항목 계열. 순서 = 지금 스냅샷 순서 → 기억해 둔 항목 → 이력에만 있는 항목
    /// (설정 탭·테두리와 같은 기본 색이 나오도록 같은 순서로 색을 매긴다).
    private func itemSeries(for id: String, since: Date) -> [AnalyticsSeries] {
        let live = manager.subItems(for: id).items
        var order = live.map(\.id)
        for k in history.itemIDs(for: id, since: since) where !order.contains(k) { order.append(k) }
        let objs: [UsageSubItem] = order.compactMap { k in
            live.first { $0.id == k } ?? UsageSubItem(id: k, remainingRatio: 0)
        }
        return objs.compactMap { item in
            let pts = history.itemSeries(for: id, item: item.id, since: since)
            guard !pts.isEmpty else { return nil }
            return AnalyticsSeries(id: "\(id)/\(item.id)", providerID: id, itemID: item.id,
                                   name: itemName(item), color: settings.subItemColor(id, item, in: objs),
                                   points: pts, events: AnalyticsMath.drops(pts), resetAt: item.resetAt)
        }
    }

    /// 분석용 짧은 항목 이름: "5시간" · "주간" · "Fable 주간".
    private func itemName(_ item: UsageSubItem) -> String {
        let kind = settings.t(item.isFiveHour ? "subItems.kind5h" : "subItems.kindWeekly")
        return item.modelLabel.map { $0 + " " + kind } ?? kind
    }

    var body: some View {
        let m = buildModel()
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                header
                aiFilter(m)
                if scope != nil, !m.scopedItems.isEmpty { itemFilter(m) }
                if m.isEmpty {
                    emptyState
                } else {
                    kpiRow(m)
                    HStack(alignment: .top, spacing: 14) {
                        if m.itemMode {
                            card(title: settings.t("analytics.itemUse"), desc: settings.t("analytics.itemUseDesc"),
                                 icon: "chart.bar.xaxis") { itemUseChart(m) }
                                .frame(width: 330, height: Self.rowHeight)
                        } else {
                            card(title: settings.t("analytics.share"), desc: settings.t("analytics.shareDesc"),
                                 icon: "chart.pie") { shareChart(m) }
                                .frame(width: 330, height: Self.rowHeight)
                        }
                        card(title: settings.t("analytics.byHour"),
                             desc: settings.t(m.itemMode ? "analytics.byHourItemDesc" : "analytics.byHourDesc"),
                             icon: "clock") { hourChart(m) }
                            .frame(maxWidth: .infinity)
                            .frame(height: Self.rowHeight)
                    }
                    card(title: settings.t("analytics.remaining"), desc: settings.t("analytics.remainingDesc"),
                         icon: "chart.line.downtrend.xyaxis") { remainingChart(m) }
                        .frame(height: 260)
                    HStack(alignment: .top, spacing: 14) {
                        card(title: settings.t("analytics.overTime"), desc: settings.t("analytics.overTimeDesc"),
                             icon: "chart.bar") { overTimeChart(m) }
                            .frame(maxWidth: .infinity)
                        card(title: settings.t("analytics.heatmap"), desc: settings.t("analytics.heatmapDesc"),
                             icon: "square.grid.3x3.fill") { heatmap(m) }
                            .frame(maxWidth: .infinity)
                    }
                    .frame(height: 250)
                    detailTable(m)
                }
                footer
            }
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .onChange(of: scope) { selectedItems = [] }
    }

    // MARK: - 필터 (AI · 세부 항목)

    /// AI 칩: "전체" + 세 AI. AI를 누르면 그 AI의 세부 항목 분석으로, 다시 누르거나 "전체"를 누르면 비교로 돌아간다.
    /// 데이터가 없는 AI는 흐리게 + 누르면 "아직 데이터 없음" 안내.
    @ViewBuilder private func aiFilter(_ m: Model) -> some View {
        HStack(spacing: 8) {
            filterChip(settings.t("analytics.allAI"), color: .accentColor, on: scope == nil, enabled: true) {
                scope = nil
            }
            ForEach(settings.providerOrder, id: \.self) { id in
                if let spec = ProviderSpec.spec(id) {
                    let hasData = m.aiAll.contains { $0.id == id }
                        || !history.itemIDs(for: id, since: since).isEmpty
                    filterChip(spec.name, color: settings.color(forProvider: id), on: scope == id, enabled: hasData) {
                        if hasData { scope = (scope == id) ? nil : id } else { flashNoData(spec.name) }
                    }
                }
            }
            if let flash = noDataFlash {
                Text(flash).font(.caption2).foregroundStyle(.secondary).transition(.opacity)
            }
            Spacer(minLength: 0)
        }
        .animation(.easeInOut(duration: 0.2), value: noDataFlash)
    }

    /// 세부 항목 칩(AI를 골랐을 때만). 여러 개 켜고 끌 수 있고, 모두 끄거나 모두 켜면 전체로 돌아간다.
    @ViewBuilder private func itemFilter(_ m: Model) -> some View {
        HStack(spacing: 6) {
            Text(settings.t("analytics.items")).font(.caption).foregroundStyle(.secondary)
            filterChip(settings.t("analytics.allAI"), color: .secondary, on: selectedItems.isEmpty, enabled: true) {
                selectedItems = []
            }
            ForEach(m.scopedItems) { s in
                let key = s.itemID ?? ""
                filterChip(s.name, color: s.color, on: selectedItems.isEmpty || selectedItems.contains(key), enabled: true) {
                    // 전체 상태에서 누르면 그 항목만, 그다음부터는 켜고 끄기.
                    var sel = selectedItems
                    if sel.isEmpty { sel = [key] } else if sel.contains(key) { sel.remove(key) } else { sel.insert(key) }
                    // 전부 켜면 '전체'와 같다. 빈 집합으로 정리.
                    selectedItems = sel.count == m.scopedItems.count ? [] : sel
                }
            }
            Spacer(minLength: 0)
        }
    }

    private func filterChip(_ title: String, color: Color, on: Bool, enabled: Bool,
                            action: @escaping () -> Void) -> some View {
        Button(action: action) {
            HStack(spacing: 5) {
                Circle().fill(on ? color : Color.secondary.opacity(enabled ? 0.4 : 0.3))
                    .frame(width: 8, height: 8)
                Text(title).font(.caption).lineLimit(1)
                    .foregroundStyle(on ? .primary : .secondary)
            }
            .padding(.horizontal, 10).padding(.vertical, 4)
            .background(Capsule().fill(on ? color.opacity(0.16) : Color.secondary.opacity(0.10)))
            .overlay(Capsule().stroke(on ? color.opacity(0.5) : .clear, lineWidth: 1))
            .opacity(enabled ? 1 : 0.5)
        }
        .buttonStyle(.plain)
    }

    /// 데이터 없는 칩 클릭 시 잠깐 안내 표시.
    private func flashNoData(_ name: String) {
        noDataFlash = settings.tf("analytics.noDataFlash", name)
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.2) {
            if noDataFlash != nil { noDataFlash = nil }
        }
    }

    // MARK: - 헤더 (기간 선택 + 수집 시작일)

    @ViewBuilder private var header: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(settings.t("analytics.title")).font(.title2).bold()
            Spacer()
            if let e = history.earliest {
                Label(settings.tf("analytics.since", Self.dateFmt.string(from: e)), systemImage: "calendar")
                    .font(.caption).foregroundStyle(.secondary)
                    .labelStyle(.titleAndIcon)
            }
            Button { settings.onOpenSettings?() } label: {
                Label(settings.t("menu.settings"), systemImage: "gearshape")
            }
        }
        Picker("", selection: $range) {
            ForEach(AnalyticsRange.allCases) { r in Text(settings.t(r.labelKey)).tag(r) }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
    }

    // MARK: - 빈 상태

    @ViewBuilder private var emptyState: some View {
        VStack(spacing: 10) {
            Image(systemName: "chart.bar.doc.horizontal")
                .font(.system(size: 38)).foregroundStyle(.secondary)
            Text(settings.t("analytics.noData"))
                .font(.callout).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, minHeight: 220)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(nsColor: .controlBackgroundColor)))
    }

    // MARK: - KPI

    @ViewBuilder private func kpiRow(_ m: Model) -> some View {
        let total = m.ai.reduce(0.0) { $0 + $1.consumed }
        let days = max(1.0, range.hours / 24)
        let hits = m.groups.flatMap(\.items).reduce(0) { $0 + AnalyticsMath.stats($1).limitHits }
        HStack(spacing: 12) {
            kpiTile("chart.bar.fill", settings.t("analytics.total"),
                    settings.tf("analytics.timesFmt", Self.times(total)), settings.t("analytics.unitDesc"))
            kpiTile("calendar", settings.t("analytics.dailyAvg"),
                    settings.tf("analytics.timesFmt", Self.times(total / days)), settings.t("analytics.unitDesc"))
            kpiTile("clock.fill", settings.t("analytics.busiest"), busiestHourText(m.ai), " ")
            kpiTile("exclamationmark.octagon", settings.t("analytics.hits"),
                    settings.tn("analytics.countFmt", hits), settings.t("analytics.hitsDesc"))
        }
    }

    private func busiestHourText(_ series: [AnalyticsSeries]) -> String {
        var byHour = [Int: Double]()
        for s in series { for e in s.events {
            byHour[Calendar.current.component(.hour, from: e.t), default: 0] += e.amount
        } }
        guard let top = byHour.max(by: { $0.value < $1.value })?.key else { return "-" }
        return settings.tn("analytics.hourFmt", top)
    }

    private func kpiTile(_ icon: String, _ title: String, _ value: String, _ sub: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Label(title, systemImage: icon)
                .font(.caption).foregroundStyle(.secondary).labelStyle(.titleAndIcon)
            Text(value).font(.title).bold().foregroundStyle(.primary)
            Text(sub).font(.caption2).foregroundStyle(.secondary).lineLimit(1)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(nsColor: .controlBackgroundColor)))
    }

    // MARK: - AI별 소비 비중(도넛) · 항목별 소비(가로 막대)

    private struct SharePoint: Identifiable { let id = UUID(); let name: String; let amount: Double }

    @ViewBuilder private func shareChart(_ m: Model) -> some View {
        let data = m.shown.map { SharePoint(name: $0.name, amount: $0.consumed) }.filter { $0.amount > 0 }
        Chart(data) { p in
            SectorMark(angle: .value(settings.t("analytics.consumption"), p.amount),
                       innerRadius: .ratio(0.62), angularInset: 1.5)
                .foregroundStyle(by: .value("AI", p.name))
                .cornerRadius(3)
        }
        .chartForegroundStyleScale(domain: m.styleDomain, range: m.styleRange)
        .chartLegend(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)   // 박스 높이에 맞춰 채움(높이 통일)
    }

    @ViewBuilder private func itemUseChart(_ m: Model) -> some View {
        let data = m.shown.map { SharePoint(name: $0.name, amount: $0.consumed * 100) }
        Chart(data) { p in
            BarMark(x: .value(settings.t("analytics.consumption"), p.amount),
                    y: .value(settings.t("analytics.col.item"), p.name))
                .foregroundStyle(by: .value("item", p.name))
                .cornerRadius(3)
                .annotation(position: .trailing) {
                    Text(ppText(p.amount)).font(.caption2).foregroundStyle(.secondary)
                }
        }
        .chartForegroundStyleScale(domain: m.styleDomain, range: m.styleRange)
        .chartLegend(.hidden)
        .chartXAxis(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - 시간대별 차트

    private struct HourPoint: Identifiable { let id = UUID(); let name: String; let hour: Int; let amount: Double }

    @ViewBuilder private func hourChart(_ m: Model) -> some View {
        let data: [HourPoint] = m.shown.flatMap { s -> [HourPoint] in
            var byHour = [Int: Double]()
            for e in s.events { byHour[Calendar.current.component(.hour, from: e.t), default: 0] += e.amount }
            return byHour.map { HourPoint(name: s.name, hour: $0.key, amount: $0.value) }
        }
        Chart(data) { p in
            BarMark(x: .value(settings.t("analytics.hour"), p.hour),
                    y: .value(settings.t("analytics.consumption"), p.amount * 100),
                    width: .fixed(7))
                .foregroundStyle(by: .value("AI", p.name))
                .cornerRadius(2)
        }
        .chartForegroundStyleScale(domain: m.styleDomain, range: m.styleRange)
        .chartXScale(domain: -0.5...23.5)
        .chartXAxis { AxisMarks(values: [0, 3, 6, 9, 12, 15, 18, 21]) { v in
            AxisGridLine(); AxisValueLabel { if let h = v.as(Int.self) { Text(settings.tn("analytics.hourFmt", h)) } }
        } }
        .chartLegend(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)   // 박스 높이에 맞춰 채움(높이 통일)
    }

    // MARK: - 잔여량 추이(선)

    private struct LinePoint: Identifiable { let id = UUID(); let name: String; let t: Date; let pct: Double }

    @ViewBuilder private func remainingChart(_ m: Model) -> some View {
        let now = Date()
        let data: [LinePoint] = m.shown.flatMap { s in
            AnalyticsMath.downsample(s.points, since: since, until: now)
                .map { LinePoint(name: s.name, t: $0.t, pct: $0.r * 100) }
        }
        VStack(alignment: .leading, spacing: 6) {
            Chart(data) { p in
                LineMark(x: .value(settings.t("analytics.hour"), p.t),
                         y: .value("%", p.pct),
                         series: .value("series", p.name))
                    .foregroundStyle(by: .value("series", p.name))
                    .lineStyle(StrokeStyle(lineWidth: 1.8))
            }
            .chartForegroundStyleScale(domain: m.styleDomain, range: m.styleRange)
            .chartXScale(domain: since...now)
            .chartYScale(domain: 0...100)
            .chartYAxis { AxisMarks(values: [0, 25, 50, 75, 100]) { v in
                AxisGridLine(); AxisValueLabel { if let n = v.as(Int.self) { Text("\(n)%") } }
            } }
            .chartLegend(.hidden)
            legend(m)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    /// 계열 색·이름 범례(차트 아래 한 줄).
    private func legend(_ m: Model) -> some View {
        HStack(spacing: 12) {
            ForEach(m.shown) { s in
                HStack(spacing: 4) {
                    Circle().fill(s.color).frame(width: 7, height: 7)
                    Text(s.name).font(.caption2).foregroundStyle(.secondary).lineLimit(1)
                }
            }
            Spacer(minLength: 0)
        }
    }

    // MARK: - 기간별 소비 추이(쌓인 막대)

    private struct BucketPoint: Identifiable { let id = UUID(); let name: String; let t: Date; let amount: Double }

    /// 이벤트 시각 → 버킷 시작(1일 = 1시간, 3일 = 3시간, 주간·한달 = 하루). 로컬 자정 기준.
    private func bucketStart(_ d: Date) -> Date {
        let day = Calendar.current.startOfDay(for: d)
        let step = range.bucketSeconds
        if step >= 86400 { return day }
        return day.addingTimeInterval((d.timeIntervalSince(day) / step).rounded(.down) * step)
    }

    @ViewBuilder private func overTimeChart(_ m: Model) -> some View {
        let data: [BucketPoint] = m.shown.flatMap { s -> [BucketPoint] in
            var by = [Date: Double]()
            for e in s.events { by[bucketStart(e.t), default: 0] += e.amount }
            return by.map { BucketPoint(name: s.name, t: $0.key, amount: $0.value * 100) }
        }
        let unit: Calendar.Component = range.bucketSeconds >= 86400 ? .day : .hour
        Chart(data) { p in
            BarMark(x: .value(settings.t("analytics.hour"), p.t, unit: unit),
                    y: .value(settings.t("analytics.consumption"), p.amount))
                .foregroundStyle(by: .value("series", p.name))
                .cornerRadius(2)
        }
        .chartForegroundStyleScale(domain: m.styleDomain, range: m.styleRange)
        .chartXScale(domain: bucketStart(since)...bucketStart(Date()).addingTimeInterval(range.bucketSeconds))
        .chartLegend(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - 요일 × 시간대 히트맵

    private struct HeatCell: Identifiable { let id = UUID(); let hour: Int; let row: Int; let amount: Double }

    /// 앱 언어의 짧은 요일 이름(일요일부터, Calendar weekday 1~7 순서).
    private var weekdaySymbols: [String] {
        var cal = Calendar.current
        cal.locale = Locale(identifier: settings.language.rawValue)
        return cal.shortWeekdaySymbols
    }

    /// 요일 × 시각 칸마다 소비(%p) 합계. 행 순서 = 달력의 첫 요일부터.
    /// 세부 항목은 단위가 서로 달라 더하지 않는다. 히트맵은 AI 대표 소비로 그린다.
    private func heatCells(_ series: [AnalyticsSeries]) -> (cells: [HeatCell], rows: [String]) {
        let syms = weekdaySymbols
        let first = Calendar.current.firstWeekday - 1
        let rows = (0..<7).map { syms[(first + $0) % 7] }
        var grid = [String: Double]()
        for s in series {
            for e in s.events {
                let c = Calendar.current.dateComponents([.weekday, .hour], from: e.t)
                grid["\(syms[(c.weekday ?? 1) - 1])|\(c.hour ?? 0)", default: 0] += e.amount * 100
            }
        }
        // 색 농도 = 최댓값 대비 제곱근(몇몇 큰 칸에 묻혀 나머지가 다 어둡게 보이지 않도록).
        let peak = max(grid.values.max() ?? 0, 0.0001)
        let cells = rows.enumerated().flatMap { i, d in (0..<24).map { h in
            HeatCell(hour: h, row: i, amount: ((grid["\(d)|\(h)"] ?? 0) / peak).squareRoot())
        } }
        return (cells, rows)
    }

    @ViewBuilder private func heatmap(_ m: Model) -> some View {
        let (cells, names) = heatCells(m.ai)
        let base = m.ai.count == 1 ? m.ai[0].color : Color.accentColor
        Chart(cells) { c in
            // 첫 요일이 맨 위에 오도록 행 번호를 뒤집어 그린다.
            RectangleMark(xStart: .value(settings.t("analytics.hour"), Double(c.hour) + 0.05),
                          xEnd: .value(settings.t("analytics.hour"), Double(c.hour) + 0.95),
                          yStart: .value("day", Double(6 - c.row) + 0.08),
                          yEnd: .value("day", Double(6 - c.row) + 0.92))
                .foregroundStyle(by: .value(settings.t("analytics.consumption"), c.amount))
                .cornerRadius(2)
        }
        .chartForegroundStyleScale(domain: 0.0...1.0, range: Gradient(colors: [Color.secondary.opacity(0.12), base]))
        .chartXScale(domain: 0...24)
        .chartYScale(domain: 0...7)
        .chartYAxis { AxisMarks(position: .leading, values: (0..<7).map { Double($0) + 0.5 }) { v in
            AxisValueLabel { if let y = v.as(Double.self) { Text(names[6 - Int(y)]) } }
        } }
        .chartXAxis { AxisMarks(values: [0.5, 6.5, 12.5, 18.5]) { v in
            AxisValueLabel { if let h = v.as(Double.self) { Text(settings.tn("analytics.hourFmt", Int(h))) } }
        } }
        .chartLegend(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    // MARK: - 세부 항목 상세 표

    @ViewBuilder private func detailTable(_ m: Model) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(settings.t("analytics.detail"), systemImage: "tablecells").font(.headline)
            Text(settings.t("analytics.detailDesc")).font(.caption).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if m.groups.allSatisfy({ $0.items.isEmpty }) {
                Text(settings.t("analytics.itemsNone")).font(.callout).foregroundStyle(.secondary)
                    .padding(.vertical, 8)
            } else {
                Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 7) {
                    GridRow {
                        ForEach(["item", "now", "used", "resets", "util", "hits", "pace", "eta"], id: \.self) { k in
                            Text(settings.t("analytics.col." + k)).font(.caption).bold().foregroundStyle(.secondary)
                        }
                    }
                    Divider()
                    ForEach(m.groups, id: \.spec.id) { g in
                        if m.groups.count > 1 {
                            GridRow {
                                Text(g.spec.name).font(.subheadline).bold()
                                    .foregroundStyle(settings.color(forProvider: g.spec.id))
                                    .gridCellColumns(8)
                            }
                        }
                        ForEach(g.items) { s in detailRow(s) }
                    }
                }
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(nsColor: .controlBackgroundColor)))
    }

    @ViewBuilder private func detailRow(_ s: AnalyticsSeries) -> some View {
        let st = AnalyticsMath.stats(s)
        GridRow {
            HStack(spacing: 6) {
                Circle().fill(s.color).frame(width: 8, height: 8)
                Text(s.name).lineLimit(1)
            }
            .frame(minWidth: 170, alignment: .leading)
            HStack(spacing: 6) {
                ZStack(alignment: .leading) {
                    Capsule().fill(s.color.opacity(0.18))
                    Capsule().fill(s.color).frame(width: 50 * CGFloat(st.current ?? 0))
                }
                .frame(width: 50, height: 6)
                Text(st.current.map { "\(Int(($0 * 100).rounded()))%" } ?? "-")
            }
            Text(ppText(st.consumed * 100))
            Text(settings.tn("analytics.countFmt", st.resets))
            Text(st.windowUse.map { "\(Int(($0 * 100).rounded()))%" } ?? "-")
            Text(settings.tn("analytics.countFmt", st.limitHits))
                .foregroundStyle(st.limitHits > 0 ? .orange : .primary)
            Text(st.ratePerHour.map { settings.tf("analytics.perHourFmt", Self.num($0 * 100)) } ?? "-")
            etaText(s, st)
        }
        .font(.caption).monospacedDigit()
    }

    /// 지금 속도로 쓰면 언제 바닥나는지. 리셋이 먼저 오면 '여유'.
    private func etaText(_ s: AnalyticsSeries, _ st: SeriesStats) -> some View {
        guard let rate = st.ratePerHour, let cur = st.current else {
            return Text(settings.t("analytics.etaIdle")).foregroundStyle(.secondary)
        }
        let empty = Date().addingTimeInterval(cur / rate * 3600)
        if let reset = s.resetAt, reset < empty {
            return Text(settings.t("analytics.etaSafe")).foregroundStyle(.green)
        }
        let f = DateFormatter()
        f.dateFormat = Calendar.current.isDateInToday(empty) ? "HH:mm" : "M/d HH:mm"
        return Text(settings.tf("analytics.etaAt", f.string(from: empty))).foregroundStyle(.orange)
    }

    private func ppText(_ v: Double) -> String { settings.tf("analytics.ppFmt", Self.num(v)) }

    /// 회분 표기: 1 미만은 소수 둘째 자리까지(조금 쓴 날이 0.0으로 보이지 않게).
    private static func times(_ v: Double) -> String { String(format: v < 1 ? "%.2f" : "%.1f", v) }

    /// 10 이상은 정수, 그 아래는 소수 한 자리.
    private static func num(_ v: Double) -> String { v >= 10 ? String(format: "%.0f", v) : String(format: "%.1f", v) }

    // MARK: - 하단 안내(설명 + 주의)

    @ViewBuilder private var footer: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(settings.t("analytics.aboutTitle"), systemImage: "info.circle")
                .font(.subheadline).bold()
            Text(settings.t("analytics.about"))
                .font(.caption).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            Divider()
            Label(settings.t("analytics.cautionTitle"), systemImage: "exclamationmark.triangle")
                .font(.subheadline).bold().foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 4) {
                ForEach(["analytics.c1", "analytics.c2", "analytics.c3", "analytics.c4", "analytics.c5", "analytics.c6"], id: \.self) { k in
                    HStack(alignment: .firstTextBaseline, spacing: 6) {
                        Text("•").foregroundStyle(.secondary)
                        Text(settings.t(k)).fixedSize(horizontal: false, vertical: true)
                    }
                    .font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(nsColor: .controlBackgroundColor).opacity(0.6)))
    }

    // MARK: - 공통

    @ViewBuilder
    private func card<Content: View>(title: String, desc: String, icon: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(title, systemImage: icon).font(.headline)
            Text(desc).font(.caption).foregroundStyle(.secondary).lineLimit(2)
            content()
        }
        .padding(16)
        // 고정 높이 프레임에 들어가면 배경까지 그 높이를 채운다(두 박스 높이 통일).
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(nsColor: .controlBackgroundColor)))
    }

    private static let dateFmt: DateFormatter = {
        let f = DateFormatter(); f.dateFormat = "yyyy/M/d"; return f
    }()
}
