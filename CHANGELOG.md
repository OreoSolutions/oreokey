# Changelog

Mọi thay đổi đáng chú ý của OreoKey ghi ở đây. Theo định dạng
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), phiên bản theo
[SemVer](https://semver.org/).

## [Chưa phát hành]

## [0.6.2] - 2026-07-10

### Đã thêm
- **Fix**:
  Em không là nàng thơ
  Anh cũng không còn là nhạc sĩ mộng mơ
  Tình này nhẹ như gió
  Lại trĩu lên tim ta những vết hằn

## [0.6.1] - 2026-07-10

### Đã thêm
- **Nút Trang chủ** trong tab Giới thiệu, dẫn tới website chính thức
  https://oreokey.vercel.app.

## [0.6.0] - 2026-07-10

### Đã thêm
- **Trạng thái cập nhật ngay trên sidebar Cài đặt**: chân sidebar hiện tên app
  và phiên bản — tick xanh ✓ *Mới nhất* khi đang chạy bản mới nhất, cảnh báo
  cam ⚠ ngay khi có bản mới; bấm vào dòng đó để kiểm tra/cài đặt. Mở cửa sổ
  Cài đặt là app tự dò bản mới ở chế độ nền, không bật hộp thoại.
- **Nút GitHub** trong tab Giới thiệu, dẫn về trang mã nguồn.
- **Tài liệu hướng dẫn**: thêm `docs/install.md` (cài đặt, cấp quyền, gỡ bỏ)
  và `docs/user-guide.md` (bảng gõ Telex/VNI, mọi tính năng, xử lý sự cố).

### Đã đổi
- **Menu bar gọn lại**: hai mục "Kiểm tra bản mới…" và "Khởi động cùng máy"
  chuyển vào cửa sổ Cài đặt (mục Hệ thống trong tab Chung và chân sidebar);
  menu chỉ còn bật/tắt tiếng Việt, kiểu gõ, Cài đặt và Thoát.
- **Kiểm tra chính tả đổi sang thanh trượt 3 nấc**: kéo càng cao kiểm tra càng
  chặt (Thoải mái → Thường → Chặt), có chấm đánh dấu từng nấc và tên mức hiện
  ngay cạnh — thay cho ô chọn 3 nút trước đây.

### Đã sửa
- **Cửa sổ Cài đặt chồng chữ vùng tiêu đề**: nội dung cuộn lên đè vào titlebar
  trong suốt, và toolbar tự sinh (nút thu sidebar, tiêu đề lệch cạnh nút đèn)
  gây xô lệch phần đầu cửa sổ. Nay titlebar hiển thị bình thường, không còn
  nút thừa hay chữ đè nhau.

## [0.5.2] - 2026-07-10

### Đã sửa
- **Gõ `loose` bị bung thành `looose`**: khi từ hóa ra không phải tiếng Việt
  ở phím muộn (sau khi đã hủy mũ bằng `ooo`), văn bản khôi phục bung lại cả
  phím hủy đã tiêu. Nay phần gõ trước khi có dấu giữ đúng dạng đã hiển thị
  (`loos` + `e` → `loose`), phần sau giữ nguyên văn phím gõ.

## [0.5.1] - 2026-07-10

### Đã sửa
- **`looos` ra `lóo` dù bật kiểm tra chính tả**: nhân âm `oo` (vốn chỉ có
  trong từ mượn đóng bằng `c`/`ng` như `xoong`, `soóc`) bị chấp nhận cả khi
  đứng cuối từ, nên `lóo` lọt lưới ở mức Chặt lẫn Thường; nay khôi phục đúng
  phím gốc. Đồng thời sửa lỗi kéo theo: gõ `soóc` kiểu đặt thanh sớm
  (`sooosc`) trước đây bị bung nhầm thành `sốc` — dấu mũ đã hủy tự quay lại.
  (Mức Thoải mái vẫn đặt dấu tự do theo đúng thiết kế.)

## [0.5.0] - 2026-07-10

### Đã thêm
- **Kiểm tra chính tả 3 mức**: ô cài đặt cũ bật/tắt giờ thành 3 mức chọn —
  **Chặt** (bảo vệ tối đa từ tiếng Anh, `mask`, `class` giữ nguyên), **Thường**
  (cho gõ tắt `đc`, `nèk` mà vẫn nhận diện tiếng Anh có cụm bất khả như `clear`,
  `sound`), **Thoải mái** (luôn đặt dấu, không bao giờ khôi phục từ tiếng Anh).
  Mặc định là Chặt.
- **Thêm ứng dụng bằng bundle ID thủ công**: mục "Nhập bundle ID…" trong Cài đặt
  → Ứng dụng cho phép loại trừ hoặc chỉnh chế độ tương thích cho app **chưa chạy**
  (vd Burp Suite), thay vì chỉ chọn được app đang mở.

### Đã sửa
- **Nháy chữ ở một số terminal**: kitty, Alacritty, WezTerm, Hyper trước đây bị
  nháy/dính chữ khi gõ; nay đi thẳng đường bơm phím như các terminal khác.
- **VNI đặt số thanh trước số mũ giữa từ**: gõ kiểu `thie16u` (bấm số thanh `1`
  ngay sau nguyên âm rồi mới số mũ `6`) trước đây bị kẹt thành `thie16u`; nay ra
  đúng `thiếu`. Tương tự với các từ có `iê` khác.
- **Nội dung cập nhật hiển thị đúng định dạng**: hộp thoại "Có bản mới" của
  Sparkle trước đây hiện ký tự Markdown thô (`###`, `-`, dấu backtick); nay render
  thành tiêu đề, danh sách, chữ đậm và mã đúng cách.

## [0.4.1] - 2026-07-10

### Đã sửa
- **Telegram Desktop (Qt) kẹt vùng chọn, không gõ được**: bản Telegram tải từ
  telegram.org (`com.tdesktop.Telegram`) nhận lệnh chọn vùng nhưng lờ lệnh thay
  chữ mà vẫn báo thành công, để lại vùng bôi đen kẹt và chữ cũ nguyên vẹn. Nay
  đã thêm hồ sơ riêng cho bản này và, sau khi ghi, OreoKey đọc lại để xác minh
  chữ đã thật sự thay — nếu app "nói dối" thì khôi phục con trỏ rồi rơi về bơm
  phím. Tự chữa cho mọi app cùng kiểu, không riêng Telegram.

## [0.4.0] - 2026-07-09

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
