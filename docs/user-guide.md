# Hướng dẫn sử dụng OreoKey

Hướng dẫn đầy đủ mọi tính năng. Chưa cài? Xem [Hướng dẫn cài đặt](install.md).

## Mục lục

- [Menu bar](#menu-bar)
- [Bật / tắt tiếng Việt](#bật--tắt-tiếng-việt)
- [Kiểu gõ Telex và VNI](#kiểu-gõ-telex-và-vni)
- [Kiểm tra chính tả](#kiểm-tra-chính-tả)
- [Các tuỳ chọn hành vi gõ](#các-tuỳ-chọn-hành-vi-gõ)
- [Gõ tắt (macro)](#gõ-tắt-macro)
- [Cài đặt theo ứng dụng](#cài-đặt-theo-ứng-dụng)
- [Xử lý app bị nháy / dính chữ](#xử-lý-app-bị-nháy--dính-chữ)
- [Cập nhật](#cập-nhật)
- [Câu hỏi thường gặp](#câu-hỏi-thường-gặp)

## Menu bar

Biểu tượng huy hiệu trên menu bar cho biết trạng thái:

- **VN** (nền đặc) — đang gõ tiếng Việt.
- **EN** (viền mảnh) — đang tắt, gõ tiếng Anh bình thường.

Bấm vào biểu tượng để mở menu: bật/tắt tiếng Việt, chọn kiểu gõ
Telex/VNI, mở **Cài đặt…** (`⌘,`) và thoát app.

## Bật / tắt tiếng Việt

Ba cách, tác dụng như nhau:

- Phím tắt — mặc định `⌃Space` (Control+Space).
- Bấm mục **Tiếng Việt** trong menu bar.
- Tự động theo app (xem [Cài đặt theo ứng dụng](#cài-đặt-theo-ứng-dụng)).

Đổi phím tắt trong **Cài đặt → Chung**, các lựa chọn: `⌃⇧Space`,
`⌃Space`, `⌘⇧Space`, `⌥Z`.

Khi bật tiếng Việt, OreoKey tự chuyển input source hệ thống về bàn phím
Latin (ABC) để không xung đột với bộ gõ khác.

## Kiểu gõ Telex và VNI

Chọn trong menu bar (mục *Kiểu gõ*) hoặc **Cài đặt → Chung**.

### Telex

| Gõ | Ra | | Gõ | Ra |
|----|----|-|----|----|
| `aa` | â | | `s` | dấu sắc |
| `aw` | ă | | `f` | dấu huyền |
| `ee` | ê | | `r` | dấu hỏi |
| `oo` | ô | | `x` | dấu ngã |
| `ow` | ơ | | `j` | dấu nặng |
| `uw` | ư | | `z` | xóa dấu |
| `dd` | đ | | | |

Ví dụ: `vieejt` → *việt*, `dduwowngf` → *đường*.

### VNI

| Gõ | Ra | | Gõ | Ra |
|----|----|-|----|----|
| `a6` | â | | `1` | dấu sắc |
| `a8` | ă | | `2` | dấu huyền |
| `e6` | ê | | `3` | dấu hỏi |
| `o6` | ô | | `4` | dấu ngã |
| `o7` | ơ | | `5` | dấu nặng |
| `u7` | ư | | `0` | xóa dấu |
| `d9` | đ | | | |

Ví dụ: `vie65t` → *việt*, `d9u7o72ng` → *đường*.

## Kiểm tra chính tả

**Cài đặt → Chung → Hành vi gõ → Kiểm tra chính tả** — thanh trượt 3
mức, kéo càng cao kiểm tra càng chặt:

| Mức | Hành vi |
|-----|---------|
| **Thoải mái** | Luôn đặt dấu, không khôi phục từ tiếng Anh. |
| **Thường** | Cho gõ tắt kiểu chat (đc, nèk), vẫn nhận diện từ tiếng Anh có cụm bất khả (clear, sound). |
| **Chặt** | Bảo vệ tối đa từ tiếng Anh (mask, class…) — từ không hợp âm tiết tiếng Việt tự khôi phục về dạng gõ thô. |

Khôi phục nghĩa là: gõ `class` ở mức Chặt, chữ không bị biến thành
*clạs* mà giữ nguyên *class*.

## Các tuỳ chọn hành vi gõ

Trong **Cài đặt → Chung → Hành vi gõ**:

- **Gõ dấu mũ linh hoạt** — chấp nhận phím dấu đặt xa: `nanag` → *nâng*,
  `viete` → *viêt*.
- **Đặt dấu kiểu mới** — *hoà, thuý* thay vì *hòa, thúy*.
- **Gõ tắt** — bật/tắt toàn bộ macro định nghĩa trong mục Gõ tắt.
- **Che từ tục tĩu** — từ nhạy cảm tự thay bằng dấu `*` khi chốt từ.

## Gõ tắt (macro)

**Cài đặt → Gõ tắt** — bảng các cụm viết tắt tự bung khi chốt từ
(gõ xong cụm rồi nhấn phím cách/dấu câu):

- Thêm: điền ô *Gõ tắt* (vd `vn`) và *Thay bằng* (vd `Việt Nam`), bấm **+**.
- Xóa: chọn dòng trong bảng, bấm **−**.
- Cụm trùng tên sẽ ghi đè cụm cũ.

Nhớ bật công tắc **Gõ tắt** trong tab Chung thì macro mới chạy.

## Cài đặt theo ứng dụng

**Cài đặt → Ứng dụng**:

- **Nhớ trạng thái theo ứng dụng** — mỗi app giữ trạng thái VN/EN riêng;
  chuyển qua lại app nào giữ nguyên app đó.
- **Tắt tiếng Việt trong các ứng dụng** — danh sách app luôn khởi động ở
  chế độ tiếng Anh (Terminal, IDE…). Phím tắt vẫn bật lại tạm được khi cần.
- **Chế độ tương thích** — override cách bơm chữ cho từng app bị lỗi
  hiển thị (xem mục dưới).

Cả hai danh sách đều có menu **Thêm…**: app đang chạy hiện sẵn để chọn;
app chưa chạy thì dùng **"Nhập bundle ID…"** — lấy ID bằng:

```bash
osascript -e 'id of app "Tên App"'
# hoặc
mdls -name kMDItemCFBundleIdentifier -raw "/Applications/Tên App.app"
```

## Xử lý app bị nháy / dính chữ

OreoKey sửa chữ theo 4 tầng (Accessibility API → diff tối thiểu → gộp
event → bảng quirk theo app) nên đa số app chạy tốt sẵn. Nếu **một app
cụ thể** vẫn nháy chữ hoặc dính chữ:

1. Mở **Cài đặt → Ứng dụng → Chế độ tương thích**.
2. **Thêm override…** → chọn app đang lỗi.
3. Thử chế độ theo thứ tự:
   - **Bơm phím nhanh** — hợp phần lớn app bị nháy (terminal, app
     Java/Swing, Electron).
   - **Bơm phím chậm** — cho app tự điền lại nội dung sau mỗi phím
     (Word/Excel/PowerPoint, vài trình soạn thảo online).
   - **Tự động** — mặc định; đưa về đây nếu muốn hoàn tác.

Các terminal phổ biến (Terminal, iTerm2, kitty, Alacritty, WezTerm,
Ghostty, Warp, Hyper, VS Code, JetBrains) đã được đặt sẵn chế độ phù hợp.

> **VS Code + extension markdown** (vd Markdown All in One): extension xử
> lý thêm sau mỗi lần buffer đổi nên file `.md` có thể nháy/dính dù file
> khác gõ bình thường. Đặt override **Bơm phím chậm** cho VS Code theo các
> bước trên, hoặc tắt `markdown.extension.completion.enabled` trong
> settings của VS Code.

Tìm được chế độ chạy tốt cho app chưa có sẵn hồ sơ? Mở
[issue](https://github.com/OreoSolutions/oreokey/issues) gửi *bundle ID +
tên app + chế độ* để bản sau hỗ trợ mặc định cho mọi người.

## Cập nhật

- App tự kiểm tra bản mới định kỳ ở chế độ nền (Sparkle).
- Chân sidebar cửa sổ Cài đặt luôn hiện phiên bản hiện tại: dấu tick
  xanh ✓ *Mới nhất* khi đang là bản mới nhất, cảnh báo cam ⚠ khi có bản
  mới — bấm vào dòng đó để kiểm tra/cài đặt.

## Câu hỏi thường gặp

**Gõ không ra dấu, app không phản ứng?**
Kiểm tra quyền Accessibility: System Settings → Privacy & Security →
Accessibility → OreoKey phải ON. Nếu ON mà vẫn không chạy, tắt/bật lại
công tắc hoặc `tccutil reset Accessibility com.oreosolutions.oreokey`
rồi cấp lại.

**Gõ ra chữ đôi (vd "masster")?**
Bản 0.5.2 trở lên đã có lưới chặn phím ma. Nếu vẫn gặp, cập nhật bản
mới nhất rồi báo lỗi kèm mô tả app đang dùng.

**Từ tiếng Anh bị biến dạng khi gõ (class → clạs)?**
Tăng mức **Kiểm tra chính tả** lên *Chặt* — từ không hợp âm tiết tiếng
Việt sẽ tự khôi phục.

**Muốn gõ tiếng Anh trong Terminal/IDE mà không phải bấm hotkey?**
Thêm app vào **Cài đặt → Ứng dụng → Tắt tiếng Việt trong các ứng dụng**.

**Cấu hình lưu ở đâu?**
`~/Library/Application Support/OreoKey/settings.json` — xóa thư mục này
là về mặc định.
