# Changelog

Mọi thay đổi đáng chú ý của OreoKey ghi ở đây. Theo định dạng
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), phiên bản theo
[SemVer](https://semver.org/).

## [Chưa phát hành]

## [0.7.2] - 2026-07-22

### Đã sửa
- **Hủy dấu mũ Telex ở mức Thường**: phím `a`/`e`/`o` lặp lại nay vẫn được
  dùng để hủy `â`/`ê`/`ô` khi cần, thay vì luôn bị coi là phần đuôi kéo dài
  của một âm tiết đã hoàn chỉnh. Ví dụ, gõ `dataaaa` cho ra `dataaa` (một
  phím `a` được dùng để hủy mũ), trong khi `yeeuuu` vẫn cho ra `yêuuu`.

## [0.7.1] - 2026-07-21

### Đã sửa
- **Gõ chữ `w` ở đầu từ**: `w` hoặc `W` nay giữ nguyên là ký tự Latin thay
  vì tự đổi thành `ư`, nên có thể gõ từ như `web`/`Windows` bình thường.
  Cách gõ `ư` vẫn là `uw`.

## [0.7.0] - 2026-07-21

### Đã đổi
- **Gõ tiếp sau âm tiết đã hoàn chỉnh ở mức Thường/Thoải mái**: khi phần đầu
  đã là âm tiết tiếng Việt đúng, mọi ký tự nối tiếp làm cụm trở nên bất khả
  được giữ nguyên phía sau thay vì làm engine bung lại phím gốc. Ví dụ:
  `đô` + `uuuu` → `đôuuuu`, `đông` + `uuuu` → `đônguuuu`, `yêu` + `uuuu`
  → `yêuuuuu`, `chào` + `ooooo` → `chàoooooo`. Xóa hết phần đuôi sẽ trở
  lại gõ tiếng Việt bình thường; mức **Chặt** giữ cơ chế hủy dấu Telex cũ
  (`gô` + `o` → `goo`).

## [0.6.6] - 2026-07-21

### Đã sửa
- **Không còn ghi nội dung gõ vào log chẩn đoán**: khi bật `OREOKEY_DEBUG`,
  log nay không lưu ký tự hay đoạn văn bản đang nhập; ô mật khẩu được nhận diện
  trước mọi xử lý và log. File log chuyển vào vùng dữ liệu riêng của OreoKey,
  chỉ người dùng hiện tại đọc được và tự giới hạn dung lượng.
- **Lưu cài đặt đồng thời không bị mất dữ liệu**: thay đổi từ cửa sổ Cài đặt và
  hotkey bật/tắt tiếng Việt nay được tuần tự hóa với thao tác ghi đĩa, tránh
  đè file tạm hoặc để trạng thái đang chạy khác với file cài đặt. Nếu không
  ghi được, Cài đặt và menu bar hiện thông báo thay vì im lặng bỏ qua.
- **Không mất phím khi sửa qua Accessibility thất bại**: chế độ tương thích
  chỉ dùng AX nay cho phím gốc đi qua và đồng bộ lại engine nếu app đích từ
  chối thay chữ, thay vì nuốt mất ký tự.
- **Khởi động/dừng bộ bắt phím ổn định hơn**: các yêu cầu start/stop đồng thời
  được tuần tự hóa; app chờ tap cũ dọn xong trước khi tạo tap mới, tránh còn
  tap cũ chạy ngầm hoặc báo đã khởi động khi đang dừng.
- **Gõ tắt có emoji/ký tự ngoài BMP**: chuỗi bơm vào ứng dụng nay được chia
  đúng theo số đơn vị UTF-16, nên không vượt giới hạn event khi macro có emoji.
- **Hồ sơ ứng dụng wildcard nhất quán**: khi nhiều wildcard cùng khớp, OreoKey
  luôn chọn tiền tố cụ thể nhất thay vì phụ thuộc thứ tự nội bộ.

### Đã đổi
- **Nhẹ hơn trên mỗi phím gõ**: khi tắt debug (mặc định), callback không còn
  tạo chuỗi log hay giải mã Unicode chỉ để ghi log; file debug cũng được giữ
  mở thay vì mở/đóng theo từng sự kiện.

## [0.6.5] - 2026-07-17

### Đã sửa
- **Gõ thanh ngay sau `qu` bị kẹt nguyên văn**: bấm phím thanh liền sau `qu`,
  trước nguyên âm chính — Telex `qusan`, VNI `qu1an` — trước đây khóa cứng cả
  từ thành phím gốc (`qusan` thay vì `quán`), gõ tiếp đúng vần cũng không hồi
  phục; ảnh hưởng 131 từ `qu-` thông dụng (`quá`, `quán`, `quấy`, `quỳnh`…).
  Nay ra đúng chữ ở mọi thứ tự gõ. Tìm ra nhờ sweep toàn từ điển bên dưới.

### Đã thêm
- **Bộ kiểm thử sweep toàn từ điển** (`core/src/bin/sweep`): sinh *mọi cách gõ*
  (Telex + VNI, mọi vị trí thanh/mũ/móc/trăng/đ, hai kiểu đặt thanh) cho 8.784
  âm tiết và đối chiếu kết quả engine — chạy hết trong ~0,2 giây. Sau bản sửa
  trên, engine sạch lỗi thứ tự gõ trên toàn bộ âm tiết hợp lệ; phần lệch còn
  lại đều là giới hạn có chủ đích (từ mượn đa âm tiết viết liền, vần/phụ âm
  ngoại lai) — chi tiết trong `docs/dict-sweep-report.md`.
- **Micro-benchmark hot path** (`core/src/bin/bench`): đo ns/phím các kịch bản
  gõ thực tế và bệnh lý làm mốc so sánh cho các lần sửa engine sau.

### Đã đổi
- **Dọn engine theo review hai vòng** (không đổi hành vi — sweep xác nhận giống
  hệt từng byte): gộp bước dựng lại từ chạy 2–3 lần mỗi phím thành 1; logic
  phím thanh Telex/VNI dùng chung một chỗ; bảng chuyển mã dựng một lần thay vì
  mỗi lượt. Gõ chuỗi dài không ngắt từ (mã hex, số seri…) nhẹ hơn ~2,5 lần nhờ
  chặn sớm token quá 64 ký tự — chắc chắn không phải tiếng Việt.

## [0.6.4] - 2026-07-12

### Đã thêm
- **Tự ghi phím tắt bật/tắt**: mục "Ghi tổ hợp phím mới…" trong Cài đặt →
  Chung — nhấn thẳng tổ hợp muốn dùng (cần kèm ⌃/⌥/⌘, ⎋ để hủy) thay vì
  chỉ chọn từ danh sách có sẵn. Thêm preset **⌃⌥Space**. Tổ hợp tùy chỉnh
  hiển thị đúng ở cả menu bar lẫn Cài đặt.

## [0.6.3] - 2026-07-11

### Đã đổi
- **Nút Ủng hộ** trong tab Giới thiệu trỏ tới trang Ko-fi của tác giả
  (https://ko-fi.com/nguyenhuyquang); README thêm badge và mục Ủng hộ.

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
