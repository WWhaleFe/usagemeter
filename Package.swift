// swift-tools-version:6.0
// usagemeter — AI 사용량 테두리 오버레이 위젯 (PoC 1단계)
// Swift Package Manager 실행파일. 전체 Xcode 없이 `swift run` 으로 빌드·실행한다.
import PackageDescription

let package = Package(
    name: "UsageMeter",
    platforms: [
        // AppKit/SwiftUI 오버레이에 필요한 최소 macOS 버전 (.onKeyPress는 14+)
        .macOS(.v14)
    ],
    targets: [
        // 실행 타겟 하나. Sources/UsageMeter/ 아래의 .swift 파일이 모두 포함된다.
        .executableTarget(
            name: "UsageMeter",
            path: "Sources/UsageMeter"
        )
    ]
)
