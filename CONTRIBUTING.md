# Đóng góp cho OreoKey

Cảm ơn bạn quan tâm! Vài hướng dẫn ngắn để đóng góp trơn tru.

## Quy trình

1. Mở [issue](https://github.com/OreoSolutions/oreokey/issues) mô tả lỗi/ý
   tưởng trước khi làm thay đổi lớn, để tránh trùng công sức.
2. Fork, tạo nhánh riêng, gửi Pull Request.
3. Giữ PR nhỏ, một mục đích. Kèm test khi sửa/ thêm hành vi engine.

## Chuẩn mã

- **Engine (`core/src/engine/`) chỉ dùng `std`** — không thêm thư viện ngoài.
- Phụ thuộc mới phải là giấy phép **thoáng** (MIT/Apache-2.0/BSD). **Không
  chép mã GPL** (OpenKey/Unikey/vi-rs/goxkey). Đây là điều kiện sống còn để
  giữ giấy phép MIT.
- Chạy `cd core && cargo test` và `cargo clippy` trước khi gửi. Thêm test cho
  luật gõ mới (xem các `#[test]` sẵn có trong `telex.rs`, `vni.rs`).
- Build qua `./scripts/build.sh` (đừng tự dựng chuỗi build khác).

## Chứng nhận nguồn gốc (DCO)

Dự án dùng [Developer Certificate of Origin](https://developercertificate.org/)
thay cho CLA. Mỗi commit cần dòng ký tên xác nhận bạn có quyền đóng góp phần
mã đó:

```
Signed-off-by: Tên Bạn <email@example.com>
```

Thêm tự động bằng `git commit -s`. Bằng việc ký, bạn xác nhận phần đóng góp là
của bạn (hoặc bạn có quyền gửi) và đồng ý phát hành theo giấy phép MIT của dự án.

## Giấy phép đóng góp

Mọi đóng góp mã nguồn được cấp phép theo [MIT](LICENSE). Lưu ý tên và logo
"OreoKey" là nhãn hiệu, không nằm trong giấy phép mã nguồn.
