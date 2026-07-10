import Combine
import Sparkle

/// Trạng thái cập nhật cho UI quan sát — nhận tin từ Sparkle qua delegate.
/// Sparkle giữ delegate yếu nên Updater phải giữ mạnh object này.
final class UpdateStatus: NSObject, ObservableObject, SPUUpdaterDelegate {
    enum State {
        /// Chưa có kết quả kiểm tra nào (mới mở app, chưa dò xong).
        case unknown
        /// Đã dò và đang chạy bản mới nhất.
        case upToDate
        /// Có bản mới đang chờ (kèm số phiên bản).
        case available(String)
    }

    @Published var state: State = .unknown

    func updater(_ updater: SPUUpdater, didFindValidUpdate item: SUAppcastItem) {
        DispatchQueue.main.async { self.state = .available(item.displayVersionString) }
    }

    func updaterDidNotFindUpdate(_ updater: SPUUpdater) {
        DispatchQueue.main.async { self.state = .upToDate }
    }
}

/// Bọc bộ cập nhật Sparkle. Khởi tạo `shared` là bắt đầu kiểm tra nền
/// theo cấu hình Info.plist (SUFeedURL, SUEnableAutomaticChecks, interval).
final class Updater {
    static let shared = Updater()

    let status: UpdateStatus
    let controller: SPUStandardUpdaterController

    private init() {
        let status = UpdateStatus()
        self.status = status
        controller = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: status,
            userDriverDelegate: nil
        )
    }

    /// Dò bản mới im lặng (chỉ báo qua delegate, không hiện UI) — dùng để
    /// cập nhật badge khi mở cửa sổ cài đặt.
    func probeQuietly() {
        guard !controller.updater.sessionInProgress else { return }
        controller.updater.checkForUpdateInformation()
    }
}
