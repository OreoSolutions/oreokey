# Chế độ "gõ thoải mái" (loose spell-check) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Biến trạng thái TẮT của nút `spell_check` từ "đặt dấu mù" thành chế độ "gõ thoải mái" — cho gõ tắt tiếng Việt (`đc`, `nèk`) tự do nhưng vẫn nhận diện từ tiếng Anh qua "cụm bất khả".

**Architecture:** `loose = strict − kiểm tra phụ âm cuối`. Thêm tham số `loose: bool` vào `spell::is_acceptable`; nhánh loose bỏ kiểm `FINALS` + luật thanh-trên-phụ-âm-tắc, và nới trường hợp không nguyên âm cho từ có `đ`. `render_word` luôn chạy bộ lọc, độ chặt = `!spell_check`. Không đổi kiểu config (vẫn `bool`), không thêm UI mới.

**Tech Stack:** Rust (engine thuần `std` + `serde` ở tầng config), Swift/SwiftUI (Settings).

## Global Constraints

- Config JSON **không đổi**: `spell_check` vẫn kiểu `bool`. Không migration, không phá `settings.json` cũ.
- **Không** đụng bảng `INITIALS` / `NUCLEI` (giữ nguyên tầng bắt tiếng Anh).
- VNI phải hưởng lợi tự động — bộ lọc thao tác trên `WordState`, độc lập telex/vni.
- Không thêm dependency mới.
- Engine core (`core/src/engine/`) chỉ import `std`.
- Tất cả comment/nhãn UI bằng tiếng Việt (theo phong cách repo).
- Đánh đổi đã chốt: trong loose, `mask`→`mák`, `task`→`ták`, `desk`→`dék` bị đặt dấu — đúng thiết kế, KHÔNG coi là bug.

## File Structure

- `core/src/engine/mod.rs` — thêm test helper `raw_render` (bypass spell); sửa `render_word` để luôn lọc với `loose = !spell_check`.
- `core/src/engine/spell.rs` — `is_acceptable` nhận `loose: bool`; thêm nhánh loose + test loose.
- `core/src/engine/telex.rs` — chuyển helper `t`/`t_modern` sang `raw_render` (tách khỏi spell).
- `core/src/engine/vni.rs` — chuyển helper `v` sang `raw_render`.
- `app/Sources/OreoKey/SettingsWindow.swift` — đổi nhãn/mô tả nút `spell_check`.

---

### Task 1: Tách test cơ chế biến đổi khỏi tầng spell (refactor, không đổi hành vi)

Các test trong `telex.rs`/`vni.rs` đang gõ qua `Engine` với `spell_check: false` để test *cơ chế đặt dấu thô*. Khi Task 2 biến TẮT thành loose, vài test sẽ bị latch. Tách chúng sang một helper render thẳng (build state + render_letters, không qua spell). Đây là refactor thuần: output hiện tại của "spell off" **đúng bằng** `render_letters(state)`, nên mọi test giữ nguyên kết quả.

**Files:**
- Modify: `core/src/engine/mod.rs` (thêm `raw_render` vào `testutil`, quanh dòng 369-407)
- Modify: `core/src/engine/telex.rs:192-198` (helper `t`, `t_modern`)
- Modify: `core/src/engine/vni.rs:128-138` (helper `v`)

**Interfaces:**
- Produces: `pub(crate) fn raw_render(method: TypingMethod, keys: &str, modern_tone: bool, flexible_marks: bool) -> String` — dựng `WordState` bằng bộ gõ chỉ định rồi render, **không** qua spell filter. Chỉ hỗ trợ chuỗi phím tiến (không backspace).

- [ ] **Step 1: Thêm helper `raw_render` vào `testutil`**

Trong `core/src/engine/mod.rs`, khối `#[cfg(test)] pub(crate) mod testutil`, thêm hàm sau (ngay dưới `type_str`):

```rust
/// Dựng `WordState` từ chuỗi phím rồi render, KHÔNG qua tầng spell.
/// Dùng cho test cơ chế biến đổi telex/vni (chỉ chuỗi phím tiến).
pub(crate) fn raw_render(
    method: TypingMethod,
    keys: &str,
    modern_tone: bool,
    flexible_marks: bool,
) -> String {
    let mut state = WordState::default();
    for c in keys.chars() {
        match method {
            TypingMethod::Telex => telex::apply_key(&mut state, c, flexible_marks),
            TypingMethod::Vni => vni::apply_key(&mut state, c),
        }
    }
    render_letters(&state, modern_tone)
}
```

- [ ] **Step 2: Chuyển helper `t`/`t_modern` trong `telex.rs` sang `raw_render`**

Thay khối `core/src/engine/telex.rs:192-198`:

```rust
    fn t(keys: &str) -> String {
        type_str(&mut engine(false), keys)
    }

    fn t_modern(keys: &str) -> String {
        type_str(&mut engine(true), keys)
    }
```

bằng:

```rust
    fn t(keys: &str) -> String {
        crate::engine::testutil::raw_render(TypingMethod::Telex, keys, false, true)
    }

    fn t_modern(keys: &str) -> String {
        crate::engine::testutil::raw_render(TypingMethod::Telex, keys, true, true)
    }
```

(Giữ nguyên hàm `engine(modern)` — vẫn dùng bởi `word_do_actions_carry_old_text`.)

- [ ] **Step 3: Chuyển helper `v` trong `vni.rs` sang `raw_render`**

Thay khối `core/src/engine/vni.rs:128-138`:

```rust
    fn v(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Vni,
            spell_check: false,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        type_str(&mut e, keys)
    }
```

bằng:

```rust
    fn v(keys: &str) -> String {
        crate::engine::testutil::raw_render(TypingMethod::Vni, keys, false, true)
    }
```

Xoá import không còn dùng nếu compiler cảnh báo: trong `vni.rs` dòng 125-126, `use crate::engine::{Engine, EngineConfig, TypingMethod};` → còn dùng `TypingMethod`; `Engine`/`EngineConfig` không còn dùng ở helper `v` nhưng có thể còn dùng ở test khác — **chỉ xoá cái compiler báo unused**, giữ lại phần còn dùng. `use ...::type_str` có thể thành unused trong `vni.rs` → xoá nếu bị cảnh báo.

- [ ] **Step 4: Chạy toàn bộ test — phải xanh y như trước**

Run: `cd core && cargo test`
Expected: PASS toàn bộ (đây là refactor, không assertion nào đổi kết quả). Không có cảnh báo unused import (đã dọn ở Step 3).

- [ ] **Step 5: Commit**

```bash
git add core/src/engine/mod.rs core/src/engine/telex.rs core/src/engine/vni.rs
git commit -m "refactor(engine): tách test cơ chế biến đổi khỏi tầng spell

Thêm testutil::raw_render (build state + render, bỏ qua spell) và
chuyển helper t/v của telex/vni sang dùng nó. Chuẩn bị cho việc đổi
spell_check:false thành chế độ loose ở bước sau. Không đổi hành vi.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Chế độ loose trong engine

Thêm tham số `loose` vào `is_acceptable` + nhánh nới lỏng, và cho `render_word` luôn lọc với độ chặt = `!spell_check`.

**Files:**
- Modify: `core/src/engine/spell.rs:65-119` (`is_acceptable`)
- Modify: `core/src/engine/mod.rs:336-349` (`render_word`)
- Test: `core/src/engine/spell.rs` (khối `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `WordState`, `Tone`, `INITIALS`, `NUCLEI`, `FINALS`, `marked_lower`, `vowel_indices`, `can_become` (đã có trong `spell.rs`).
- Produces: `pub fn is_acceptable(state: &WordState, loose: bool) -> bool`. `loose == false` giữ nguyên hành vi cũ 100%; `loose == true` bỏ kiểm phụ âm cuối + luật thanh-trên-tắc, và nới no-vowel cho từ có `đ`.

- [ ] **Step 1: Viết test loose (thất bại)**

Thêm vào cuối khối `#[cfg(test)] mod tests` trong `core/src/engine/spell.rs` một helper + test cho loose (engine `spell_check: false` = loose):

```rust
    fn loose(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_check: false, // false = chế độ gõ thoải mái (loose)
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        type_str(&mut e, keys)
    }

    #[test]
    fn loose_allows_abbreviations() {
        // Từ viết tắt có dấu, phụ âm cuối/không nguyên âm → được giữ.
        assert_eq!(loose("ddc"), "đc"); // đ + c, không nguyên âm
        assert_eq!(loose("nefk"), "nèk"); // đuôi k không hợp lệ vẫn cho
    }

    #[test]
    fn loose_still_restores_english() {
        // Cụm bất khả (phụ âm đầu / nguyên âm / nguyên âm không liền) vẫn bắt.
        assert_eq!(loose("clear"), "clear"); // cl đầu bất khả
        assert_eq!(loose("sound"), "sound"); // ou bất khả
        assert_eq!(loose("for"), "for"); // f đầu bất khả
        assert_eq!(loose("class"), "class");
        assert_eq!(loose("dies"), "dies"); // ie + thanh → bất khả
        assert_eq!(loose("status"), "status"); // a…u không liên tục
    }

    #[test]
    fn loose_keeps_valid_vietnamese() {
        assert_eq!(loose("vieetj"), "việt");
        assert_eq!(loose("dduongwf"), "đường");
        assert_eq!(loose("toans"), "toán");
    }

    #[test]
    fn loose_transforms_ambiguous_english_by_design() {
        // Đánh đổi đã chấp nhận: cùng cấu trúc với nèk nên bị đặt dấu.
        assert_eq!(loose("mask"), "mák");
        assert_eq!(loose("task"), "ták");
    }
```

- [ ] **Step 2: Chạy test — phải thất bại ở các ca khôi phục tiếng Anh**

Run: `cd core && cargo test`
Expected: Biên dịch OK (test mới đi qua `Engine`, chưa đụng chữ ký `is_acceptable`), nhưng `loose_still_restores_english` **FAIL** vì `spell_check:false` vẫn đang "đặt dấu mù": `loose("clear")` → `clẻa` (kỳ vọng `clear`), `loose("status")` → `státu` (kỳ vọng `status`). (`loose_allows_abbreviations` có thể PASS sẵn vì đặt dấu mù cũng cho ra `đc`/`nèk` — nhưng đó là trùng hợp, chưa phải hành vi đúng.)

- [ ] **Step 3: Thêm nhánh loose vào `is_acceptable`**

Thay toàn bộ hàm `is_acceptable` (`core/src/engine/spell.rs:64-119`) bằng:

```rust
/// Âm tiết chấp nhận được (hợp lệ hoặc là tiền tố hợp lý của từ đang gõ).
/// `loose == true`: chế độ "gõ thoải mái" — thả kiểm tra phụ âm cuối và
/// luật thanh-trên-phụ-âm-tắc, nới trường hợp không nguyên âm cho từ có
/// `đ`. Vẫn giữ mọi phép chặn cụm bất khả (phụ âm đầu, cụm nguyên âm,
/// nguyên âm liên tục).
pub fn is_acceptable(state: &WordState, loose: bool) -> bool {
    let letters = &state.letters;
    if letters.iter().any(|l| !l.base.is_ascii_alphabetic()) {
        return false;
    }
    // đ chỉ đứng đầu từ.
    if letters.iter().skip(1).any(|l| l.stroke) {
        return false;
    }
    let vidx = vowel_indices(letters);
    let Some(&run_start) = vidx.first() else {
        // Không có nguyên âm.
        if loose {
            // Từ viết tắt kiểu đc/đk/đt: chấp nhận nếu chữ đầu mang gạch.
            return letters.first().is_some_and(|l| l.stroke);
        }
        // Strict: chỉ chấp nhận mỗi "đ" trơ (đang gõ dở "đi"...).
        return letters.len() == 1 && letters[0].stroke;
    };
    // Cụm nguyên âm phải liên tục; nguyên âm sau phụ âm cuối → không hợp lệ.
    let mut run_end = run_start;
    for &i in &vidx[1..] {
        if i == run_end + 1 {
            run_end = i;
        } else {
            return false;
        }
    }
    let render_range =
        |a: usize, b: usize| letters[a..b].iter().map(marked_lower).collect::<String>();

    let initial = render_range(0, run_start);
    if !INITIALS.contains(&initial.as_str()) {
        return false;
    }
    let nucleus = render_range(run_start, run_end + 1);
    let nucleus_ok = NUCLEI.contains(&nucleus.as_str())
        || (state.tone.is_none() && NUCLEI.iter().any(|n| can_become(&nucleus, n)));
    if !nucleus_ok {
        return false;
    }
    // Loose: thả tự do phụ âm cuối và bỏ luật thanh trên phụ âm tắc.
    if loose {
        return true;
    }
    let final_c = render_range(run_end + 1, letters.len());
    if !FINALS.contains(&final_c.as_str()) {
        return false;
    }
    // Âm tiết đóng bằng phụ âm tắc (p t c ch) chỉ mang thanh sắc/nặng.
    let stop_final = matches!(final_c.as_str(), "c" | "ch" | "p" | "t");
    if stop_final
        && matches!(
            state.tone,
            Some(Tone::Grave) | Some(Tone::Hook) | Some(Tone::Tilde)
        )
    {
        return false;
    }
    true
}
```

- [ ] **Step 4: Cập nhật `render_word` để luôn lọc với độ chặt = `!spell_check`**

Thay khối `core/src/engine/mod.rs:336-349` (hàm `render_word`):

```rust
    fn render_word(&self, raw: &str) -> (String, bool) {
        if raw.is_empty() {
            return (String::new(), false);
        }
        let state = self.build_state(raw);
        // Từ bị biến đổi nhưng không phải âm tiết chấp nhận được → trả phím
        // gốc. spell_check BẬT = kiểm tra chặt; TẮT = chế độ "gõ thoải mái".
        if spell::is_transformed(&state)
            && !spell::is_acceptable(&state, !self.cfg.spell_check)
        {
            return (raw.to_string(), true);
        }
        (render_letters(&state, self.cfg.modern_tone), false)
    }
```

- [ ] **Step 5: Cập nhật lời gọi `is_acceptable` cũ trong test strict của `spell.rs`**

Các test strict hiện có gọi qua `Engine` (helper `t`, `spell_check: true`) nên **không cần đổi**. Chỉ đảm bảo không còn lời gọi `is_acceptable(...)` một-tham-số nào. Kiểm tra:

Run: `cd core && grep -rn "is_acceptable(" src/`
Expected: mọi lời gọi đều có 2 tham số (`&state, false` hoặc `&state, !self.cfg.spell_check`). Chỉ có 1 call site thật (trong `render_word`).

- [ ] **Step 6: Chạy toàn bộ test — phải xanh**

Run: `cd core && cargo test`
Expected: PASS toàn bộ, gồm 4 test loose mới + toàn bộ test strict cũ (`english_words_restored`, `vietnamese_words_kept`, `spell_off_transforms_anyway`, v.v.).

Lưu ý: `spell_off_transforms_anyway` (`spell.rs:213-224`) vẫn PASS vì `mask`→`mák` đúng cả trong loose. Nếu muốn, đổi tên test cho khớp nghĩa mới nhưng không bắt buộc.

- [ ] **Step 7: Commit**

```bash
git add core/src/engine/spell.rs core/src/engine/mod.rs
git commit -m "feat(engine): chế độ gõ thoải mái (loose spell-check)

spell_check:false giờ = loose thay vì đặt dấu mù. loose = strict trừ
kiểm tra phụ âm cuối + luật thanh-trên-tắc, nới no-vowel cho từ có đ.
Cho gõ tắt (đc, nèk) nhưng vẫn bắt tiếng Anh có cụm bất khả (clear,
sound, status). Đánh đổi: mask→mák.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Cập nhật nhãn UI trong Settings

Đổi nhãn/mô tả nút `spell_check` cho đúng nghĩa mới (BẬT = chặt, TẮT = gõ thoải mái).

**Files:**
- Modify: `app/Sources/OreoKey/SettingsWindow.swift:151-154`

**Interfaces:**
- Consumes: `binding.spell_check` (đã có), component `ToggleRow(title:detail:isOn:)`.

- [ ] **Step 1: Đổi `ToggleRow` của `spell_check`**

Thay khối `app/Sources/OreoKey/SettingsWindow.swift:151-154`:

```swift
                    ToggleRow(
                        title: "Kiểm tra chính tả",
                        detail: "Từ không phải tiếng Việt tự trả về phím gốc (mask, class...)",
                        isOn: binding.spell_check)
```

bằng:

```swift
                    ToggleRow(
                        title: "Kiểm tra chính tả (chặt)",
                        detail: "Tắt = gõ thoải mái: cho gõ tắt (đc, nèk) mà vẫn nhận diện tiếng Anh có cụm bất khả (clear, sound). Bật: bảo vệ tối đa từ tiếng Anh (mask, class).",
                        isOn: binding.spell_check)
```

- [ ] **Step 2: Build app để chắc không lỗi cú pháp Swift**

Run: `cd app && swift build`
Expected: `Build complete!` (không lỗi biên dịch).

- [ ] **Step 3: Commit**

```bash
git add app/Sources/OreoKey/SettingsWindow.swift
git commit -m "ui: nhãn nút kiểm tra chính tả phản ánh chế độ gõ thoải mái

Tắt nút = loose (gõ tắt đc/nèk vẫn nhận tiếng Anh) thay vì đặt dấu mù.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage:**
- Mô hình `loose = strict − phụ âm cuối` → Task 2 Step 3 (nhánh loose).
- Nới no-vowel cho `đ` (`đc`) → Task 2 Step 3 (nhánh `loose` trong `else` không-nguyên-âm) + test `loose_allows_abbreviations`.
- Đổi nghĩa `spell_check` (`render_word` luôn lọc) → Task 2 Step 4.
- Giữ tầng bắt tiếng Anh (`clear`, `sound`, `status`) → test `loose_still_restores_english`.
- Đánh đổi `mask`→`mák` → test `loose_transforms_ambiguous_english_by_design`.
- VNI hưởng lợi → bộ lọc trên `WordState`, `raw_render` phủ cả vni (Task 1).
- Config JSON không đổi → không sửa `config.rs`, đúng ràng buộc.
- Nhãn UI → Task 3.
- Không đụng `INITIALS`/`NUCLEI` → Task 2 chỉ sửa nhánh finals/no-vowel.

**Placeholder scan:** Không có TBD/TODO; mọi step có code/command cụ thể.

**Type consistency:** `is_acceptable(&state, loose)` dùng nhất quán ở `render_word` (`!self.cfg.spell_check`) và test (`false`/qua Engine). `raw_render(method, keys, modern_tone, flexible_marks)` khớp giữa định nghĩa (Task 1 Step 1) và lời gọi (telex/vni Step 2-3).
