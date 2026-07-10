# Thiết kế — Cải thiện hành vi gõ (Spell 3 mức + Issue #1 + Issue #4)

Ngày: 2026-07-10
Nhánh: `feat/typing-behavior-improvements`

Ba phần độc lập, triển khai và review riêng. Chung một chủ đề "hành vi gõ",
chung vùng code (`engine/`, `data/app-profiles.json`).

- **A. Kiểm tra chính tả 3 mức** — nâng công tắc bật/tắt thành Chặt / Thường / Thoải mái.
- **B. Issue #1** — nháy chữ ở Java Swing (Burp Suite) & TUI trong terminal (kitty): thiếu app-profile.
- **C. Issue #4** — VNI gõ số thanh trước số mũ giữa âm tiết (`thie16u`) bị kẹt raw.

---

## A. Kiểm tra chính tả 3 mức

### Bối cảnh

Hôm nay `EngineConfig.spell_check: bool` điều khiển hai chế độ:
- `true` (Chặt): kiểm tra phonotactics đầy đủ — bảo vệ tối đa từ tiếng Anh
  (`mask`, `class`, `dies`, `mart` giữ nguyên).
- `false` (Thường): thả kiểm tra phụ âm cuối + luật thanh-trên-phụ-âm-tắc, cho
  gõ tắt tiếng Việt (`đc`, `nèk`), **nhưng vẫn** bắt cụm bất khả (`clear`,
  `sound`, `for`, `class`, `dies`, `status`) và khôi phục về nguyên văn.

Người dùng muốn thêm mức thứ ba **buông lỏng hết**: không bao giờ khôi phục,
luôn đặt dấu.

### Ba mức (mạnh → yếu)

| Mức | Nhãn UI | = hiện tại | `mask` | `class`/`clear` | `đc`/`nèk` |
|-----|---------|-----------|--------|-----------------|-----------|
| Strict   | **Chặt**     | Bật  | `mask` (giữ Anh) | giữ Anh | bị đặt dấu |
| Standard | **Thường**   | Tắt  | `mák`            | giữ Anh | `đc`/`nèk` ✓ |
| Loose    | **Thoải mái**| *mới*| `mák`            | **bị đặt dấu** | `đc`/`nèk` ✓ |

Mặc định (cài mới): **Chặt** — giữ nguyên hành vi mặc định hiện tại.

### Thay đổi engine (`core/src/engine/`)

**`mod.rs`:**
- Thêm enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum SpellMode { Strict, Standard, Loose }
  ```
- `EngineConfig`: đổi `spell_check: bool` → `spell_mode: SpellMode`.
  `Default` = `SpellMode::Strict`.
- `render_word`: khác biệt của mức Loose gói gọn trong **một điều kiện**:
  ```rust
  if self.cfg.spell_mode != SpellMode::Loose
      && spell::is_transformed(&state)
      && !spell::is_acceptable(&state, self.cfg.spell_mode == SpellMode::Standard)
  {
      return (raw.to_string(), true);
  }
  ```
  Mức Loose → bỏ qua gate → không bao giờ vào `raw_mode`, luôn render có dấu.

**`spell.rs`:** `is_acceptable(state, loose)` **giữ nguyên chữ ký và logic**.
`loose` bây giờ = `(spell_mode == Standard)`. Không viết lại phonotactics.

### Thay đổi config (`core/src/config.rs`)

- `Settings.spell_check: bool` → `spell_mode: SpellMode`
  (serde `rename_all = "snake_case"` → `"strict" | "standard" | "loose"`),
  `#[serde(default)]` = `Strict`.
- `Settings::engine_config()` truyền thẳng `spell_mode`.

**Migration (tránh reset người dùng cũ):** trong `load()`, parse JSON thành
`serde_json::Value` trước; nếu object có khóa `spell_check` mà **thiếu**
`spell_mode` thì chèn:
```
spell_mode = if spell_check == true { "strict" } else { "standard" }
```
rồi mới deserialize thành `Settings`. File hỏng / không phải object → về default
như cũ.

### FFI (`core/src/ffi.rs`)

Không đổi. Swift đọc/ghi cả khối settings JSON qua `ok_settings_json_get` /
`ok_settings_json_set`. Chỉ cần Swift biết field mới `spell_mode`.

### UI Swift (Cài đặt → mục "Hành vi gõ")

Đổi hàng `Toggle("Kiểm tra chính tả (chặt)")` thành:

```
Kiểm tra chính tả
[ Chặt | Thường | Thoải mái ]              ← Picker .segmented (bind spell_mode)
<mô tả mức đang chọn>
```

Ba câu mô tả:
- **Chặt**: Bảo vệ tối đa từ tiếng Anh (mask, class).
- **Thường**: Cho gõ tắt (đc, nèk), vẫn nhận diện tiếng Anh có cụm bất khả (clear, sound).
- **Thoải mái**: Luôn đặt dấu, không khôi phục từ tiếng Anh.

### Kiểm thử

- `engine`/`spell.rs`: thêm nhóm test mức **Loose** — `class`, `clear`, `sound`,
  `for`, `dies` **bị đặt dấu** (khẳng định hành vi mới); các test Strict/Standard
  hiện có giữ nguyên không đổi.
- `config.rs`: 3 test migration — file cũ `spell_check:true` → Strict,
  `spell_check:false` → Standard, file mới có `spell_mode` → dùng thẳng.
- Round-trip JSON với `spell_mode`.

---

## B. Issue #1 — Nháy chữ ở Java Swing & TUI trong terminal

### Nguyên nhân (đã xác minh qua code)

`kitty` (`net.kovidgoyal.kitty`) và app Java Swing (Burp Suite) **không có trong
`data/app-profiles.json`** → rơi vào `FixMode::Auto` → engine thử **AX API
trước**. Terminal/Swing không cho sửa text qua AX → thất bại rồi mới fallback bơm
phím → nháy. Các terminal đã liệt kê (`iterm2`, `ghostty`, `warp`, `Terminal`)
dùng `inject_fast` nên không bị.

### Thay đổi (thuần dữ liệu — giống hệt commit Telegram vừa merge)

Thêm vào `data/app-profiles.json`:
- `net.kovidgoyal.kitty` → `inject_fast`
- Burp Suite → `inject_fast`. Bundle ID lấy chính xác lúc implement bằng
  `osascript -e 'id of app "Burp Suite Professional"'` (hoặc `lsappinfo info -only
  bundleid <pid>` khi app đang chạy) — không đoán trong spec.
- Rà thêm terminal/Swing phổ biến còn thiếu (Alacritty `org.alacritty`,
  WezTerm `com.github.wez.wezterm`, Hyper `co.zeit.hyper`…) → `inject_fast`,
  xác nhận từng bundle ID theo cùng cách trên trước khi thêm.

Test: `profiles.rs` khẳng định các bundle mới resolve ra `InjectFast`.

### Giới hạn (ghi rõ cho người dùng, không hứa quá)

Với TUI/Swing, `Replace` = *backspace N + gõ lại*, nên khi cần **sửa lùi** thì
backspace vẫn hiện → nháy là **cố hữu**, không xoá hoàn toàn được. Engine đã diff
tối thiểu. Phần data trên loại bỏ nháy do thử-AX-rồi-mới-fallback; nháy còn lại
(nếu có) khi sửa dấu lùi là bản chất của TUI. Không mở rộng phạm vi phần B để
đụng vào cơ chế inject.

---

## C. Issue #4 — VNI: số thanh trước số mũ giữa âm tiết

### Repro (đã chạy trên engine thật)

| Gõ | Ra | |
|----|----|----|
| `thieu61`  | `thiếu`   | ✅ số ở cuối |
| `thie61t`  | `thiết`   | ✅ mũ `6` trước thanh `1` |
| **`thie16u`** | **`thie16u`** | ❌ **hỏng** |

### Nguyên nhân gốc

Người dùng có thói quen bấm **số thanh ngay sau nguyên âm, rồi mới số mũ**
(`thie` → `1` → `6` → `u`). Tại lúc bấm `1`, nhân âm mới là `ie` (chưa thành
`iê`). `spell.rs:106` cấm hoàn thiện nhân âm khi đã có thanh:

```rust
let nucleus_ok = NUCLEI.contains(&nucleus.as_str())
    || (state.tone.is_none() && NUCLEI.iter().any(|n| can_become(&nucleus, n)));
//     ^^^^^^^^^^^^^^^^^^^^^ có thanh → "ie" bị coi là sai
```

→ `thié` bị coi không hợp lệ → **latch `raw_mode` vĩnh viễn** (chỉ gỡ được bằng
backspace). Các phím `6`, `u` sau đó chỉ nối raw → `thie16u`.

Luật `state.tone.is_none()` này **cố tình** để bắt tiếng Anh `dies`/`lies`/`ties`
(`ie` + thanh). Bỏ nó ngây thơ sẽ phá tính năng khôi phục tiếng Anh.

### Nguyên tắc chỉ đạo (chốt sau brainstorm)

Nguyên tắc gốc OreoKey: **không nháy chữ, không đặt dấu sai lên từ tiếng Anh**
(*"thà bỏ sót còn hơn phá từ đúng"*). Vì thế **không** dùng hướng "hiển thị lạc
quan" (sẽ hiện `dié` trên `dies` rồi bật lại — vừa đặt dấu sai vừa nháy, phạm
đúng điều issue #1 than).

Hướng chọn: **tách "chết hẳn" khỏi "còn sống", đừng latch `raw_mode` cho trạng
thái còn-sống.**

- **Chết hẳn (dead):** chứa cụm không bao giờ thành tiếng Việt (phụ âm đầu bất
  khả `cl`/`fr`, nguyên âm rời `status`) → latch ngay như hiện tại.
- **Còn sống (live/tiền tố):** chưa hợp lệ nhưng còn là **tiền tố hợp lệ** (gõ
  thêm dấu/chữ sẽ thành âm tiết đúng) → **không latch**; giữ engine sống để phím
  sau (`6`) có cơ hội hoàn thiện; màn hình giữ **raw** (kể cả số) cho tới khi âm
  tiết thật sự hợp lệ mới lật sang có dấu. Quyết định "tiếng Anh" **dời tới lúc
  ngắt từ**: nếu tới space vẫn không hợp lệ → khôi phục nguyên văn (như `dies`).

Hệ quả: `dies` vẫn hiện `dies` suốt, **không nháy**; `thie16u` → `thie1` (hiện
số `1` thoáng qua) → `thiế` (khi bấm `6`) → `thiếu`. Số `1` không phải "nháy chữ"
— là phím thật sự bấm, biến mất ngay khi âm tiết hoàn chỉnh.

### Hướng triển khai (chi tiết để writing-plans khai triển)

Cần một hàm phân biệt **live-prefix** với **dead** ở tầng spell, ví dụ
`spell::is_live_prefix(state) -> bool`: đúng khi cụm hiện tại có thể trở thành âm
tiết hợp lệ bằng cách thêm dấu/chữ (phụ âm đầu là tiền tố hợp lệ của INITIALS,
nhân âm là tiền tố của một NUCLEI qua `can_become`, kể cả khi đã có thanh).

Trong `on_char`: khi `render_word` báo `restored`, **chỉ** đặt `raw_mode = true`
nếu **không** phải live-prefix; nếu là live-prefix thì giữ raw hiển thị nhưng
**không** khóa — lần gõ sau vẫn `render_word` bình thường.

Khôi phục tiếng Anh cuối cùng đã có sẵn qua đường backspace + (bổ sung) đường
word-break: khi `WordBreak`, nếu từ đang ở trạng thái live-prefix chưa hợp lệ thì
để nguyên raw (đã đúng vì raw đang hiển thị).

### Kiểm thử

- Thêm test VNI: `thie16u` → `thiếu`, `tie16t` → `tiết`, và giữ `thieu61`,
  `thie61t`.
- Test không hồi quy tiếng Anh: `dies`/`lies`/`ties` → nguyên văn; `class` →
  `class` (dead latch ngay); `status` → `status`.
- Áp cho cả Telex nơi tương đương (thanh trước dấu mũ trên nhân âm dở).

### Quan hệ với phần A

Ở mức **Thoải mái** không có khôi phục nên `raw_mode` không bao giờ bật →
`thie16u` tự đúng. Phần C sửa cho hai mức **Chặt/Thường**.

---

## Thứ tự triển khai đề xuất

1. **B** (thuần dữ liệu, rủi ro thấp, ship nhanh).
2. **A** (feature rõ ràng, migration có test).
3. **C** (đụng logic spell tinh tế nhất — làm sau cùng, test kỹ hồi quy).
