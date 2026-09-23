# Quy trình phát hành

Phát hành chạy **tại máy dev** bằng `scripts/release.sh` — khóa ký không rời
máy. Tài liệu này là checklist quanh script đó: phần script không tự kiểm
(chất lượng engine) và phần cần làm tay sau khi nó chạy xong.

## 1. Trước khi phát hành

```bash
# Cây làm việc sạch, đang ở main, đã pull
git status --porcelain && git pull --ff-only

# Toàn bộ unit test core
cargo test -p oreokey-core --lib

# Gate sweep toàn từ điển — bắt hồi quy engine mà unit test không phủ.
# Từ điển: 8.581 âm tiết trích từ Viet74K (xem docs/dict-sweep-report.md).
# Mốc chấp nhận hiện tại: 1.662 lỗi (0.7.5). Lệch lên = hồi quy, dừng lại.
cargo build --release --bin sweep
./target/release/sweep <syllables.txt> --method both --out /tmp/sweep.jsonl
./target/release/sweep <syllables.txt> --method both --modern --out /tmp/sweep-modern.jsonl
```

Sweep phải so **tập lỗi**, không chỉ đếm: hai bản có thể cùng số lỗi mà lỗi
khác nhau. Sort hai file rồi `comm -13 cũ mới` để lấy lỗi *mới* — mọi dòng
trong đó phải được truy nguyên (từ điển noise / từ mượn / từ đa âm tiết là
lành tính; từ phổ thông là hồi quy).

Điền `CHANGELOG.md` mục `## [Chưa phát hành]`, theo Keep a Changelog
(`### Đã thêm` / `### Đã đổi` / `### Đã sửa`). Viết cho người dùng: hiện
tượng họ thấy → giờ ra sao → ví dụ phím gõ. `roll-changelog.py` sẽ cuốn
mục này thành `[X.Y.Z] - ngày` và dùng làm release notes trên GitHub lẫn
trong cửa sổ Sparkle, nên nội dung phải tự đứng được.

Chọn số phiên bản theo SemVer: chỉ sửa lỗi → tăng patch (0.7.4 → 0.7.5);
thêm tính năng → tăng minor.

## 2. Phát hành

```bash
CODESIGN_ID="Developer ID Application: Tên (TEAMID)" ./scripts/release.sh 0.7.5
```

Script tự làm theo thứ tự, dừng ngay nếu lỗi:

1. Kiểm tra tiền đề: nhánh `main`, cây sạch, tag chưa tồn tại, `gh` đã đăng
   nhập, có `CODESIGN_ID`.
2. Bump version: `CFBundleShortVersionString` + tăng `CFBundleVersion` trong
   `app/Info.plist`, `version` trong `core/Cargo.toml` (Cargo.lock theo).
3. Cuốn changelog (`scripts/roll-changelog.py`) → release notes.
4. `scripts/build.sh --universal` → `scripts/make-dmg.sh`: build, ký
   Developer ID, notarize (`NOTARY_PROFILE`, mặc định `oreokey-notary`),
   staple, đóng DMG vào `dist/OreoKey.dmg`.
5. Ký EdDSA cho Sparkle bằng `sign_update` (khóa riêng trong login keychain).
6. Chèn `<item>` mới vào `appcast.xml` (`scripts/update-appcast.py`).
7. Commit `release: X.Y.Z` gồm Info.plist, Cargo.toml, Cargo.lock,
   CHANGELOG.md, appcast.xml; tag `vX.Y.Z`; push tag.
8. Tạo GitHub Release kèm DMG — **trước** khi appcast lên `main`, để Sparkle
   của người dùng không gặp 404.
9. Push `main`.
10. Sync changelog sang repo website (`../oreokey-website`, hoặc
    `OREOKEY_WEBSITE_DIR`) và push — Vercel tự deploy. Lỗi ở bước này chỉ
    cảnh báo, không hủy release.

Nếu script chết giữa chừng sau bước 7 (đã tag): xóa tag local + remote
(`git tag -d vX.Y.Z && git push origin :vX.Y.Z`), `git reset --hard
origin/main`, sửa nguyên nhân rồi chạy lại. `roll-changelog.py` idempotent
nên chạy lại không tạo mục changelog trùng.

## 3. Sau khi phát hành

- Mở `https://raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml`
  — item mới có `sparkle:shortVersionString` đúng, `enclosure url` trỏ tới
  release vừa tạo, `length` khớp kích cỡ DMG.
- Trên một máy đang chạy bản cũ: menu **"Kiểm tra bản mới…"** phải thấy bản
  mới, changelog hiển thị đúng định dạng (đậm, mã), cài xong app khởi động
  lại được và **vẫn còn quyền Accessibility** (bản ký Developer ID giữ
  được quyền; bản ad-hoc thì không — xem README).
- Gõ thử đúng các ca trong changelog trên app đã cài (không phải bản dev).
- Đóng issue liên quan trên GitHub, dẫn link release.
