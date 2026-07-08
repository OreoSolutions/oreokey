import AppKit

/// Cửa sổ hướng dẫn cấp quyền Accessibility lần đầu. Tự phát hiện khi
/// quyền đã được cấp (poll mỗi giây) rồi gọi `onGranted`.
final class OnboardingController {
    private var window: NSWindow?
    private var timer: Timer?
    private let onGranted: () -> Void

    init(onGranted: @escaping () -> Void) {
        self.onGranted = onGranted
    }

    func show() {
        if window == nil {
            window = makeWindow()
        }
        NSApp.activate(ignoringOtherApps: true)
        window?.center()
        window?.makeKeyAndOrderFront(nil)
        startPolling()
    }

    private func startPolling() {
        timer?.invalidate()
        timer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            guard let self, Core.axTrusted() else { return }
            self.timer?.invalidate()
            self.timer = nil
            self.window?.close()
            self.window = nil
            self.onGranted()
        }
    }

    private func makeWindow() -> NSWindow {
        let win = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 460, height: 260),
            styleMask: [.titled, .closable],
            backing: .buffered, defer: false)
        win.title = "Chào mừng đến với OreoKey"
        win.isReleasedWhenClosed = false

        let stack = NSStackView()
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 12
        stack.edgeInsets = NSEdgeInsets(top: 20, left: 20, bottom: 20, right: 20)

        let title = NSTextField(labelWithString: "OreoKey cần quyền Trợ năng (Accessibility)")
        title.font = .boldSystemFont(ofSize: 16)

        let body = NSTextField(wrappingLabelWithString: """
        Để gõ được tiếng Việt trong mọi ứng dụng, OreoKey cần quyền theo dõi \
        bàn phím của macOS:

        1. Bấm nút bên dưới để mở System Settings
        2. Tìm OreoKey trong danh sách và bật công tắc
        3. Cửa sổ này sẽ tự đóng khi hoàn tất
        """)
        body.preferredMaxLayoutWidth = 420

        let button = NSButton(
            title: "Mở System Settings → Privacy → Accessibility",
            target: self, action: #selector(openSettings))
        button.bezelStyle = .rounded
        button.keyEquivalent = "\r"

        stack.addArrangedSubview(title)
        stack.addArrangedSubview(body)
        stack.addArrangedSubview(button)
        win.contentView = stack
        return win
    }

    @objc private func openSettings() {
        let url = URL(
            string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!
        NSWorkspace.shared.open(url)
    }
}
