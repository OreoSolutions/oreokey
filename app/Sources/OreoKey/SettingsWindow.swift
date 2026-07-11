import AppKit
import ServiceManagement
import Sparkle
import SwiftUI

final class SettingsWindowController {
    private var window: NSWindow?

    func show() {
        if window == nil {
            let win = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 760, height: 500),
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

private enum Pane: String, CaseIterable, Identifiable {
    case general, macros, apps, about
    var id: String { rawValue }

    var title: String {
        switch self {
        case .general: return "Chung"
        case .macros: return "Gõ tắt"
        case .apps: return "Ứng dụng"
        case .about: return "Giới thiệu"
        }
    }

    var symbol: String {
        switch self {
        case .general: return "keyboard.fill"
        case .macros: return "bolt.fill"
        case .apps: return "square.grid.2x2.fill"
        case .about: return "info"
        }
    }

    var color: Color {
        switch self {
        case .general: return .blue
        case .macros: return .orange
        case .apps: return .purple
        case .about: return .gray
        }
    }
}

struct SettingsView: View {
    @StateObject private var model = SettingsModel()
    @State private var pane: Pane = .general

    // Sidebar tự dựng thay vì NavigationSplitView: split view tự sinh
    // NSToolbar (nút thu sidebar + tiêu đề trồi lên titlebar) gây lệch,
    // và .toolbar(.hidden) không gỡ được khi host trong NSWindow thường.
    var body: some View {
        HStack(spacing: 0) {
            List(Pane.allCases, selection: $pane) { p in
                Label {
                    Text(p.title)
                } icon: {
                    Image(systemName: p.symbol)
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.white)
                        .frame(width: 22, height: 22)
                        .background(
                            RoundedRectangle(cornerRadius: 6).fill(p.color.gradient))
                }
                .tag(p)
            }
            .listStyle(.sidebar)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                UpdateFooter()
            }
            .frame(width: 190)

            Divider()

            Group {
                if model.settings != nil {
                    switch pane {
                    case .general: GeneralPane(model: model)
                    case .macros: MacrosPane(model: model)
                    case .apps: AppsPane(model: model)
                    case .about: AboutPane()
                    }
                } else {
                    ContentUnavailableCompat()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(minWidth: 760, minHeight: 500)
    }
}

/// Chân sidebar: tên app + dòng "v… ✓ Mới nhất" căn giữa. Cả dòng bấm
/// được để kiểm tra/cài bản mới; đổi thành cảnh báo cam khi có update
/// (dò lại im lặng mỗi lần mở cửa sổ).
private struct UpdateFooter: View {
    @ObservedObject private var status = Updater.shared.status
    @State private var hovering = false

    private var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev"
    }

    var body: some View {
        VStack(spacing: 5) {
            Text("OreoKey")
                .font(.system(size: 14, weight: .bold))
            statusLine
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 12)
        .onAppear { Updater.shared.probeQuietly() }
    }

    private var statusLine: some View {
        Button {
            Updater.shared.controller.checkForUpdates(nil)
        } label: {
            HStack(spacing: 5) {
                Text("v\(version)")
                    .foregroundStyle(.secondary)
                switch status.state {
                case .available(let newVersion):
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundStyle(.orange)
                    Text("Bản \(newVersion)")
                        .fontWeight(.semibold)
                        .foregroundStyle(.orange)
                case .upToDate:
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundStyle(.green)
                    Text("Mới nhất")
                        .foregroundStyle(.secondary)
                case .unknown:
                    Image(systemName: "arrow.triangle.2.circlepath")
                        .foregroundStyle(.secondary)
                    Text("Kiểm tra")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.caption)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(Color.primary.opacity(hovering ? 0.08 : 0)))
            .contentShape(RoundedRectangle(cornerRadius: 6))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
    }
}

private struct ContentUnavailableCompat: View {
    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle")
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            Text("Không đọc được cài đặt").foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Chung

private struct GeneralPane: View {
    @ObservedObject var model: SettingsModel
    @State private var launchAtLogin = SMAppService.mainApp.status == .enabled

    private let hotkeyPresets: [(String, CoreHotkey)] = [
        ("⌃⇧Space", CoreHotkey(ctrl: true, shift: true, alt: false, cmd: false, keycode: 49)),
        ("⌃Space", CoreHotkey(ctrl: true, shift: false, alt: false, cmd: false, keycode: 49)),
        ("⌘⇧Space", CoreHotkey(ctrl: false, shift: true, alt: false, cmd: true, keycode: 49)),
        ("⌥Z", CoreHotkey(ctrl: false, shift: false, alt: true, cmd: false, keycode: 6)),
    ]

    var body: some View {
        if let binding = Binding($model.settings) {
            Form {
                Section("Kiểu gõ") {
                    Picker("Kiểu gõ", selection: binding.method) {
                        Text("Telex").tag("telex")
                        Text("VNI").tag("vni")
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()

                    Picker("Phím tắt bật / tắt tiếng Việt", selection: hotkeyBinding(binding)) {
                        ForEach(hotkeyPresets, id: \.0) { name, _ in
                            Text(name).tag(name)
                        }
                    }
                }

                Section("Hành vi gõ") {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(spacing: 10) {
                            Text("Kiểm tra chính tả")
                            Spacer()
                            Text(spellModeName(binding.wrappedValue.spell_mode))
                                .foregroundStyle(.secondary)
                            StepSlider(index: spellIndexBinding(binding), steps: 3)
                                .frame(width: 150)
                        }
                        Text(spellModeDetail(binding.wrappedValue.spell_mode))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                    ToggleRow(
                        title: "Gõ dấu mũ linh hoạt",
                        detail: "nanag → nâng, viete → viêt",
                        isOn: binding.flexible_marks)
                    ToggleRow(
                        title: "Đặt dấu kiểu mới",
                        detail: "hoà, thuý thay vì hòa, thúy",
                        isOn: binding.modern_tone)
                    ToggleRow(
                        title: "Gõ tắt",
                        detail: "Mở rộng cụm tắt định nghĩa trong mục Gõ tắt",
                        isOn: binding.macros_enabled)
                    ToggleRow(
                        title: "Che từ tục tĩu",
                        detail: "Từ nhạy cảm tự thay bằng dấu * khi chốt từ",
                        isOn: binding.censor_enabled)
                }

                Section("Hệ thống") {
                    ToggleRow(
                        title: "Khởi động cùng máy",
                        detail: "Tự chạy OreoKey khi đăng nhập",
                        isOn: $launchAtLogin)
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Chung")
            .onChange(of: launchAtLogin) { on in
                let service = SMAppService.mainApp
                guard on != (service.status == .enabled) else { return }
                do {
                    if on {
                        try service.register()
                    } else {
                        try service.unregister()
                    }
                } catch {
                    NSLog("OreoKey: launch-at-login error: \(error)")
                    launchAtLogin = service.status == .enabled
                }
            }
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

    /// Thứ tự nấc slider: mức độ kiểm tra tăng dần — full là chặt nhất.
    private static let spellModes = ["loose", "standard", "strict"]

    private func spellIndexBinding(_ binding: Binding<CoreSettings>) -> Binding<Int> {
        Binding<Int>(
            get: {
                Self.spellModes.firstIndex(of: binding.wrappedValue.spell_mode) ?? 0
            },
            set: { index in
                let clamped = min(max(index, 0), Self.spellModes.count - 1)
                binding.wrappedValue.spell_mode = Self.spellModes[clamped]
            })
    }

    private func spellModeName(_ mode: String) -> String {
        switch mode {
        case "loose": return "Thoải mái"
        case "standard": return "Thường"
        default: return "Chặt"
        }
    }

    private func spellModeDetail(_ mode: String) -> String {
        switch mode {
        case "loose":
            return "Luôn đặt dấu, không khôi phục từ tiếng Anh."
        case "standard":
            return "Cho gõ tắt (đc, nèk), vẫn nhận diện tiếng Anh có cụm bất khả (clear, sound)."
        default:
            return "Bảo vệ tối đa từ tiếng Anh (mask, class)."
        }
    }
}

/// Slider nấc rời tự vẽ: track dày, chấm đánh dấu từng nấc, núm tròn màu
/// accent — Slider hệ thống không cho chỉnh độ dày track.
private struct StepSlider: View {
    @Binding var index: Int
    let steps: Int

    private let trackHeight: CGFloat = 7
    private let thumbSize: CGFloat = 16

    var body: some View {
        GeometryReader { geo in
            let span = geo.size.width - thumbSize
            let thumbX = span * CGFloat(index) / CGFloat(steps - 1)

            ZStack(alignment: .leading) {
                Capsule()
                    .fill(Color.primary.opacity(0.18))
                    .frame(height: trackHeight)
                Capsule()
                    .fill(Color.accentColor)
                    .frame(width: thumbX + thumbSize / 2, height: trackHeight)
                ForEach(0..<steps, id: \.self) { i in
                    Circle()
                        .fill(.white.opacity(i <= index ? 0.95 : 0.45))
                        .frame(width: 3, height: 3)
                        .offset(x: thumbSize / 2 + span * CGFloat(i) / CGFloat(steps - 1) - 1.5)
                }
                Circle()
                    .fill(Color.accentColor)
                    .frame(width: thumbSize, height: thumbSize)
                    .shadow(color: .black.opacity(0.3), radius: 1.5, y: 1)
                    .offset(x: thumbX)
            }
            .frame(maxHeight: .infinity)
            .contentShape(Rectangle())
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in
                        let t = min(max((value.location.x - thumbSize / 2) / span, 0), 1)
                        index = Int((t * CGFloat(steps - 1)).rounded())
                    })
            .animation(.easeOut(duration: 0.12), value: index)
        }
        .frame(height: 20)
    }
}

private struct ToggleRow: View {
    let title: String
    let detail: String
    let isOn: Binding<Bool>

    var body: some View {
        Toggle(isOn: isOn) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                Text(detail).font(.caption).foregroundStyle(.secondary)
            }
        }
        .toggleStyle(.switch)
        .controlSize(.small)
    }
}

// MARK: - Gõ tắt

private struct MacrosPane: View {
    @ObservedObject var model: SettingsModel
    @State private var selection: Set<String> = []
    @State private var newFrom = ""
    @State private var newTo = ""

    var body: some View {
        if let binding = Binding($model.settings) {
            VStack(spacing: 0) {
                Table(binding.wrappedValue.macros, selection: $selection) {
                    TableColumn("Gõ tắt") { m in Text(m.from).monospaced() }
                        .width(min: 80, ideal: 120, max: 180)
                    TableColumn("Thay bằng", value: \.to)
                }

                Divider()

                HStack(spacing: 8) {
                    TextField("vn", text: $newFrom)
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 110)
                        .monospaced()
                    Image(systemName: "arrow.right")
                        .foregroundStyle(.tertiary)
                    TextField("Việt Nam", text: $newTo)
                        .textFieldStyle(.roundedBorder)
                    Button {
                        let from = newFrom.trimmingCharacters(in: .whitespaces)
                        guard !from.isEmpty, !newTo.isEmpty else { return }
                        binding.wrappedValue.macros.removeAll { $0.from == from }
                        binding.wrappedValue.macros.append(CoreMacro(from: from, to: newTo))
                        newFrom = ""
                        newTo = ""
                    } label: {
                        Image(systemName: "plus")
                    }
                    .disabled(newFrom.isEmpty || newTo.isEmpty)
                    .keyboardShortcut(.defaultAction)

                    Button {
                        binding.wrappedValue.macros.removeAll { selection.contains($0.from) }
                        selection = []
                    } label: {
                        Image(systemName: "minus")
                    }
                    .disabled(selection.isEmpty)
                }
                .padding(10)
            }
            .navigationTitle("Gõ tắt")
        }
    }
}

// MARK: - Ứng dụng

private struct AppsPane: View {
    @ObservedObject var model: SettingsModel

    private let modes: [(String, String)] = [
        ("auto", "Tự động"),
        ("inject_fast", "Bơm phím nhanh"),
        ("inject_slow", "Bơm phím chậm"),
    ]

    var body: some View {
        if let binding = Binding($model.settings) {
            Form {
                Section {
                    ToggleRow(
                        title: "Nhớ trạng thái theo ứng dụng",
                        detail: "Mỗi app giữ trạng thái VN/EN riêng khi chuyển qua lại",
                        isOn: binding.remember_per_app)
                }

                Section {
                    ForEach(binding.wrappedValue.excluded_apps, id: \.self) { bundle in
                        HStack {
                            AppLabel(bundle: bundle)
                            Spacer()
                            Button {
                                binding.wrappedValue.excluded_apps.removeAll { $0 == bundle }
                            } label: {
                                Image(systemName: "minus.circle.fill")
                                    .foregroundStyle(.secondary)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    RunningAppPicker(title: "Thêm ứng dụng…") { bundle in
                        if !binding.wrappedValue.excluded_apps.contains(bundle) {
                            binding.wrappedValue.excluded_apps.append(bundle)
                        }
                    }
                } header: {
                    Text("Tắt tiếng Việt trong các ứng dụng")
                } footer: {
                    Text("Terminal, IDE... — nơi bạn không muốn gõ tiếng Việt. Hotkey vẫn bật lại tạm được.")
                }

                Section {
                    ForEach(
                        binding.wrappedValue.per_app_mode.keys.sorted(), id: \.self
                    ) { bundle in
                        HStack {
                            AppLabel(bundle: bundle)
                            Spacer()
                            Picker("", selection: modeBinding(binding, bundle)) {
                                ForEach(modes, id: \.0) { value, label in
                                    Text(label).tag(value)
                                }
                            }
                            .labelsHidden()
                            .frame(width: 160)
                            Button {
                                binding.wrappedValue.per_app_mode.removeValue(forKey: bundle)
                            } label: {
                                Image(systemName: "minus.circle.fill")
                                    .foregroundStyle(.secondary)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    RunningAppPicker(title: "Thêm override…") { bundle in
                        if binding.wrappedValue.per_app_mode[bundle] == nil {
                            binding.wrappedValue.per_app_mode[bundle] = "inject_slow"
                        }
                    }
                } header: {
                    Text("Chế độ tương thích")
                } footer: {
                    Text("Chỉnh khi một app bị dính hoặc nháy chữ.")
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Ứng dụng")
        }
    }

    private func modeBinding(
        _ binding: Binding<CoreSettings>, _ bundle: String
    ) -> Binding<String> {
        Binding<String>(
            get: { binding.wrappedValue.per_app_mode[bundle] ?? "auto" },
            set: { binding.wrappedValue.per_app_mode[bundle] = $0 })
    }
}

private struct AppLabel: View {
    let bundle: String

    var body: some View {
        let app = NSWorkspace.shared.runningApplications
            .first { $0.bundleIdentifier == bundle }
        HStack(spacing: 8) {
            if let icon = app?.icon {
                Image(nsImage: icon).resizable().frame(width: 20, height: 20)
            } else {
                Image(systemName: "app.dashed").frame(width: 20, height: 20)
                    .foregroundStyle(.secondary)
            }
            Text(app?.localizedName ?? bundle)
        }
    }
}

private struct RunningAppPicker: View {
    let title: String
    let onPick: (String) -> Void

    @State private var showingManualEntry = false
    @State private var manualBundle = ""

    var body: some View {
        Menu {
            let apps = NSWorkspace.shared.runningApplications
                .filter { $0.activationPolicy == .regular }
                .sorted { ($0.localizedName ?? "") < ($1.localizedName ?? "") }
            ForEach(apps, id: \.processIdentifier) { app in
                if let bundle = app.bundleIdentifier {
                    Button(app.localizedName ?? bundle) { onPick(bundle) }
                }
            }
            Divider()
            Button("Nhập bundle ID…") {
                manualBundle = ""
                showingManualEntry = true
            }
        } label: {
            Label(title, systemImage: "plus.circle")
        }
        .menuStyle(.borderlessButton)
        .fixedSize()
        .alert("Nhập bundle ID", isPresented: $showingManualEntry) {
            TextField("com.example.app", text: $manualBundle)
                .autocorrectionDisabled()
            Button("Thêm", action: addManual)
            Button("Huỷ", role: .cancel) {}
        } message: {
            Text("""
            Dùng cho app không đang chạy. Khớp chính xác chuỗi — không hỗ trợ \
            ký tự đại diện (com.foo.*).

            Lấy ID: mở Terminal chạy
            osascript -e 'id of app "Tên App"'
            hoặc  mdls -name kMDItemCFBundleIdentifier -raw "/Applications/Tên App.app"
            """)
        }
    }

    private func addManual() {
        let bundle = manualBundle.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !bundle.isEmpty,
              !bundle.contains(where: \.isWhitespace) else { return }
        onPick(bundle)
    }
}

// MARK: - Chuyển mã

// MARK: - Giới thiệu

private struct AboutPane: View {
    /// Trang chủ chính thức của app.
    private static let websiteURL = URL(string: "https://oreokey.vercel.app")!
    /// Trang chủ mã nguồn — kèm docs hướng dẫn cài đặt/sử dụng.
    private static let githubURL = URL(string: "https://github.com/OreoSolutions/oreokey")!
    /// Trang Issues để người dùng báo lỗi.
    private static let bugReportURL = URL(string: "https://github.com/OreoSolutions/oreokey/issues")!
    /// Kênh ủng hộ tác giả (Ko-fi).
    private static let sponsorsURL = URL(string: "https://ko-fi.com/nguyenhuyquang")!

    private var version: String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev"
    }

    var body: some View {
        VStack(spacing: 14) {
            Spacer()
            Image(nsImage: NSApp.applicationIconImage ?? NSImage())
                .resizable()
                .frame(width: 110, height: 110)
                .shadow(color: .black.opacity(0.18), radius: 10, y: 5)
            Text("OreoKey").font(.title.bold())
            Text("Bộ gõ tiếng Việt cho macOS — nhanh, nhẹ, không dính chữ")
                .foregroundStyle(.secondary)
            Text("Phiên bản \(version)")
                .font(.caption)
                .foregroundStyle(.tertiary)
                .padding(.top, 2)

            HStack(spacing: 10) {
                Link(destination: Self.websiteURL) {
                    Label("Trang chủ", systemImage: "globe")
                }
                .buttonStyle(.bordered)
                Link(destination: Self.githubURL) {
                    Label("GitHub", systemImage: "chevron.left.forwardslash.chevron.right")
                }
                .buttonStyle(.bordered)
                Link(destination: Self.bugReportURL) {
                    Label("Báo lỗi", systemImage: "ladybug.fill")
                }
                .buttonStyle(.bordered)
                Link(destination: Self.sponsorsURL) {
                    Label("Ủng hộ", systemImage: "heart.fill")
                }
                .buttonStyle(.borderedProminent)
                .tint(.pink)
            }
            .padding(.top, 6)

            Spacer()
            Text("© 2026 Oreo Solutions")
                .font(.caption2)
                .foregroundStyle(.quaternary)
                .padding(.bottom, 16)
        }
        .frame(maxWidth: .infinity)
        .navigationTitle("Giới thiệu")
    }
}
