import Carbon

/// Khi bật tiếng Việt của OreoKey, input source hệ thống phải là bàn
/// phím Latin thuần (ABC/US...) — nếu đang đứng ở một input method khác
/// (Tiếng Việt có sẵn của macOS, tiếng Trung/Nhật...) thì hai bộ gõ sẽ
/// xử lý chồng nhau. Layout Latin của người dùng (US, Colemak...) được
/// giữ nguyên; chỉ input method mới bị chuyển về ABC.
enum InputSource {
    /// Input source hiện tại là một input method (Nhật/Hàn/Trung, Tiếng Việt
    /// có sẵn của macOS…) chứ không phải keylayout tĩnh?
    static var currentIsInputMethod: Bool {
        guard let current = TISCopyCurrentKeyboardInputSource()?.takeRetainedValue(),
              let ptr = TISGetInputSourceProperty(current, kTISPropertyInputSourceID)
        else { return false }
        let id = Unmanaged<CFString>.fromOpaque(ptr).takeUnretainedValue() as String
        return !id.hasPrefix("com.apple.keylayout.")
    }

    /// Treo tiếng Việt khi người dùng chuyển sang input method khác, gỡ treo
    /// khi quay về keylayout. Gọi một lần lúc khởi động.
    static func watchForOtherInputMethods() {
        let sync = { Core.setSuspended(currentIsInputMethod) }
        DistributedNotificationCenter.default().addObserver(
            forName: Notification.Name(kTISNotifySelectedKeyboardInputSourceChanged as String),
            object: nil, queue: .main
        ) { _ in sync() }
        sync()
    }

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
                as? [TISInputSource]
        else { return }
        // ABC có thể chưa bật (người dùng chỉ có U.S., Colemak…): dùng bàn
        // phím ASCII-capable hiện có thay vì để IME chạy chồng.
        guard let target = list.first
            ?? TISCopyCurrentASCIICapableKeyboardLayoutInputSource()?.takeRetainedValue()
        else { return }
        TISSelectInputSource(target)
    }
}
