import SwiftUI
import CoreImage.CIFilterBuiltins

/// 후원 링크(앱 여러 곳에서 공유). 모두 수수료 없이 전달된다.
enum SupportLinks {
    /// GitHub Sponsors — 플랫폼·결제 수수료 0%.
    static let sponsors = "https://github.com/sponsors/WWhaleFe"
    /// 카카오페이 수신 QR/링크 — 국내 개인 송금(수수료 0원).
    static let kakaoPay = "https://qr.kakaopay.com/Fa89McYNg"
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

/// 설정 창 '후원' 탭: GitHub Sponsors 버튼 + 카카오페이 QR(+링크).
struct SupportView: View {
    @ObservedObject var settings: OverlaySettings

    var body: some View {
        VStack(spacing: 18) {
            Text(settings.t("support.desc"))
                .font(.callout).foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity)

            // GitHub Sponsors
            Button {
                open(SupportLinks.sponsors)
            } label: {
                Label(settings.t("support.sponsors"), systemImage: "heart.fill")
                    .frame(maxWidth: 280)
            }
            .controlSize(.large)
            .tint(.pink)

            Divider().frame(maxWidth: 320)

            // 카카오페이 QR — 가운데 정렬.
            VStack(spacing: 10) {
                Text(settings.t("support.kakaoTitle")).font(.headline)
                Text(settings.t("support.kakaoScan"))
                    .font(.callout).foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                if let qr = QRCode.image(from: SupportLinks.kakaoPay, size: 190) {
                    Image(nsImage: qr)
                        .interpolation(.none)
                        .resizable()
                        .frame(width: 190, height: 190)
                        .padding(12)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 12))  // QR 대비 위해 항상 흰 배경
                        .overlay(RoundedRectangle(cornerRadius: 12).stroke(.secondary.opacity(0.25)))
                        .shadow(color: .black.opacity(0.12), radius: 6, y: 2)
                }
                Button {
                    open(SupportLinks.kakaoPay)
                } label: {
                    Label(settings.t("support.kakaoOpen"), systemImage: "link")
                        .frame(maxWidth: 280)
                }
                .controlSize(.large)
            }

            Text(settings.t("support.thanks"))
                .font(.callout).foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity)
        .multilineTextAlignment(.center)
    }

    private func open(_ s: String) {
        guard let url = URL(string: s) else { return }
        NSWorkspace.shared.open(url)
    }
}
