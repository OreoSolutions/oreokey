//! Cấu hình ứng dụng. Rust là chủ sở hữu duy nhất: Swift đọc/ghi qua
//! FFI bằng chuỗi JSON, file lưu tại `~/Library/Application Support/OreoKey/`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

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
    pub spell_check: bool,
    pub modern_tone: bool,
    pub macros_enabled: bool,
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
            spell_check: true,
            modern_tone: false,
            macros_enabled: true,
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
            spell_check: self.spell_check,
            modern_tone: self.modern_tone,
            macros_enabled: self.macros_enabled,
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

/// Đọc settings; file hỏng hoặc chưa có → mặc định (không phá app vì
/// một file lỗi).
pub fn load() -> Settings {
    match fs::read_to_string(settings_path()) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Ghi atomic: ghi file tạm rồi rename để không bao giờ để lại file dở.
pub fn save(settings: &Settings) -> io::Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("settings.json.tmp");
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(&tmp, json)?;
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
}
