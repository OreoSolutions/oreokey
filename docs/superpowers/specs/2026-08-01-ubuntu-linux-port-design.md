# OreoKey cho Ubuntu — Thiết kế

## Mục tiêu

Đưa OreoKey thành bộ gõ tiếng Việt tích hợp đúng với desktop Linux, phát hành
cho Ubuntu 22.04 LTS và 24.04 LTS trên kiến trúc amd64 và arm64. Bản phát hành
bao gồm adapter cho **IBus** và **Fcitx5**, cùng một ứng dụng GTK để cấu hình
dùng chung.

Người dùng có thể chọn một trong hai framework đang dùng trên máy; OreoKey
không tự cài hay tự kích hoạt đồng thời IBus và Fcitx5. IBus là đường mặc định
cho Ubuntu GNOME. Fcitx5 phục vụ KDE và các phiên đã chọn Fcitx5 qua
`im-config`.

## Phạm vi

### Có trong bản Ubuntu đầu tiên

- Gõ Telex và VNI bằng engine Rust hiện tại.
- Ba mức kiểm tra chính tả, gõ dấu mũ muộn, kiểu đặt dấu hiện đại, gõ tắt và
  bộ lọc từ tục tĩu.
- Bật/tắt Việt-Anh, cấu hình hotkey, chuyển mã Unicode/VNI-Windows/TCVN3.
- IBus engine tương thích GNOME trên X11 và Wayland.
- Fcitx5 input-method addon tương thích KDE, X11 và Wayland theo khả năng của
  frontend/compositor.
- Ứng dụng GTK 4 cấu hình chung, khởi động qua menu ứng dụng và lệnh
  `oreokey-config`.
- Gói Debian, tài liệu cài đặt/gỡ cài đặt, CI phát hành cho hai LTS và hai kiến
  trúc mục tiêu.

### Không nằm trong phạm vi

- Daemon bắt phím toàn hệ thống, giả lập phím, Accessibility API, hay workaround
  nháy/dính chữ của macOS. IBus/Fcitx5 là chủ sở hữu input context và phải là
  con đường duy nhất để sửa/commit text.
- Port UI Swift/AppKit, Sparkle, DMG, appcast hay auto-update macOS.
- Hỗ trợ distro ngoài Ubuntu, Flatpak/Snap/AppImage, hoặc kho APT riêng.
  Thiết kế gói không khóa vào Ubuntu để có thể mở rộng sau này.
- Chuyển nguyên xi các hồ sơ `per_app_mode`: chúng điều khiển cách bơm phím
  trên macOS và không có ý nghĩa trên Linux IM framework.

## Kiến trúc

```
                         +-----------------------+
                         | oreokey-config (GTK4) |
                         +-----------+-----------+
                                     | read/write + reload signal
                                     v
 +--------------------+    +-------------------------------+
 | IBus Engine        |--->| oreokey-core (Rust thuần)      |
 | C/GObject adapter  |    | Engine, settings schema, macro |
 +--------------------+    +-------------------------------+
                                     ^
 +--------------------+              |
 | Fcitx5 addon       |--------------+
 | C++ adapter        |
 +--------------------+
```

### Lõi Rust

`core/` trở thành phần thuần đa nền tảng. Engine, spell check, macro, censor
và chuyển mã tiếp tục nằm trong crate này; kiểm thử hiện tại phải chạy mà không
cần desktop session.

Runtime macOS (`platform`, `ffi`, event tap, AX, injection, ghost guard và
profile bơm phím) vẫn được biên dịch riêng dưới `cfg(target_os = "macos")`.
Các kiểu config được chuyển thành schema trung lập: keycode/hotkey không còn
được mô tả như virtual keycode macOS. Linux giữ shortcut dưới dạng tên phím và
modifier của framework; cấu hình nằm ở XDG path riêng, không tự đọc hay sửa
file `~/Library/Application Support/OreoKey/` của macOS.

Một API Rust hẹp, platform-neutral được thêm cho adapter:

- tạo/hủy một session engine cho từng input context;
- đưa một phím đã chuẩn hóa vào session;
- nhận `PassThrough`, `Preedit`, `Commit` hoặc `DeleteSurrounding`;
- reset khi input context, focus, hoặc kiểu nhập đổi;
- tải lại snapshot setting đã được xác thực.

Mỗi context có `Engine` riêng. Không dùng singleton buffer toàn cục, để hai cửa
sổ hoặc hai client không lẫn từ đang gõ. Macro và setting được snapshot khi
tạo context hoặc khi adapter nhận tín hiệu reload.

### IBus adapter

Adapter IBus là dịch vụ engine GObject/C mỏng, đăng ký component XML và chạy
qua cơ chế engine chuẩn của IBus. Nó chuyển `process_key_event` thành API core,
rồi gọi API IBus để forward phím chưa xử lý, cập nhật preedit, commit chuỗi,
hoặc xóa surrounding text theo action trả về.

Package cài component XML, executable/dịch vụ engine và icon vào các thư mục
IBus/XDG chuẩn. Không hook thiết bị input hay dùng quyền root. Người dùng thêm
"OreoKey" trong Settings > Keyboard > Input Sources sau khi cài.

### Fcitx5 adapter

Adapter Fcitx5 là shared-library addon C++ dùng `InputMethodEngineV2`. Nó
chuyển `KeyEvent` thành API core, trả preedit/commit qua `InputContext` và chỉ
lọc event khi core yêu cầu thay thế. Metadata addon và input method được cài vào
`share/fcitx5/addon` và `share/fcitx5/inputmethod`; thư viện nằm trong đường
dẫn addon của Fcitx5 theo multiarch.

Người dùng chọn Fcitx5 bằng `im-config`, đăng xuất/đăng nhập khi môi trường
desktop yêu cầu, rồi thêm OreoKey từ Fcitx5 Configuration. Adapter không tự
khởi động Fcitx5 hoặc sửa biến môi trường của người dùng.

### Cấu hình và UI

Config chuẩn ở `${XDG_CONFIG_HOME:-~/.config}/oreokey/settings.json`, quyền
file `0600`; ghi atomic bằng file tạm trong cùng thư mục rồi rename. Các field
Linux chỉ chứa tính năng có ý nghĩa chung: kiểu gõ, spell mode, modern tone,
macro, flexible marks, censor, enabled và hotkey. Unknown fields được bỏ qua để
giữ forward compatibility.

`oreokey-config` là ứng dụng GTK4 gọi API Rust an toàn thay vì tự tái hiện
logic engine. Nó có bốn trang: Chung, Gõ tắt, Chuyển mã, Giới thiệu. Sau khi
lưu, ứng dụng phát tín hiệu D-Bus tên riêng tới adapter đang chạy; nếu adapter
không chạy, bản settings mới được nạp khi framework tạo engine/context tiếp
theo. Cả adapter lẫn UI phải chịu được config hỏng: dùng default, không ghi đè
file hỏng và hiển thị lỗi có thể hành động trong UI.

### Luồng gõ

1. Framework gửi key press và input context tới adapter hiện hành.
2. Adapter chuẩn hóa ký tự ASCII, modifier, backspace và word break; shortcut
   ứng dụng/system không thuộc engine được forward nguyên vẹn và reset buffer.
3. Core trả action. Adapter forward event, cập nhật preedit hoặc commit/delete
   qua API framework tương ứng.
4. Khi focus chuyển, framework báo reset; không cần dò app foreground hay đọc
   text ngoài input context.
5. Khi người dùng đổi hotkey, adapter chỉ tiêu thụ đúng hotkey đã đăng ký và
   phát trạng thái Việt-Anh qua property/preedit của framework nếu API hỗ trợ.

## Đóng gói và phát hành

Mỗi release xuất ba artifact độc lập, có version cùng với crate:

| Package | Nội dung | Dependency runtime chính |
| --- | --- | --- |
| `oreokey-config` | GTK config app, binary, desktop entry, icon, core shared library | GTK4 và GLib |
| `ibus-oreokey` | IBus adapter, component XML, icon | IBus, `oreokey-config` cùng version |
| `fcitx5-oreokey` | Fcitx5 addon, metadata và input method config | Fcitx5, `oreokey-config` cùng version |

Không có meta-package buộc cài hai framework. Release note nêu rõ lệnh cài cho
từng lựa chọn; installer không dùng `sudo` ngoài luồng `apt install ./file.deb`
do chính người dùng chạy.

Debian source packaging nằm dưới `packaging/debian/`. Build tạo package trong
container Ubuntu 22.04 và 24.04 có dependency development được pin theo LTS.
Artifact đặt tên có distro series và kiến trúc, ví dụ
`ibus-oreokey_0.8.0-1_ubuntu24.04_amd64.deb`.

GitHub Actions chạy kiểm thử lõi trên Linux, build IBus/Fcitx5 adapters trong
hai container LTS cho amd64, và build arm64 native qua runner arm64 hoặc một
builder đã được xác thực. Release chỉ xuất artifact khi cả engine test,
adapter test và kiểm tra Debian package đều đạt; checksum SHA-256 được kèm
theo mỗi release.

## Kiểm thử và tiêu chí chấp nhận

### Tự động

- `cargo test --workspace` cho engine/config, gồm các regression Telex/VNI,
  macro, spell mode, reset context và config migration.
- Unit test adapter với fake input context: pass-through, replace, preedit,
  commit, backspace, word break, hotkey, focus reset và settings reload.
- Smoke test tạo IBus component và Fcitx5 addon từ package staging, kiểm tra
  metadata/file path và dynamic dependencies.
- `lintian` và `dpkg-deb --info/--contents` cho mọi `.deb`; kiểm tra không có
  file macOS, config người dùng hay credential trong package.

### Thủ công trên máy thật

- Ubuntu 22.04 và 24.04, mỗi bản trên GNOME Wayland và GNOME Xorg với IBus.
- Ubuntu 22.04 và 24.04, KDE Plasma Wayland/X11 với Fcitx5; ghi rõ hạn chế của
  app nào/compositor nào nếu frontend không cấp surrounding text.
- Thêm engine qua UI framework, gõ bộ case Telex/VNI chuẩn, chuyển Việt-Anh,
  restart session, mở lại config, thêm/xóa macro, cập nhật và gỡ package.
- Xác nhận không xử lý mật khẩu hay phím shortcut không thuộc input context
  ngoài cơ chế mà IBus/Fcitx5 đã cấp cho engine.

## Rủi ro và cách xử lý

| Rủi ro | Giảm thiểu |
| --- | --- |
| IBus và Fcitx5 có API C/C++ khác nhau | Giữ adapter mỏng, không đưa logic gõ vào adapter; contract core được test độc lập. |
| Wayland/compositor không cấp surrounding text | Dùng preedit/commit chính thức trước; chỉ bật delete-surrounding khi framework xác nhận hỗ trợ; test theo desktop session. |
| Config bị hai adapter/UI ghi đồng thời | Một framework được chọn mỗi session; atomic write, khóa liên tiến trình và reload snapshot. |
| Dependency khác nhau giữa 22.04 và 24.04 | CI matrix theo LTS, build package riêng từng series và kiểm tra dependency bằng `dpkg-shlibdeps`. |
| Port vô tình thay đổi hành vi macOS | Giữ module macOS và test hiện có; Linux code ở crate/module riêng, không sửa hot path macOS trừ refactor có regression test. |

## Trình tự thực hiện

1. Trích xuất core/session/config đa nền tảng và bảo toàn test macOS hiện có.
2. Cài IBus adapter, component, unit/smoke test và package `ibus-oreokey`.
3. Cài Fcitx5 addon, test và package `fcitx5-oreokey` cùng contract core.
4. Cài GTK config app, config reload, tài liệu cài/gỡ và test luồng thật.
5. Thêm Debian packaging, CI matrix, artifact checksum và checklist release.

Mỗi bước phải giữ build/test của các bước trước xanh. Không phát hành package
Ubuntu cho tới khi đã kiểm thử tay IBus và Fcitx5 trên desktop session thật của
mỗi LTS mục tiêu.
