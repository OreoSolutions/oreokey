//! Cấu hình ứng dụng. Rust là chủ sở hữu duy nhất: Swift đọc/ghi qua
//! FFI bằng chuỗi JSON, file lưu tại `~/Library/Application Support/OreoKey/`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// All writes originate in this process (the event tap and the Swift FFI).
/// Serializing them keeps the atomic rename protocol atomic for callers too.
static SAVE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroEntry {
    pub from: String,
    pub to: String,
}

/// Phím tắt toàn cục bật/tắt tiếng Việt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hotkey {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub cmd: bool,
    /// Virtual keycode macOS (49 = Space). `None` = chỉ dùng modifier.
    pub keycode: Option<u16>,
}

impl Default for Hotkey {
    fn default() -> Self {
        // ⌃⇧Space — tránh đụng ⌃Space (đổi input source của hệ thống).
        Hotkey {
            ctrl: true,
            shift: true,
            alt: false,
            cmd: false,
            keycode: Some(49),
        }
    }
}

/// Chế độ tương thích sửa chữ cho từng app (bảng quirk).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixMode {
    /// AX trước, rơi về key injection.
    Auto,
    /// Chỉ key injection, không delay.
    InjectFast,
    /// Key injection với delay vi mô giữa các event.
    InjectSlow,
    /// Chỉ AX, không fallback (app đặc biệt).
    AxOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "telex" | "vni".
    pub method: String,
    pub enabled: bool,
    /// "strict" | "standard" | "loose".
    pub spell_mode: String,
    pub modern_tone: bool,
    pub macros_enabled: bool,
    /// Gõ dấu mũ muộn: `nanag` → `nâng`.
    pub flexible_marks: bool,
    /// Che từ tục tĩu bằng dấu *.
    pub censor_enabled: bool,
    /// Viết hoa chữ đầu câu.
    pub auto_capitalize: bool,
    /// Esc hoàn từ đang gõ về phím gốc.
    pub esc_revert: bool,
    pub hotkey: Hotkey,
    pub macros: Vec<MacroEntry>,
    /// Bundle ID các app tự tắt tiếng Việt.
    pub excluded_apps: Vec<String>,
    /// Override chế độ tương thích theo bundle ID.
    pub per_app_mode: HashMap<String, FixMode>,
    /// Nhớ trạng thái VN/EN riêng cho từng app.
    pub remember_per_app: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            method: "telex".into(),
            enabled: true,
            spell_mode: "strict".into(),
            modern_tone: false,
            macros_enabled: true,
            flexible_marks: true,
            censor_enabled: false,
            auto_capitalize: false,
            esc_revert: false,
            hotkey: Hotkey::default(),
            macros: Vec::new(),
            excluded_apps: Vec::new(),
            per_app_mode: HashMap::new(),
            remember_per_app: false,
        }
    }
}

impl Settings {
    pub fn engine_config(&self) -> crate::engine::EngineConfig {
        crate::engine::EngineConfig {
            method: if self.method == "vni" {
                crate::engine::TypingMethod::Vni
            } else {
                crate::engine::TypingMethod::Telex
            },
            spell_mode: match self.spell_mode.as_str() {
                "loose" => crate::engine::SpellMode::Loose,
                "standard" => crate::engine::SpellMode::Standard,
                _ => crate::engine::SpellMode::Strict,
            },
            modern_tone: self.modern_tone,
            macros_enabled: self.macros_enabled,
            flexible_marks: self.flexible_marks,
            censor_enabled: self.censor_enabled,
            auto_capitalize: self.auto_capitalize,
            esc_revert: self.esc_revert,
        }
    }

    pub fn macro_table(&self) -> crate::engine::macros::MacroTable {
        crate::engine::macros::MacroTable::new(
            self.macros.iter().map(|m| (m.from.clone(), m.to.clone())),
        )
    }
}

pub fn config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Application Support/OreoKey")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// Đọc settings. File chưa có → mặc định. File lỗi → giữ các field còn
/// hợp lệ, sao lưu bản gốc sang `settings.json.bad` (để lần save kế tiếp
/// không xóa mất macro của người dùng) rồi dùng phần đọc được.
pub fn load() -> Settings {
    load_from(&settings_path())
}

fn load_from(path: &Path) -> Settings {
    // Only a missing file means "first run". Anything else unreadable must
    // be backed up first: the next save would replace it with defaults.
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Settings::default(),
        Err(_) => {
            back_up(path);
            return Settings::default();
        }
    };
    // Lossy keeps the valid fields of a file with a stray non-UTF-8 byte.
    let text = String::from_utf8_lossy(&bytes);
    let utf8 = matches!(text, std::borrow::Cow::Borrowed(_));
    // Editors may prepend a BOM, which serde_json rejects.
    let (settings, clean) = parse_lenient(text.strip_prefix('\u{FEFF}').unwrap_or(&text));
    if !clean || !utf8 {
        back_up(path);
    }
    settings
}

/// Keep the original as `settings.json.bad`; move it if it cannot be read.
fn back_up(path: &Path) {
    let bad = path.with_extension("json.bad");
    if fs::copy(path, &bad).is_err() {
        let _ = fs::rename(path, &bad);
    }
}

/// Trả về (settings, nguyên vẹn?). Parse chặt trước; nếu hỏng thì thử từng
/// field riêng lẻ để một field sai (vd. FixMode lạ) không làm mất cả file.
fn parse_lenient(text: &str) -> (Settings, bool) {
    let migrated = migrate_json(text);
    if let Ok(s) = serde_json::from_str(&migrated) {
        return (s, true);
    }
    let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(&migrated) else {
        return (Settings::default(), false);
    };
    let good: serde_json::Map<_, _> = obj
        .into_iter()
        .filter(|(k, v)| {
            let one = serde_json::Map::from_iter([(k.clone(), v.clone())]);
            serde_json::from_value::<Settings>(one.into()).is_ok()
        })
        .collect();
    (
        serde_json::from_value(good.into()).unwrap_or_default(),
        false,
    )
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

/// Ghi atomic: ghi file tạm rồi rename để không bao giờ để lại file dở.
pub fn save(settings: &Settings) -> io::Result<()> {
    let _guard = SAVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("settings.json.tmp");
    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    // fsync before rename so a crash cannot leave an empty settings.json.
    let mut f = fs::File::create(&tmp)?;
    io::Write::write_all(&mut f, json.as_bytes())?;
    // sync_all() is F_FULLFSYNC on macOS (milliseconds to tens of ms), and
    // hotkey toggles save from the event-tap callback. Plain fsync orders
    // the data before the rename, which is what this needs.
    #[cfg(target_os = "macos")]
    if unsafe { libc::fsync(std::os::fd::AsRawFd::as_raw_fd(&f)) } != 0 {
        return Err(io::Error::last_os_error());
    }
    #[cfg(not(target_os = "macos"))]
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, settings_path())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let mut s = Settings::default();
        s.method = "vni".into();
        s.macros.push(MacroEntry {
            from: "vn".into(),
            to: "Việt Nam".into(),
        });
        s.per_app_mode
            .insert("com.microsoft.Excel".into(), FixMode::InjectSlow);
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn unknown_fields_and_missing_fields_tolerated() {
        let s: Settings = serde_json::from_str(r#"{"method":"vni","future_field":1}"#).unwrap();
        assert_eq!(s.method, "vni");
        assert!(s.enabled); // field thiếu lấy default
    }

    #[test]
    fn corrupt_file_falls_back_to_default() {
        let s: Settings = serde_json::from_str("not json").unwrap_or_default();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn one_bad_field_keeps_the_rest() {
        let (s, clean) = parse_lenient(
            r#"{"method":"vni","macros":[{"from":"a","to":"b"}],"per_app_mode":{"x":"bogus"}}"#,
        );
        assert!(!clean);
        assert_eq!(s.method, "vni");
        assert_eq!(s.macros.len(), 1);
        assert!(s.per_app_mode.is_empty());
        assert!(parse_lenient(r#"{"method":"vni"}"#).1);
        assert!(!parse_lenient("not json").1);
    }

    #[test]
    fn load_handles_bom_encoding_and_missing_file() {
        // Never the real config dir: a scratch dir unique to this run.
        let dir = std::env::temp_dir().join(format!("oreokey-cfg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        let bad = dir.join("settings.json.bad");

        // Missing → defaults, nothing backed up.
        assert_eq!(load_from(&path), Settings::default());
        assert!(!bad.exists());

        // UTF-8 BOM is stripped; the file is otherwise clean.
        fs::write(&path, "\u{FEFF}{\"method\":\"vni\"}").unwrap();
        assert_eq!(load_from(&path).method, "vni");
        assert!(!bad.exists());

        // UTF-16 → defaults, original kept for the user.
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain(
                "{\"method\":\"vni\"}"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes),
            )
            .collect();
        fs::write(&path, &utf16).unwrap();
        assert_eq!(load_from(&path), Settings::default());
        assert_eq!(fs::read(&bad).unwrap(), utf16);

        // Invalid byte inside a value: other fields survive, backed up.
        let _ = fs::remove_file(&bad);
        let mut stray = br#"{"method":"vni","macros":[{"from":"a","to":"x"#.to_vec();
        stray.extend_from_slice(&[0xE1, b'"', b'}', b']', b'}']);
        fs::write(&path, &stray).unwrap();
        let s = load_from(&path);
        assert_eq!(s.method, "vni");
        assert_eq!(s.macros.len(), 1);
        assert_eq!(fs::read(&bad).unwrap(), stray);

        // Truncated mid-character → defaults, backed up.
        let _ = fs::remove_file(&bad);
        let mut cut = "{\"method\":\"vni\",\"macros\":[{\"from\":\"vn\",\"to\":\"Vi"
            .as_bytes()
            .to_vec();
        cut.push(0xE1);
        fs::write(&path, &cut).unwrap();
        assert_eq!(load_from(&path), Settings::default());
        assert_eq!(fs::read(&bad).unwrap(), cut);

        // Unreadable (a directory) → defaults, moved aside.
        let _ = fs::remove_file(&bad);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(load_from(&path), Settings::default());
        assert!(bad.is_dir() && !path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_legacy_spell_check() {
        // File cũ chỉ có spell_check → map sang spell_mode.
        let on: Settings = serde_json::from_str(&migrate_json(r#"{"spell_check":true}"#)).unwrap();
        assert_eq!(on.spell_mode, "strict");
        let off: Settings =
            serde_json::from_str(&migrate_json(r#"{"spell_check":false}"#)).unwrap();
        assert_eq!(off.spell_mode, "standard");
    }

    #[test]
    fn keeps_explicit_spell_mode() {
        let s: Settings = serde_json::from_str(&migrate_json(r#"{"spell_mode":"loose"}"#)).unwrap();
        assert_eq!(s.spell_mode, "loose");
    }

    #[test]
    fn migrate_passes_through_non_object() {
        // Không phải JSON hợp lệ → trả nguyên văn (load sẽ về default).
        assert_eq!(migrate_json("not json"), "not json");
    }
}
