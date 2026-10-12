//! Tầng platform macOS: trạng thái runtime dùng chung giữa event tap,
//! FFI và thông báo từ Swift.

pub mod ax;
pub mod event_tap;
pub mod ghost;
pub mod inject;
pub mod profiles;

use std::collections::HashMap;
use std::sync::Mutex;

use crate::config::{self, Settings};
use crate::engine::Engine;

/// Callback Swift đăng ký để cập nhật icon menu bar khi trạng thái
/// VN/EN đổi (do hotkey hoặc đổi app).
pub type StatusCallback = Option<extern "C" fn(bool)>;

pub struct Runtime {
    pub settings: Settings,
    pub engine: Engine,
    /// Công tắc tiếng Việt toàn cục.
    pub enabled: bool,
    /// Override trạng thái theo bundle ID (hotkey bấm trong app cụ thể
    /// khi bật "nhớ theo app", hoặc bật tạm trong app bị loại trừ).
    pub per_app_enabled: HashMap<String, bool>,
    /// Treo tạm vì hệ thống đang ở input method khác (Nhật/Hàn/Trung…):
    /// tiếng Việt coi như tắt nhưng trạng thái người dùng không đổi.
    pub suspended: bool,
    pub current_bundle: String,
    pub profiles: profiles::Profiles,
    /// Bundle ID → lúc AX bị coi là không dùng được. Hết hạn sau
    /// [`inject::AX_RETRY`] để một lỗi AX thoáng qua không tắt AX cả phiên.
    pub ax_failed: HashMap<String, std::time::Instant>,
    /// Bundle ID → when an AX read timed out (app busy or hung). While
    /// fresh the tap makes no AX calls at all for it, not even reads.
    pub ax_hung: HashMap<String, std::time::Instant>,
    pub status_cb: StatusCallback,
    /// Lọc phím bóng ma (bản sao keydown do WindowServer giao lại khi
    /// callback chậm vì Replace). Xem [`ghost`].
    pub ghost: ghost::GhostGuard,
}

pub static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Runtime {
        let settings = config::load();
        let engine = Engine::new(settings.engine_config());
        let mut rt = Runtime {
            enabled: settings.enabled,
            engine,
            settings,
            per_app_enabled: HashMap::new(),
            suspended: false,
            current_bundle: String::new(),
            profiles: profiles::Profiles::load_default(),
            ax_failed: HashMap::new(),
            ax_hung: HashMap::new(),
            status_cb: None,
            ghost: ghost::GhostGuard::new(ghost::window_ticks()),
        };
        rt.engine.set_macros(rt.settings.macro_table());
        rt
    }

    /// Apply a settings snapshot from Swift. `enabled` is runtime-owned
    /// (hotkey, menu and per-app toggles change it behind the snapshot's
    /// back), so a possibly stale snapshot must not flip it: the live value
    /// wins. `method` is applied as sent — the menu and the Settings picker
    /// both change it only through this setter.
    pub fn apply_settings(&mut self, mut settings: Settings) {
        settings.enabled = self.enabled;
        self.engine.set_config(settings.engine_config());
        self.engine.set_macros(settings.macro_table());
        self.settings = settings;
        self.notify_status();
    }

    /// Trạng thái tiếng Việt thực tế cho app đang focus.
    pub fn effective_enabled(&self) -> bool {
        if self.suspended {
            return false;
        }
        if let Some(&on) = self.per_app_enabled.get(&self.current_bundle) {
            return on;
        }
        self.enabled && !self.settings.excluded_apps.contains(&self.current_bundle)
    }

    /// Người dùng chủ động bật/tắt (menu bar): đặt trạng thái toàn cục,
    /// xóa override của app hiện tại để không gây bất ngờ.
    /// In an excluded app the global switch cannot change what the user
    /// sees, so (like the hotkey) only this app is switched.
    pub fn set_enabled(&mut self, on: bool) {
        self.suspended = false;
        if self.settings.excluded_apps.contains(&self.current_bundle) {
            self.per_app_enabled.insert(self.current_bundle.clone(), on);
        } else {
            self.enabled = on;
            self.per_app_enabled.remove(&self.current_bundle);
            self.persist_enabled();
        }
        self.engine.reset();
        self.notify_status();
    }

    /// Hotkey: đảo trạng thái của ngữ cảnh hiện tại.
    pub fn toggle(&mut self) {
        let now = !self.effective_enabled();
        self.suspended = false;
        if self.settings.remember_per_app
            || self.settings.excluded_apps.contains(&self.current_bundle)
        {
            self.per_app_enabled
                .insert(self.current_bundle.clone(), now);
        } else {
            self.enabled = now;
            self.persist_enabled();
        }
        self.engine.reset();
        self.notify_status();
    }

    pub fn app_changed(&mut self, bundle: String) {
        if bundle != self.current_bundle {
            self.current_bundle = bundle;
            self.engine.reset();
            self.notify_status();
        }
    }

    fn persist_enabled(&mut self) {
        self.settings.enabled = self.enabled;
        if config::save(&self.settings).is_err() {
            dlog("settings persistence failed");
        }
    }

    /// Input source hệ thống đổi sang/khỏi input method khác. Không ghi
    /// settings, không đụng bộ nhớ theo app: về ABC là trạng thái cũ hiện lại.
    pub fn set_suspended(&mut self, on: bool) {
        if self.suspended == on {
            return;
        }
        self.suspended = on;
        self.engine.reset();
        self.notify_status();
    }

    pub fn notify_status(&self) {
        // Invoked with the runtime lock held: the callback must not call
        // back into OreoKey synchronously.
        if let Some(cb) = self.status_cb {
            cb(self.effective_enabled());
        }
    }
}

/// Chạy closure với runtime (khởi tạo lười ở lần chạm đầu tiên).
pub fn with_runtime<R>(f: impl FnOnce(&mut Runtime) -> R) -> R {
    let mut guard = RUNTIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = guard.get_or_insert_with(Runtime::new);
    f(rt)
}

/// Whether diagnostic logging is enabled. Callers on the event-tap hot path
/// must check this before formatting a message or extracting event text.
pub fn debug_enabled() -> bool {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("OREOKEY_DEBUG").is_ok())
}

/// Log chẩn đoán, bật bằng OREOKEY_DEBUG=1. Log ghi keycode, cờ autorepeat
/// và timestamp của từng phím nhưng KHÔNG ghi ký tự/văn bản gõ; chỉ bật
/// khi chẩn đoán và không bao giờ khi Secure Input đang bật.
pub fn dlog(msg: &str) {
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    if !debug_enabled() {
        return;
    }
    static FILE: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    let file = FILE.get_or_init(|| {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let path = config::config_dir().join("debug.log");
        let _ = std::fs::create_dir_all(config::config_dir());
        let truncate = std::fs::metadata(&path)
            .map(|m| m.len() > 1_048_576)
            .unwrap_or(false);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(!truncate)
            .truncate(truncate)
            .mode(0o600)
            .open(&path)
            .ok()?;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        Some(Mutex::new(file))
    });
    let Some(file) = file else {
        return;
    };
    if let Ok(mut f) = file.lock() {
        let _ = writeln!(f, "{msg}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_toggle_in_excluded_app_leaves_global_switch_alone() {
        // A regression would persist settings: keep that away from the
        // real config dir. No other test reads HOME.
        std::env::set_var(
            "HOME",
            std::env::temp_dir().join(format!("oreokey-rt-{}", std::process::id())),
        );
        let settings = Settings {
            enabled: false,
            excluded_apps: vec!["excluded".into()],
            ..Settings::default()
        };
        let mut rt = Runtime {
            engine: Engine::new(settings.engine_config()),
            enabled: false,
            settings,
            per_app_enabled: HashMap::new(),
            suspended: false,
            current_bundle: "excluded".into(),
            profiles: profiles::Profiles::default(),
            ax_failed: HashMap::new(),
            ax_hung: HashMap::new(),
            status_cb: None,
            ghost: ghost::GhostGuard::new(1),
        };
        rt.set_enabled(true);
        assert!(rt.effective_enabled(), "on in this app");
        assert!(!rt.enabled, "global switch untouched");
        rt.app_changed("other".into());
        assert!(!rt.effective_enabled(), "still off elsewhere");
    }

    #[test]
    fn suspension_hides_vietnamese_without_touching_user_state() {
        std::env::set_var(
            "HOME",
            std::env::temp_dir().join(format!("oreokey-rt-susp-{}", std::process::id())),
        );
        let settings = Settings::default();
        let mut rt = Runtime {
            engine: Engine::new(settings.engine_config()),
            enabled: true,
            settings,
            per_app_enabled: HashMap::new(),
            suspended: false,
            current_bundle: "app".into(),
            profiles: profiles::Profiles::default(),
            ax_failed: HashMap::new(),
            ax_hung: HashMap::new(),
            status_cb: None,
            ghost: ghost::GhostGuard::new(1),
        };
        rt.set_suspended(true);
        assert!(!rt.effective_enabled(), "off while another IME is selected");
        assert!(rt.enabled, "user's switch untouched");
        rt.set_suspended(false);
        assert!(rt.effective_enabled(), "back on when returning to a Latin layout");
        // Hotkey lúc đang treo = "tôi muốn gõ tiếng Việt": gỡ treo và bật
        // (Swift sẽ ép input source về ABC như mọi lần bật).
        rt.set_suspended(true);
        rt.toggle();
        assert!(!rt.suspended);
        assert!(rt.effective_enabled());
    }

}
