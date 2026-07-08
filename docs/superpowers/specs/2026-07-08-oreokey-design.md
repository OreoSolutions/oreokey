# OreoKey — Bộ gõ tiếng Việt cho macOS — Thiết kế

**Ngày:** 2026-07-08
**Trạng thái:** Đã duyệt thiết kế, chờ lập kế hoạch triển khai

## Mục tiêu & bối cảnh

Bộ gõ tiếng Việt cho macOS, phát hành rộng rãi dưới dạng **miễn phí, closed source**. Ưu tiên hàng đầu: **tốc độ, nhẹ RAM**, và **khắc phục hai lỗi cố hữu của các bộ gõ hiện có (OpenKey/EVKey): dính chữ và nháy chữ**.

Vì closed source: **không dùng lại bất kỳ code GPL nào** (OpenKey, goxkey, vi-rs chỉ tham khảo kiến trúc, không copy code). Mọi dependency phải có giấy phép permissive (MIT/Apache-2.0/BSD) — kiểm tra từng crate/framework khi lập kế hoạch.

## Phạm vi v1

- Kiểu gõ: **Telex** và **VNI**
- Kiểm tra chính tả (âm tiết hợp lệ) — tự khôi phục phím gốc với từ không hợp lệ
- Gõ tắt (macro) do người dùng định nghĩa
- Loại trừ app / smart switch (nhớ trạng thái VN–EN theo từng app)
- Chuyển mã văn bản: Unicode ↔ VNI-Windows ↔ TCVN3
- Phím tắt toàn cục bật/tắt tiếng Việt, icon menu bar, ghi nhớ trạng thái, khởi động cùng máy
- Chống dính chữ / nháy chữ (mục 3b — trọng tâm khác biệt hóa)
- Phát hành: ký Developer ID + notarize, DMG, tự cập nhật qua Sparkle

Ngoài phạm vi v1: VIQR, Mac App Store (không khả thi với event tap), Windows/Linux (engine thiết kế sẵn để port sau).

## 1. Kiến trúc tổng quan

App chạy nền dạng menu bar (`LSUIElement`, không icon Dock), hai tầng:

```
┌─────────────────────────────────────────┐
│  OreoKey.app (Swift)                    │
│  • Menu bar (AppKit/NSStatusItem)       │
│  • Cửa sổ Cài đặt (SwiftUI, load khi mở)│
│  • Onboarding xin quyền Accessibility   │
└──────────────┬──────────────────────────┘
               │ FFI (C ABI)
┌──────────────┴──────────────────────────┐
│  oreokey-core (Rust, thư viện tĩnh)     │
│  • engine   — bộ gõ thuần (không OS)    │
│  • platform — CGEventTap, bơm phím,     │
│               AX API, app đang focus    │
│  • config   — settings, macro, lưu JSON │
└─────────────────────────────────────────┘
```

**Nguyên tắc ranh giới:**

- `engine` là module thuần túy: đầu vào là phím, đầu ra là hành động sửa chữ. Không import gì từ OS. Test được 100% bằng unit test.
- Toàn bộ luồng nóng (chặn phím → xử lý → bơm phím) nằm trọn trong Rust. Swift không nằm trên đường đi của phím.
- Swift chỉ làm UI, gọi xuống Rust qua FFI để đọc/ghi cấu hình và bật/tắt.
- Cách chọn kiến trúc: event tap (như OpenKey/EVKey) thay vì IMKit — giữ trải nghiệm người Việt đã quen (không chuyển input source, không marked text gạch chân), đổi lại cần quyền Accessibility và không lên được Mac App Store (chấp nhận vì tự phát hành).

## 2. Engine gõ (Rust, thuần túy)

- **Bộ đệm theo từ:** giữ buffer từ đang gõ; ký tự ngắt từ (space, dấu câu, Enter, di chuyển con trỏ, click chuột) chốt và reset buffer. Backspace lùi buffer tương ứng.
- **Bảng luật Telex + VNI:** cùng một engine, hai bảng ánh xạ. Bao phủ các ca kinh điển:
  - Đặt dấu đúng vị trí, tùy chọn kiểu cũ/mới (`hòa` vs `hoà`)
  - Gõ dấu lặp để hủy (`ss` → trả lại `s`)
  - Gõ dấu muộn (`vietj` → `việt`)
  - `w` → `ư` đầu từ, `uw/ưo` các biến thể
- **Kiểm tra chính tả:** kiểm âm tiết tiếng Việt hợp lệ; từ không hợp lệ → tự trả về chuỗi phím gốc (tránh phá từ tiếng Anh khi quên tắt). Có thể tắt trong Cài đặt.
- **Gõ tắt (macro):** bảng người dùng định nghĩa, khớp khi chốt từ.
- **Chuyển mã:** hàm thuần chuyển chuỗi Unicode ↔ VNI-Windows ↔ TCVN3, dùng cho công cụ chuyển mã clipboard trong UI.
- **Đầu ra:** `enum Action { PassThrough, Replace { backspaces: usize, text: String } }` — dễ test, dễ port.

## 3. Tầng platform (Rust, macOS)

- `CGEventTap` chặn keydown toàn hệ thống. Khi engine trả `Replace`: nuốt phím gốc, thực hiện sửa chữ theo chiến lược mục 3b.
- **Smart switch / loại trừ app:** theo dõi app focus (NSWorkspace notification từ Swift báo xuống); nhớ trạng thái VN/EN theo bundle ID; tự tắt với app trong danh sách loại trừ.
- **Trường mật khẩu:** phát hiện Secure Input (`IsSecureEventInputEnabled`) → passthrough hoàn toàn.
- **Tự phục hồi:** nhận `tapDisabledByTimeout` / `tapDisabledByUserInput` → bật lại tap ngay.
- **Phím tắt toàn cục** (mặc định `⌃Space`, tùy chỉnh được) xử lý ngay trong tap.

## 3b. Chống dính chữ / nháy chữ (trọng tâm)

Nguyên nhân gốc của lỗi ở các bộ gõ event-tap hiện có: sửa chữ bằng N backspace + gõ lại cả từ → (1) app vẽ lại từng bước gây **nháy**; (2) sự kiện đến sai thứ tự / app xử lý không kịp gây **dính, lặp chữ**; (3) autocomplete trình duyệt bôi đen gợi ý khiến backspace đầu **xóa nhầm cả đoạn**.

Chiến lược xếp tầng, từ tốt nhất xuống dự phòng:

| Tầng | Chiến lược | Cơ chế |
|---|---|---|
| 1 | **Sửa trực tiếp qua Accessibility API** | Với app có text field chuẩn: dùng `AXUIElement` (`kAXSelectedTextRangeAttribute` + `kAXSelectedTextAttribute`) chọn N ký tự cuối và ghi đè bằng chuỗi mới trong một thao tác nguyên tử. Không backspace → không thể nháy/dính. |
| 2 | **Diff tối thiểu** | Khi phải dùng key injection: chỉ xóa phần đuôi thực sự thay đổi (`vieet`→`viêt`: 2 backspace + `êt`, không phải 5 + 4). Ít sự kiện = ít cơ hội lỗi. |
| 3 | **Gộp sự kiện, bơm có kiểm soát** | Chuỗi thay thế trong một event (`CGEventKeyboardSetUnicodeString` cả chuỗi); backspace + text đẩy cùng tap location, thứ tự chặt; độ trễ vi mô tùy chọn cho app chậm. |
| 4 | **Bảng quirk theo app** | Hồ sơ theo bundle ID: phương thức ưu tiên (AX/diff/gõ chậm), độ trễ, fix autocomplete trình duyệt (gửi phím vô hại hủy bôi đen gợi ý trước khi sửa). Ship sẵn hồ sơ cho app hay lỗi (Chrome, Excel, Messenger, Notion, JetBrains...). |

**Yêu cầu thiết kế kèm theo:**

- Bảng quirk là **dữ liệu, không phải code**: file JSON đóng gói kèm app, cập nhật được qua bản phát hành mà không đổi logic; người dùng override được từng app trong Cài đặt → Ứng dụng (chọn chế độ tương thích) — không phải chờ bản mới khi gặp app lạ bị lỗi.
- Kỳ vọng thực tế: AX API không phủ được mọi app (app không expose text field rơi về tầng 2–4); bảng quirk nuôi dần theo phản hồi người dùng.

## 4. Lớp vỏ Swift

- **Menu bar:** `NSStatusItem`, icon đổi theo trạng thái VN/EN, click đổi nhanh; menu: chọn kiểu gõ, mở Cài đặt, thoát.
- **Cài đặt (SwiftUI, 4 tab):**
  1. *Chung* — kiểu gõ, phím tắt, chính tả, kiểu đặt dấu, khởi động cùng máy (`SMAppService`)
  2. *Gõ tắt* — bảng CRUD macro
  3. *Ứng dụng* — danh sách loại trừ, nhớ trạng thái theo app, chế độ tương thích từng app
  4. *Công cụ* — chuyển mã văn bản (clipboard)
- **Onboarding:** hướng dẫn cấp quyền Accessibility từng bước, tự phát hiện khi đã cấp; phát hiện quyền bị thu hồi khi đang chạy → hiện lại onboarding.
- **Phát hành:** Developer ID + notarize, DMG, auto-update qua **Sparkle** (MIT, hợp lệ cho closed source).

## 5. Cấu hình & dữ liệu

- JSON tại `~/Library/Application Support/OreoKey/`: `settings.json`, `macros.json`, `app-profiles.json` (quirk overrides).
- **Rust là chủ sở hữu duy nhất của config.** Swift đọc/ghi qua FFI — một nguồn sự thật duy nhất.

## 6. FFI (C ABI)

Bề mặt tối thiểu giữa Swift và Rust: khởi động/dừng tap, bật/tắt tiếng Việt, get/set settings (JSON string), CRUD macro, đổi chế độ tương thích app, hàm chuyển mã. Dùng header C thuần hoặc `swift-bridge`.

## 7. Kiểm thử

- **Engine (trọng tâm):** golden test dày đặc — chuỗi phím vào → văn bản ra, bao phủ Telex/VNI, hủy dấu, dấu muộn, backspace giữa chừng, chính tả, macro, chuyển mã. Chạy bằng `cargo test`, không cần máy Mac.
- **FFI:** smoke test từ Swift gọi đủ bề mặt API.
- **Tích hợp (test tay, có checklist):** ma trận app tập trung ca dính/nháy — thanh địa chỉ Chrome/Safari, Excel, Messenger, Notion, Word, Spotlight, Terminal, VS Code, ô mật khẩu, và stress gõ nhanh liên tục trong app Electron.

## 8. Yêu cầu hệ thống

macOS 13 Ventura trở lên (cần `SMAppService`), Universal binary (Apple Silicon + Intel).
