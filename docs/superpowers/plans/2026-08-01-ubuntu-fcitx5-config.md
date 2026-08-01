# OreoKey Ubuntu Fcitx5 and Config App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Fcitx5 adapter and GTK4 configuration application that share the tested Linux core contract.

**Architecture:** The Fcitx5 C++ addon maps its input contexts to opaque Rust sessions. A GTK4 Rust application edits `LinuxSettings`, validates before atomic save, and sends a best-effort D-Bus reload signal; a new adapter context always loads the latest valid snapshot.

**Tech Stack:** Rust 2021, C++17, Fcitx5, CMake, GTK4/GLib, GTest, D-Bus.

## Global Constraints

- Implement only after `2026-08-01-ubuntu-core-ibus.md` is green and its header ABI is frozen.
- Target Ubuntu 22.04/24.04 on amd64/arm64; all runtime code is unprivileged.
- Never duplicate Telex/VNI transformation logic in C++ or GTK.
- The UI exposes no macOS injection profiles or bundle-ID compatibility settings.
- Config writes use `LinuxSettings`; invalid or missing config must never crash an adapter.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `linux/fcitx5/src/oreokey_engine.h` | Fcitx5 `InputMethodEngineV2` declaration. |
| `linux/fcitx5/src/oreokey_engine.cpp` | Key translation, preedit/commit and context lifetime. |
| `linux/fcitx5/src/factory.cpp` | Addon factory registration. |
| `linux/fcitx5/data/oreokey.conf` | Addon metadata. |
| `linux/fcitx5/data/oreokey-vi.conf` | Input-method metadata. |
| `linux/fcitx5/CMakeLists.txt` | Native build/install rules. |
| `linux/fcitx5/tests/engine_test.cpp` | Fake-context regression tests. |
| `linux/config/Cargo.toml` | GTK app package. |
| `linux/config/src/main.rs` | Application lifecycle and save/reload orchestration. |
| `linux/config/src/settings_page.rs` | General setting validation/bindings. |
| `linux/config/src/macros_page.rs` | Macro CRUD validation. |
| `linux/config/src/convert_page.rs` | Text conversion view. |

### Task 1: Implement the Fcitx5 addon

**Files:**
- Create: `linux/fcitx5/src/oreokey_engine.h`, `linux/fcitx5/src/oreokey_engine.cpp`, `linux/fcitx5/src/factory.cpp`, `linux/fcitx5/data/oreokey.conf`, `linux/fcitx5/data/oreokey-vi.conf`, `linux/fcitx5/CMakeLists.txt`, `linux/fcitx5/tests/engine_test.cpp`
- Test: `linux/fcitx5/tests/engine_test.cpp`

**Interfaces:**
- Consumes: `ok_linux_session_new`, `ok_linux_session_key`, `ok_linux_session_action_text`, and `ok_linux_session_free`.
- Produces: addon library `oreokey` and input-method id `oreokey-vi`.

- [ ] **Step 1: Write failing fake-context test**

```cpp
TEST(OreoKeyEngine, TelexUpdatesPreedit) {
    TestInputContext context;
    OreoKeyEngine engine;
    engine.keyEvent(entry(), keyEvent(context, FcitxKey_a));
    EXPECT_EQ(context.preedit(), "a");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cmake -S linux/fcitx5 -B linux/fcitx5/build && cmake --build linux/fcitx5/build`

Expected: FAIL because the addon files and Fcitx5 target do not exist.

- [ ] **Step 3: Implement InputMethodEngineV2 mapping**

Store one Rust session in each input context's user data and free it at context destruction. Map printable keys, backspace, word breaks, focus loss and navigation to the C ABI exactly as IBus. Use client preedit plus `updateUserInterface` for preedit and `commitString` for commits. Return unfiltered events for forward, disabled, and commit-and-forward actions.

- [ ] **Step 4: Install Fcitx metadata**

Install the library to Fcitx5's multiarch addon directory, addon config to `share/fcitx5/addon`, and input-method config to `share/fcitx5/inputmethod`. Use addon `oreokey`, label `VI`, language `vi`, and no unneeded dependencies.

- [ ] **Step 5: Add regression cases and run them**

Test tone commit, macro commit, backspace, disabled forwarding, focus reset and two independent contexts.

Run: `ctest --test-dir linux/fcitx5/build --output-on-failure`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add linux/fcitx5
git commit -m "feat(fcitx5): add OreoKey input method addon"
```

### Task 2: Build the GTK4 configuration application

**Files:**
- Create: `linux/config/Cargo.toml`, `linux/config/src/main.rs`, `linux/config/src/settings_page.rs`, `linux/config/src/macros_page.rs`, `linux/config/src/convert_page.rs`, `linux/config/data/com.oreosolutions.OreoKey.desktop`
- Modify: `Cargo.toml`
- Test: `linux/config/src/settings_page.rs`, `linux/config/src/macros_page.rs`

**Interfaces:**
- Consumes: `oreokey_core::linux_config::LinuxSettings` and `oreokey_core::engine::encoding::convert`.
- Produces: executable `oreokey-config` and desktop entry `com.oreosolutions.OreoKey`.

- [ ] **Step 1: Write failing validation tests**

```rust
#[test]
fn rejects_unknown_spell_mode_before_save() {
    assert_eq!(validate_spell_mode("casual"), Err("Chế độ chính tả không hợp lệ".into()));
}

#[test]
fn duplicate_macro_source_is_rejected() {
    let entries = vec![MacroEntry { from: "vn".into(), to: "Việt Nam".into() }];
    assert!(add_macro(entries, "vn", "Vietnam").is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oreokey-config`

Expected: FAIL because package and validation functions do not exist.

- [ ] **Step 3: Implement the four-page app**

Create a `gtk::Application` with `Chung`, `Gõ tắt`, `Chuyển mã`, and `Giới thiệu`. Bind method, spell mode, enabled, modern tone, flexible marks, censor and hotkey to in-memory settings. Save validates all fields then calls `LinuxSettings::save_at(config_path())`; display the exact error without writing partial JSON. Macro additions reject empty or duplicate sources. Conversion uses the core encoding function for Unicode, VNI-Windows and TCVN3.

- [ ] **Step 4: Test without a display and build release binary**

Run: `cargo test -p oreokey-config && cargo build -p oreokey-config --release`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock linux/config
git commit -m "feat(linux): add GTK configuration app"
```

### Task 3: Add safe config reload signaling

**Files:**
- Create: `linux/common/oreokey-reload.xml`, `linux/config/tests/reload_test.rs`, `linux/fcitx5/tests/smoke-install.sh`
- Modify: `core/src/linux_config.rs`, `linux/ibus/src/main.c`, `linux/fcitx5/src/oreokey_engine.cpp`, `linux/config/src/main.rs`
- Test: `linux/config/tests/reload_test.rs`, `linux/fcitx5/tests/smoke-install.sh`

**Interfaces:**
- Consumes: saved `LinuxSettings` and session D-Bus.
- Produces: an empty `org.oreosolutions.OreoKey.Reload` signal and context-creation fallback reload.

- [ ] **Step 1: Write failing save/reload ordering test**

```rust
#[test]
fn save_requests_reload_after_atomic_rename() {
    let events = RecordingReload::default();
    save_and_request_reload(&settings, &path, &events).unwrap();
    assert!(path.exists());
    assert_eq!(events.count(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oreokey-config --test reload_test`

Expected: FAIL because save-and-reload orchestration is absent.

- [ ] **Step 3: Implement best-effort signal and adapter reload**

Save and fsync before emitting the signal. A missing session bus or engine is a nonfatal UI status, never a reason to roll back the save. IBus/Fcitx5 reset existing contexts on the signal and load settings when constructing a new session.

- [ ] **Step 4: Add Fcitx staging assertions**

```sh
test -f "$1/usr/share/fcitx5/addon/oreokey.conf"
test -f "$1/usr/share/fcitx5/inputmethod/oreokey-vi.conf"
find "$1/usr/lib" -name 'oreokey.so' -print -quit | grep -q .
```

- [ ] **Step 5: Run full adapter/config checks**

Run: `cargo test -p oreokey-core -p oreokey-config && ctest --test-dir linux/fcitx5/build --output-on-failure`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add linux/common linux/config core/src/linux_config.rs linux/ibus linux/fcitx5
git commit -m "feat(linux): reload OreoKey settings in adapters"
```

