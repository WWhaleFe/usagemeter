import SwiftUI
import Charts

/// 드롭다운에 넣는 24시간 잔여율 미니 차트(AI별 라인).
struct MiniChartView: View {
    struct Line: Identifiable {
        let id: String
        let name: String
        let color: Color
        let points: [(Date, Double)]
    }
    let title: String
    let lines: [Line]
    /// 차트가 보여줄 기간(시간). X축은 [지금-기간, 지금]으로 고정해 **실제 시간 스케일**을
    /// 보여준다(#24시간 안 보임 수정) — 데이터가 적으면 그만큼만 채워지고 오른쪽 끝 = 지금.
    var hours: Double = 24
    /// 마우스를 올리면 그 시점 값을 툴팁으로 보여줄지(#차트 호버).
    var hoverEnabled: Bool = true

    /// 현재 호버 중인 시점(없으면 nil).
    @State private var hoverDate: Date? = nil

    private var xDomain: ClosedRange<Date>? {
        let now = Date()
        return now.addingTimeInterval(-hours * 3600)...now
    }

    /// 호버한 시점에서 각 라인의 (가장 가까운) 값.
    private func valuesAt(_ date: Date) -> [(name: String, color: Color, value: Double)] {
        lines.compactMap { line in
            guard let nearest = line.points.min(by: {
                abs($0.0.timeIntervalSince(date)) < abs($1.0.timeIntervalSince(date))
            }) else { return nil }
            return (line.name, line.color, nearest.1 * 100)
        }
    }

    private static let hoverFmt: DateFormatter = {
        let f = DateFormatter(); f.dateFormat = "M/d HH:mm"; return f
    }()

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.caption).foregroundStyle(.secondary)
            Chart {
                ForEach(lines) { line in
                    ForEach(Array(line.points.enumerated()), id: \.offset) { _, p in
                        LineMark(
                            x: .value("t", p.0),
                            y: .value("r", p.1 * 100),
                            series: .value("ai", line.name)
                        )
                        .foregroundStyle(line.color)
                        .interpolationMethod(.monotone)
                    }
                }
                if let hoverDate {
                    RuleMark(x: .value("t", hoverDate))
                        .foregroundStyle(.secondary.opacity(0.5))
                        .lineStyle(StrokeStyle(lineWidth: 1, dash: [3, 3]))
                }
            }
            .chartYScale(domain: 0...100)
            .modifier(XDomainModifier(domain: xDomain))
            .chartYAxis {
                AxisMarks(values: [0, 50, 100]) { v in
                    AxisGridLine(); AxisValueLabel { if let i = v.as(Int.self) { Text("\(i)") } }
                }
            }
            .chartXAxis {
                AxisMarks(values: .automatic(desiredCount: 3)) { _ in
                    AxisGridLine(); AxisValueLabel(format: .dateTime.hour())
                }
            }
            .chartOverlay { proxy in
                GeometryReader { geo in
                    if hoverEnabled {
                        Rectangle().fill(.clear).contentShape(Rectangle())
                            .onContinuousHover { phase in
                                switch phase {
                                case .active(let pt):
                                    let origin = geo[proxy.plotAreaFrame].origin
                                    let x = pt.x - origin.x
                                    hoverDate = proxy.value(atX: x, as: Date.self)
                                case .ended:
                                    hoverDate = nil
                                }
                            }
                    }
                }
            }
            .overlay(alignment: .topLeading) {
                if let hoverDate {
                    hoverTooltip(hoverDate)
                }
            }
            .frame(maxWidth: .infinity, minHeight: 92, maxHeight: 92)   // 메뉴 폭에 꽉 차게
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    /// 호버 시점의 값 툴팁(시간 + AI별 %).
    @ViewBuilder
    private func hoverTooltip(_ date: Date) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(Self.hoverFmt.string(from: date)).font(.caption2).foregroundStyle(.secondary)
            ForEach(Array(valuesAt(date).enumerated()), id: \.offset) { _, v in
                HStack(spacing: 5) {
                    Circle().fill(v.color).frame(width: 7, height: 7)
                    Text("\(v.name) \(Int(v.value.rounded()))%").font(.caption2)
                }
            }
        }
        .padding(6)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 6))
        .padding(4)
    }
}

/// X축 도메인이 있으면 고정, 없으면(데이터 부족) 자동 유지.
private struct XDomainModifier: ViewModifier {
    let domain: ClosedRange<Date>?
    func body(content: Content) -> some View {
        if let domain {
            content.chartXScale(domain: domain)
        } else {
            content
        }
    }
}
