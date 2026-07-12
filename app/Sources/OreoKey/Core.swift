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

extension CoreHotkey {
    /// Tên phím theo virtual keycode ANSI của macOS (bàn phím US).
    private static let keyNames: [UInt16: String] = [
        0: "A", 1: "S", 2: "D", 3: "F", 4: "H", 5: "G", 6: "Z", 7: "X",
        8: "C", 9: "V", 11: "B", 12: "Q", 13: "W", 14: "E", 15: "R",
        16: "Y", 17: "T", 18: "1", 19: "2", 20: "3", 21: "4", 22: "6",
        23: "5", 24: "=", 25: "9", 26: "7", 27: "-", 28: "8", 29: "0",
        30: "]", 31: "O", 32: "U", 33: "[", 34: "I", 35: "P", 37: "L",
        38: "J", 39: "'", 40: "K", 41: ";", 42: "\\", 43: ",", 44: "/",
        45: "N", 46: "M", 47: ".", 50: "`",
        36: "↩", 48: "Tab", 49: "Space", 51: "⌫", 53: "⎋", 76: "⌤",
        96: "F5", 97: "F6", 98: "F7", 99: "F3", 100: "F8", 101: "F9",
        103: "F11", 109: "F10", 111: "F12", 118: "F4", 120: "F2", 122: "F1",
        115: "↖", 116: "⇞", 117: "⌦", 119: "↘", 121: "⇟",
        123: "←", 124: "→", 125: "↓", 126: "↑",
    ]

    /// Chuỗi hiển thị dạng "⌃⌥Space" — dùng chung cho menu bar và Cài đặt.
    var display: String {
        var parts = ""
        if ctrl { parts += "⌃" }
        if alt { parts += "⌥" }
        if shift { parts += "⇧" }
        if cmd { parts += "⌘" }
        if let code = keycode {
            parts += Self.keyNames[code] ?? "#\(code)"
        }
        return parts
    }
}

struct CoreMacro: Codable, Equatable, Identifiable {
    var from: String
    var to: String
    var id: String { from }
}

struct CoreSettings: Codable, Equatable {
    var method: String
    var enabled: Bool
    var spell_mode: String
    var modern_tone: Bool
    var macros_enabled: Bool
    var flexible_marks: Bool
    var censor_enabled: Bool
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

    static func setStatusCallback(_ cb: @escaping @convention(c) (Bool) -> Void) {
        ok_set_status_callback(cb)
    }
}
