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

    /// 데이터의 실제 시간 범위. Charts의 자동 도메인이 눈금 경계까지 늘려
    /// 오른쪽에 빈 공간을 만들지 않도록 이 범위로 X축을 고정한다(#빈공간 채움).
    private var xDomain: ClosedRange<Date>? {
        let all = lines.flatMap { $0.points.map(\.0) }
        guard let lo = all.min(), let hi = all.max(), hi > lo else { return nil }
        return lo...hi
    }

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
            .frame(maxWidth: .infinity, minHeight: 92, maxHeight: 92)   // 메뉴 폭에 꽉 차게
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
        .frame(maxWidth: .infinity, alignment: .leading)
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
