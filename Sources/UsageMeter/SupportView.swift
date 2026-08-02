import SwiftUI
import CoreImage.CIFilterBuiltins

/// 후원 링크(앱 여러 곳에서 공유). 모두 수수료 없이 전달된다.
enum SupportLinks {
    /// GitHub Sponsors — 플랫폼·결제 수수료 0%.
    static let sponsors = "https://github.com/sponsors/WWhaleFe"
    /// 카카오페이 수신 QR/링크 — 국내 개인 송금(수수료 0원).
    static let kakaoPay = "https://qr.kakaopay.com/Fa89McYNg"
    /// 리포트(문의·버그·제안) 수신 이메일.
    static let reportEmail = "feedback.wwhale@gmail.com"
}

/// 문자열로 QR 코드 이미지를 만든다(CoreImage, 네트워크·외부 의존성 없음).
enum QRCode {
    static func image(from string: String, size: CGFloat = 180) -> NSImage? {
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)
        filter.correctionLevel = "M"
        guard let output = filter.outputImage else { return nil }
        let scale = size / output.extent.width
        let scaled = output.transformed(by: CGAffineTransform(scaleX: scale, y: scale))
        let ctx = CIContext()
        guard let cg = ctx.createCGImage(scaled, from: scaled.extent) else { return nil }
        return NSImage(cgImage: cg, size: NSSize(width: size, height: size))
    }
}

/// 설정 창 '개발자에게' 탭: (상단) 개발자 리포트 이메일 + 메일앱 실행/주소 복사,
/// (하단, 큰 간격) GitHub Sponsors 버튼 + 카카오페이 QR(+링크).
struct SupportView: View {
    @ObservedObject var settings: OverlaySettings
    @State private var copied = false

    var body: some View {
        // 다른 탭과 동일한 섹션 박스 스타일은 유지하되, 내부 컨텐츠는 가운데 정렬.
        VStack(spacing: 12) {
            reportBox
            supportBox
        }
        .frame(maxWidth: .infinity)
    }

    /// 다른 탭의 section()과 동일한 박스 스타일(단, 내용은 가운데 정렬).
    @ViewBuilder private func box<Content: View>(_ title: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(spacing: 8) {
            Text(title).font(.headline)
                .frame(maxWidth: .infinity, alignment: .center)
            content()
        }
        .padding(12)
        .frame(maxWidth: .infinity)
        .background(RoundedRectangle(cornerRadius: 8).fill(Color(nsColor: .controlBackgroundColor)))
        .multilineTextAlignment(.center)
    }

    // MARK: - 리포트 박스

    @ViewBuilder private var reportBox: some View {
        box(settings.t("report.title")) {
            Text(settings.t("report.desc"))
                .font(.callout).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity)

            // 이메일 주소 — 배경/테두리가 주소 길이에 맞게(가운데 정렬).
            Text(SupportLinks.reportEmail)
                .font(.system(.body, design: .monospaced).weight(.semibold))
                .textSelection(.enabled)
                .lineLimit(1).fixedSize()
                .padding(.horizontal, 12).padding(.vertical, 8)
                .background(Color.accentColor.opacity(0.10), in: RoundedRectangle(cornerRadius: 6))
                .overlay(RoundedRectangle(cornerRadius: 6).stroke(Color.accentColor.opacity(0.30)))
                .frame(maxWidth: .infinity)

            // 메일앱 실행(왼쪽) + 주소 복사(오른쪽) — 가운데 정렬.
            HStack(spacing: 10) {
                Button {
                    openMail()
                } label: {
                    Label(settings.t("report.openMail"), systemImage: "paperplane.fill")
                }
                .buttonStyle(.borderedProminent)

                Button {
                    copyEmail()
                } label: {
                    Label(copied ? settings.t("report.copied") : settings.t("report.copyEmail"),
                          systemImage: copied ? "checkmark" : "doc.on.doc")
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.top, 2)
        }
    }

    // MARK: - 후원 박스

    @ViewBuilder private var supportBox: some View {
        box(settings.t("support.moreTitle")) {
            Text(settings.t("support.desc"))
                .font(.callout).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity)

            // GitHub Sponsors
            Button {
                open(SupportLinks.sponsors)
            } label: {
                Label(settings.t("support.sponsors"), systemImage: "heart.fill")
            }
            .tint(.pink)

            // 카카오페이 QR — 가운데 정렬.
            VStack(spacing: 6) {
                Text(settings.t("support.kakaoTitle")).font(.subheadline.weight(.semibold))
                Text(settings.t("support.kakaoScan"))
                    .font(.caption).foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: .infinity)
                if let qr = QRCode.image(from: SupportLinks.kakaoPay, size: 130) {
                    Image(nsImage: qr)
                        .interpolation(.none)
                        .resizable()
                        .frame(width: 130, height: 130)
                        .padding(8)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))  // QR 대비 위해 항상 흰 배경
                        .overlay(RoundedRectangle(cornerRadius: 8).stroke(.secondary.opacity(0.25)))
                }
                Button {
                    open(SupportLinks.kakaoPay)
                } label: {
                    Label(settings.t("support.kakaoOpen"), systemImage: "link")
                }
            }
            .frame(maxWidth: .infinity)
            .padding(.top, 4)

            Text(settings.t("support.thanks"))
                .font(.callout).foregroundStyle(.secondary)
                .frame(maxWidth: .infinity)
                .padding(.top, 2)
        }
    }

    /// 기본 메일 앱으로 작성 창 열기(mailto). 제목은 미리 채운다.
    private func openMail() {
        let subject = settings.t("report.mailSubject")
        let encoded = subject.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? subject
        open("mailto:\(SupportLinks.reportEmail)?subject=\(encoded)")
    }

    /// 이메일 주소를 클립보드에 복사하고 잠시 '복사됨' 표시.
    private func copyEmail() {
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(SupportLinks.reportEmail, forType: .string)
        copied = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.6) { copied = false }
    }

    private func open(_ s: String) {
        guard let url = URL(string: s) else { return }
        NSWorkspace.shared.open(url)
    }
}
