//! CGEventTap: chặn phím toàn hệ thống, đưa qua engine, thực hiện sửa
//! chữ. Chạy trên thread riêng có CFRunLoop; tự phục hồi khi macOS tắt
//! tap vì timeout.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;

use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType, EventField,
};

use crate::config::Hotkey;
use crate::engine::{Action, KeyInput};

use super::{inject, with_runtime};

extern "C" {
    fn CGEventGetTimestamp(event: *const c_void) -> u64;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CFRunLoopStop(rl: *mut c_void);
    fn CGEventKeyboardGetUnicodeString(
        event: *mut c_void,
        max_len: libc::c_ulong,
        actual_len: *mut libc::c_ulong,
        buf: *mut u16,
    );
    fn IsSecureEventInputEnabled() -> u8;
}

static TAP_PORT: AtomicUsize = AtomicUsize::new(0);
static RUN_LOOP: AtomicUsize = AtomicUsize::new(0);
static RUNNING: AtomicBool = AtomicBool::new(false);

/// Khởi động event tap trên thread riêng. Trả về false nếu không tạo
/// được tap (thường do chưa cấp quyền Accessibility).
pub fn start() -> bool {
    if RUNNING.load(Ordering::SeqCst) {
        return true;
    }
    let (tx, rx) = mpsc::channel::<bool>();
    std::thread::Builder::new()
        .name("oreokey-tap".into())
        .spawn(move || {
            let tap = CGEventTap::new(
                CGEventTapLocation::HID,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                vec![
                    CGEventType::KeyDown,
                    CGEventType::LeftMouseDown,
                    CGEventType::RightMouseDown,
                    CGEventType::OtherMouseDown,
                ],
                callback,
            );
            let tap = match tap {
                Ok(t) => t,
                Err(()) => {
                    let _ = tx.send(false);
                    return;
                }
            };
            let source = match tap.mach_port().create_runloop_source(0) {
                Ok(s) => s,
                Err(()) => {
                    let _ = tx.send(false);
                    return;
                }
            };
            use core_foundation::base::TCFType;
            TAP_PORT.store(
                tap.mach_port().as_concrete_TypeRef() as usize,
                Ordering::SeqCst,
            );
            let rl = CFRunLoop::get_current();
            RUN_LOOP.store(rl.as_concrete_TypeRef() as usize, Ordering::SeqCst);
            unsafe { rl.add_source(&source, kCFRunLoopCommonModes) };
            tap.enable();
            RUNNING.store(true, Ordering::SeqCst);
            let _ = tx.send(true);
            CFRunLoop::run_current();
            // Run loop dừng (ok_stop) → dọn dẹp.
            RUNNING.store(false, Ordering::SeqCst);
            TAP_PORT.store(0, Ordering::SeqCst);
            RUN_LOOP.store(0, Ordering::SeqCst);
            drop(tap);
        })
        .expect("spawn tap thread");
    rx.recv().unwrap_or(false)
}

pub fn stop() {
    let rl = RUN_LOOP.load(Ordering::SeqCst);
    if rl != 0 {
        unsafe { CFRunLoopStop(rl as *mut c_void) };
    }
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

fn callback(proxy: CGEventTapProxy, etype: CGEventType, event: &CGEvent) -> CallbackKeep {
    match etype {
        CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
            // macOS tắt tap khi callback chậm → bật lại ngay. Event đang
            // xử lý dở bị hệ thống giao lại — lưới last_dropped bên dưới
            // chặn bản sao đó.
            super::dlog(&format!("TAP DISABLED ({etype:?}) — re-enabling"));
            let port = TAP_PORT.load(Ordering::SeqCst);
            if port != 0 {
                unsafe { CGEventTapEnable(port as *mut c_void, true) };
            }
            CallbackKeep::Keep
        }
        CGEventType::LeftMouseDown
        | CGEventType::RightMouseDown
        | CGEventType::OtherMouseDown => {
            // Click chuột: con trỏ có thể đã dời — bỏ theo dõi từ hiện tại.
            with_runtime(|rt| rt.engine.reset());
            CallbackKeep::Keep
        }
        CGEventType::KeyDown => handle_key(proxy, event),
        _ => CallbackKeep::Keep,
    }
}

// Alias để thân callback đọc gọn hơn.
use core_graphics::event::CallbackResult as CallbackKeep;

fn handle_key(proxy: CGEventTapProxy, event: &CGEvent) -> CallbackKeep {
    let user_data = event.get_integer_value_field(EventField::EVENT_SOURCE_USER_DATA);
    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;
    let autorepeat =
        event.get_integer_value_field(EventField::KEYBOARD_EVENT_AUTOREPEAT) != 0;
    let ev_ts = {
        use foreign_types::ForeignType;
        unsafe { CGEventGetTimestamp(event.as_ptr() as *const c_void) }
    };
    let src_pid = event.get_integer_value_field(EventField::EVENT_SOURCE_UNIX_PROCESS_ID);
    super::dlog(&format!(
        "keydown code={keycode} rep={autorepeat} magic={} ts={ev_ts} srcpid={src_pid} char={:?}",
        user_data == inject::MAGIC,
        event_char(event)
    ));
    // Bỏ qua event do chính mình bơm ra.
    if user_data == inject::MAGIC {
        return CallbackKeep::Keep;
    }

    let flags = event.get_flags();

    with_runtime(|rt| {
        // Bản sao do hệ thống giao lại (srcpid=0): cùng keycode với một
        // phím ĐÃ BỊ NUỐT gần đây và timestamp PHẦN CỨNG chỉ chênh vài
        // ms (bản sao đến muộn 150-400ms đồng hồ tường nhưng hw-ts giữ
        // nguyên gốc — chỉ hw-ts là bất biến tin được). Không ai bấm lại
        // cùng phím trong <30ms phần cứng; autorepeat có cờ riêng.
        // So với TẤT CẢ phím bị nuốt gần đây: bóng ma của `s` thứ nhất
        // trong chuỗi `ss` đến sau khi `s` thứ hai thật đã xử lý.
        let is_ghost = rt.recent_dropped.iter().any(|&(code, dropped_ts)| {
            code == keycode && ev_ts.abs_diff(dropped_ts) < 30_000_000
        });
        if is_ghost {
            super::dlog("  dup re-delivery suppressed");
            return CallbackKeep::Drop;
        }
        // Hotkey bật/tắt tiếng Việt.
        if matches_hotkey(&rt.settings.hotkey, keycode, flags) {
            rt.toggle();
            return CallbackKeep::Drop;
        }
        if !rt.effective_enabled() {
            return CallbackKeep::Keep;
        }
        // Ô mật khẩu (Secure Input): không đụng vào bất cứ thứ gì.
        if unsafe { IsSecureEventInputEnabled() } != 0 {
            rt.engine.reset();
            return CallbackKeep::Keep;
        }
        // Phím tắt hệ thống (⌘/⌃/⌥/fn): không xử lý, bỏ theo dõi từ.
        if flags.intersects(
            CGEventFlags::CGEventFlagCommand
                | CGEventFlags::CGEventFlagControl
                | CGEventFlags::CGEventFlagAlternate
                | CGEventFlags::CGEventFlagSecondaryFn,
        ) {
            rt.engine.reset();
            return CallbackKeep::Keep;
        }

        let input = match keycode {
            51 => {
                // Backspace: autorepeat vẫn phải đồng bộ buffer từng lần.
                KeyInput::Backspace
            }
            36 | 76 | 48 | 53 | 115 | 116 | 117 | 119 | 121 | 123 | 124 | 125 | 126 => {
                // Enter, Tab, Esc, Home/End/PgUp/PgDn, Forward-Delete, mũi tên.
                KeyInput::WordBreak(None)
            }
            _ => {
                if autorepeat {
                    // Giữ phím lặp ký tự: không phải gõ tiếng Việt.
                    rt.engine.reset();
                    return CallbackKeep::Keep;
                }
                match event_char(event) {
                    Some(c) if c.is_ascii_alphanumeric() => KeyInput::Char(c),
                    Some(c) if !c.is_control() => KeyInput::WordBreak(Some(c)),
                    _ => KeyInput::WordBreak(None),
                }
            }
        };

        match rt.engine.on_key(input) {
            Action::PassThrough => CallbackKeep::Keep,
            Action::Replace { old, text } => {
                // Một lượt tra cứu AX duy nhất: chủ sở hữu thật của ô
                // focus (Spotlight và panel nổi không đổi app frontmost)
                // + vùng chọn. Callback phải nhanh — chậm là WindowServer
                // giao lại event, phím nhân đôi.
                let finfo = super::ax::focused_info();
                let profile = rt.profiles.resolve(
                    &rt.current_bundle,
                    &rt.settings.per_app_mode,
                    finfo.proc_name.as_deref(),
                );
                let bundle = rt.current_bundle.clone();
                inject::apply(
                    proxy,
                    &old,
                    &text,
                    &profile,
                    &bundle,
                    &mut rt.ax_ok,
                    finfo.selection_len,
                );
                rt.recent_dropped.push_back((keycode, ev_ts));
                if rt.recent_dropped.len() > 16 {
                    rt.recent_dropped.pop_front();
                }
                CallbackKeep::Drop
            }
        }
    })
}

/// Ký tự unicode của event theo layout bàn phím hiện tại.
fn event_char(event: &CGEvent) -> Option<char> {
    use foreign_types::ForeignType;
    let mut buf = [0u16; 8];
    let mut len: libc::c_ulong = 0;
    unsafe {
        CGEventKeyboardGetUnicodeString(
            event.as_ptr() as *mut c_void,
            buf.len() as libc::c_ulong,
            &mut len,
            buf.as_mut_ptr(),
        );
    }
    if len == 0 {
        return None;
    }
    char::decode_utf16(buf[..len as usize].iter().copied())
        .next()
        .and_then(Result::ok)
}

fn matches_hotkey(hk: &Hotkey, keycode: u16, flags: CGEventFlags) -> bool {
    let Some(hk_code) = hk.keycode else {
        return false;
    };
    if keycode != hk_code {
        return false;
    }
    let want = |on: bool, flag: CGEventFlags| flags.contains(flag) == on;
    want(hk.ctrl, CGEventFlags::CGEventFlagControl)
        && want(hk.shift, CGEventFlags::CGEventFlagShift)
        && want(hk.alt, CGEventFlags::CGEventFlagAlternate)
        && want(hk.cmd, CGEventFlags::CGEventFlagCommand)
}
