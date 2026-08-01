# OreoKey Ubuntu Core and IBus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a testable IBus engine for Ubuntu that reuses OreoKey's Rust typing engine without changing the macOS key path.

**Architecture:** Add a Linux composition-session API above the existing engine. A thin C ABI exposes one session per IBus input context; a GObject IBus engine owns preedit and commit calls, never hooks keyboard devices or injects keys.

**Tech Stack:** Rust 2021, IBus 1.5/GObject C, Meson, pkg-config, GLib test, Debian packaging primitives.

## Global Constraints

- Support Ubuntu 22.04 LTS and 24.04 LTS on amd64 and arm64.
- Keep `core/src/platform`, `core/src/ffi.rs`, Swift, DMG and Sparkle macOS-only.
- Linux must use framework preedit/commit APIs; never capture `/dev/input`, inject keys, or require root at runtime.
- A session owns exactly one `Engine`; input contexts must not share in-progress text.
- Linux config is XDG-scoped and must not read or modify macOS config paths.
- Every behavioral change begins with a failing automated test and ends with a focused commit.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `core/src/ime.rs` | Platform-neutral composition-session contract built around `Engine`. |
| `core/src/linux_config.rs` | Linux settings schema, validated JSON load/save and XDG path handling. |
| `core/src/linux_ffi.rs` | C ABI for allocating sessions, processing keys and reading actions. |
| `include/oreokey_linux.h` | Stable C declarations consumed by IBus and Fcitx5 adapters. |
| `linux/ibus/src/main.c` | IBus component registration and process lifecycle. |
| `linux/ibus/src/engine.c` | `IBusEngine` subclass translating IBus events to the C ABI. |
| `linux/ibus/data/oreokey.xml` | IBus component metadata and engine registration. |
| `linux/ibus/meson.build` | Native IBus build/install rules. |
| `linux/ibus/tests/test-engine.c` | GLib tests with an IBus test context. |

### Task 1: Add composition sessions

**Files:**
- Create: `core/src/ime.rs`
- Modify: `core/src/lib.rs`
- Test: `core/src/ime.rs`

**Interfaces:**
- Consumes: `engine::{Action, Engine, EngineConfig, KeyInput}`.
- Produces: `ImeSession::new(EngineConfig, MacroTable)`, `ImeSession::handle(KeyInput) -> ImeAction`, `ImeSession::set_enabled(bool)`, and `ImeSession::reset()`.

- [ ] **Step 1: Write failing session tests**

```rust
#[test]
fn composes_telex_without_delete_surrounding() {
    let mut s = ImeSession::default();
    assert_eq!(s.handle(KeyInput::Char('a')), ImeAction::Preedit("a".into()));
    assert_eq!(s.handle(KeyInput::Char('s')), ImeAction::Preedit("á".into()));
    assert_eq!(s.handle(KeyInput::WordBreak(Some(' '))), ImeAction::Commit("á ".into()));
}

#[test]
fn navigation_commits_preedit_then_forwards_key() {
    let mut s = ImeSession::default();
    s.handle(KeyInput::Char('a'));
    assert_eq!(s.handle(KeyInput::WordBreak(None)), ImeAction::CommitAndForward("a".into()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oreokey-core ime::tests`

Expected: FAIL because module `ime` and `ImeSession` do not exist.

- [ ] **Step 3: Write the minimal session API**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeAction {
    Forward,
    Preedit(String),
    Commit(String),
    CommitAndForward(String),
    Disabled,
}

pub struct ImeSession {
    engine: Engine,
    enabled: bool,
}
```

Consume printable characters into preedit. For a real word break, commit the preedit plus that break; when a macro expansion returns `Action::Replace`, commit its replacement. For a navigation reset, commit current preedit then return `CommitAndForward`. A disabled session resets and returns `Disabled`.

- [ ] **Step 4: Add regression coverage**

Add tests where backspace updates preedit, macro `vn -> Việt Nam` commits on space, focus reset clears composition, and two sessions preserve independent words.

- [ ] **Step 5: Run tests**

Run: `cargo test -p oreokey-core`

Expected: PASS, including existing engine tests.

- [ ] **Step 6: Commit**

```bash
git add core/src/ime.rs core/src/lib.rs
git commit -m "feat(core): add Linux IME composition sessions"
```

### Task 2: Add XDG Linux settings

**Files:**
- Create: `core/src/linux_config.rs`
- Modify: `core/src/lib.rs`, `core/Cargo.toml`
- Test: `core/src/linux_config.rs`

**Interfaces:**
- Consumes: `EngineConfig`, `MacroTable`, `ImeSession`.
- Produces: `LinuxSettings::load_at(&Path)`, `LinuxSettings::save_at(&Path)`, `config_path()`, `engine_config()`, `macro_table()`, and `ImeSession::from_settings(&LinuxSettings)`.

- [ ] **Step 1: Write failing path and invalid-config tests**

```rust
#[test]
fn xdg_path_uses_config_home() {
    assert_eq!(config_path_from(Some(Path::new("/tmp/config"))),
               PathBuf::from("/tmp/config/oreokey/settings.json"));
}

#[test]
fn invalid_config_is_not_overwritten() {
    let path = tempfile::tempdir().unwrap().path().join("settings.json");
    std::fs::write(&path, r#"{"method":"bad"}"#).unwrap();
    assert!(LinuxSettings::load_at(&path).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), r#"{"method":"bad"}"#);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oreokey-core linux_config::tests`

Expected: FAIL because `linux_config` and its test dependency do not exist.

- [ ] **Step 3: Implement validated atomic settings**

Implement `LinuxSettings` with method, enabled, spell mode, modern tone, macros, flexible marks, censor and Linux hotkey. Resolve `XDG_CONFIG_HOME` or `$HOME/.config`; create the directory 0700, acquire an advisory exclusive lock on a 0600 sibling lock file, write a 0600 temporary sibling, `sync_all`, then rename and release the lock. Invalid JSON or enum values return a typed error and leave the input file unchanged.

- [ ] **Step 4: Add session construction test**

```rust
#[test]
fn settings_build_a_vni_session() {
    let settings = LinuxSettings { method: "vni".into(), ..LinuxSettings::default() };
    let mut session = ImeSession::from_settings(&settings);
    session.handle(KeyInput::Char('a'));
    assert_eq!(session.handle(KeyInput::Char('6')), ImeAction::Preedit("â".into()));
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p oreokey-core`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add core/src/linux_config.rs core/src/lib.rs core/Cargo.toml Cargo.lock
git commit -m "feat(core): add XDG Linux settings"
```

### Task 3: Expose the Linux C ABI

**Files:**
- Create: `core/src/linux_ffi.rs`, `include/oreokey_linux.h`, `core/tests/linux_ffi_smoke.rs`
- Modify: `core/src/lib.rs`, `core/Cargo.toml`
- Test: `core/tests/linux_ffi_smoke.rs`

**Interfaces:**
- Consumes: `ImeSession`, `LinuxSettings`.
- Produces: opaque `ok_linux_session`, `ok_linux_session_new`, `ok_linux_session_key`, `ok_linux_session_action_text`, `ok_linux_session_free`.

- [ ] **Step 1: Write a failing FFI smoke test**

```rust
#[test]
fn ffi_returns_preedit_for_telex() {
    let session = unsafe { ok_linux_session_new(std::ptr::null()) };
    assert!(!session.is_null());
    assert_eq!(unsafe { ok_linux_session_key(session, 'a' as u32, OK_KEY_CHAR) }, OK_ACTION_PREEDIT);
    unsafe { ok_linux_session_free(session) };
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p oreokey-core --test linux_ffi_smoke`

Expected: FAIL because the exported functions and test target do not exist.

- [ ] **Step 3: Implement C ownership and action contract**

Header constants define char/backspace/word-break/reset key kinds and forward/preedit/commit/commit-and-forward/disabled action kinds. The opaque session owns its most recent UTF-8 action text; the returned pointer remains valid until the next call on that session. Invalid scalar values and unknown kinds return forward; no Rust panic may cross C.

- [ ] **Step 4: Compile a C consumer**

```c
struct ok_linux_session *s = ok_linux_session_new(NULL);
g_assert_cmpint(ok_linux_session_key(s, 'a', OK_KEY_CHAR), ==, OK_ACTION_PREEDIT);
g_assert_cmpstr(ok_linux_session_action_text(s), ==, "a");
ok_linux_session_free(s);
```

- [ ] **Step 5: Run FFI verification**

Run: `cargo test -p oreokey-core && cargo build -p oreokey-core --release`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add core/src/linux_ffi.rs core/src/lib.rs core/Cargo.toml include/oreokey_linux.h core/tests/linux_ffi_smoke.rs
git commit -m "feat(core): expose Linux IME C bridge"
```

### Task 4: Implement and stage the IBus engine

**Files:**
- Create: `linux/ibus/src/main.c`, `linux/ibus/src/engine.c`, `linux/ibus/src/engine.h`, `linux/ibus/data/oreokey.xml`, `linux/ibus/meson.build`, `linux/ibus/tests/test-engine.c`, `linux/ibus/tests/smoke-install.sh`
- Test: `linux/ibus/tests/test-engine.c`, `linux/ibus/tests/smoke-install.sh`

**Interfaces:**
- Consumes: all declarations in `include/oreokey_linux.h`.
- Produces: executable `ibus-engine-oreokey` and IBus engine id `oreokey`.

- [ ] **Step 1: Write failing GLib test**

```c
static void test_key_event_updates_preedit(void) {
    IBusEngine *engine = oreokey_engine_new();
    g_assert_true(oreokey_engine_process_key_event(engine, 'a', 0, 0));
    g_assert_cmpstr(oreokey_engine_test_preedit(engine), ==, "a");
    g_object_unref(engine);
}
```

- [ ] **Step 2: Configure build to verify it fails**

Run: `meson setup linux/ibus/build linux/ibus && meson test -C linux/ibus/build`

Expected: FAIL because the native engine sources are absent.

- [ ] **Step 3: Implement adapter behavior**

Subclass `IBusEngine`. Map unmodified printable keys to char, backspace to backspace, real breaks to word break, and navigation/focus/shortcuts to reset-plus-forward. On preedit update IBus preedit text; on commit hide preedit then commit text; on commit-and-forward commit then return `FALSE`; on forward or disabled return `FALSE`. Never call key injection or read from keyboard devices.

- [ ] **Step 4: Register and install component metadata**

Install XML under `share/ibus/component`, executable under libexec, engine name `oreokey`, language `vi`, layout `us`, icon `oreokey`, rank 80. Add staging script assertions for executable and XML paths plus the engine id.

- [ ] **Step 5: Run IBus and core verification**

Run: `meson compile -C linux/ibus/build && meson test -C linux/ibus/build --print-errorlogs && cargo test -p oreokey-core`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add linux/ibus
git commit -m "feat(ibus): add OreoKey input method engine"
```
