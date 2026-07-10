//! Bảng quirk theo app: dữ liệu, không phải code. Hồ sơ mặc định đóng
//! gói kèm binary (data/app-profiles.json), người dùng override từng
//! app trong Cài đặt (settings.per_app_mode).

use std::collections::HashMap;

use serde::Deserialize;

use crate::config::FixMode;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppProfile {
    #[serde(default)]
    pub mode: Option<FixMode>,
    #[serde(default)]
    pub browser_fix: bool,
    /// Luôn gửi phím hủy autocomplete trước backspace, bất kể AX nói gì
    /// — cho app có ghost text mà AXSelectedTextRange báo 0 (Spotlight).
    #[serde(default)]
    pub force_clear: bool,
    /// Sửa chữ bằng Shift+← chọn ngược rồi gõ đè, thay vì backspace —
    /// cho app điền lại ghost text sau MỌI event khiến backspace luôn
    /// bị nuốt (Spotlight). Shift+← tự hủy ghost text.
    #[serde(default)]
    pub select_replace: bool,
    #[serde(default)]
    pub delay_ms: u64,
}

#[derive(Debug, Deserialize, Default)]
struct ProfileFile {
    apps: HashMap<String, AppProfile>,
}

#[derive(Debug, Default)]
pub struct Profiles {
    apps: HashMap<String, AppProfile>,
}

/// Hồ sơ đã hợp nhất cho app đang focus.
#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub mode: FixMode,
    pub browser_fix: bool,
    pub force_clear: bool,
    pub select_replace: bool,
    pub delay_ms: u64,
}

impl Default for ResolvedProfile {
    fn default() -> Self {
        ResolvedProfile {
            mode: FixMode::Auto,
            browser_fix: false,
            force_clear: false,
            select_replace: false,
            delay_ms: 3,
        }
    }
}

impl Profiles {
    pub fn load_default() -> Profiles {
        let file: ProfileFile = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/app-profiles.json"
        )))
        .unwrap_or_default();
        Profiles { apps: file.apps }
    }

    fn lookup(&self, bundle: &str) -> Option<&AppProfile> {
        if let Some(p) = self.apps.get(bundle) {
            return Some(p);
        }
        // Wildcard: key dạng "com.jetbrains.*" khớp theo tiền tố.
        self.apps.iter().find_map(|(k, v)| {
            k.strip_suffix('*')
                .filter(|prefix| bundle.starts_with(prefix))
                .map(|_| v)
        })
    }

    /// Override của người dùng thắng hồ sơ mặc định. `focused_proc` là
    /// tên process sở hữu ô focus — panel nổi (Spotlight...) không đổi
    /// app frontmost nên tra theo key `proc:<tên>` với ưu tiên cao nhất.
    pub fn resolve(
        &self,
        bundle: &str,
        user_modes: &HashMap<String, FixMode>,
        focused_proc: Option<&str>,
    ) -> ResolvedProfile {
        if let Some(name) = focused_proc {
            let key = format!("proc:{name}");
            if let Some(p) = self.apps.get(&key) {
                return Self::merge(p);
            }
        }
        let mut resolved = ResolvedProfile::default();
        if let Some(p) = self.lookup(bundle) {
            resolved = Self::merge(p);
        }
        if let Some(&m) = user_modes.get(bundle) {
            resolved.mode = m;
        }
        resolved
    }

    fn merge(p: &AppProfile) -> ResolvedProfile {
        let mut resolved = ResolvedProfile::default();
        if let Some(m) = p.mode {
            resolved.mode = m;
        }
        resolved.browser_fix = p.browser_fix;
        resolved.force_clear = p.force_clear;
        resolved.select_replace = p.select_replace;
        if p.delay_ms > 0 {
            resolved.delay_ms = p.delay_ms;
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profiles_parse_and_resolve() {
        let p = Profiles::load_default();
        let none = HashMap::new();

        let chrome = p.resolve("com.google.Chrome", &none, None);
        assert!(chrome.browser_fix);

        let excel = p.resolve("com.microsoft.Excel", &none, None);
        assert_eq!(excel.mode, FixMode::InjectSlow);
        assert!(excel.delay_ms > 0);

        // Wildcard JetBrains
        let idea = p.resolve("com.jetbrains.intellij", &none, None);
        assert_eq!(idea.mode, FixMode::InjectFast);

        // App lạ → Auto
        let unknown = p.resolve("com.example.unknown", &none, None);
        assert_eq!(unknown.mode, FixMode::Auto);

        // Telegram có HAI bản trên macOS: bản Swift trên App Store
        // (ru.keepcoder.Telegram) và Telegram Desktop Qt từ telegram.org
        // (com.tdesktop.Telegram). Bản Qt nhận lệnh chọn vùng AX (thấy
        // bôi đen) nhưng lờ lệnh ghi AXSelectedText mà vẫn trả success
        // → kẹt vùng chọn, không gõ được (issue #2). Cả hai phải đi
        // đường bơm phím.
        let tg_native = p.resolve("ru.keepcoder.Telegram", &none, None);
        assert_eq!(tg_native.mode, FixMode::InjectFast);
        let tg_desktop = p.resolve("com.tdesktop.Telegram", &none, None);
        assert_eq!(tg_desktop.mode, FixMode::InjectFast);
    }

    #[test]
    fn focused_proc_overrides_frontmost_bundle() {
        // Spotlight nhận phím nhưng frontmost vẫn là app phía sau —
        // profile phải theo chủ ô focus.
        let p = Profiles::load_default();
        let none = HashMap::new();
        let r = p.resolve("com.mitchellh.ghostty", &none, Some("Spotlight"));
        assert!(r.select_replace); // ghost text nuốt backspace → phải gõ đè
        assert_eq!(r.mode, FixMode::InjectFast);
        // Process lạ → rơi về bundle như cũ.
        let r = p.resolve("com.mitchellh.ghostty", &none, Some("ghostty"));
        assert!(!r.browser_fix);
    }

    #[test]
    fn user_override_wins() {
        let p = Profiles::load_default();
        let mut user = HashMap::new();
        user.insert("com.google.Chrome".to_string(), FixMode::InjectSlow);
        assert_eq!(
            p.resolve("com.google.Chrome", &user, None).mode,
            FixMode::InjectSlow
        );
    }
}
