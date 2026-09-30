import AppKit
// §6: derive из реального visibleFrame; окно 980x640 (полноценный UI-размер),
// правый-нижний угол с отступом 24pt; если экран меньше — кламп.
guard let screen = NSScreen.main else { exit(2) }
let v = screen.visibleFrame
let winW = min(980.0, v.width - 48)
let winH = min(640.0, v.height - 48)
let x = max(v.minX + 24, v.maxX - winW - 24)
let y = v.minY + 24
print("\(Int(x)),\(Int(y)),\(Int(winW))x\(Int(winH))")
