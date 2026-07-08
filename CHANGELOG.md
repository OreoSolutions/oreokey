# Changelog

Mọi thay đổi đáng chú ý của OreoKey ghi ở đây. Theo định dạng
[Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), phiên bản theo
[SemVer](https://semver.org/lang/vi/).

## [Chưa phát hành]

## [0.2.0] - 2026-07-08

### Đã thêm
- **Tự động cập nhật** qua Sparkle: kiểm tra bản mới định kỳ (24h), hỏi trước
  khi cài kèm changelog, và mục menu **"Kiểm tra bản mới…"**. Bản phát hành
  được ký EdDSA và phân phối qua GitHub Releases.

### Đã đổi
- Mở nguồn theo giấy phép MIT.

## [0.1.0] - 2026-07-08

Bản phát hành đầu tiên.

### Đã thêm
- Kiểu gõ **Telex** và **VNI**.
- **Kiểm tra chính tả**: từ không phải tiếng Việt tự trả về phím gốc.
- **Gõ dấu mũ linh hoạt**: `nanag → nâng`, `viete → viêt`.
- **Gõ tắt** (macro) và **che từ tục tĩu** khi chốt từ.
- **Loại trừ theo app** và nhớ trạng thái VN–EN riêng từng app.
- Phím tắt bật/tắt toàn cục, biểu tượng menu bar, khởi động cùng máy.
- Chống **dính chữ / nháy chữ**: ưu tiên Accessibility API, diff tối thiểu,
  gộp event, bảng quirk theo app (`data/app-profiles.json`).
- Cửa sổ hướng dẫn cấp quyền và cửa sổ Cài đặt (Chung / Gõ tắt / Ứng dụng /
  Giới thiệu).

[Chưa phát hành]: https://github.com/OreoSolutions/oreokey/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/OreoSolutions/oreokey/releases/tag/v0.1.0
