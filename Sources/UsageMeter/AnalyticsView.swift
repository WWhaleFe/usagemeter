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

/// AI 사용량 분석 대시보드: 시간대별 사용 패턴 + 기간별 소비 추이 + 요약 + 안내.
/// 데이터 = HistoryStore의 잔여율 하락(=소비)을 집계. 5시간 한도 기준 상대 소비량.
struct AnalyticsView: View {
    @ObservedObject var settings: OverlaySettings
    @ObservedObject var manager: ProviderManager
    @ObservedObject var history: HistoryStore
    @State private var range: AnalyticsRange = .day1
    /// 보이게 할 AI 집합(비어 있으면 전체). 모두·일부·하나만 선택 가능(#AI별 선택).
    @State private var selectedAIs: Set<String> = []
    /// 데이터 없는 AI 칩을 눌렀을 때 잠깐 뜨는 안내.
    @State private var noDataFlash: String? = nil

    /// 상단 2단 차트 박스의 공통 높이(박스 높이 통일).
    static let rowHeight: CGFloat = 250

    private struct Series { let id: String; let name: String; let color: Color; let events: [(t: Date, amount: Double)] }

    private var since: Date { Date().addingTimeInterval(-range.hours * 3600) }

    /// 데이터가 있는 모든 AI 계열(순서 = providerOrder). AI 필터 옵션의 원천.
    private var allSeries: [Series] {
        settings.providerOrder.compactMap { id in
            guard let spec = ProviderSpec.spec(id) else { return nil }
            let ev = history.consumptionEvents(for: id, since: since)
            guard !ev.isEmpty else { return nil }
            return Series(id: id, name: spec.name, color: settings.color(forProvider: id), events: ev)
        }
    }

    /// 실제로 차트·요약에 쓰는 계열(AI 필터 반영).
    private var seriesList: [Series] {
        selectedAIs.isEmpty ? allSeries : allSeries.filter { selectedAIs.contains($0.id) }
    }

    /// AI 칩 토글(최소 1개는 유지 — 모두 끄면 전체로 복귀).
    private func toggleAI(_ id: String) {
        var s = selectedAIs.isEmpty ? Set(allSeries.map { $0.id }) : selectedAIs
        if s.contains(id) { s.remove(id) } else { s.insert(id) }
        if s.isEmpty { s = Set(allSeries.map { $0.id }) }
        selectedAIs = s
    }

    private var styleScale: (domain: [String], range: [Color]) {
        let s = seriesList
        return (s.map(\.name), s.map(\.color))
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                header
                aiFilter                   // AI 선택 칩(항상 3개 표시, 색-이름 = 범례 겸 필터)
                if seriesList.isEmpty {
                    emptyState
                } else {
                    // 왼쪽 = AI별 소비 비중(도넛), 오른쪽 = 시간대별 사용 패턴. 박스 높이 통일.
                    HStack(alignment: .top, spacing: 14) {
                        card(title: settings.t("analytics.share"), desc: settings.t("analytics.shareDesc"),
                             icon: "chart.pie") { shareChart }
                            .frame(width: 330, height: Self.rowHeight)
                        card(title: settings.t("analytics.byHour"), desc: settings.t("analytics.byHourDesc"),
                             icon: "clock") { hourChart }
                            .frame(maxWidth: .infinity)
                            .frame(height: Self.rowHeight)
                    }
                    // 그 아래: 정보(요약 + 설명·주의)
                    kpiRow
                }
                footer
            }
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    /// AI 선택 칩 — 세 AI를 항상 표시(색·이름 = 범례 겸). 데이터 있는 것만 선택 가능,
    /// 없는 것은 흐리게 + 클릭/호버 시 "아직 데이터 없음" 안내.
    @ViewBuilder private var aiFilter: some View {
        HStack(spacing: 8) {
            ForEach(settings.providerOrder, id: \.self) { id in
                if let spec = ProviderSpec.spec(id) { chip(for: spec) }
            }
            if let flash = noDataFlash {
                Text(flash).font(.caption2).foregroundStyle(.secondary).transition(.opacity)
            }
            Spacer(minLength: 0)
        }
        .animation(.easeInOut(duration: 0.2), value: noDataFlash)
    }

    @ViewBuilder private func chip(for spec: ProviderSpec) -> some View {
        let hasData = allSeries.contains { $0.id == spec.id }
        let on = hasData && (selectedAIs.isEmpty || selectedAIs.contains(spec.id))
        let color = settings.color(forProvider: spec.id)
        Button {
            if hasData { toggleAI(spec.id) } else { flashNoData(spec.name) }
        } label: {
            HStack(spacing: 5) {
                Circle().fill(on ? color : Color.secondary.opacity(hasData ? 0.4 : 0.3))
                    .frame(width: 8, height: 8)
                Text(spec.name).font(.caption)
                    .foregroundStyle(on ? .primary : .secondary)
            }
            .padding(.horizontal, 10).padding(.vertical, 4)
            .background(Capsule().fill(on ? color.opacity(0.16) : Color.secondary.opacity(0.10)))
            .overlay(Capsule().stroke(on ? color.opacity(0.5) : .clear, lineWidth: 1))
            .opacity(hasData ? 1 : 0.5)
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

    @ViewBuilder private var kpiRow: some View {
        let total = seriesList.flatMap { $0.events }.reduce(0.0) { $0 + $1.amount }
        let days = max(1.0, range.hours / 24)
        HStack(spacing: 12) {
            kpiTile("chart.bar.fill", settings.t("analytics.total"),
                    settings.tf("analytics.timesFmt", String(format: "%.1f", total)), settings.t("analytics.unitDesc"))
            kpiTile("calendar", settings.t("analytics.dailyAvg"),
                    settings.tf("analytics.timesFmt", String(format: "%.1f", total / days)), settings.t("analytics.unitDesc"))
            kpiTile("clock.fill", settings.t("analytics.busiest"), busiestHourText, " ")
        }
    }

    private var busiestHourText: String {
        var byHour = [Int: Double]()
        for s in seriesList { for e in s.events {
            byHour[Calendar.current.component(.hour, from: e.t), default: 0] += e.amount
        } }
        guard let top = byHour.max(by: { $0.value < $1.value })?.key else { return "—" }
        return settings.tn("analytics.hourFmt", top)
    }

    private func kpiTile(_ icon: String, _ title: String, _ value: String, _ sub: String) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Label(title, systemImage: icon)
                .font(.caption).foregroundStyle(.secondary).labelStyle(.titleAndIcon)
            Text(value).font(.title).bold().foregroundStyle(.primary)
            Text(sub).font(.caption2).foregroundStyle(.secondary)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(nsColor: .controlBackgroundColor)))
    }

    // MARK: - 시간대별 차트

    private struct HourPoint: Identifiable { let id = UUID(); let name: String; let hour: Int; let amount: Double }

    private var hourData: [HourPoint] {
        var out: [HourPoint] = []
        for s in seriesList {
            var byHour = [Int: Double](); for e in s.events { byHour[Calendar.current.component(.hour, from: e.t), default: 0] += e.amount }
            for h in 0..<24 where (byHour[h] ?? 0) > 0 { out.append(HourPoint(name: s.name, hour: h, amount: byHour[h] ?? 0)) }
        }
        return out
    }

    @ViewBuilder private var hourChart: some View {
        Chart(hourData) { p in
            BarMark(x: .value(settings.t("analytics.hour"), p.hour),
                    y: .value(settings.t("analytics.consumption"), p.amount * 100),
                    width: .fixed(7))
                .foregroundStyle(by: .value("AI", p.name))
                .cornerRadius(2)
        }
        .chartForegroundStyleScale(domain: styleScale.domain, range: styleScale.range)
        .chartXScale(domain: -0.5...23.5)
        .chartXAxis { AxisMarks(values: [0, 3, 6, 9, 12, 15, 18, 21]) { v in
            AxisGridLine(); AxisValueLabel { if let h = v.as(Int.self) { Text(settings.tn("analytics.hourFmt", h)) } }
        } }
        .chartLegend(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)   // 박스 높이에 맞춰 채움(높이 통일)
    }

    // MARK: - AI별 소비 비중(도넛)

    private struct SharePoint: Identifiable { let id = UUID(); let name: String; let color: Color; let amount: Double }

    private var shareData: [SharePoint] {
        seriesList.map { s in SharePoint(name: s.name, color: s.color, amount: s.events.reduce(0) { $0 + $1.amount }) }
            .filter { $0.amount > 0 }
    }

    @ViewBuilder private var shareChart: some View {
        Chart(shareData) { p in
            SectorMark(angle: .value(settings.t("analytics.consumption"), p.amount),
                       innerRadius: .ratio(0.62), angularInset: 1.5)
                .foregroundStyle(by: .value("AI", p.name))
                .cornerRadius(3)
        }
        .chartForegroundStyleScale(domain: styleScale.domain, range: styleScale.range)
        .chartLegend(.hidden)
        .frame(maxWidth: .infinity, maxHeight: .infinity)   // 박스 높이에 맞춰 채움(높이 통일)
    }

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
                ForEach(["analytics.c1", "analytics.c2", "analytics.c3", "analytics.c4"], id: \.self) { k in
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
