// 테두리 기준 이미지 생성기 (Rust 이식 검증용, macOS 전용).
// 실제 Sources/UsageMeter/BorderView.swift를 그대로 컴파일해 SwiftUI ImageRenderer(scale 2)로 PNG를 뽑는다.
// 사용: border-fixtures <cases.json> <출력 디렉터리> [케이스 이름...]
// 빌드: ./build.sh
import AppKit
import SwiftUI
import ImageIO
import UniformTypeIdentifiers

struct CaseModel {
    let name: String
    let size: CGSize
    let settings: [String: Any]
    let ais: [[String: Any]]
}

func fail(_ msg: String) -> Never {
    FileHandle.standardError.write(Data((msg + "\n").utf8))
    exit(1)
}

/// ais 항목 -> UsageSnapshot. 세부 항목(5h·weekly·모델 버킷)의 원천 값을 채운다.
func makeSnapshot(_ ai: [String: Any]) -> UsageSnapshot {
    let id = ai["id"] as! String
    let ratio = (ai["ratio"] as! NSNumber).doubleValue
    var models: [UsageModelBucket] = []
    for m in (ai["models"] as? [[String: Any]]) ?? [] {
        models.append(UsageModelBucket(label: m["label"] as! String,
                                       remainingRatio: (m["ratio"] as! NSNumber).doubleValue,
                                       resetAt: nil,
                                       isFiveHour: (m["fiveHour"] as? Bool) ?? false))
    }
    var snap = UsageSnapshot(id: id, remainingRatio: ratio,
                             secondaryRatio: (ai["weekly"] as? NSNumber)?.doubleValue,
                             opusRatio: nil, resetAt: nil, secondaryResetAt: nil, opusResetAt: nil,
                             status: .ok, lastUpdated: Date(timeIntervalSince1970: 0))
    snap.fiveHourRatio = (ai["fiveHour"] as? NSNumber)?.doubleValue
    snap.modelBuckets = models.isEmpty ? nil : models
    return snap
}

/// sRGB 8비트 premultiplied RGBA로 다시 그려 PNG로 저장(색 공간을 고정).
func writePNG(_ image: CGImage, to url: URL) {
    let w = image.width, h = image.height
    let cs = CGColorSpace(name: CGColorSpace.sRGB)!
    guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0, space: cs,
                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { fail("CGContext 실패") }
    ctx.clear(CGRect(x: 0, y: 0, width: w, height: h))
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
    guard let out = ctx.makeImage(),
          let dest = CGImageDestinationCreateWithURL(url as CFURL, UTType.png.identifier as CFString, 1, nil)
    else { fail("PNG 대상 생성 실패") }
    CGImageDestinationAddImage(dest, out, nil)
    if !CGImageDestinationFinalize(dest) { fail("PNG 쓰기 실패: \(url.path)") }
}

@MainActor
func renderCase(_ c: CaseModel, outDir: URL) {
    // 설정: 케이스의 settings(SettingsSnapshot JSON)를 그대로 apply.
    // OverlaySettings() 초기화가 기기 맞춤을 건드릴 수 있어도 apply가 모든 테두리 값을 덮어쓴다.
    let data: Data
    do { data = try JSONSerialization.data(withJSONObject: c.settings) } catch { fail("settings 직렬화 실패: \(c.name)") }
    guard let snapshot = try? JSONDecoder().decode(SettingsSnapshot.self, from: data) else {
        fail("SettingsSnapshot 디코딩 실패: \(c.name)")
    }
    let settings = OverlaySettings()
    settings.apply(snapshot)
    let manager = ProviderManager()
    // ProviderSpec 순서(= 실제 ProviderManager.order)대로 넣는다.
    var active: [(spec: ProviderSpec, snap: UsageSnapshot)] = []
    for spec in ProviderSpec.all {
        if let ai = c.ais.first(where: { ($0["id"] as? String) == spec.id }) {
            active.append((spec, makeSnapshot(ai)))
        }
    }
    manager.active = active

    let view = BorderView(settings: settings, manager: manager, screenSize: c.size, originInScreen: .zero)
        .frame(width: c.size.width, height: c.size.height)
    let renderer = ImageRenderer(content: view)
    renderer.scale = 2
    renderer.isOpaque = false
    guard let cg = renderer.cgImage else { fail("렌더 실패: \(c.name)") }
    writePNG(cg, to: outDir.appendingPathComponent("\(c.name).png"))
    print("ok \(c.name) \(cg.width)x\(cg.height)")
}

@MainActor
func run() {
    let args = CommandLine.arguments
    guard args.count >= 3 else { fail("usage: border-fixtures <cases.json> <outdir> [case...]") }
    let only = Set(args.dropFirst(3))
    guard let raw = try? Data(contentsOf: URL(fileURLWithPath: args[1])),
          let arr = try? JSONSerialization.jsonObject(with: raw) as? [[String: Any]] else { fail("cases.json 읽기 실패") }
    let outDir = URL(fileURLWithPath: args[2], isDirectory: true)
    try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
    for obj in arr {
        let name = obj["name"] as! String
        if !only.isEmpty && !only.contains(name) { continue }
        let sz = obj["screen"] as! [NSNumber]
        let c = CaseModel(name: name, size: CGSize(width: sz[0].doubleValue, height: sz[1].doubleValue),
                          settings: obj["settings"] as! [String: Any], ais: obj["ais"] as! [[String: Any]])
        renderCase(c, outDir: outDir)
    }
    // OverlaySettings 자동 저장이 이 도구의 UserDefaults 영역에 남기는 값을 지운다.
    if let id = Bundle.main.bundleIdentifier { UserDefaults.standard.removePersistentDomain(forName: id) }
}

MainActor.assumeIsolated { run() }
