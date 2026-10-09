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
# Mốc chấp nhận: 1.662 lỗi (0.7.5) với từ điển 8.581 âm tiết; từ điển đầy đủ
# (9.061 âm tiết, có chữ ư, đủ người/được/thương) là 1.800 lỗi. Phải trích
# từ điển với bảng chữ cái gồm ư — bản cũ lọc mất ư nên không quét các từ đó.
# Lệch lên = hồi quy, dừng lại.
cargo build --release --bin sweep
./target/release/sweep <syllables.txt> --method both --out /tmp/sweep.jsonl
./target/release/sweep <syllables.txt> --method both --modern --out /tmp/sweep-modern.jsonl
```

Sweep phải so **tập lỗi**, không chỉ đếm: hai bản có thể cùng số lỗi mà lỗi
khác nhau. Sort hai file rồi `comm -13 cũ mới` để lấy lỗi *mới* — mọi dòng
trong đó phải được truy nguyên (từ điển noise / từ mượn / từ đa âm tiết là
lành tính; từ phổ thông là hồi quy).

Cờ chẩn đoán (ngoài gate — mốc trên chỉ áp cho lệnh mặc định: mức Thường,
không Backspace): `--mode strict|standard|loose` đổi mức chính tả;
`--backspace N` gõ xong, xóa N ký tự rồi gõ lại chúng (chỉ biến thể gõ thẳng
đã đúng và có N ký tự cuối là chữ ASCII trơn; số bị bỏ qua ở
`backspace_skipped`). Mỗi lỗi kèm `why`: `untracked` (engine mất dấu từ sau
khi xóa — gõ tắt, phím dấu sau đó sẽ sai), `screen` (gõ lại ra sai chữ),
`replace` (Replace không khớp màn hình). Trong `keys`, `<` là Backspace.

Điền `CHANGELOG.md` mục `## [Chưa phát hành]`, theo Keep a Changelog
(`### Đã thêm` / `### Đã đổi` / `### Đã sửa`). Viết cho người dùng: hiện
tượng họ thấy → giờ ra sao → ví dụ phím gõ. `roll-changelog.py` sẽ cuốn
mục này thành `[X.Y.Z] - ngày` và dùng làm release notes trên GitHub lẫn
trong cửa sổ Sparkle, nên nội dung phải tự đứng được.

Chọn số phiên bản theo SemVer: chỉ sửa lỗi → tăng patch (0.7.4 → 0.7.5);
thêm tính năng → tăng minor.

## 2. Phát hành

```bash
CODESIGN_ID="<SHA-1 của cert>" ./scripts/release.sh 0.7.5
```

Nên đặt `CODESIGN_ID` bằng **mã băm SHA-1** của chứng chỉ (lấy bằng
`security find-identity -v -p codesigning`) thay vì tên: khi keychain có hai
cert cùng tên (vd cert cũ chưa hết hạn + cert mới), tên gây lỗi
"ambiguous", còn mã băm luôn chọn đúng một cert.

Script tự làm theo thứ tự, dừng ngay nếu lỗi:

1. Kiểm tra tiền đề: nhánh `main`, cây sạch, `HEAD == origin/main` (sau
   `git fetch`), tag chưa tồn tại, `gh` đã đăng nhập, có `CODESIGN_ID`.
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

### Khôi phục khi script chết giữa chừng

Script in sẵn gợi ý khôi phục khi lỗi. Hai trường hợp:

- **Chết trước khi tag** (bước 1-6: cây không sạch, build/notarize/ký lỗi,
  changelog trống...): chưa có gì lên remote, nhưng Info.plist, Cargo.toml,
  CHANGELOG.md, appcast.xml có thể đã bị sửa dở. Hoàn tác bằng
  `git checkout -- app/Info.plist core/Cargo.toml Cargo.lock CHANGELOG.md appcast.xml`
  (cây không sạch thì script dừng ngay từ đầu, chưa sửa gì), sửa nguyên nhân
  rồi chạy lại.
- **Chết sau khi tag** (bước 7 trở đi): nếu GitHub Release đã tạo, xóa cả
  release lẫn tag remote bằng `gh release delete vX.Y.Z --cleanup-tag --yes`;
  nếu chưa có release thì `git push origin :refs/tags/vX.Y.Z`. Sau đó
  `git tag -d vX.Y.Z && git reset --hard origin/main`, sửa nguyên nhân rồi
  chạy lại. `roll-changelog.py` idempotent nên chạy lại không tạo mục
  changelog trùng.

## 3. Sau khi phát hành

- Mở `https://raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml`
  — item mới có `sparkle:shortVersionString` đúng, `enclosure url` trỏ tới
  release vừa tạo, `length` khớp kích cỡ DMG.
- Trên một máy đang chạy bản cũ: dòng phiên bản ở chân sidebar **Cài đặt**
  (bấm để kiểm tra bản mới) phải thấy bản
  mới, changelog hiển thị đúng định dạng (đậm, mã), cài xong app khởi động
  lại được và **vẫn còn quyền Accessibility** (bản ký Developer ID giữ
  được quyền; bản ad-hoc thì không — xem README).
- Gõ thử đúng các ca trong changelog trên app đã cài (không phải bản dev).
- Đóng issue liên quan trên GitHub, dẫn link release.
