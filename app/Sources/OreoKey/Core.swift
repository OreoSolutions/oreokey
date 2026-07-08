// Wrapper Swift quanh C ABI của oreokey-core. Rust là chủ sở hữu config;
// Swift chỉ đọc/ghi qua JSON.

import Foundation
import COreoKey

struct CoreHotkey: Codable, Equatable {
    var ctrl: Bool
    var shift: Bool
    var alt: Bool
    var cmd: Bool
    var keycode: UInt16?
}

struct CoreMacro: Codable, Equatable, Identifiable {
    var from: String
    var to: String
    var id: String { from }
}

struct CoreSettings: Codable, Equatable {
    var method: String
    var enabled: Bool
    var spell_check: Bool
    var modern_tone: Bool
    var macros_enabled: Bool
    var flexible_marks: Bool
    var hotkey: CoreHotkey
    var macros: [CoreMacro]
    var excluded_apps: [String]
    var per_app_mode: [String: String]
    var remember_per_app: Bool
}

enum Core {
    static func start() -> Bool { ok_start() }
    static func stop() { ok_stop() }
    static func axTrusted() -> Bool { ok_ax_trusted() }
    static func vnEnabled() -> Bool { ok_get_enabled() }
    static func setVnEnabled(_ on: Bool) { ok_set_enabled(on) }

    static func loadSettings() -> CoreSettings? {
        guard let raw = ok_settings_json_get() else { return nil }
        defer { ok_str_free(raw) }
        let json = String(cString: raw)
        return try? JSONDecoder().decode(CoreSettings.self, from: Data(json.utf8))
    }

    @discardableResult
    static func save(_ settings: CoreSettings) -> Bool {
        guard let data = try? JSONEncoder().encode(settings),
              let json = String(data: data, encoding: .utf8) else { return false }
        return json.withCString { ok_settings_json_set($0) }
    }

    static func notifyFrontmostApp(_ bundleId: String) {
        bundleId.withCString { ok_notify_frontmost_app($0) }
    }

    /// 0 = Unicode, 1 = VNI-Windows, 2 = TCVN3.
    static func convert(_ text: String, from: Int32, to: Int32) -> String {
        text.withCString { c in
            guard let raw = ok_convert(c, from, to) else { return text }
            defer { ok_str_free(raw) }
            return String(cString: raw)
        }
    }

    static func setStatusCallback(_ cb: @escaping @convention(c) (Bool) -> Void) {
        ok_set_status_callback(cb)
    }
}
