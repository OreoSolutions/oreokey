# Changelog

Mọi thay đổi đáng chú ý của OreoKey ghi ở đây. Theo định dạng
[Keep a Changelog](https://keepachangelog.com/vi/1.1.0/), phiên bản theo
[SemVer](https://semver.org/lang/vi/).

## [Chưa phát hành]

### Đã thêm
- **Gõ `đ` linh hoạt (Telex)**: gõ `đ` bằng cả `dd` liền kề *hoặc* đặt `d` muộn
  ở cuối âm tiết — `did` → `đi`, `dangd` → `đang`. Song song với "gõ mũ muộn"
  (`nanag` → `nâng`) và chung công tắc **"Gõ dấu mũ linh hoạt"**; chỉ áp khi ra
  âm tiết tiếng Việt hợp lệ nên từ tiếng Anh (`dryad`) giữ nguyên.

### Đã sửa
- **Cặp `ươ` (Telex) khi còn nguyên âm cuối**: một chữ `w` giờ móc đúng cả cặp
  trong `ươi`/`ươu` — `nguoiwf` → `người`, `cuoiwf` → `cười`, `ruouwj` → `rượu`.
  Trước đây chỉ móc mỗi chữ `o`, buộc phải gõ tay từng dấu (`cuwowif`).

## [0.3.0] - 2026-07-09

### Đã đổi
- **Chế độ "gõ thoải mái"**: khi tắt "Kiểm tra chính tả (chặt)", engine giờ cho
  gõ tắt tiếng Việt (`đc`, `nèk`, `ko`…) mà vẫn tự nhận diện và giữ nguyên từ
  tiếng Anh có cụm bất khả (`clear`, `sound`, `status`…), thay vì đặt dấu bừa
  như trước. Bật lại để ưu tiên bảo vệ tối đa từ tiếng Anh.

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
