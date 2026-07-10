import AppKit

// Callback C không capture được context — trỏ về singleton.
private func statusChanged(_ vnOn: Bool) {
    DispatchQueue.main.async {
        AppDelegate.instance?.syncUI(vnOn: vnOn)
        // Bật tiếng Việt → input source hệ thống phải là bàn phím Latin,
        // tránh hai bộ gõ xử lý chồng nhau.
        if vnOn {
            InputSource.ensureLatin()
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    static var instance: AppDelegate?

    private var statusItem: NSStatusItem!
    private var toggleItem: NSMenuItem!
    private var telexItem: NSMenuItem!
    private var vniItem: NSMenuItem!
    private var onboarding: OnboardingController?
    private var settingsWindow: SettingsWindowController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        AppDelegate.instance = self
        setupStatusItem()
        _ = Updater.shared  // bắt đầu kiểm tra cập nhật nền
        Core.setStatusCallback(statusChanged)

        // Báo app frontmost cho core (smart switch / loại trừ app).
        let center = NSWorkspace.shared.notificationCenter
        center.addObserver(
            forName: NSWorkspace.didActivateApplicationNotification,
            object: nil, queue: .main
        ) { note in
            let app = note.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication
            Core.notifyFrontmostApp(app?.bundleIdentifier ?? "")
        }
        if let front = NSWorkspace.shared.frontmostApplication?.bundleIdentifier {
            Core.notifyFrontmostApp(front)
        }

        startOrOnboard()
    }

    func applicationWillTerminate(_ notification: Notification) {
        Core.stop()
    }

    // MARK: - Khởi động tap / onboarding

    func startOrOnboard() {
        if Core.axTrusted(), Core.start() {
            let vnOn = Core.vnEnabled()
            updateIcon(vnOn: vnOn)
            if vnOn {
                InputSource.ensureLatin()
            }
        } else {
            showOnboarding()
        }
    }

    private func showOnboarding() {
        if onboarding == nil {
            onboarding = OnboardingController { [weak self] in
                self?.onboarding = nil
                self?.startOrOnboard()
            }
        }
        onboarding?.show()
    }

    // MARK: - Menu bar

    private func setupStatusItem() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        let menu = NSMenu()

        // Mọi item cùng một kiểu: chỉ dùng cột tick hệ thống, không icon,
        // không thụt lề — tick thẳng hàng tuyệt đối.
        toggleItem = NSMenuItem(
            title: "Tiếng Việt", action: #selector(toggleVietnamese), keyEquivalent: "")
        toggleItem.target = self
        menu.addItem(toggleItem)
        menu.addItem(.separator())

        menu.addItem(sectionHeader("KIỂU GÕ"))
        telexItem = NSMenuItem(title: "Telex", action: #selector(useTelex), keyEquivalent: "")
        telexItem.target = self
        menu.addItem(telexItem)
        vniItem = NSMenuItem(title: "VNI", action: #selector(useVni), keyEquivalent: "")
        vniItem.target = self
        menu.addItem(vniItem)
        menu.addItem(.separator())

        let settings = NSMenuItem(
            title: "Cài đặt…", action: #selector(openSettings), keyEquivalent: ",")
        settings.target = self
        menu.addItem(settings)
        menu.addItem(.separator())

        let quit = NSMenuItem(
            title: "Thoát OreoKey", action: #selector(NSApplication.terminate(_:)),
            keyEquivalent: "q")
        menu.addItem(quit)

        menu.delegate = self
        statusItem.menu = menu
        updateIcon(vnOn: Core.vnEnabled())
    }

    private func sectionHeader(_ title: String) -> NSMenuItem {
        if #available(macOS 14.0, *) {
            return NSMenuItem.sectionHeader(title: title)
        }
        let item = NSMenuItem(title: title, action: nil, keyEquivalent: "")
        item.isEnabled = false
        item.attributedTitle = NSAttributedString(
            string: title,
            attributes: [
                .font: NSFont.systemFont(ofSize: 11, weight: .semibold),
                .foregroundColor: NSColor.secondaryLabelColor,
            ])
        return item
    }

    /// Cập nhật MỌI nơi hiển thị trạng thái (icon + tick) — kể cả khi
    /// menu đang mở, để hotkey bấm lúc popup hiện vẫn thấy tick đổi.
    func syncUI(vnOn: Bool) {
        updateIcon(vnOn: vnOn)
        toggleItem?.state = vnOn ? .on : .off
    }

    func updateIcon(vnOn: Bool) {
        guard let button = statusItem.button else { return }
        button.title = ""
        button.image = Self.badge(text: vnOn ? "VN" : "EN", filled: vnOn)
        button.toolTip = vnOn
            ? "OreoKey: đang gõ tiếng Việt" : "OreoKey: đang tắt tiếng Việt"
    }

    /// Icon menu bar dạng huy hiệu bo góc: đặc khi bật tiếng Việt, viền
    /// mảnh khi tắt. Template image để tự đổi màu theo sáng/tối.
    private static func badge(text: String, filled: Bool) -> NSImage {
        let size = NSSize(width: 26, height: 16)
        let image = NSImage(size: size, flipped: false) { rect in
            let path = NSBezierPath(roundedRect: rect.insetBy(dx: 0.5, dy: 0.5),
                                    xRadius: 4.5, yRadius: 4.5)
            NSColor.black.setStroke()
            NSColor.black.setFill()

            let font = NSFont.systemFont(ofSize: 10.5, weight: .bold)
            let attrs: [NSAttributedString.Key: Any] = [
                .font: font, .foregroundColor: NSColor.black,
            ]
            let str = NSAttributedString(string: text, attributes: attrs)
            let textSize = str.size()
            let textRect = NSRect(
                x: (rect.width - textSize.width) / 2,
                y: (rect.height - textSize.height) / 2 - 0.5,
                width: textSize.width, height: textSize.height)

            if filled {
                // Nền đặc, chữ đục lỗ — nổi bật khi đang bật tiếng Việt.
                path.fill()
                if let ctx = NSGraphicsContext.current?.cgContext {
                    ctx.saveGState()
                    ctx.setBlendMode(.destinationOut)
                    str.draw(in: textRect.offsetBy(dx: 0, dy: 0.5))
                    ctx.restoreGState()
                }
            } else {
                path.lineWidth = 1
                path.stroke()
                str.draw(in: textRect.offsetBy(dx: 0, dy: 0.5))
            }
            return true
        }
        image.isTemplate = true
        return image
    }

    @objc private func toggleVietnamese() {
        Core.setVnEnabled(!Core.vnEnabled())
    }

    @objc private func useTelex() { setMethod("telex") }
    @objc private func useVni() { setMethod("vni") }

    private func setMethod(_ method: String) {
        guard var s = Core.loadSettings() else { return }
        s.method = method
        Core.save(s)
    }

    @objc func openSettings() {
        if settingsWindow == nil {
            settingsWindow = SettingsWindowController()
        }
        settingsWindow?.show()
    }
}

extension AppDelegate: NSMenuDelegate {
    // Cập nhật checkmark theo trạng thái thật mỗi lần mở menu — đồng
    // thời đồng bộ lại icon từ cùng một lần đọc, để tick và icon không
    // bao giờ nói hai điều khác nhau.
    func menuNeedsUpdate(_ menu: NSMenu) {
        syncUI(vnOn: Core.vnEnabled())
        let settings = Core.loadSettings()
        let method = settings?.method ?? "telex"
        telexItem.state = method == "telex" ? .on : .off
        vniItem.state = method == "vni" ? .on : .off
        if #available(macOS 15.0, *), let hotkey = settings?.hotkey {
            toggleItem.subtitle = Self.hotkeyDisplay(hotkey)
        }
    }

    private static func hotkeyDisplay(_ hk: CoreHotkey) -> String {
        var parts = ""
        if hk.ctrl { parts += "⌃" }
        if hk.alt { parts += "⌥" }
        if hk.shift { parts += "⇧" }
        if hk.cmd { parts += "⌘" }
        let keyNames: [UInt16: String] = [49: "Space", 6: "Z", 48: "Tab"]
        if let code = hk.keycode {
            parts += keyNames[code] ?? "#\(code)"
        }
        return parts
    }
}
