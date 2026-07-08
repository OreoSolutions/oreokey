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
    pub delay_ms: u64,
}

impl Default for ResolvedProfile {
    fn default() -> Self {
        ResolvedProfile {
            mode: FixMode::Auto,
            browser_fix: false,
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

    /// Override của người dùng thắng hồ sơ mặc định.
    pub fn resolve(
        &self,
        bundle: &str,
        user_modes: &HashMap<String, FixMode>,
    ) -> ResolvedProfile {
        let mut resolved = ResolvedProfile::default();
        if let Some(p) = self.lookup(bundle) {
            if let Some(m) = p.mode {
                resolved.mode = m;
            }
            resolved.browser_fix = p.browser_fix;
            if p.delay_ms > 0 {
                resolved.delay_ms = p.delay_ms;
            }
        }
        if let Some(&m) = user_modes.get(bundle) {
            resolved.mode = m;
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

        let chrome = p.resolve("com.google.Chrome", &none);
        assert!(chrome.browser_fix);

        let excel = p.resolve("com.microsoft.Excel", &none);
        assert_eq!(excel.mode, FixMode::InjectSlow);
        assert!(excel.delay_ms > 0);

        // Wildcard JetBrains
        let idea = p.resolve("com.jetbrains.intellij", &none);
        assert_eq!(idea.mode, FixMode::InjectFast);

        // App lạ → Auto
        let unknown = p.resolve("com.example.unknown", &none);
        assert_eq!(unknown.mode, FixMode::Auto);
    }

    #[test]
    fn user_override_wins() {
        let p = Profiles::load_default();
        let mut user = HashMap::new();
        user.insert("com.google.Chrome".to_string(), FixMode::InjectSlow);
        assert_eq!(
            p.resolve("com.google.Chrome", &user).mode,
            FixMode::InjectSlow
        );
    }
}
