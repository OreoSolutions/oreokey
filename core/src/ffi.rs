//! Bề mặt C ABI cho lớp vỏ Swift. Mọi chuỗi trả về phải giải phóng
//! bằng `ok_str_free`.

use std::ffi::{c_char, CStr, CString};

use crate::config;
use crate::engine::encoding::{self, Encoding};
use crate::platform::{ax, event_tap, with_runtime, StatusCallback};

fn to_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

unsafe fn from_c_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

/// Khởi động event tap. false = chưa có quyền Accessibility.
#[no_mangle]
pub extern "C" fn ok_start() -> bool {
    // Chạm runtime trước để nạp settings từ đĩa.
    with_runtime(|_| {});
    event_tap::start()
}

#[no_mangle]
pub extern "C" fn ok_stop() {
    event_tap::stop();
}

#[no_mangle]
pub extern "C" fn ok_is_running() -> bool {
    event_tap::is_running()
}

#[no_mangle]
pub extern "C" fn ok_ax_trusted() -> bool {
    ax::is_trusted()
}

/// Trạng thái tiếng Việt thực tế của app đang focus.
#[no_mangle]
pub extern "C" fn ok_get_enabled() -> bool {
    with_runtime(|rt| rt.effective_enabled())
}

#[no_mangle]
pub extern "C" fn ok_set_enabled(on: bool) {
    with_runtime(|rt| rt.set_enabled(on));
}

/// JSON toàn bộ settings (Swift render UI từ đây).
#[no_mangle]
pub extern "C" fn ok_settings_json_get() -> *mut c_char {
    let json = with_runtime(|rt| serde_json::to_string(&rt.settings).unwrap_or_default());
    to_c_string(json)
}

/// Nhận JSON settings từ Swift: áp vào runtime + ghi đĩa.
#[no_mangle]
pub unsafe extern "C" fn ok_settings_json_set(json: *const c_char) -> bool {
    let Some(json) = from_c_str(json) else {
        return false;
    };
    let Ok(settings) = serde_json::from_str::<config::Settings>(json) else {
        return false;
    };
    // Keep persistence and the in-memory update in the same runtime critical
    // section. Otherwise a simultaneous hotkey toggle can be overwritten by
    // this stale settings snapshot after it has already been persisted.
    with_runtime(|rt| {
        if config::save(&settings).is_err() {
            return false;
        }
        rt.apply_settings(settings);
        true
    })
}

/// Đăng ký callback cập nhật icon menu bar khi trạng thái VN/EN đổi.
#[no_mangle]
pub extern "C" fn ok_set_status_callback(cb: StatusCallback) {
    with_runtime(|rt| {
        rt.status_cb = Some(cb);
        rt.notify_status();
    });
}

/// Swift báo app đang focus đổi (NSWorkspace notification).
#[no_mangle]
pub unsafe extern "C" fn ok_notify_frontmost_app(bundle_id: *const c_char) {
    let bundle = from_c_str(bundle_id).unwrap_or("").to_string();
    with_runtime(|rt| rt.app_changed(bundle));
}

/// Chuyển mã văn bản. from/to: 0 = Unicode, 1 = VNI-Windows, 2 = TCVN3.
#[no_mangle]
pub unsafe extern "C" fn ok_convert(text: *const c_char, from: i32, to: i32) -> *mut c_char {
    let Some(text) = from_c_str(text) else {
        return std::ptr::null_mut();
    };
    let enc = |v: i32| match v {
        1 => Encoding::VniWindows,
        2 => Encoding::Tcvn3,
        _ => Encoding::Unicode,
    };
    to_c_string(encoding::convert(text, enc(from), enc(to)))
}

#[no_mangle]
pub unsafe extern "C" fn ok_str_free(p: *mut c_char) {
    if !p.is_null() {
        drop(CString::from_raw(p));
    }
}
