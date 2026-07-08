import Carbon

/// Khi bật tiếng Việt của OreoKey, input source hệ thống phải là bàn
/// phím Latin thuần (ABC/US...) — nếu đang đứng ở một input method khác
/// (Tiếng Việt có sẵn của macOS, tiếng Trung/Nhật...) thì hai bộ gõ sẽ
/// xử lý chồng nhau. Layout Latin của người dùng (US, Colemak...) được
/// giữ nguyên; chỉ input method mới bị chuyển về ABC.
enum InputSource {
    static func ensureLatin() {
        guard let current = TISCopyCurrentKeyboardInputSource()?.takeRetainedValue()
        else { return }
        if let ptr = TISGetInputSourceProperty(current, kTISPropertyInputSourceID) {
            let id = Unmanaged<CFString>.fromOpaque(ptr).takeUnretainedValue() as String
            // Keylayout = bàn phím tĩnh, không phải IME → không đụng vào.
            if id.hasPrefix("com.apple.keylayout.") { return }
        }
        let filter =
            [kTISPropertyInputSourceID as String: "com.apple.keylayout.ABC"] as CFDictionary
        guard
            let list = TISCreateInputSourceList(filter, false)?.takeRetainedValue()
                as? [TISInputSource],
            let abc = list.first
        else { return }
        TISSelectInputSource(abc)
    }
}
