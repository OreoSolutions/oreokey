import Sparkle

/// Bọc bộ cập nhật Sparkle. Khởi tạo `shared` là bắt đầu kiểm tra nền
/// theo cấu hình Info.plist (SUFeedURL, SUEnableAutomaticChecks, interval).
final class Updater {
    static let shared = Updater()

    let controller: SPUStandardUpdaterController

    private init() {
        controller = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: nil,
            userDriverDelegate: nil
        )
    }
}
