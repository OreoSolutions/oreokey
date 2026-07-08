import AppKit
import SwiftUI

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
            let win = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 440, height: 520),
                styleMask: [.titled, .closable, .fullSizeContentView],
                backing: .buffered, defer: false)
            win.titlebarAppearsTransparent = true
            win.titleVisibility = .hidden
            win.isMovableByWindowBackground = true
            win.isReleasedWhenClosed = false
            win.contentViewController = NSHostingController(rootView: OnboardingView())
            window = win
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
}

private struct OnboardingView: View {
    @State private var showTrouble = false

    var body: some View {
        VStack(spacing: 0) {
            // Logo + tiêu đề
            VStack(spacing: 12) {
                Image(nsImage: NSApp.applicationIconImage ?? NSImage())
                    .resizable()
                    .frame(width: 88, height: 88)
                    .shadow(color: .black.opacity(0.2), radius: 8, y: 4)

                Text("Chào mừng đến với OreoKey")
                    .font(.title2.bold())

                Text("Bộ gõ tiếng Việt nhanh, nhẹ, không dính chữ")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }
            .padding(.top, 28)
            .padding(.bottom, 24)

            // Các bước
            VStack(alignment: .leading, spacing: 14) {
                StepRow(
                    number: "1", symbol: "hand.tap",
                    text: "Bấm nút bên dưới để mở System Settings")
                StepRow(
                    number: "2", symbol: "switch.2",
                    text: "Bật công tắc cho OreoKey trong danh sách Accessibility")
                StepRow(
                    number: "3", symbol: "checkmark.seal",
                    text: "Cửa sổ này tự đóng — gõ tiếng Việt được ngay")
            }
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(Color.primary.opacity(0.05)))
            .padding(.horizontal, 24)

            // Trạng thái chờ
            HStack(spacing: 8) {
                ProgressView().controlSize(.small)
                Text("Đang chờ quyền Trợ năng…")
                    .font(.footnote)
                    .foregroundStyle(.secondary)
            }
            .padding(.vertical, 14)

            // Nút chính
            Button {
                let url = URL(string:
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")!
                NSWorkspace.shared.open(url)
            } label: {
                Label("Mở System Settings", systemImage: "gearshape.fill")
                    .font(.body.weight(.semibold))
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 6)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)
            .keyboardShortcut(.defaultAction)
            .padding(.horizontal, 24)

            // Mẹo xử lý sự cố (thu gọn)
            DisclosureGroup(isExpanded: $showTrouble) {
                Text("""
                Thường gặp sau khi cập nhật app: macOS giữ quyền của bản cũ. \
                Trong System Settings → Accessibility, tắt công tắc OreoKey \
                rồi bật lại; hoặc chọn OreoKey, bấm nút − để xóa rồi thêm lại.
                """)
                .font(.footnote)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.top, 6)
            } label: {
                Text("Công tắc đã bật mà chưa được?")
                    .font(.footnote.weight(.medium))
                    .foregroundStyle(.secondary)
            }
            .padding(.horizontal, 24)
            .padding(.top, 14)
            .padding(.bottom, 20)
        }
        .frame(width: 440)
    }
}

private struct StepRow: View {
    let number: String
    let symbol: String
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            ZStack {
                Circle()
                    .fill(Color.accentColor.opacity(0.15))
                    .frame(width: 34, height: 34)
                Image(systemName: symbol)
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(Color.accentColor)
            }
            VStack(alignment: .leading, spacing: 1) {
                Text("Bước \(number)")
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.tertiary)
                Text(text)
                    .font(.callout)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }
}
