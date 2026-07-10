# Hướng dẫn cài đặt OreoKey

OreoKey chạy trên **macOS 13 (Ventura) trở lên**, hỗ trợ cả Apple Silicon
và Intel (universal binary).

## 1. Tải và cài

1. Tải file `.dmg` mới nhất tại trang
   [Releases](https://github.com/OreoSolutions/oreokey/releases).
2. Mở file DMG, kéo **OreoKey** vào thư mục **Applications**.
3. Mở OreoKey từ Applications (hoặc Spotlight).

Bản phát hành đã được ký Developer ID và notarize với Apple nên mở
bình thường, không bị Gatekeeper chặn.

## 2. Cấp quyền Accessibility (bắt buộc)

Bộ gõ cần quyền **Accessibility** để nhận phím toàn hệ thống. Lần đầu
chạy, OreoKey sẽ hiện màn hình hướng dẫn:

1. Bấm nút mở **System Settings → Privacy & Security → Accessibility**.
2. Bật công tắc cạnh **OreoKey**.
3. Quay lại app — OreoKey tự nhận quyền và bắt đầu chạy, biểu tượng
   **VN/EN** xuất hiện trên menu bar.

> Nếu đã bật công tắc mà app vẫn báo thiếu quyền: tắt rồi bật lại công
> tắc, hoặc chạy `tccutil reset Accessibility com.oreosolutions.oreokey`
> trong Terminal rồi cấp lại.

## 3. Thiết lập ban đầu

- **Kiểu gõ**: mặc định Telex. Đổi sang VNI ngay trên menu bar
  (mục *Kiểu gõ*) hoặc trong **Cài đặt → Chung**.
- **Phím tắt bật/tắt tiếng Việt**: mặc định `⌃Space` (Control+Space).
  Đổi trong **Cài đặt → Chung**.
- **Khởi động cùng máy**: bật trong **Cài đặt → Chung → Hệ thống**.

Lưu ý: nên **tắt hoặc gỡ các bộ gõ tiếng Việt khác** (kể cả bộ gõ tiếng
Việt có sẵn của macOS) để hai bộ gõ không xử lý chồng nhau. Khi bật
tiếng Việt, OreoKey tự chuyển input source hệ thống về bàn phím Latin
(ABC).

## 4. Cập nhật

OreoKey tự kiểm tra bản mới định kỳ (Sparkle). Khi có bản mới, chân
sidebar cửa sổ Cài đặt hiện cảnh báo cam — bấm vào để cài. Kiểm tra thủ
công: mở **Cài đặt**, bấm dòng phiên bản ở chân sidebar.

## 5. Gỡ cài đặt

1. Thoát OreoKey (menu bar → **Thoát OreoKey**).
2. Xóa `OreoKey.app` khỏi Applications.
3. (Tuỳ chọn) Xóa cấu hình: `~/Library/Application Support/OreoKey/`.

## Cài từ mã nguồn

Yêu cầu: Rust (`cargo`) và Xcode Command Line Tools.

```bash
git clone https://github.com/OreoSolutions/oreokey.git
cd oreokey
./scripts/build.sh    # → dist/OreoKey.app
```

Bản tự build ký ad-hoc: sau mỗi lần rebuild, quyền Accessibility cũ vô
hiệu (công tắc vẫn ON nhưng không tác dụng) — reset bằng
`tccutil reset Accessibility com.oreosolutions.oreokey` rồi cấp lại.

---

Tiếp theo: xem [Hướng dẫn sử dụng đầy đủ](user-guide.md).
