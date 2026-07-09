# Thiết kế: Chế độ "gõ thoải mái" (loose spell-check)

**Ngày:** 2026-07-09
**Trạng thái:** Đã duyệt, chờ lập kế hoạch triển khai

## Mục tiêu

Dung hoà hai nhu cầu đang xung đột trong bộ gõ:

1. **Gõ tắt / cố tình sai** tiếng Việt (`đc`, `nèk`, `ko`…) mà không bị
   engine "sửa" ngược về phím gốc.
2. **Bảo vệ từ tiếng Anh** (`clear`, `sound`, `just`…) khỏi bị đặt dấu
   bừa (`clear` → `clẻa`).

Hiện tại hai nhu cầu này chia nhau một công tắc `spell_check` toàn cục, và
**đầu TẮT thì vô dụng** (đặt dấu mù mọi thứ). Thiết kế này biến đầu TẮT
thành một chế độ **thông minh vừa đủ**: cho gõ tắt tự do nhưng vẫn bắt
được từ tiếng Anh qua "cụm bất khả".

## Bối cảnh

- Engine gõ thuần Rust (`core/src/engine/`), kiến trúc re-render + diff.
- `spell.rs` quyết định một từ đã-bị-biến-đổi có phải âm tiết tiếng Việt
  hợp lệ không (`is_acceptable`); nếu không → engine trả về **phím gốc**
  (`raw_mode` latch tới khi ngắt từ).
- `is_acceptable` (strict) kiểm 3 bảng trắng + 4 luật cấu trúc:
  - `INITIALS` (28 phụ âm đầu), `NUCLEI` (~60 vần), `FINALS` (9 phụ âm cuối)
  - luật: nguyên âm liên tục; `đ` chỉ đứng đầu; phụ âm tắc cuối chỉ mang
    sắc/nặng; không nguyên âm → chỉ chấp nhận mỗi `đ`.
- Config `spell_check: bool` (`core/src/config.rs`), Swift đọc/ghi qua FFI.

## Quyết định thiết kế (đã chốt)

| Vấn đề | Lựa chọn |
|--------|----------|
| Mô hình loose | `loose = strict − kiểm tra phụ âm cuối`. Giữ mọi phép chặn "cụm bất khả" (phụ âm đầu, cụm nguyên âm, nguyên âm liên tục, `đ` đứng đầu); chỉ bỏ luật phụ âm cuối và nới trường hợp toàn phụ âm. |
| Phơi bày | **Đổi nghĩa nút `spell_check` sẵn có** — không thêm config/enum/UI mới. `true` = strict (như cũ), `false` = loose (thay vì "đặt dấu mù"). |
| Đánh đổi | **Chấp nhận** nhóm từ tiếng Anh cùng cấu trúc `mask`→`mák`, `task`→`ták`, `desk`→`dék` bị đặt dấu trong loose. Không thuật toán nào phân biệt được `nèk` (cố ý) với `mask` (vô tình) — cùng cấu trúc "phụ âm đầu hợp lệ + 1 nguyên âm + chữ tạo dấu + phụ âm đuôi". |
| Whitelist cá nhân | **Không làm** (YAGNI) — loose đã đủ cho nhu cầu gõ tắt; từ lạ một lần dùng phím `⌃⇧Space` sẵn có. |

**Vì sao "cụm bất khả" không tự cứu được `nèk`/`đc`:** engine đã coi `ea`
và mọi cặp nguyên âm ngoài `NUCLEI` là bất khả (đó là lý do `clear` an toàn
khi strict). Nhưng `nèk`/`đc` không dính cụm nguyên âm bất khả — chúng vướng
**luật phụ âm cuối** (`k` không hợp lệ) và **luật không-nguyên-âm**. Đó chính
là hai luật loose gỡ bỏ.

## Định nghĩa "cụm bất khả" mà loose GIỮ NGUYÊN

Đây là phần bắt từ tiếng Anh, không đụng tới:

- **Phụ âm đầu sai:** mọi thứ ngoài `INITIALS` — vd `cl`, `br`, `st`,
  và mọi chữ `f j w z`. → bắt `clear`, `for`, `just`, `class`.
- **Cụm nguyên âm bất khả** (14 cặp thực tế xuất hiện qua Telex):
  `ae ea ei ey ii io iy ou oy ya yi yo yu yy`. → bắt `sound` (ou),
  `bear` (ea), `boy` (oy)…
- **Nguyên âm không liên tục** (nguyên âm sau phụ âm cuối) → bắt `maker`.
- **`đ` (gạch) chỉ đứng đầu từ.**

## Hành vi loose GỠ BỎ so với strict

1. **Bỏ kiểm tra `FINALS`** — mọi phụ âm cuối được chấp nhận.
   → `nèk` (đuôi `k`), `mák` (từ `mask`).
2. **Bỏ luật phụ âm tắc cuối chỉ mang sắc/nặng.**
   → `mảt`/`mart` giờ giữ `mảt` thay vì khôi phục (đánh đổi đã chấp nhận).
3. **Nới trường hợp không nguyên âm** — chấp nhận chuỗi phụ âm có phụ âm
   đầu hợp lệ (không chỉ mỗi `đ` trơ). → `đc` (`đ`+`c`).

## Kiến trúc & thay đổi

### 1. `core/src/engine/spell.rs` — thay đổi chính

Thêm tham số độ chặt vào bộ kiểm tra. Chữ ký đề xuất:

```rust
pub fn is_acceptable(state: &WordState, loose: bool) -> bool
```

Phần chung (bảng phụ âm đầu, cụm nguyên âm, luật nguyên âm liên tục, `đ`
đứng đầu) giữ nguyên cho cả hai. Khi `loose == true`:

- Bỏ nhánh kiểm `FINALS.contains(final_c)`.
- Bỏ nhánh kiểm `stop_final && matches!(tone, Grave|Hook|Tilde)`.
- Trường hợp không nguyên âm: chấp nhận nếu **chữ đầu mang gạch (`đ`)** —
  tức là một từ viết tắt kiểu `đc`, `đk`, `đt`. Đây là nhóm no-vowel
  *đã-bị-biến-đổi* duy nhất đáng quan tâm (dấu mũ/móc/trăng bắt buộc phải
  có nguyên âm để bám, nên transformed + no-vowel ⟺ có `đ`). Rule cụ thể:
  `letters[0].stroke == true`. An toàn vì tiếng Anh gần như không sinh `đ`.

Giữ nguyên `is_transformed` (không đổi).

### 2. `core/src/engine/mod.rs` — `render_word`

Hiện tại:

```rust
if self.cfg.spell_check
    && spell::is_transformed(&state)
    && !spell::is_acceptable(&state)
{
    return (raw.to_string(), true);
}
```

Đổi thành: **luôn** chạy bộ lọc, chỉ khác độ chặt. `spell_check == true`
→ strict; `false` → loose:

```rust
if spell::is_transformed(&state)
    && !spell::is_acceptable(&state, /*loose=*/ !self.cfg.spell_check)
{
    return (raw.to_string(), true);
}
```

Nhánh "đặt dấu mù không kiểm tra gì" biến mất hoàn toàn.

### 3. `app/Sources/OreoKey/SettingsWindow.swift` — nhãn UI

Đổi nhãn/tooltip nút `spell_check` cho đúng nghĩa mới. Đề xuất trình bày
là lựa chọn 2 trạng thái rõ ràng, ví dụ:

- Nhãn: **"Ưu tiên tiếng Anh (kiểm tra chặt)"** (bật = strict)
- Tooltip: "Bật: bảo vệ tối đa từ tiếng Anh, nhưng từ viết tắt như `đc`,
  `nèk` bị sửa lại. Tắt: cho gõ tắt tự do, vẫn nhận diện từ tiếng Anh có
  cụm bất khả (`clear`, `sound`…)."

Không thêm control mới, không đổi kiểu config.

## Xử lý lỗi & ràng buộc

- Config JSON **không đổi** (`spell_check` vẫn `bool`) → không phá file
  settings cũ, không cần migration.
- VNI hưởng lợi tự động: bộ lọc thao tác trên `WordState`, độc lập
  telex/vni.
- `modern_tone`, `flexible_marks`, macro, censor — không đụng.
- `raw_mode` latch, `on_backspace` gỡ khoá — logic giữ nguyên, chỉ đổi
  điều kiện qua `is_acceptable(loose)`.

## Kiểm thử

Cập nhật lời gọi `is_acceptable` hiện có (thêm tham số). Rà toàn bộ test
dùng `spell_check: false` / helper `telex_no_spell()`:

- `spell_off_transforms_anyway`: `mask` → `mák` **vẫn đúng** trong loose
  (đuôi `k` được thả) — nhưng đổi tên/ý nghĩa cho khớp mô hình mới.
- Các test `telex_no_spell()` khác dùng từ TV hợp lệ (`vieet`→`việt`,
  `hn`, backspace sync) → không đổi kết quả.

Bộ test mới cho loose (thêm vào `spell.rs`):

| Gõ (Telex) | Strict (BẬT) | Loose (TẮT) |
|---|---|---|
| `clear` | `clear` | `clear` (cl đầu bất khả) |
| `sound` | `sound` | `sound` (ou bất khả) |
| `ddc` (`đc`) | `ddc` | `đc` |
| `nefk` (`nèk`) | `nefk` | `nèk` |
| `mask` | `mask` | `mák` (đánh đổi) |
| `status` | `status` | `status` (nguyên âm a…u không liên tục) |
| `vieetj` | `việt` | `việt` |
| `for` | `for` | `for` |
| `class` | `class` | `class` |
| `dies` | `dies` | `dies` (ie + thanh) |

Giữ lại toàn bộ test strict hiện có (chạy với `loose=false`) để bảo đảm
không hồi quy.

## Ngoài phạm vi (YAGNI)

- Whitelist / từ điển cá nhân.
- Phím "ép tiếng Việt" phản ứng.
- Chế độ thứ 3 (strict/loose/off riêng biệt) — chỉ 2 trạng thái.
- Nới lỏng bảng `INITIALS`/`NUCLEI` (giữ nguyên để không phá bắt tiếng Anh).
