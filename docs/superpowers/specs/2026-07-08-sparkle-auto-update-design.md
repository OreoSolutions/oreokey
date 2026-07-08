# Thiết kế: Tích hợp Sparkle Auto-Update cho OreoKey

**Ngày:** 2026-07-08
**Trạng thái:** Đã duyệt, chờ lập kế hoạch triển khai

## Mục tiêu

Cho phép OreoKey tự kiểm tra và cài đặt bản cập nhật từ GitHub Releases,
dùng framework [Sparkle](https://sparkle-project.org) 2.x. Người dùng nhận
thông báo khi có bản mới kèm changelog, tự quyết định cài hay không.

## Bối cảnh

- App: SwiftPM executable (`app/Package.swift`), đóng gói `.app` thủ công
  qua `scripts/build.sh` (chỉ copy binary, chưa nhúng framework nào).
- Phát hành: DMG kéo-thả trên GitHub Releases (`OreoSolutions/oreokey`),
  changelog theo Keep a Changelog trong `CHANGELOG.md`.
- App ký **ad-hoc** khi dev; ký **Developer ID + notarize** khi phát hành
  thật (tài khoản Apple Developer đang chờ cấp).

## Quyết định thiết kế (đã chốt)

| Vấn đề | Lựa chọn |
|--------|----------|
| Host appcast | Raw file trong repo: `raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml` |
| Hành vi update | Tự kiểm tra định kỳ (24h), **hỏi trước khi cài** kèm changelog |
| Quy trình phát hành | Script cục bộ `scripts/release.sh` (không dùng CI — giữ khóa riêng EdDSA trên máy) |
| Delta update | Không (YAGNI — cập nhật full DMG) |

## Kiến trúc

App khởi động → tạo `SPUStandardUpdaterController` chạy nền. Bộ điều khiển
đọc `SUFeedURL`, mỗi `SUScheduledCheckInterval` (24h) tải `appcast.xml`, so
version với `CFBundleShortVersionString`. Có bản mới hơn → Sparkle hiện hộp
thoại chuẩn kèm release notes → người dùng bấm **Cài đặt** → Sparkle tải
DMG từ GitHub Releases, xác minh chữ ký **EdDSA** bằng `SUPublicEDKey`, thay
app, khởi động lại. Mục menu bar **"Kiểm tra bản mới…"** cho phép bấm thủ công.

Chữ ký EdDSA (Sparkle) **độc lập** với chữ ký Apple code-signing. Sparkle
dùng EdDSA để đảm bảo DMG tải về đúng bản do chủ dự án phát hành; Apple
code-signing/notarization để qua Gatekeeper và cho phép auto-install liền mạch.

## Các thành phần

### 1. Phụ thuộc Sparkle (`app/Package.swift`)

Thêm:
```swift
.package(url: "https://github.com/sparkle-project/Sparkle", from: "2.6.0")
```
Link `.product(name: "Sparkle", package: "Sparkle")` vào target `OreoKey`.

Sparkle 2.x phân phối dưới dạng **XCFramework** (kèm XPC services, Autoupdate,
Updater.app nằm bên trong). SwiftPM link được nhưng **không tự nhúng**
framework vào bundle `.app` dựng thủ công.

### 2. Nhúng framework (`scripts/build.sh`)

Sau khi `swift build`, trước/trong bước dựng bundle:

1. Thêm linker rpath để binary tìm được framework:
   `-Xlinker -rpath -Xlinker @executable_path/../Frameworks`
2. Định vị `Sparkle.framework` (slice macOS đúng) trong
   `.build/artifacts/…/Sparkle.xcframework/…` và copy vào
   `dist/OreoKey.app/Contents/Frameworks/`.
3. Ký **từ trong ra ngoài** (bắt buộc với hardened runtime):
   XPC services → Autoupdate → Updater.app → `Sparkle.framework` → app.
   Ký ad-hoc (`-`) khi dev, Developer ID khi có `CODESIGN_ID`.

### 3. Khóa EdDSA (một lần, thủ công)

- Chạy `generate_keys` (kèm trong artifact Sparkle) một lần → khóa **riêng**
  lưu Keychain đăng nhập, khóa **công khai** in ra.
- Đặt khóa công khai vào `Info.plist` khóa `SUPublicEDKey`.
- Khóa riêng **không lên git**, dùng cho `sign_update` mỗi lần phát hành.

### 4. Info.plist (`app/Info.plist`) — thêm khóa

```
SUFeedURL                = https://raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml
SUPublicEDKey            = <khóa công khai từ generate_keys>
SUEnableAutomaticChecks  = true
SUScheduledCheckInterval = 86400
```

### 5. Mã Swift

- **`app/Sources/OreoKey/Updater.swift`** (mới): bọc
  `SPUStandardUpdaterController(startingUpdater: true, updaterDelegate: nil,
  userDriverDelegate: nil)`; phơi `checkForUpdates()`.
- **`AppDelegate` / menu bar**: thêm mục **"Kiểm tra bản mới…"** gọi
  `updater.checkForUpdates()`.
- Lần chạy đầu Sparkle tự hỏi người dùng có bật tự-kiểm-tra không (hành vi
  chuẩn Sparkle), khớp lựa chọn "hỏi trước khi cài".

### 6. Appcast (`appcast.xml` ở gốc repo)

File RSS Sparkle liệt kê các `<item>` phát hành. Mỗi item chứa version,
URL DMG (trỏ tới asset trên GitHub Releases), độ dài file, chữ ký
`sparkle:edSignature`, và release notes (từ CHANGELOG). Sinh/cập nhật tự
động bởi `release.sh`; commit vào `main` để URL raw phục vụ.

### 7. Quy trình phát hành (`scripts/release.sh`, mới)

`./scripts/release.sh 0.2.0`:

1. Cập nhật version trong `app/Info.plist` (`CFBundleShortVersionString` +
   tăng `CFBundleVersion`) và `core/Cargo.toml`.
2. Chuyển mục `[Chưa phát hành]` trong `CHANGELOG.md` thành `[0.2.0] - <ngày>`.
3. `build.sh --universal` (ký Developer ID nếu có `CODESIGN_ID`) →
   `make-dmg.sh` (notarize nếu có `NOTARY_PROFILE`).
4. `sign_update dist/OreoKey.dmg` → lấy `sparkle:edSignature` + length.
5. Chèn `<item>` mới vào `appcast.xml` (release notes lấy từ CHANGELOG),
   `git commit` + `git push` appcast.
6. `git tag v0.2.0` → `gh release create v0.2.0 dist/OreoKey.dmg …`.

## Kiểm thử

- **Engine (Rust)**: không đổi → `cargo test` vẫn xanh, không ảnh hưởng.
- **Sparkle UI**: tạm trỏ `SUFeedURL` tới appcast thử có version giả cao
  (0.9.9), chạy app 0.1.0, xác nhận hộp thoại update hiện kèm changelog và
  tải được. Ký ad-hoc đủ để kiểm thử luồng kiểm-tra/tải/hiện-notes.
- **Auto-install liền mạch**: cần Developer ID + notarize; hoàn thiện khi có
  cert (đặt `CODESIGN_ID`/`NOTARY_PROFILE`) — không phải làm lại mã.

## Phụ thuộc & rủi ro

- **Ký Developer ID (đang chờ)**: bước tự-cài của Sparkle chạy tin cậy nhất
  khi app ký Developer ID + notarize. Mã/UI/tải chạy được với ad-hoc; chỉ
  auto-install liền mạch cần cert. Không có việc phải làm lại.
- **Nhúng framework thủ công**: đây là phần dễ sai nhất (rpath, đường dẫn
  slice XCFramework, thứ tự ký nested). Plan phải ghi đường dẫn/lệnh chính xác.
- **Kích thước app tăng**: Sparkle.framework thêm ~vài MB vào bundle. Chấp
  nhận được; không ảnh hưởng RAM lúc chạy đáng kể.

## Ngoài phạm vi (YAGNI)

- Delta/patch update (chỉ full DMG).
- Kênh beta/stable riêng.
- Tự động hóa phát hành qua CI (giữ khóa riêng cục bộ).
