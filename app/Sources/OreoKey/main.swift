import AppKit

let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
// Menu bar app: không icon Dock, không menu chính.
app.setActivationPolicy(.accessory)
app.run()
