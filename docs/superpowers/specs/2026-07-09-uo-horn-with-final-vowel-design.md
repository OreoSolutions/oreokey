# Cặp `ươ` (Telex) móc đúng khi còn nguyên âm cuối — Thiết kế

**Ngày:** 2026-07-09

## Vấn đề

Trong kiểu gõ Telex, để có cặp `ươ` người dùng gõ một chữ `w` sau `uo`.
Cơ chế này **đã tồn tại** và chạy đúng khi `uo` là hai nguyên âm cuối của
từ:

- `duongw` → **dương** ✓
- `uwowng` → **ương** ✓

Nhưng khi sau `uo` còn một nguyên âm nữa (cụm `ươi` / `ươu` như trong
"người", "cười", "rượu"), cặp `uo` không còn nằm ở cuối, và logic hiện tại
**chỉ xét đúng hai nguyên âm cuối** nên bỏ sót, chỉ móc mỗi chữ `o`:

| Gõ        | Hiện tại        | Mong muốn |
|-----------|-----------------|-----------|
| `nguoiwf` | ngu**ờ**i ✗     | ng**ườ**i |
| `cuoiwf`  | cu**ờ**i ✗      | c**ườ**i  |
| `tuoiw`   | tu**ơ**i ✗      | t**ươ**i  |
| `ruouwj`  | ru**oự** ✗ (hỏng) | r**ượ**u |

Hệ quả: người dùng phải gõ workaround, móc tay từng nguyên âm
(`cuwowif` cho "cười").

## Nguyên nhân

`core/src/engine/telex.rs`, nhánh `'w'`:

- **Chiều áp dụng** (dòng ~119–133): chỉ kiểm cặp ở `vidx[len-2]`,
  `vidx[len-1]` (hai nguyên âm cuối). Có nguyên âm cuối theo sau → cặp `uo`
  không ở vị trí đó → không khớp → rơi xuống nhánh móc một nguyên âm gần
  cuối (chỉ móc `o`).
- **Chiều hủy** (dòng ~91–102): chỉ kiểm hai **chữ cái** cuối
  (`letters[n-2]`, `letters[n-1]`). Cùng khuyết điểm đối xứng: khi cặp
  `ươ` móc nằm giữa cụm (có nguyên âm cuối theo sau), bấm `w` để hủy không
  khớp cặp và sinh ra rác.

`vowel_indices` (syllable.rs) đã loại `u` trong `qu` khỏi cụm nguyên âm khi
sau nó còn nguyên âm — nên `quow` → **quơ** không bị ảnh hưởng.

## Giải pháp

Chỉ sửa nhánh `'w'` trong `telex.rs`. Không đụng VNI (dùng phím số, cơ chế
khác), không đụng spell-check, không đổi UI.

### Thay đổi 1 — chiều áp dụng (`uo → ươ`)

Thay việc chỉ xét hai nguyên âm cuối bằng **quét các cặp nguyên âm liền kề
trong cụm** (theo `vowel_indices`), tìm cặp `u`+`o` đầu tiên mà:

- hai chỉ số nguyên âm liền nhau trong từ (`j == i + 1`), và
- `letters[i].base == 'u'` và `letters[j].base == 'o'`, và
- cả hai **chưa mang dấu** (`!has_mark()`).

Khớp thì móc cả hai (`horn = true`) rồi `return`. Không khớp → giữ nguyên
các nhánh sau (móc một nguyên âm; `w` đơn → `ư`).

### Thay đổi 2 — chiều hủy (`ươ → uo`)

Thay việc chỉ xét hai chữ cái cuối bằng **quét các cặp chữ cái liền kề**,
tìm cặp `u`(móc)+`o`(móc) đầu tiên (`letters[k].base == 'u' && horn`,
`letters[k+1].base == 'o' && horn`). Khớp thì gỡ móc cả hai, giữ **nguyên
hành vi cũ**: đánh dấu `w` chết (`dead.push('w')`) và thêm `w` thành chữ
thường (`letters.push(Letter::plain(c))`).

Chiều hủy phải đặt **trước** chiều áp dụng (như hiện tại) để `w` sau một
cặp `ươ` sẵn có mang nghĩa hủy.

## Bất biến phải giữ (đưa vào test)

- `quow` → **quơ** (u sau q không thuộc cụm nguyên âm)
- `buonw` → **bươn** (muốn "buồn" gõ `buoonf`; ô đã mang mũ nên không phải
  cặp `uo` chưa dấu)
- `duongw` → **dương**, `uwowng` → **ương** (không hồi quy)
- `aw`/`ow`/`uw`/`w` đơn và các case móc/hủy đơn hiện có giữ nguyên

## Test mới

Chiều áp dụng (đỏ trước khi sửa, xanh sau):

- `nguoiwf` → **người**
- `cuoiwf` → **cười**
- `tuoiw` → **tươi**
- `ruouwj` → **rượu**

Chiều hủy (đối xứng): sau khi tạo `ươ` giữa cụm, bấm thêm `w` gỡ móc cả
hai, không sinh rác (vd `cuoiww` gỡ về dạng không móc thay vì để lại `ư`
thừa).

## Rủi ro

Thấp. Trong chính tả tiếng Việt, cụm `uo` (chưa dấu) + nguyên âm cuối chỉ
xuất hiện ở `ươi` / `ươu`, đều cần móc cả hai. Việc mở rộng vùng quét đúng
theo chuẩn các bộ gõ phổ biến (Unikey, EVKey). Các cụm `uô` (buồn, cuốn)
không bị ảnh hưởng vì chữ `o` mang mũ (`has_mark()` = true) nên bị loại
khỏi điều kiện.
