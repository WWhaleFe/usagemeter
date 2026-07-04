import AppKit

// 진입점. SPM 실행파일이라 Xcode 앱 번들 없이도 여기서 NSApplication을 손수 구성한다.
let app = NSApplication.shared

// .accessory: Dock 아이콘 없이 배경에서 도는 보조 앱(메뉴바 위젯류에 적합).
app.setActivationPolicy(.accessory)

// delegate를 지역 변수로 두면 run() 도중 해제될 수 있으므로 전역에 보관한다.
let delegate = AppDelegate()
app.delegate = delegate

app.run()
