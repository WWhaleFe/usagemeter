import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let l = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as! [[String: Any]]
for w in l where (w[kCGWindowOwnerPID as String] as? Int32) == pid {
  let b = w[kCGWindowBounds as String] as! [String: Any]
  print(w[kCGWindowNumber as String]!, b["Width"]!, b["Height"]!, b["X"]!, b["Y"]!, w[kCGWindowName as String] ?? "", w[kCGWindowLayer as String]!)
}
