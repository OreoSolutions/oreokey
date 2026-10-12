# Checklist kiểm thử tay OreoKey

Chạy trước mỗi bản phát hành. Trọng tâm: **không dính chữ, không nháy chữ**.

## Chuẩn bị

- [ ] Build: `./scripts/build.sh` → mở `dist/OreoKey.app`
- [ ] Onboarding hiện khi chưa có quyền Accessibility; tự đóng sau khi cấp
- [ ] Icon menu bar hiện `VN`; hotkey ⌃⇧Space đổi `VN` ↔ `EN`

## Gõ cơ bản (mỗi app trong ma trận bên dưới)

Câu chuẩn: `Tôi yêu tiếng nước tôi từ khi mới ra đời` (Telex:
`Toooi yeeu tieengs nuwowcs tooi tuwf khi mowis ra ddowif` — gõ tự nhiên)

- [ ] Không dính chữ khi gõ nhanh liên tục (~120 wpm burst)
- [ ] Không nháy rõ rệt khi bỏ dấu (`vieetj` — quan sát lúc gõ `j`)
- [ ] Backspace giữa từ rồi gõ tiếp: không lệch buffer
- [ ] Gõ tiếng Anh với spell check bật: `expression`, `windows`, `class` giữ nguyên
- [ ] Gõ tắt: định nghĩa `vn → Việt Nam`, gõ `vn␣` ra `Việt Nam ` (cả trước dấu phẩy)

## Ma trận app

| App | Kênh sửa chữ kỳ vọng | Đạt |
|---|---|---|
| TextEdit / Notes | AX (không nháy) | ☐ |
| Safari — trang web | AX | ☐ |
| Safari — thanh địa chỉ (autocomplete!) | browser_fix | ☐ |
| Chrome — trang web | inject_fast (không nháy bôi đen) | ☐ |
| Chrome — thanh địa chỉ (autocomplete!) | browser_fix | ☐ |
| Excel | inject_slow | ☐ |
| Word | inject_slow | ☐ |
| Messenger | inject_fast | ☐ |
| Notion | inject_fast | ☐ |
| VS Code | inject_fast | ☐ |
| JetBrains (bất kỳ) | inject_fast (wildcard) | ☐ |
| Terminal / iTerm2 | inject_fast | ☐ |
| Spotlight | mặc định | ☐ |
| Slack / Discord / Telegram | inject_fast | ☐ |

## Trường hợp đặc biệt

- [ ] Ô mật khẩu (Safari, cửa sổ đăng nhập): OreoKey không can thiệp
- [ ] Click chuột giữa từ đang gõ → gõ tiếp không phá chữ cũ
- [ ] Mũi tên/Home/End giữa từ → buffer reset
- [ ] ⌘C/⌘V trong lúc gõ dở → không sinh ký tự lạ
- [ ] Giữ phím `a` (autorepeat) → ra `aaaa`, không thành `â`
- [ ] App trong danh sách loại trừ: tự về EN; hotkey vẫn bật lại tạm được
- [ ] "Nhớ theo app": tắt VN trong Terminal, chuyển Chrome vẫn VN, quay lại Terminal vẫn EN
- [ ] Đổi kiểu gõ Telex ↔ VNI từ menu có hiệu lực ngay
- [ ] Viết hoa đầu câu (bật trong Chung): `xong. vieejt` → `xong. Việt`; `google.com` không viết hoa; Enter rồi gõ → viết hoa
- [ ] Esc hoàn về phím gốc (bật trong Chung): `vieejt` + Esc → `vieejt`, app không nhận Esc; Esc khi không gõ dở → app nhận Esc bình thường
- [ ] Gõ tắt `ko → không`: gõ `Ko␣` ra `Không `, `KO␣` ra `KHÔNG `
- [ ] Gõ `go go go` rồi ⌫⌫⌫ rồi `x` → `go gõ`; click chuột rồi ⌫ thì không nối lại
- [ ] Thêm bàn phím Japanese (Romaji) trong System Settings; chuyển sang nó → icon EN, gõ ra tiếng Nhật; về ABC → icon VN lại; bấm hotkey khi đang ở Japanese → về ABC và VN bật
- [ ] Tab Chuyển mã: dán `Vieät Nam` (VNI-Windows) → Unicode ra `Việt Nam`; nút Chép kết quả đưa vào clipboard
- [ ] Sleep/wake máy → gõ vẫn hoạt động (tap tự phục hồi)
- [ ] Thu hồi quyền Accessibility khi đang chạy → onboarding hiện lại (cần chạy lại app)

## Hiệu năng

- [ ] RAM (Activity Monitor) sau 30 phút gõ: < 30MB
- [ ] Không cảm nhận được độ trễ phím ở chế độ Auto/InjectFast
