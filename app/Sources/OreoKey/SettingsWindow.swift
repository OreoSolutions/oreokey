import AppKit
import SwiftUI

final class SettingsWindowController {
    private var window: NSWindow?

    func show() {
        if window == nil {
            let win = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 560, height: 440),
                styleMask: [.titled, .closable, .miniaturizable],
                backing: .buffered, defer: false)
            win.title = "Cài đặt OreoKey"
            win.isReleasedWhenClosed = false
            win.contentViewController = NSHostingController(rootView: SettingsView())
            window = win
        }
        NSApp.activate(ignoringOtherApps: true)
        window?.center()
        window?.makeKeyAndOrderFront(nil)
    }
}

// Nguồn sự thật là Rust: đọc khi mở, ghi ngay mỗi lần đổi.
final class SettingsModel: ObservableObject {
    @Published var settings: CoreSettings? {
        didSet {
            guard let s = settings, s != oldValue, oldValue != nil else { return }
            Core.save(s)
        }
    }

    init() {
        settings = Core.loadSettings()
    }
}

struct SettingsView: View {
    @StateObject private var model = SettingsModel()

    var body: some View {
        if model.settings != nil {
            TabView {
                GeneralTab(model: model)
                    .tabItem { Label("Chung", systemImage: "keyboard") }
                MacrosTab(model: model)
                    .tabItem { Label("Gõ tắt", systemImage: "text.badge.plus") }
                AppsTab(model: model)
                    .tabItem { Label("Ứng dụng", systemImage: "app.badge") }
                ToolsTab()
                    .tabItem { Label("Công cụ", systemImage: "arrow.left.arrow.right") }
            }
            .padding()
            .frame(width: 560, height: 440)
        } else {
            Text("Không đọc được cài đặt").padding()
        }
    }
}

// MARK: - Tab Chung

private struct GeneralTab: View {
    @ObservedObject var model: SettingsModel

    private let hotkeyPresets: [(String, CoreHotkey)] = [
        ("⌃⇧Space", CoreHotkey(ctrl: true, shift: true, alt: false, cmd: false, keycode: 49)),
        ("⌃Space", CoreHotkey(ctrl: true, shift: false, alt: false, cmd: false, keycode: 49)),
        ("⌘⇧Space", CoreHotkey(ctrl: false, shift: true, alt: false, cmd: true, keycode: 49)),
        ("⌥Z", CoreHotkey(ctrl: false, shift: false, alt: true, cmd: false, keycode: 6)),
    ]

    var body: some View {
        if let binding = Binding($model.settings) {
            Form {
                Picker("Kiểu gõ:", selection: binding.method) {
                    Text("Telex").tag("telex")
                    Text("VNI").tag("vni")
                }
                .pickerStyle(.radioGroup)

                Picker("Phím tắt bật/tắt:", selection: hotkeyBinding(binding)) {
                    ForEach(hotkeyPresets, id: \.0) { name, _ in
                        Text(name).tag(name)
                    }
                }
                .frame(maxWidth: 260)

                Toggle("Kiểm tra chính tả (khôi phục từ không phải tiếng Việt)",
                       isOn: binding.spell_check)
                Toggle("Đặt dấu kiểu mới (hoà thay vì hòa)", isOn: binding.modern_tone)
                Toggle("Bật gõ tắt", isOn: binding.macros_enabled)
            }
            .padding()
            Spacer()
        }
    }

    private func hotkeyBinding(_ binding: Binding<CoreSettings>) -> Binding<String> {
        Binding<String>(
            get: {
                hotkeyPresets.first { $0.1 == binding.wrappedValue.hotkey }?.0
                    ?? hotkeyPresets[0].0
            },
            set: { name in
                if let preset = hotkeyPresets.first(where: { $0.0 == name }) {
                    binding.wrappedValue.hotkey = preset.1
                }
            })
    }
}

// MARK: - Tab Gõ tắt

private struct MacrosTab: View {
    @ObservedObject var model: SettingsModel
    @State private var newFrom = ""
    @State private var newTo = ""

    var body: some View {
        if let binding = Binding($model.settings) {
            VStack(alignment: .leading, spacing: 8) {
                Table(binding.wrappedValue.macros) {
                    TableColumn("Gõ tắt", value: \.from)
                    TableColumn("Thay bằng", value: \.to)
                }

                HStack {
                    TextField("Cụm tắt (vd: vn)", text: $newFrom)
                        .frame(width: 140)
                    TextField("Nội dung thay thế", text: $newTo)
                    Button("Thêm") {
                        let from = newFrom.trimmingCharacters(in: .whitespaces)
                        guard !from.isEmpty, !newTo.isEmpty else { return }
                        binding.wrappedValue.macros.removeAll { $0.from == from }
                        binding.wrappedValue.macros.append(CoreMacro(from: from, to: newTo))
                        newFrom = ""
                        newTo = ""
                    }
                    .disabled(newFrom.isEmpty || newTo.isEmpty)
                }
                HStack {
                    Text("Chọn dòng muốn xóa rồi bấm:")
                        .foregroundStyle(.secondary)
                    MacroDeleteMenu(binding: binding)
                }
            }
            .padding()
        }
    }
}

private struct MacroDeleteMenu: View {
    let binding: Binding<CoreSettings>

    var body: some View {
        Menu("Xóa gõ tắt…") {
            ForEach(binding.wrappedValue.macros) { m in
                Button("\(m.from) → \(m.to)") {
                    binding.wrappedValue.macros.removeAll { $0.from == m.from }
                }
            }
        }
        .frame(width: 160)
        .disabled(binding.wrappedValue.macros.isEmpty)
    }
}

// MARK: - Tab Ứng dụng

private struct AppsTab: View {
    @ObservedObject var model: SettingsModel
    @State private var selectedBundle = ""

    private let modes: [(String, String)] = [
        ("auto", "Tự động (AX → bơm phím)"),
        ("inject_fast", "Bơm phím nhanh"),
        ("inject_slow", "Bơm phím chậm (app hay dính chữ)"),
    ]

    var body: some View {
        if let binding = Binding($model.settings) {
            VStack(alignment: .leading, spacing: 12) {
                Toggle("Nhớ trạng thái VN/EN riêng cho từng ứng dụng",
                       isOn: binding.remember_per_app)

                GroupBox("Tắt tiếng Việt trong các ứng dụng này") {
                    VStack(alignment: .leading) {
                        ForEach(binding.wrappedValue.excluded_apps, id: \.self) { bundle in
                            HStack {
                                Text(appName(bundle))
                                Spacer()
                                Button("Bỏ") {
                                    binding.wrappedValue.excluded_apps
                                        .removeAll { $0 == bundle }
                                }
                            }
                        }
                        RunningAppPicker(title: "Thêm ứng dụng đang chạy…") { bundle in
                            if !binding.wrappedValue.excluded_apps.contains(bundle) {
                                binding.wrappedValue.excluded_apps.append(bundle)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(6)
                }

                GroupBox("Chế độ tương thích (khi một app bị dính/nháy chữ)") {
                    VStack(alignment: .leading) {
                        ForEach(
                            binding.wrappedValue.per_app_mode.keys.sorted(), id: \.self
                        ) { bundle in
                            HStack {
                                Text(appName(bundle)).frame(width: 160, alignment: .leading)
                                Picker("", selection: modeBinding(binding, bundle)) {
                                    ForEach(modes, id: \.0) { value, label in
                                        Text(label).tag(value)
                                    }
                                }
                                .labelsHidden()
                                Button("Bỏ") {
                                    binding.wrappedValue.per_app_mode
                                        .removeValue(forKey: bundle)
                                }
                            }
                        }
                        RunningAppPicker(title: "Thêm override cho app đang chạy…") { bundle in
                            if binding.wrappedValue.per_app_mode[bundle] == nil {
                                binding.wrappedValue.per_app_mode[bundle] = "inject_slow"
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(6)
                }
                Spacer()
            }
            .padding()
        }
    }

    private func modeBinding(
        _ binding: Binding<CoreSettings>, _ bundle: String
    ) -> Binding<String> {
        Binding<String>(
            get: { binding.wrappedValue.per_app_mode[bundle] ?? "auto" },
            set: { binding.wrappedValue.per_app_mode[bundle] = $0 })
    }

    private func appName(_ bundle: String) -> String {
        NSWorkspace.shared.runningApplications
            .first { $0.bundleIdentifier == bundle }?.localizedName ?? bundle
    }
}

private struct RunningAppPicker: View {
    let title: String
    let onPick: (String) -> Void

    var body: some View {
        Menu(title) {
            let apps = NSWorkspace.shared.runningApplications
                .filter { $0.activationPolicy == .regular }
                .sorted { ($0.localizedName ?? "") < ($1.localizedName ?? "") }
            ForEach(apps, id: \.processIdentifier) { app in
                if let bundle = app.bundleIdentifier {
                    Button(app.localizedName ?? bundle) { onPick(bundle) }
                }
            }
        }
        .frame(maxWidth: 280)
    }
}

// MARK: - Tab Công cụ (chuyển mã)

private struct ToolsTab: View {
    @State private var input = ""
    @State private var output = ""
    @State private var fromEnc: Int32 = 0
    @State private var toEnc: Int32 = 1

    private let encodings: [(Int32, String)] = [
        (0, "Unicode"), (1, "VNI-Windows"), (2, "TCVN3 (ABC)"),
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Picker("Từ:", selection: $fromEnc) {
                    ForEach(encodings, id: \.0) { v, name in Text(name).tag(v) }
                }
                Picker("Sang:", selection: $toEnc) {
                    ForEach(encodings, id: \.0) { v, name in Text(name).tag(v) }
                }
                Button("Chuyển mã") {
                    output = Core.convert(input, from: fromEnc, to: toEnc)
                }
                .disabled(fromEnc == toEnc || input.isEmpty)
            }
            Text("Văn bản gốc:").foregroundStyle(.secondary)
            TextEditor(text: $input)
                .font(.body)
                .border(Color.gray.opacity(0.3))
            Text("Kết quả:").foregroundStyle(.secondary)
            TextEditor(text: $output)
                .font(.body)
                .border(Color.gray.opacity(0.3))
            HStack {
                Spacer()
                Button("Chép kết quả") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(output, forType: .string)
                }
                .disabled(output.isEmpty)
            }
        }
        .padding()
    }
}
