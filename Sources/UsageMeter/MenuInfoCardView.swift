import SwiftUI

/// 드롭다운에 넣는 AI별 사용량 정보 한 항목의 데이터.
struct MenuAIInfo: Identifiable {
    let id: String
    let name: String
    let color: Color
    /// 구독 플랜/모델(예: "Max (5x)"). 없으면 nil.
    let plan: String?
    /// 표시할 정보 줄들(예: "5시간 잔여  95% · 34분 후").
    let lines: [String]
}

/// 메뉴바 드롭다운의 AI 정보 카드.
///
/// 기존엔 비활성 NSMenuItem으로 그려 macOS가 텍스트를 흐리게(dim) 만들어 가독성이 낮았다.
/// SwiftUI로 직접 그려 `.primary`(고대비) 텍스트를 쓰고, 라이트/다크에 자동 대응한다.
struct MenuInfoCardView: View {
    let infos: [MenuAIInfo]

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            ForEach(Array(infos.enumerated()), id: \.element.id) { idx, info in
                if idx > 0 { Divider() }
                VStack(alignment: .leading, spacing: 3) {
                    HStack(spacing: 8) {
                        Text(info.name).font(.headline).foregroundStyle(info.color)
                        if let plan = info.plan, !plan.isEmpty {
                            Text(plan)
                                .font(.caption).bold()
                                .padding(.horizontal, 6).padding(.vertical, 1)
                                .background(info.color.opacity(0.16), in: Capsule())
                                .foregroundStyle(info.color)
                        }
                    }
                    ForEach(Array(info.lines.enumerated()), id: \.offset) { _, line in
                        Text(line)
                            .font(.callout)
                            .foregroundStyle(.primary)      // 고대비 — 라이트/다크 자동
                            .lineLimit(1)
                            .fixedSize(horizontal: true, vertical: false)   // 줄바꿈 금지(#Fable 줄 안 넘어가게)
                    }
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .frame(minWidth: 340, alignment: .leading)
        .fixedSize()                                        // 가장 긴 줄에 맞춰 카드 폭 자동 확장
    }
}
