# Báo cáo Sweep toàn từ điển — bộ gõ tiếng Việt OreoKey

Bin sweep: `core/src/bin/sweep.rs` — sinh mọi cách gõ (Telex + VNI, mọi vị trí phím thanh/mũ/móc/trăng/đ, kiểu đặt thanh cũ + mới) cho từng âm tiết rồi chạy qua `Engine` đầy đủ (có spell-check/raw-mode) và so kết quả với âm tiết gốc.

Cách chạy:

```
cargo build --release --bin sweep
./target/release/sweep <syllables.txt> --method both --out failures.jsonl [--modern] [--limit N]
```

Từ điển: 8.784 âm tiết duy nhất trích từ Viet74K (tách theo khoảng trắng/gạch nối, lọc bảng chữ tiếng Việt, NFC).

## 1. Kết quả tổng quan (từ điển thật, kiểu cũ)

- **8.579 âm tiết** xử lý được (205 âm tiết bỏ qua do không phân rã được), **83.134 biến thể cách gõ**.
- **2.035 lỗi → pass rate 97,6%**. Kiểu đặt thanh mới cho kết quả giống hệt.
- Toàn bộ lỗi phân lớp được thành 4 nhóm; sau điều tra đầy đủ chỉ có **1 bug engine thật (BUG 1, đã sửa)** — nhóm từng nghi là "BUG 2" hóa ra là vần ngoài bảng NUCLEI (mục 3):

| Nhóm | Lỗi | Từ | Bản chất |
|---|---|---|---|
| Từ đa âm tiết viết liền (đăngten, vôlăng, bêtông, đôla…) | 1.012 | 190 | Giới hạn theo thiết kế — spell gate chỉ nhận 1 âm tiết |
| **BUG 1: gõ thanh/dấu ngay sau "qu"/"gi"** | 386 | 132 | **Bug engine, đã xác minh 3 lần độc lập** |
| Phụ âm đầu ngoại lai (đr-, xt-, cr-, bl-, pl-…) | ~320 | ~76 | Giới hạn theo thiết kế — từ mượn/dân tộc |
| ~~BUG 2~~ → vần ngoài bảng `NUCLEI` | 321 | 25 | **Không phải bug** (điều tra 2026-07-17, xem mục 3) — noise từ điển + từ mượn + biến thể phương ngữ |

## 2. BUG 1 — khóa raw-mode vĩnh viễn khi gõ thanh ngay sau "qu" — **ĐÃ SỬA (2026-07-17)**

**Fix**: `core/src/engine/syllable.rs` — `vowel_indices()` giờ loại chữ `u` ngay sau `q` khỏi nhân âm cả khi **chưa có gì theo sau** (trạng thái gõ dở), không chỉ khi đã có nguyên âm đứng sau. Không sửa nhánh "gi" (chữ `i` sau `g` có thể là nhân âm thật: "gì", "gị").

**Kiểm chứng**:
- TDD: test hồi quy `tone_right_after_qu_onset_stays_live` (spell.rs) viết trước, fail đúng như chẩn đoán (`qu1an` kẹt nguyên văn), pass sau fix; test đối chứng `tone_right_after_gi_onset_still_works` pass cả trước lẫn sau.
- Toàn bộ test suite: 83/83 pass.
- Sweep lại toàn từ điển: **2.035 → 1.654 lỗi (−382, đúng toàn bộ nhóm qu-early-tone, 0 hồi quy)**. Một "lỗi mới" duy nhất (`ăcquy` gõ `acquwy`) thực chất là biến thể mới được generator sinh thêm sau fix (oracle giờ tái tạo được), rơi vào nhóm giới-hạn-đa-âm-tiết có sẵn — không phải hồi quy hành vi.
- Known-limitation phát hiện kèm: từ `quýu` fail ở **mọi** thứ tự gõ (cả trước fix) vì sau khi "qu" chiếm `u`, nhân âm còn lại "yu" không có trong `NUCLEI` (khác `khuỷu` = nhân âm "uyu"). Từ phương ngữ hiếm — chưa xử lý; nếu muốn hỗ trợ cần thêm "yu" vào NUCLEI (cân nhắc tác dụng phụ với từ tiếng Anh).

Chi tiết cơ chế bug (lưu để tham khảo):

Repro tối giản (đã xác minh trên Engine thật, cả hai phương thức):

- VNI: gõ `qu1an` → ra nguyên văn `qu1an`, kỳ vọng `quán`. Đối chứng `qua1n`/`quan1` → đúng.
- Telex: gõ `qusan` → ra `qusan`, kỳ vọng `quán`. Đối chứng `quasn`/`quans` → đúng.
- Cùng lỗ hổng với "gi": `gi2ey`/`gifey` kẹt tương tự.

Cơ chế (chuỗi nhân quả qua 3 file):

1. `core/src/engine/syllable.rs:15-21` — `vowel_indices()` chỉ loại chữ `u` trong "qu" khỏi nhân âm khi **đã có nguyên âm khác đứng sau** (`letters[i+1..].iter().any(is_vowel)`). Ở trạng thái gõ dở `[q,u]` + thanh, lookahead rỗng → `u` bị tính là nhân âm, phụ âm đầu suy ra là `"q"` trơ.
2. `core/src/engine/spell.rs` — `INITIALS` chỉ có `"qu"`, không có `"q"` → `is_acceptable` và `is_live_prefix` đều trả `false` (nhánh đã-có-nguyên-âm của `is_live_prefix` dùng khớp chính xác, không khớp tiền tố như nhánh chưa-có-nguyên-âm — thiếu nhất quán).
3. `core/src/engine/mod.rs:267-268` — `is_transformed && !is_live_prefix` → khóa `raw_mode = true` **vĩnh viễn**; mọi phím sau chỉ echo nguyên văn, không bao giờ hồi phục dù gõ đủ và đúng phần vần còn lại.

Đây là biến thể chưa được vá của lớp bug Issue #4 (thanh gõ trước khi vần hoàn thiện), tại vị trí sớm hơn: ngay sau phụ âm đầu digraph.

Hướng sửa đã được kiểm chứng sơ bộ: bỏ điều kiện lookahead trong `vowel_indices()` (chữ `u` ngay sau `q` **luôn** thuộc phụ âm đầu — tiếng Việt không có âm tiết mà "qu" + u làm nhân âm). Một agent phân tích đã thử vá tạm và toàn bộ test suite hiện có vẫn pass. Cần cân nhắc thêm nhánh khớp-tiền-tố trong `is_live_prefix` cho "gi". Test hồi quy cần thêm: VNI `qu1an→quán`, `qu1a6y→quấy`; Telex `qusan→quán`, `qusaya→quấy`; tương tự cho "gi".

## 3. ~~BUG 2~~ — KHÔNG PHẢI BUG: vần nằm ngoài bảng `NUCLEI` (điều tra 2026-07-17)

Giả thuyết ban đầu ("thanh sớm + mũ/trăng muộn gây kẹt như BUG 1") **bị bác bỏ** bằng thực nghiệm: chạy sweep riêng từng từ của nhóm này cho thấy **cả 25/25 từ fail ở MỌI thứ tự gõ** (failures == variants), kể cả thứ tự chuẩn nhất — tức không có bug trạng thái trung gian nào; các từ này đơn giản là không thể render vì cụm nguyên âm của chúng không có trong bảng vần `NUCLEI` (spell.rs). Sau khi sửa BUG 1, **toàn bộ 1.654 lỗi còn lại được quy kết trọn vẹn, không còn lỗi nào chưa giải thích được**: 1.013 đa-âm-tiết viết liền + 321 vần-ngoài-bảng + 320 onset ngoại lai.

Phân loại 25 từ vần-ngoài-bảng:

- **Từ mượn đa âm tiết mà ranh giới âm tiết rơi giữa chuỗi nguyên âm liền** (bộ phân lớp theo cụm không nhìn thấy): `kiôt` (ki-ốt), `mayô` (may-ô), `layơn` (lay-ơn), `nôen` (Nô-en), `nêông` (nê-ông), `điot`/`điop` (đi-ốt/đi-ốp) — thực chất thuộc nhóm đa-âm-tiết ở mục 4.
- **Biến thể phương ngữ/chính tả của từ thật**: `khuắng` (~khoắng), `khuều` (~khều), `ngoẩy` (~nguẩy/ngoảy), `quýu` (~quíu) — bảng NUCLEI chuẩn đúng khi không chứa "uă", "uêu", "oây", "yu".
- **Noise thuần của Viet74K**: `aỏi`, `gièy`, `gỵa`, `sìi`, `miẻo`, `tiẻn`, `thôể`, `sêếu`, `âớu`, `âởu`, `côống`, `khôống`, `pôông`, `yô`.

Kết luận: engine từ chối các từ này là **đúng thiết kế**. Việc cần làm không nằm ở engine mà ở từ điển test: lọc 3 nhóm lành tính khỏi danh sách âm tiết để sweep trong CI có kỳ vọng **0 lỗi**.

## 4. Nhóm không phải bug (ghi nhận để quyết định sản phẩm)

- **Từ đa âm tiết viết liền** (1.012 lỗi/190 từ): `đăngten`, `vôlăng`, `bêtông`, `đôminô`, `đôla`… — engine xử lý theo đơn vị 1 âm tiết nên spell gate từ chối, người dùng không gõ được dấu cho các từ này thành một token liền. Unikey xử lý được lớp từ này. Đây là quyết định thiết kế: nếu muốn hỗ trợ, cần tách/ghép âm tiết trong một token hoặc whitelist từ mượn phổ biến.
- **Phụ âm đầu ngoại lai** (~320 lỗi/76 từ): `đrông`, `xtốp`, `crếp`, `blốc`, `plăng`… — từ mượn/tiếng dân tộc, ngoài phạm vi chính tả tiếng Việt chuẩn. Chấp nhận bỏ qua (hoặc gộp vào cùng quyết định whitelist ở trên).
- **VNI nuốt chữ số dính liền từ** (review engine 2026-07-17): gõ `nam2026` kiểu VNI ra `nầm` — các số 2/0/2/6 bị tiêu thành huyền/xóa-thanh/huyền/mũ, và vì "nầm" là âm tiết hợp lệ nên spell-restore không cứu. Đây là bản chất của phương thức VNI (chữ số = phím dấu), Unikey/EVKey hành xử tương tự. **Quyết định đã chốt (người dùng xác nhận): GIỮ NGUYÊN** — không thêm heuristic chuỗi-số, người dùng VNI gõ space trước số như thói quen chung. Đừng "sửa" hành vi này như một bug.
- Nit vô hại: `quw` (Telex) → "quư" được spell chấp nhận dù không phải âm tiết thật — input không ai gõ, không xử lý.

## 5. Lịch sử & độ tin cậy

- Lần chạy đầu của workflow dùng nhầm từ điển tổng hợp 2.753 âm tiết (do lỗi truyền tham số đường dẫn); toàn bộ 10 bucket của lần đó đều quy về BUG 1 và đã được 10 agent phân tích độc lập xác nhận cùng root cause. Lần chạy lại trên từ điển thật 8.784 âm tiết (số liệu ở trên) xác nhận BUG 1 và phát hiện thêm BUG 2 + hai nhóm giới-hạn-thiết-kế.
- Sweep chạy ~0,2 giây/lượt → phù hợp đưa vào CI làm regression gate sau khi sửa BUG 1 (lọc trước các nhóm không-phải-bug khỏi từ điển, hoặc cho phép baseline file).
