# OreoKey

Bộ gõ tiếng Việt cho macOS — nhanh, nhẹ RAM, và tập trung xử lý triệt để
hai lỗi kinh điển của các bộ gõ event-tap: **dính chữ** và **nháy chữ**.

- Telex + VNI, kiểm tra chính tả (tự khôi phục từ tiếng Anh), gõ tắt,
  loại trừ app / nhớ trạng thái theo app, chuyển mã Unicode/VNI-Windows/TCVN3
- Engine viết bằng Rust (thư viện tĩnh ~1MB), UI Swift/AppKit + SwiftUI
- macOS 13+, chạy nền dạng menu bar, RAM ~20MB

## Chống dính/nháy chữ

Sửa chữ theo 4 tầng, tốt nhất trước:

1. **Accessibility API** — thay thẳng đoạn text quanh con trỏ trong một
   thao tác nguyên tử (TextEdit, Notes, Safari...): không backspace nào
   được gửi → không thể nháy/dính.
2. **Diff tối thiểu** — khi phải bơm phím, chỉ xóa phần đuôi thực sự đổi
   (`vieet`→`viêt` = 2 backspace + `êt`, không phải xóa cả từ).
3. **Gộp event** — chuỗi thay thế gửi trong ít event nhất, thứ tự chặt.
4. **Bảng quirk theo app** — `data/app-profiles.json` (đóng gói kèm app):
   Chrome/Safari có fix autocomplete thanh địa chỉ, Excel bơm chậm,
   VS Code/JetBrains/Electron bơm nhanh không AX. Người dùng override
   từng app trong Cài đặt → Ứng dụng mà không cần chờ bản mới.

## Build

```bash
./scripts/build.sh              # build dev (máy hiện tại) → dist/OreoKey.app
./scripts/build.sh --universal  # universal binary (arm64 + x86_64)
./scripts/make-dmg.sh           # đóng gói DMG
```

Yêu cầu: Rust (cargo), Xcode Command Line Tools. Test engine: `cargo test`.

Lần chạy đầu app sẽ hướng dẫn cấp quyền Accessibility (bắt buộc để chặn
phím toàn hệ thống). Kiểm thử tay theo `docs/testing-checklist.md`.

**Lưu ý khi dev**: bản build ký ad-hoc → mỗi lần rebuild, macOS coi là
app khác và quyền Accessibility cũ thành vô hiệu (công tắc vẫn hiện ON
nhưng không có tác dụng). Xử lý: `tccutil reset Accessibility
com.oreosolutions.oreokey` rồi cấp lại, hoặc tắt/bật công tắc trong
System Settings. Bản phát hành ký Developer ID không bị vấn đề này.

## Phát hành thật (cần tài khoản Apple Developer)

1. `CODESIGN_ID="Developer ID Application: ..." ./scripts/build.sh --universal`
2. `CODESIGN_ID=... NOTARY_PROFILE=... ./scripts/make-dmg.sh` (ký + notarize + staple)
3. Auto-update (Sparkle) chưa tích hợp — cần: thêm dependency
   `sparkle-project/Sparkle` vào `app/Package.swift`, nhúng framework vào
   bundle trong `build.sh`, sinh cặp khóa EdDSA, host `appcast.xml` và đặt
   `SUFeedURL` trong `app/Info.plist`.

## Kiến trúc

```
core/   — Rust: engine gõ thuần (re-render + diff), spell check,
          macro, chuyển mã, config (chủ sở hữu duy nhất),
          platform macOS (CGEventTap, AX API, bơm phím, quirk)
app/    — Swift: menu bar (AppKit), Cài đặt 4 tab (SwiftUI),
          onboarding Accessibility. Không nằm trên đường đi của phím.
data/   — app-profiles.json: quirk mặc định theo bundle ID
```

Thiết kế chi tiết: `docs/superpowers/specs/2026-07-08-oreokey-design.md`.

## Báo lỗi & đóng góp

- Báo lỗi: https://github.com/OreoSolutions/oreokey/issues
- Đóng góp: xem [CONTRIBUTING.md](CONTRIBUTING.md).

## Giấy phép

Mã nguồn theo **[MIT License](LICENSE)** — miễn phí, dùng lại tự do. Giấy
phép các thư viện bên thứ ba (đều permissive, không GPL):
[THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md).

Engine được viết lại từ đầu, **không sao chép mã GPL** của các bộ gõ khác.

### Nhãn hiệu

Giấy phép MIT chỉ áp dụng cho **mã nguồn**. Tên **"OreoKey"** và **logo** là
nhãn hiệu của Oreo Solutions, **không** thuộc phạm vi MIT. Nếu bạn fork hoặc
phát hành bản chỉnh sửa, vui lòng **đổi tên và logo** và không ngụ ý có liên
kết/chứng thực từ Oreo Solutions.
