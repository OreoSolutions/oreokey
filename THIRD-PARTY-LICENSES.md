# Giấy phép bên thứ ba

OreoKey dùng các thư viện mã nguồn mở dưới đây. Tất cả đều theo giấy phép
**thoáng (permissive)** — tương thích với MIT và cho phép phân phối trong
sản phẩm này. Không có thư viện nào theo copyleft (GPL/LGPL/MPL).

Cập nhật danh sách khi đổi phụ thuộc: `cargo install cargo-license && cargo license`.

## Phụ thuộc Rust

| Thư viện | Giấy phép |
|---|---|
| bitflags | MIT OR Apache-2.0 |
| core-foundation, core-foundation-sys | MIT OR Apache-2.0 |
| core-graphics, core-graphics-types | MIT OR Apache-2.0 |
| foreign-types, foreign-types-macros, foreign-types-shared | MIT OR Apache-2.0 |
| itoa | MIT OR Apache-2.0 |
| libc | MIT OR Apache-2.0 |
| memchr | Unlicense OR MIT |
| proc-macro2 | MIT OR Apache-2.0 |
| quote | MIT OR Apache-2.0 |
| serde, serde_core, serde_derive | MIT OR Apache-2.0 |
| serde_json | MIT OR Apache-2.0 |
| syn | MIT OR Apache-2.0 |
| unicode-ident | (MIT OR Apache-2.0) AND Unicode-3.0 |

## Khung nền tảng của Apple

Ứng dụng liên kết với các framework hệ thống của macOS (AppKit, SwiftUI,
ApplicationServices/Accessibility, Carbon/TIS, CoreGraphics). Đây là API của
hệ điều hành, dùng theo thỏa thuận cấp phép Apple, không kèm vào mã nguồn này.

## Nguồn gốc engine (clean-room)

Engine gõ tiếng Việt (Telex/VNI, kiểm tra chính tả, đặt dấu) được **viết lại
từ đầu**. Không sao chép mã nguồn của các bộ gõ khác (OpenKey, Unikey, vi-rs,
goxkey — vốn theo GPL). Các bảng mã hóa/quy tắc âm vị học được xem là **dữ
liệu (facts)**, không phải mã nguồn có bản quyền.
