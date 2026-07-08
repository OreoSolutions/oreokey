import AppKit
import ServiceManagement

// Callback C không capture được context — trỏ về singleton.
private func statusChanged(_ vnOn: Bool) {
    DispatchQueue.main.async {
        AppDelegate.instance?.updateIcon(vnOn: vnOn)
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate {
    static var instance: AppDelegate?

    private var statusItem: NSStatusItem!
    private var toggleItem: NSMenuItem!
    private var telexItem: NSMenuItem!
    private var vniItem: NSMenuItem!
    private var loginItem: NSMenuItem!
    private var onboarding: OnboardingController?
    private var settingsWindow: SettingsWindowController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        AppDelegate.instance = self
        setupStatusItem()
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
            updateIcon(vnOn: Core.vnEnabled())
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

        toggleItem = NSMenuItem(
            title: "Tiếng Việt", action: #selector(toggleVietnamese), keyEquivalent: "")
        toggleItem.target = self
        menu.addItem(toggleItem)
        menu.addItem(.separator())

        telexItem = NSMenuItem(title: "Kiểu gõ Telex", action: #selector(useTelex), keyEquivalent: "")
        telexItem.target = self
        menu.addItem(telexItem)
        vniItem = NSMenuItem(title: "Kiểu gõ VNI", action: #selector(useVni), keyEquivalent: "")
        vniItem.target = self
        menu.addItem(vniItem)
        menu.addItem(.separator())

        let settings = NSMenuItem(title: "Cài đặt…", action: #selector(openSettings), keyEquivalent: ",")
        settings.target = self
        menu.addItem(settings)

        loginItem = NSMenuItem(
            title: "Khởi động cùng máy", action: #selector(toggleLaunchAtLogin), keyEquivalent: "")
        loginItem.target = self
        menu.addItem(loginItem)
        menu.addItem(.separator())

        let quit = NSMenuItem(title: "Thoát OreoKey", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        menu.addItem(quit)

        menu.delegate = self
        statusItem.menu = menu
        updateIcon(vnOn: Core.vnEnabled())
    }

    func updateIcon(vnOn: Bool) {
        guard let button = statusItem.button else { return }
        button.title = vnOn ? "VN" : "EN"
        button.font = NSFont.systemFont(ofSize: 12, weight: .bold)
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

    @objc private func toggleLaunchAtLogin() {
        let service = SMAppService.mainApp
        do {
            if service.status == .enabled {
                try service.unregister()
            } else {
                try service.register()
            }
        } catch {
            NSLog("OreoKey: launch-at-login error: \(error)")
        }
    }
}

extension AppDelegate: NSMenuDelegate {
    // Cập nhật checkmark theo trạng thái thật mỗi lần mở menu.
    func menuNeedsUpdate(_ menu: NSMenu) {
        toggleItem.state = Core.vnEnabled() ? .on : .off
        let method = Core.loadSettings()?.method ?? "telex"
        telexItem.state = method == "telex" ? .on : .off
        vniItem.state = method == "vni" ? .on : .off
        loginItem.state = SMAppService.mainApp.status == .enabled ? .on : .off
    }
}
