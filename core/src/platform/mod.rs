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
pub type StatusCallback = extern "C" fn(bool);

pub struct Runtime {
    pub settings: Settings,
    pub engine: Engine,
    /// Công tắc tiếng Việt toàn cục.
    pub enabled: bool,
    /// Override trạng thái theo bundle ID (hotkey bấm trong app cụ thể
    /// khi bật "nhớ theo app", hoặc bật tạm trong app bị loại trừ).
    pub per_app_enabled: HashMap<String, bool>,
    pub current_bundle: String,
    pub profiles: profiles::Profiles,
    /// Cache khả năng sửa chữ qua AX API theo bundle ID.
    pub ax_ok: HashMap<String, bool>,
    pub status_cb: Option<StatusCallback>,
    /// Lọc phím bóng ma (bản sao keydown do WindowServer giao lại khi
    /// callback chậm vì Replace). Xem [`ghost`].
    pub ghost: ghost::GhostGuard,
}

pub static RUNTIME: Mutex<Option<Runtime>> = Mutex::new(None);

impl Runtime {
    pub fn new() -> Runtime {
        let settings = config::load();
        let engine = Engine::new(settings.engine_config());
        let mut rt = Runtime {
            enabled: settings.enabled,
            engine,
            settings,
            per_app_enabled: HashMap::new(),
            current_bundle: String::new(),
            profiles: profiles::Profiles::load_default(),
            ax_ok: HashMap::new(),
            status_cb: None,
            ghost: ghost::GhostGuard::new(ghost::window_ticks()),
        };
        rt.engine.set_macros(rt.settings.macro_table());
        rt
    }

    pub fn apply_settings(&mut self, settings: Settings) {
        self.enabled = settings.enabled;
        self.engine.set_config(settings.engine_config());
        self.engine.set_macros(settings.macro_table());
        self.settings = settings;
        self.notify_status();
    }

    /// Trạng thái tiếng Việt thực tế cho app đang focus.
    pub fn effective_enabled(&self) -> bool {
        if let Some(&on) = self.per_app_enabled.get(&self.current_bundle) {
            return on;
        }
        self.enabled && !self.settings.excluded_apps.contains(&self.current_bundle)
    }

    /// Người dùng chủ động bật/tắt (menu bar): đặt trạng thái toàn cục,
    /// xóa override của app hiện tại để không gây bất ngờ.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        self.per_app_enabled.remove(&self.current_bundle);
        self.engine.reset();
        self.persist_enabled();
        self.notify_status();
    }

    /// Hotkey: đảo trạng thái của ngữ cảnh hiện tại.
    pub fn toggle(&mut self) {
        let now = !self.effective_enabled();
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
        let _ = config::save(&self.settings);
    }

    pub fn notify_status(&self) {
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

/// Log chẩn đoán, bật bằng OREOKEY_DEBUG=1, ghi vào /tmp/oreokey-debug.log.
pub fn dlog(msg: &str) {
    use std::io::Write;
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if !*ENABLED.get_or_init(|| std::env::var("OREOKEY_DEBUG").is_ok()) {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/oreokey-debug.log")
    {
        let _ = writeln!(f, "{msg}");
    }
}
