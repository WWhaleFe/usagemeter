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
            .frame(width: 268, height: 92)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 6)
    }
}
