# Typing Behavior Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Nâng "Kiểm tra chính tả" thành 3 mức (Chặt/Thường/Thoải mái), sửa nháy chữ ở kitty/Java-Swing (issue #1), và sửa VNI gõ số thanh trước số mũ giữa âm tiết (issue #4).

**Architecture:** Engine Rust thuần `std` (không serde) sở hữu logic gõ; `config.rs` map `Settings` (serde JSON) → `EngineConfig`; FFI trao đổi cả khối JSON với vỏ Swift. Ba phần độc lập, làm theo thứ tự B → A → C.

**Tech Stack:** Rust (cargo, engine + FFI C ABI), Swift/SwiftUI (vỏ macOS), JSON settings.

## Global Constraints

- Engine (`core/src/engine/`) chỉ dùng `std` — **không** thêm `serde`/crate ngoài vào module engine. `SpellMode` của engine là enum thuần.
- `EngineConfig` map từ `Settings` trong `config.rs::engine_config()`; field mode lưu dạng **String** trong `Settings` (theo đúng tiền lệ `method: String`), map sang enum trong `engine_config()`.
- Nhãn 3 mức (verbatim): **Chặt** = `"strict"`, **Thường** = `"standard"`, **Thoải mái** = `"loose"`. Mặc định `"strict"`.
- Test engine: `cargo test`. Build app: `./scripts/build.sh`.
- Commit message tiếng Việt, kèm dòng `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- Nhánh làm việc: `feat/typing-behavior-improvements`.

---

## Task 1 (B / Issue #1): App-profiles cho kitty & app nháy chữ

**Files:**
- Modify: `data/app-profiles.json`
- Test: `core/src/platform/profiles.rs` (module `tests`)

**Interfaces:**
- Consumes: `Profiles::load_default()`, `resolve(bundle, &HashMap::new(), None) -> ResolvedProfile`, `FixMode::InjectFast` (đã có).
- Produces: không có API mới — chỉ dữ liệu + test.

- [ ] **Step 1: Lấy bundle ID Burp Suite (không đoán)**

Nếu có Burp cài máy, chạy để lấy ID thật:
```bash
osascript -e 'id of app "Burp Suite Professional"' 2>/dev/null \
  || osascript -e 'id of app "Burp Suite Community Edition"' 2>/dev/null
```
Ghi lại giá trị (ví dụ khả dĩ `com.portswigger.burp.suite`). Nếu **không** có Burp
để xác nhận, **bỏ Burp khỏi lần này** và chỉ thêm các bundle terminal chắc chắn ở
Step 3 — không ghi ID phỏng đoán vào file.

- [ ] **Step 2: Viết test thất bại**

Thêm vào `mod tests` trong `core/src/platform/profiles.rs`:
```rust
#[test]
fn terminals_and_swing_use_inject_fast() {
    let p = Profiles::load_default();
    let none = HashMap::new();
    for bundle in [
        "net.kovidgoyal.kitty",
        "org.alacritty",
        "com.github.wez.wezterm",
        "co.zeit.hyper",
    ] {
        assert_eq!(
            p.resolve(bundle, &none, None).mode,
            FixMode::InjectFast,
            "{bundle} phải là InjectFast (tránh thử AX gây nháy)"
        );
    }
}
```

- [ ] **Step 3: Chạy test — kỳ vọng FAIL**

Run: `cargo test -p oreokey-core terminals_and_swing_use_inject_fast`
Expected: FAIL (`net.kovidgoyal.kitty` hiện resolve ra `Auto`).

- [ ] **Step 4: Thêm profile vào `data/app-profiles.json`**

Thêm các dòng sau vào object `"apps"` (khối terminal, sau `com.mitchellh.ghostty`).
Nếu Step 1 lấy được ID Burp, thêm dòng Burp với ID **thật** đó; nếu không, bỏ dòng Burp:
```json
    "net.kovidgoyal.kitty": { "mode": "inject_fast" },
    "org.alacritty": { "mode": "inject_fast" },
    "com.github.wez.wezterm": { "mode": "inject_fast" },
    "co.zeit.hyper": { "mode": "inject_fast" },
```

- [ ] **Step 5: Chạy test — kỳ vọng PASS**

Run: `cargo test -p oreokey-core terminals_and_swing_use_inject_fast`
Expected: PASS.

- [ ] **Step 6: Chạy toàn bộ test profiles để chắc không hồi quy**

Run: `cargo test -p oreokey-core platform::profiles`
Expected: tất cả PASS.

- [ ] **Step 7: Commit**

```bash
git add data/app-profiles.json core/src/platform/profiles.rs
git commit -m "$(printf 'fix(platform): kitty & terminal Swing nháy chữ vì thiếu profile (issue #1)\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 2 (A1): Engine `SpellMode` 3 mức

**Files:**
- Modify: `core/src/engine/mod.rs` (enum + `EngineConfig` + `render_word` + test sites)
- Modify: `core/src/config.rs:105` (bridge tạm trong `engine_config()`)
- Modify các test site còn dùng `spell_check`: `core/src/engine/spell.rs:139`, `core/src/engine/censor.rs:32`, `core/src/engine/macros.rs:35`, `core/src/engine/telex.rs:203,344,390,404,421`, `core/src/engine/mod.rs:434`

**Interfaces:**
- Produces:
  - `pub enum SpellMode { Strict, Standard, Loose }` (derive `Debug, Clone, Copy, PartialEq, Eq`) trong `engine/mod.rs`.
  - `EngineConfig.spell_mode: SpellMode` thay cho `spell_check: bool`; `Default` = `Strict`.
- Consumes: `spell::is_transformed`, `spell::is_acceptable(state, loose)` (chữ ký giữ nguyên).

- [ ] **Step 1: Viết test Loose thất bại**

Thêm vào `mod tests` trong `core/src/engine/mod.rs`:
```rust
fn engine_mode(mode: SpellMode) -> Engine {
    Engine::new(EngineConfig {
        method: TypingMethod::Telex,
        spell_mode: mode,
        modern_tone: false,
        macros_enabled: false,
        flexible_marks: true,
        censor_enabled: false,
    })
}

#[test]
fn loose_never_restores_english() {
    // Thoải mái: KHÔNG khôi phục — từ có dấu vẫn giữ dấu.
    let mut e = engine_mode(SpellMode::Loose);
    assert_eq!(type_str(&mut e, "mask"), "mák"); // strict sẽ bung "mask"
    let mut e = engine_mode(SpellMode::Loose);
    assert_eq!(type_str(&mut e, "class"), "clas"); // s hủy s (ass→as), không bung raw
}

#[test]
fn strict_still_restores_english() {
    let mut e = engine_mode(SpellMode::Strict);
    assert_eq!(type_str(&mut e, "mask"), "mask");
    assert_eq!(type_str(&mut e, "class"), "class");
}
```

- [ ] **Step 2: Chạy — kỳ vọng FAIL (compile error: `spell_mode`/`SpellMode` chưa có)**

Run: `cargo test -p oreokey-core loose_never_restores_english`
Expected: FAIL biên dịch (`SpellMode` không tồn tại).

- [ ] **Step 3: Thêm enum `SpellMode` và đổi field `EngineConfig`**

Trong `core/src/engine/mod.rs`, thêm sau `enum TypingMethod` (khoảng dòng 53):
```rust
/// Mức kiểm tra chính tả: Chặt (bảo vệ tối đa tiếng Anh) → Thường (gõ
/// tắt, vẫn bắt cụm bất khả) → Thoải mái (không khôi phục, luôn đặt dấu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellMode {
    Strict,
    Standard,
    Loose,
}
```
Trong `EngineConfig`, đổi:
```rust
    pub spell_check: bool,
```
thành:
```rust
    pub spell_mode: SpellMode,
```
Trong `impl Default for EngineConfig`, đổi `spell_check: true,` thành `spell_mode: SpellMode::Strict,`.

- [ ] **Step 4: Sửa gate trong `render_word`**

Trong `core/src/engine/mod.rs::render_word`, đổi khối:
```rust
        if spell::is_transformed(&state)
            && !spell::is_acceptable(&state, !self.cfg.spell_check)
        {
            return (raw.to_string(), true);
        }
```
thành:
```rust
        // Thoải mái (Loose) không bao giờ khôi phục; Chặt/Thường dùng gate.
        if self.cfg.spell_mode != SpellMode::Loose
            && spell::is_transformed(&state)
            && !spell::is_acceptable(&state, self.cfg.spell_mode == SpellMode::Standard)
        {
            return (raw.to_string(), true);
        }
```

- [ ] **Step 5: Cập nhật mọi test-site trong engine**

Thay `spell_check: true,` → `spell_mode: SpellMode::Strict,` và `spell_check: false,` → `spell_mode: SpellMode::Standard,` tại các dòng: `mod.rs:434`, `spell.rs:139`, `censor.rs:32`, `macros.rs:35`, `telex.rs:203,344,390,404,421`. (Các file test dùng `use crate::engine::...` — thêm `SpellMode` vào `use` nếu thiếu.)

Kiểm nhanh không sót:
```bash
grep -rn "spell_check" core/src/engine/
```
Kỳ vọng: chỉ còn (nếu có) trong chuỗi tên hàm `late_circumflex_respects_spell_check` (telex.rs:387) — đổi tên hàm đó thành `late_circumflex_respects_spell_mode` cho khớp.

- [ ] **Step 6: Bridge tạm trong `config.rs` để crate biên dịch**

Trong `core/src/config.rs::engine_config()` (dòng 105), đổi:
```rust
            spell_check: self.spell_check,
```
thành:
```rust
            spell_mode: if self.spell_check {
                crate::engine::SpellMode::Strict
            } else {
                crate::engine::SpellMode::Standard
            },
```
(`Settings.spell_check: bool` giữ nguyên ở task này — task 3 mới đổi.)

- [ ] **Step 7: Chạy test — kỳ vọng PASS**

Run: `cargo test -p oreokey-core`
Expected: toàn bộ PASS (gồm 2 test mới + mọi test cũ).

- [ ] **Step 8: Commit**

```bash
git add core/src/engine core/src/config.rs
git commit -m "$(printf 'feat(engine): SpellMode 3 mức (Chặt/Thường/Thoải mái)\n\nThoải mái bỏ qua gate khôi phục, luôn đặt dấu.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 3 (A2): `Settings.spell_mode` + migration

**Files:**
- Modify: `core/src/config.rs` (field, default, `engine_config`, `load`, migration fn, tests)

**Interfaces:**
- Consumes: `crate::engine::SpellMode` (từ Task 2).
- Produces: `Settings.spell_mode: String` (`"strict"|"standard"|"loose"`, default `"strict"`); JSON của `Settings` từ nay chứa `spell_mode`, **không** còn `spell_check`.

- [ ] **Step 1: Viết test migration thất bại**

Thêm vào `mod tests` trong `core/src/config.rs`:
```rust
#[test]
fn migrates_legacy_spell_check() {
    // File cũ chỉ có spell_check → map sang spell_mode.
    let on: Settings = serde_json::from_str(&migrate_json(r#"{"spell_check":true}"#)).unwrap();
    assert_eq!(on.spell_mode, "strict");
    let off: Settings = serde_json::from_str(&migrate_json(r#"{"spell_check":false}"#)).unwrap();
    assert_eq!(off.spell_mode, "standard");
}

#[test]
fn keeps_explicit_spell_mode() {
    let s: Settings =
        serde_json::from_str(&migrate_json(r#"{"spell_mode":"loose"}"#)).unwrap();
    assert_eq!(s.spell_mode, "loose");
}

#[test]
fn migrate_passes_through_non_object() {
    // Không phải JSON hợp lệ → trả nguyên văn (load sẽ về default).
    assert_eq!(migrate_json("not json"), "not json");
}
```

- [ ] **Step 2: Chạy — kỳ vọng FAIL (`spell_mode`/`migrate_json` chưa có)**

Run: `cargo test -p oreokey-core config::`
Expected: FAIL biên dịch.

- [ ] **Step 3: Đổi field `Settings` + default + `engine_config`**

Trong `core/src/config.rs`:
- `struct Settings`: đổi `pub spell_check: bool,` → `pub spell_mode: String,`.
- `impl Default for Settings`: đổi `spell_check: true,` → `spell_mode: "strict".into(),`.
- `engine_config()`: thay khối bridge (Task 2 Step 6) bằng map từ String:
```rust
            spell_mode: match self.spell_mode.as_str() {
                "loose" => crate::engine::SpellMode::Loose,
                "standard" => crate::engine::SpellMode::Standard,
                _ => crate::engine::SpellMode::Strict,
            },
```

- [ ] **Step 4: Thêm `migrate_json` và gọi trong `load`**

Trong `core/src/config.rs`, đổi `load()`:
```rust
pub fn load() -> Settings {
    match fs::read_to_string(settings_path()) {
        Ok(text) => serde_json::from_str(&migrate_json(&text)).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Di trú file cũ: `spell_check: bool` (đã bỏ) → `spell_mode`. Trả nguyên
/// văn nếu không phải object JSON (load sẽ về default).
fn migrate_json(text: &str) -> String {
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(text) else {
        return text.to_string();
    };
    if let Some(obj) = v.as_object_mut() {
        if !obj.contains_key("spell_mode") {
            if let Some(sc) = obj.get("spell_check").and_then(|x| x.as_bool()) {
                let mode = if sc { "strict" } else { "standard" };
                obj.insert("spell_mode".into(), serde_json::Value::String(mode.into()));
            }
        }
    }
    v.to_string()
}
```

- [ ] **Step 5: Chạy test — kỳ vọng PASS**

Run: `cargo test -p oreokey-core config::`
Expected: PASS (gồm `json_round_trip`, `unknown_fields...`, `corrupt_file...` cũ vẫn xanh — chú ý `json_round_trip` không đụng `spell_mode` nên vẫn hợp lệ).

- [ ] **Step 6: Chạy toàn bộ test**

Run: `cargo test -p oreokey-core`
Expected: toàn bộ PASS.

- [ ] **Step 7: Commit**

```bash
git add core/src/config.rs
git commit -m "$(printf 'feat(config): spell_mode 3 mức + migrate spell_check cũ\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 4 (A3): UI Swift — segmented control 3 mức

**Files:**
- Modify: `app/Sources/OreoKey/Core.swift:24` (struct field)
- Modify: `app/Sources/OreoKey/SettingsWindow.swift:150-171` (Section "Hành vi gõ")

**Interfaces:**
- Consumes: JSON settings từ Rust nay có `spell_mode: String` (Task 3).
- Produces: không có API mới.

- [ ] **Step 1: Đổi field trong `CoreSettings`**

Trong `app/Sources/OreoKey/Core.swift`, đổi:
```swift
    var spell_check: Bool
```
thành:
```swift
    var spell_mode: String
```

- [ ] **Step 2: Thay `ToggleRow` chính tả bằng segmented Picker**

Trong `app/Sources/OreoKey/SettingsWindow.swift`, trong `Section("Hành vi gõ")`,
thay khối:
```swift
                    ToggleRow(
                        title: "Kiểm tra chính tả (chặt)",
                        detail: "Tắt = gõ thoải mái: cho gõ tắt (đc, nèk) mà vẫn nhận diện tiếng Anh có cụm bất khả (clear, sound). Bật: bảo vệ tối đa từ tiếng Anh (mask, class).",
                        isOn: binding.spell_check)
```
bằng:
```swift
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Kiểm tra chính tả")
                        Picker("Kiểm tra chính tả", selection: binding.spell_mode) {
                            Text("Chặt").tag("strict")
                            Text("Thường").tag("standard")
                            Text("Thoải mái").tag("loose")
                        }
                        .pickerStyle(.segmented)
                        .labelsHidden()
                        Text(spellModeDetail(binding.wrappedValue.spell_mode))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
```

- [ ] **Step 3: Thêm helper mô tả mức**

Trong `struct GeneralPane`, thêm method (cạnh `hotkeyBinding`):
```swift
    private func spellModeDetail(_ mode: String) -> String {
        switch mode {
        case "loose":
            return "Luôn đặt dấu, không khôi phục từ tiếng Anh."
        case "standard":
            return "Cho gõ tắt (đc, nèk), vẫn nhận diện tiếng Anh có cụm bất khả (clear, sound)."
        default:
            return "Bảo vệ tối đa từ tiếng Anh (mask, class)."
        }
    }
```

- [ ] **Step 4: Build app — kỳ vọng biên dịch sạch**

Run: `./scripts/build.sh`
Expected: build thành công → `dist/OreoKey.app`, không lỗi Swift.

- [ ] **Step 5: Kiểm tra tay (checklist ngắn)**

Mở app → Cài đặt → Chung → "Hành vi gõ": thấy segmented `[Chặt|Thường|Thoải mái]`,
đổi mức thì dòng mô tả đổi theo và được lưu (đóng/mở lại giữ đúng mức).

- [ ] **Step 6: Commit**

```bash
git add app/Sources/OreoKey/Core.swift app/Sources/OreoKey/SettingsWindow.swift
git commit -m "$(printf 'feat(ui): chọn mức kiểm tra chính tả bằng segmented (Chặt/Thường/Thoải mái)\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 5 (C1 / Issue #4): `spell::is_live_prefix`

**Files:**
- Modify: `core/src/engine/spell.rs` (thêm `is_live_prefix` + `can_become_prefix` + tests)

**Interfaces:**
- Consumes: `INITIALS`, `NUCLEI`, `FINALS`, `base_of`, `marked_lower`, `vowel_indices` (đã có trong `spell.rs`).
- Produces: `pub fn is_live_prefix(state: &WordState) -> bool`.

- [ ] **Step 1: Viết test thất bại**

Thêm vào `mod tests` trong `core/src/engine/spell.rs`:
```rust
#[test]
fn live_prefix_recognizes_incomplete_toned_nucleus() {
    use crate::engine::vni;
    use crate::engine::WordState;
    let build = |keys: &str| {
        let mut s = WordState::default();
        for c in keys.chars() {
            vni::apply_key(&mut s, c);
        }
        s
    };
    // "thie1" (thié): nhân âm "ie" + thanh — CÒN SỐNG (chờ mũ 6).
    assert!(super::is_live_prefix(&build("thie1")));
    // "die" + s tương đương telex là live, nhưng dead-cluster thì không:
    // "cla" (cl không phải phụ âm đầu tiếng Việt) → CHẾT.
    let cla = {
        let mut s = WordState::default();
        for c in "cla".chars() { vni::apply_key(&mut s, c); }
        s
    };
    assert!(!super::is_live_prefix(&cla));
}
```

- [ ] **Step 2: Chạy — kỳ vọng FAIL (`is_live_prefix` chưa có)**

Run: `cargo test -p oreokey-core live_prefix_recognizes_incomplete_toned_nucleus`
Expected: FAIL biên dịch.

- [ ] **Step 3: Thêm `can_become_prefix` và `is_live_prefix`**

Thêm vào `core/src/engine/spell.rs` (sau `can_become`):
```rust
/// `cluster` có phải TIỀN TỐ của `valid` không (cho phép nguyên âm trơn
/// khớp bản có dấu: "ie" là tiền tố của "iê", "iêu"). Khác `can_become`
/// ở chỗ không đòi cùng độ dài.
fn can_become_prefix(cluster: &str, valid: &str) -> bool {
    let cc: Vec<char> = cluster.chars().collect();
    let vc: Vec<char> = valid.chars().collect();
    if cc.len() > vc.len() {
        return false;
    }
    cc.iter()
        .zip(vc.iter())
        .all(|(&c, &v)| c == v || (c == base_of(c) && base_of(v) == c))
}

/// Cụm hiện tại còn có thể trở thành âm tiết hợp lệ bằng cách gõ THÊM
/// dấu/chữ không (tiền tố hợp lệ)? Dùng để KHÔNG khóa `raw_mode` quá sớm:
/// trạng thái "còn sống" khác "chết hẳn" (cụm bất khả như `cl`, nguyên âm
/// rời). Cho phép nhân âm dở kể cả khi ĐÃ có thanh (khác `is_acceptable`).
pub fn is_live_prefix(state: &WordState) -> bool {
    let letters = &state.letters;
    if letters.iter().any(|l| !l.base.is_ascii_alphabetic()) {
        return false;
    }
    if letters.iter().skip(1).any(|l| l.stroke) {
        return false;
    }
    let vidx = vowel_indices(letters);
    let Some(&run_start) = vidx.first() else {
        // Chưa có nguyên âm: phụ âm đầu phải là tiền tố của một INITIAL.
        let initial = letters.iter().map(marked_lower).collect::<String>();
        return INITIALS.iter().any(|i| i.starts_with(&initial));
    };
    // Cụm nguyên âm phải liên tục.
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
    // Đã có nguyên âm → phụ âm đầu phải khớp HẲN một INITIAL.
    let initial = render_range(0, run_start);
    if !INITIALS.contains(&initial.as_str()) {
        return false;
    }
    let nucleus = render_range(run_start, run_end + 1);
    if !NUCLEI.iter().any(|n| can_become_prefix(&nucleus, n)) {
        return false;
    }
    // Phụ âm cuối đang gõ dở phải là tiền tố của một FINAL.
    let final_c = render_range(run_end + 1, letters.len());
    FINALS.iter().any(|f| f.starts_with(&final_c))
}
```

- [ ] **Step 4: Chạy test — kỳ vọng PASS**

Run: `cargo test -p oreokey-core live_prefix_recognizes_incomplete_toned_nucleus`
Expected: PASS.

- [ ] **Step 5: Chạy toàn bộ spell tests**

Run: `cargo test -p oreokey-core engine::spell`
Expected: PASS (không hồi quy).

- [ ] **Step 6: Commit**

```bash
git add core/src/engine/spell.rs
git commit -m "$(printf 'feat(spell): is_live_prefix phân biệt trạng thái còn-sống với chết-hẳn\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 6 (C2 / Issue #4): Không khóa `raw_mode` cho trạng thái còn-sống

**Files:**
- Modify: `core/src/engine/mod.rs::on_char` (điều kiện set `raw_mode`)
- Test: `core/src/engine/mod.rs` (module `tests`) và `core/src/engine/vni.rs` (module `tests`)

**Interfaces:**
- Consumes: `spell::is_live_prefix(&state)` (Task 5), `self.build_state(&self.raw)`.
- Produces: không có API mới — thay đổi hành vi `on_char`.

- [ ] **Step 1: Viết test thất bại (repro issue #4 + không hồi quy)**

Thêm vào `mod tests` trong `core/src/engine/mod.rs` (dùng `engine_mode` từ Task 2):
```rust
#[test]
fn vni_tone_before_circumflex_midsyllable() {
    // Issue #4: gõ số thanh trước số mũ giữa âm tiết không được kẹt raw.
    let mut e = Engine::new(EngineConfig {
        method: TypingMethod::Vni,
        spell_mode: SpellMode::Strict,
        modern_tone: false,
        macros_enabled: false,
        flexible_marks: true,
        censor_enabled: false,
    });
    assert_eq!(type_str(&mut e, "thie16u"), "thiếu");
    let mut e2 = Engine::new(EngineConfig {
        method: TypingMethod::Vni,
        spell_mode: SpellMode::Strict,
        modern_tone: false,
        macros_enabled: false,
        flexible_marks: true,
        censor_enabled: false,
    });
    assert_eq!(type_str(&mut e2, "tie61t"), "tiết");
}

#[test]
fn english_still_restored_after_live_prefix_fix() {
    // Trạng thái còn-sống KHÔNG được phá auto-restore tiếng Anh (telex).
    let mut e = engine_mode(SpellMode::Strict);
    assert_eq!(type_str(&mut e, "dies"), "dies");
    let mut e = engine_mode(SpellMode::Strict);
    assert_eq!(type_str(&mut e, "lies"), "lies");
    let mut e = engine_mode(SpellMode::Strict);
    assert_eq!(type_str(&mut e, "class"), "class"); // dead-cluster latch ngay
}
```

- [ ] **Step 2: Chạy — kỳ vọng FAIL**

Run: `cargo test -p oreokey-core vni_tone_before_circumflex_midsyllable`
Expected: FAIL — hiện ra `"thie16u"` (kẹt raw).

- [ ] **Step 3: Sửa `on_char` — chỉ khóa `raw_mode` khi CHẾT hẳn**

Trong `core/src/engine/mod.rs::on_char`, trong nhánh `if restored {`, thay:
```rust
            if restored {
                self.raw_mode = true;
```
bằng:
```rust
            if restored {
                // Chỉ khóa khi từ CHẾT hẳn (cụm bất khả). Trạng thái CÒN
                // SỐNG (tiền tố hợp lệ, vd nhân âm dở chờ dấu mũ) giữ raw
                // hiển thị nhưng KHÔNG khóa — phím sau còn cơ hội hoàn
                // thiện âm tiết (issue #4).
                let state = self.build_state(&self.raw);
                if !spell::is_live_prefix(&state) {
                    self.raw_mode = true;
                }
```
(Phần còn lại của nhánh — `if was_settled { ... } else { text }` — giữ nguyên.)

Đảm bảo `use` của `mod.rs` thấy `spell` (đã có `use syllable::render_letters;`; `spell`
được gọi qua `spell::...` với khai báo `pub mod spell;` ở đầu file — không cần thêm import).

- [ ] **Step 4: Chạy test mới — kỳ vọng PASS**

Run: `cargo test -p oreokey-core vni_tone_before_circumflex_midsyllable english_still_restored_after_live_prefix_fix`
Expected: PASS cả hai.

- [ ] **Step 5: Thêm test VNI trong `vni.rs` (bám sát repro của issue)**

Thêm vào `mod tests` trong `core/src/engine/vni.rs` một test dùng full engine
(khác `v()` chỉ raw_render — cần đường qua spell). Đặt helper riêng:
```rust
#[test]
fn issue4_tone_before_mark_full_engine() {
    use crate::engine::{Engine, EngineConfig, SpellMode, TypingMethod};
    use crate::engine::testutil::type_str;
    let mk = || Engine::new(EngineConfig {
        method: TypingMethod::Vni,
        spell_mode: SpellMode::Strict,
        modern_tone: false,
        macros_enabled: false,
        flexible_marks: true,
        censor_enabled: false,
    });
    assert_eq!(type_str(&mut mk(), "thie16u"), "thiếu");
    assert_eq!(type_str(&mut mk(), "thieu61"), "thiếu"); // thứ tự cũ vẫn đúng
    assert_eq!(type_str(&mut mk(), "thie61t"), "thiết");
}
```

- [ ] **Step 6: Chạy toàn bộ test engine — kỳ vọng PASS, không hồi quy**

Run: `cargo test -p oreokey-core`
Expected: toàn bộ PASS.

- [ ] **Step 7: Commit**

```bash
git add core/src/engine/mod.rs core/src/engine/vni.rs
git commit -m "$(printf 'fix(engine): VNI số thanh trước số mũ giữa âm tiết bị kẹt raw (issue #4)\n\nKhông khóa raw_mode cho trạng thái còn-sống; chỉ khóa khi cụm chết hẳn.\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Task 7: Changelog + đóng gói

**Files:**
- Modify: `CHANGELOG.md` (mục "Chưa phát hành")

- [ ] **Step 1: Thêm mục changelog**

Trong `CHANGELOG.md`, dưới `## [Chưa phát hành]`, thêm mục "Đã thêm" (spell 3 mức)
và bổ sung "Đã sửa" (issue #1, #4), theo văn phong các mục hiện có (tiếng Việt,
hướng người dùng, không thuật ngữ code).

- [ ] **Step 2: Chạy toàn bộ test lần cuối**

Run: `cargo test -p oreokey-core`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add CHANGELOG.md
git commit -m "$(printf 'docs: changelog cho spell 3 mức + issue #1/#4\n\nCo-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>')"
```

---

## Ghi chú thứ tự & rủi ro

- **Task 2 → 3 → 4 phải theo thứ tự**: sau Task 3, JSON của Rust không còn
  `spell_check`; vỏ Swift cũ decode sẽ nil (Cài đặt hiện "Không đọc được") cho tới
  khi Task 4 xong. Ba task này nên land liền nhau trong cùng đợt.
- Task 1, 5, 6 độc lập với 2–4 (chỉ đụng engine/data), có thể review riêng.
- Không có thay đổi FFI (C ABI) — không cần rebuild header cầu nối.
