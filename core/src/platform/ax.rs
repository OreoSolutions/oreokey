//! Sửa chữ trực tiếp qua Accessibility API: chọn N ký tự trước con trỏ
//! và ghi đè bằng chuỗi mới trong MỘT thao tác — không backspace, không
//! thể nháy hay dính chữ. Không phải app nào cũng hỗ trợ; mọi lỗi trả
//! Err để caller rơi về key injection — except [`AxFail::Unknown`], where a
//! write may still land and injecting could delete real text.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use core_foundation::base::{CFRange, CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};

type AXUIElementRef = *const c_void;
type AXError = i32;

const K_AX_VALUE_TYPE_CFRANGE: u32 = 4;
const AX_SUCCESS: AXError = 0;
/// kAXErrorCannotComplete: the call timed out (app busy/hung). The message
/// is still queued in the target app and may be applied later.
const AX_ERROR_CANNOT_COMPLETE: AXError = -25204;

extern "C" {
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        parameter: CFTypeRef,
        value: *mut CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> AXError;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f32) -> AXError;
    fn AXValueCreate(value_type: u32, value_ptr: *const c_void) -> CFTypeRef;
    fn AXValueGetValue(value: CFTypeRef, value_type: u32, out: *mut c_void) -> u8;
    fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> AXError;
    pub fn AXIsProcessTrusted() -> u8;
    // libproc (libSystem)
    fn proc_name(pid: i32, buffer: *mut c_void, buffersize: u32) -> i32;
}

/// Per-call AX timeout in seconds. The default is ~6s; a hung target app would
/// then block the event-tap callback (RUNTIME lock held) long enough for
/// macOS to disable the tap. 0.3s is far above a healthy AX round-trip.
const AX_TIMEOUT_SECS: f32 = 0.3;

/// Total time one tap callback may spend on AX calls before the first write.
/// Past it the remaining reads are skipped and the edit falls back to keys.
pub const AX_BUDGET: Duration = Duration::from_millis(600);

/// Timeout for the write calls, set on the element once it has answered
/// the reads. A write that times out may still land later (outcome
/// unknown), so waiting longer makes that rarer; healthy apps answer in ms.
const AX_WRITE_TIMEOUT_SECS: f32 = 1.0;

/// Timeout for the caret restore after a timed-out write: the restore is
/// queued behind the late write and lands after it, so waiting for the
/// reply would only stall the tap.
const AX_CLEANUP_TIMEOUT_SECS: f32 = 0.05;

/// Create the system-wide element with the messaging timeout applied (the
/// system-wide timeout is the global default for all AX calls in the process).
unsafe fn create_system_wide() -> AXUIElementRef {
    let sw = AXUIElementCreateSystemWide();
    if !sw.is_null() {
        AXUIElementSetMessagingTimeout(sw, AX_TIMEOUT_SECS);
    }
    sw
}

fn attr(name: &'static str) -> CFString {
    CFString::from_static_string(name)
}

/// Lý do AX không dùng được cho lần sửa này.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxFail {
    /// Văn bản trước caret khác kỳ vọng — thường do ký tự passthrough
    /// chưa kịp vào app (gõ nhanh). Tạm thời; đừng cache app là hỏng.
    Mismatch,
    /// App không expose text field qua AX / lỗi API. Cache được.
    Unsupported,
    /// A write was issued but did not finish cleanly (timed out, or the
    /// caret could not be put back): the app may still apply it, so
    /// injecting keys now could delete real text. Drop the edit instead.
    Unknown,
}

/// Thay đoạn `old` ngay trước con trỏ bằng `text` — CHỈ khi xác minh
/// được app thực sự đang hiển thị `old` tại đó. Không xác minh mù:
/// ký tự passthrough đến app bất đồng bộ, ghi đè mù sẽ nuốt nhầm chữ
/// (bug thực địa: gõ nhanh "đó" thành "óo").
/// Reads stop once `deadline` (see [`AX_BUDGET`]) has passed; nothing has
/// been written then, so falling back to keys is safe.
pub fn replace_tail(old: &str, text: &str, deadline: Instant) -> Result<(), AxFail> {
    // AX làm việc theo đơn vị UTF-16.
    let old_len = old.encode_utf16().count() as isize;
    let out_of_time = || Instant::now() >= deadline;

    unsafe {
        if out_of_time() {
            return Err(AxFail::Unsupported);
        }
        let system_wide = create_system_wide();
        if system_wide.is_null() {
            return Err(AxFail::Unsupported);
        }
        // Giữ ownership để CFRelease khi ra khỏi scope.
        let _system_wide_guard = CFType::wrap_under_create_rule(system_wide as CFTypeRef);

        let mut focused: CFTypeRef = std::ptr::null();
        if AXUIElementCopyAttributeValue(
            system_wide,
            attr("AXFocusedUIElement").as_concrete_TypeRef(),
            &mut focused,
        ) != AX_SUCCESS
            || focused.is_null()
        {
            return Err(AxFail::Unsupported);
        }
        let focused_guard = CFType::wrap_under_create_rule(focused);
        let element = focused_guard.as_CFTypeRef() as AXUIElementRef;

        // Ô focus thuộc CHÍNH OreoKey (cửa sổ Cài đặt): lệnh AX sẽ quay
        // ngược vào process mình trên thread tap → TSM assert main queue
        // → crash (bug thực địa: sập khi gõ tiếng Việt vào ô Gõ tắt).
        // Bơm phím vào chính mình thì an toàn.
        let mut pid: i32 = 0;
        if AXUIElementGetPid(element, &mut pid) == AX_SUCCESS && pid == libc::getpid() {
            return Err(AxFail::Unsupported);
        }

        if out_of_time() {
            return Err(AxFail::Unsupported);
        }
        // Vị trí con trỏ hiện tại; đòi hỏi không có vùng chọn (selection
        // nghĩa là trạng thái đã lệch so với buffer của engine).
        let mut range_value: CFTypeRef = std::ptr::null();
        if AXUIElementCopyAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            &mut range_value,
        ) != AX_SUCCESS
            || range_value.is_null()
        {
            return Err(AxFail::Unsupported);
        }
        let range_guard = CFType::wrap_under_create_rule(range_value);
        let mut caret = CFRange {
            location: 0,
            length: 0,
        };
        if AXValueGetValue(
            range_guard.as_CFTypeRef(),
            K_AX_VALUE_TYPE_CFRANGE,
            &mut caret as *mut CFRange as *mut c_void,
        ) == 0
        {
            return Err(AxFail::Unsupported);
        }
        if super::debug_enabled() {
            super::dlog(&format!(
                "  ax caret loc={} len={} old_len={old_len}",
                caret.location, caret.length
            ));
        }
        if caret.length != 0 || caret.location < old_len {
            return Err(AxFail::Mismatch);
        }

        let target = CFRange {
            location: caret.location - old_len,
            length: old_len,
        };

        if out_of_time() {
            return Err(AxFail::Unsupported);
        }
        // XÁC MINH: app phải đang thực sự hiển thị `old` ngay trước caret.
        // Nếu ký tự passthrough trước đó chưa vào app (gõ nhanh), đoạn
        // này sẽ khác → fallback bơm phím (được xếp hàng đúng thứ tự).
        match read_string_for_range(element, &target) {
            Some(actual) if actual == old => {}
            Some(actual) => {
                let _ = actual;
                if super::debug_enabled() {
                    super::dlog("  ax verify failed");
                }
                return Err(AxFail::Mismatch);
            }
            // App không hỗ trợ đọc theo range → không xác minh được →
            // không đủ an toàn để ghi đè.
            None => {
                super::dlog("  ax StringForRange unsupported");
                return Err(AxFail::Unsupported);
            }
        }
        // Last exit before writing: once a write is issued the edit must run
        // to the end, whatever the budget says.
        if out_of_time() {
            return Err(AxFail::Unsupported);
        }
        // Chọn đúng đoạn cần thay...
        let target_value = AXValueCreate(
            K_AX_VALUE_TYPE_CFRANGE,
            &target as *const CFRange as *const c_void,
        );
        if target_value.is_null() {
            return Err(AxFail::Unsupported);
        }
        let target_guard = CFType::wrap_under_create_rule(target_value);
        // The element answered the reads: give the writes more time, since
        // a write that times out may still be applied later.
        AXUIElementSetMessagingTimeout(element, AX_WRITE_TIMEOUT_SECS);
        match AXUIElementSetAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            target_guard.as_CFTypeRef(),
        ) {
            AX_SUCCESS => {}
            AX_ERROR_CANNOT_COMPLETE => return Err(abandon(element, caret.location, true)),
            // Rejected outright: nothing changed, injecting is safe.
            _ => return Err(AxFail::Unsupported),
        }

        // Một số app (Chrome...) trả thành công nhưng lờ lệnh chọn vùng —
        // nếu ghi text lúc đó sẽ CHÈN thay vì THAY. Đọc lại để chắc chắn.
        let mut check: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            &mut check,
        );
        if err != AX_SUCCESS || check.is_null() {
            // The selection may be up: a backspace would eat it.
            return Err(abandon(
                element,
                caret.location,
                err == AX_ERROR_CANNOT_COMPLETE,
            ));
        }
        let check_guard = CFType::wrap_under_create_rule(check);
        let mut applied = CFRange {
            location: 0,
            length: 0,
        };
        if AXValueGetValue(
            check_guard.as_CFTypeRef(),
            K_AX_VALUE_TYPE_CFRANGE,
            &mut applied as *mut CFRange as *mut c_void,
        ) == 0
            || applied.location != target.location
            || applied.length != target.length
        {
            if super::debug_enabled() {
                super::dlog(&format!(
                    "  ax range set ignored: applied=({},{}) target=({},{})",
                    applied.location, applied.length, target.location, target.length
                ));
            }
            // The app may still apply the range later (Chromium forwards it
            // to the renderer asynchronously): put the caret back first.
            return Err(abandon(element, caret.location, false));
        }

        // ...và ghi đè bằng chuỗi mới trong một thao tác nguyên tử.
        let new_len = text.encode_utf16().count() as isize;
        let replacement = CFString::new(text);
        let err = AXUIElementSetAttributeValue(
            element,
            attr("AXSelectedText").as_concrete_TypeRef(),
            replacement.as_CFTypeRef(),
        );
        if err == AX_ERROR_CANNOT_COMPLETE {
            // The text may still be replaced. Collapse the selection where
            // the caret ends up if it is, so neither outcome leaves a
            // selection for the next key to overwrite.
            return Err(abandon(element, target.location + new_len, true));
        }
        if err != AX_SUCCESS {
            // App nhận range nhưng từ chối ghi text: khôi phục con trỏ
            // về vị trí cũ trước khi báo lỗi để fallback không phá vùng chọn.
            return Err(abandon(element, caret.location, false));
        }

        // Một số app nhận range (vùng chọn HIỆN LÊN màn hình) nhưng lờ
        // lệnh ghi AXSelectedText mà vẫn trả success (Telegram Desktop/Qt,
        // issue #2): tin vào success là kẹt luôn vùng chọn bôi đen, chữ cũ
        // còn nguyên, người dùng không gõ tiếp được. Đọc lại để chắc text
        // đã thật sự thay; chưa thay → trả con trỏ về chỗ cũ (hủy vùng
        // chọn kẹt) rồi báo Unsupported để caller cache và bơm phím.
        let written = CFRange {
            location: target.location,
            length: new_len,
        };
        match read_string_for_range(element, &written) {
            Some(actual) if actual != text => {
                let _ = actual;
                if super::debug_enabled() {
                    super::dlog("  ax write ignored");
                }
                return Err(abandon(element, caret.location, false));
            }
            // Khớp → ghi thành công thật. Không đọc lại được → không đủ
            // bằng chứng app nói dối, giữ nguyên hành vi cũ (tin success).
            _ => {}
        }
        Ok(())
    }
}

/// Clean up after a write went wrong by collapsing the selection at
/// `location`. Key injection is safe only if no write timed out (a
/// timed-out write may still land) and the app confirmed the restore.
unsafe fn abandon(element: AXUIElementRef, location: isize, timed_out: bool) -> AxFail {
    if timed_out {
        AXUIElementSetMessagingTimeout(element, AX_CLEANUP_TIMEOUT_SECS);
    }
    if restore_caret(element, location) && !timed_out {
        AxFail::Unsupported
    } else {
        AxFail::Unknown
    }
}

/// Đưa con trỏ về `location` (hủy mọi vùng chọn) — dọn dẹp sau khi lệnh
/// ghi AX thất bại giữa chừng, để fallback bơm phím xóa đúng chữ.
unsafe fn restore_caret(element: AXUIElementRef, location: isize) -> bool {
    let restore = CFRange {
        location,
        length: 0,
    };
    let restore_value = AXValueCreate(
        K_AX_VALUE_TYPE_CFRANGE,
        &restore as *const CFRange as *const c_void,
    );
    if restore_value.is_null() {
        return false;
    }
    let restore_guard = CFType::wrap_under_create_rule(restore_value);
    AXUIElementSetAttributeValue(
        element,
        attr("AXSelectedTextRange").as_concrete_TypeRef(),
        restore_guard.as_CFTypeRef(),
    ) == AX_SUCCESS
}

/// Đọc chuỗi app đang hiển thị tại `range` (AXStringForRange).
unsafe fn read_string_for_range(element: AXUIElementRef, range: &CFRange) -> Option<String> {
    let range_value = AXValueCreate(
        K_AX_VALUE_TYPE_CFRANGE,
        range as *const CFRange as *const c_void,
    );
    if range_value.is_null() {
        return None;
    }
    let range_guard = CFType::wrap_under_create_rule(range_value);
    let mut out: CFTypeRef = std::ptr::null();
    if AXUIElementCopyParameterizedAttributeValue(
        element,
        attr("AXStringForRange").as_concrete_TypeRef(),
        range_guard.as_CFTypeRef(),
        &mut out,
    ) != AX_SUCCESS
        || out.is_null()
    {
        return None;
    }
    let out_guard = CFType::wrap_under_create_rule(out);
    let s = out_guard.downcast::<CFString>()?;
    Some(s.to_string())
}

/// Đã được cấp quyền Accessibility chưa (Swift dùng cho onboarding).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

/// Thông tin ô focus, lấy trong MỘT lượt tra cứu AX — callback tap phải
/// nhanh (WindowServer giao lại event nếu tap phản hồi chậm → phím bị
/// nhân đôi, bug thực địa "master → masster").
#[derive(Default)]
pub struct FocusedInfo {
    /// Tên process sở hữu ô focus. Khác app frontmost: panel nổi như
    /// Spotlight nhận phím nhưng không phát sự kiện đổi app.
    pub proc_name: Option<String>,
    /// Độ dài vùng chọn (None = không đọc được) — quyết định có cần
    /// phím hủy gợi ý autocomplete không.
    pub selection_len: Option<isize>,
    /// An AX call timed out: the app is busy or hung, and every further AX
    /// call would cost another timeout.
    pub hung: bool,
}

/// Skips the remaining reads once `deadline` has passed.
pub fn focused_info(deadline: Instant) -> FocusedInfo {
    let mut info = FocusedInfo::default();
    unsafe {
        let system_wide = create_system_wide();
        if system_wide.is_null() {
            return info;
        }
        let _guard = CFType::wrap_under_create_rule(system_wide as CFTypeRef);
        let mut focused: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            attr("AXFocusedUIElement").as_concrete_TypeRef(),
            &mut focused,
        );
        if err != AX_SUCCESS || focused.is_null() {
            info.hung = err == AX_ERROR_CANNOT_COMPLETE;
            return info;
        }
        let focused_guard = CFType::wrap_under_create_rule(focused);
        let element = focused_guard.as_CFTypeRef() as AXUIElementRef;

        let mut pid: i32 = 0;
        // Own window (Settings): AX calls back into this process from the
        // tap thread; see the same guard in replace_tail.
        if AXUIElementGetPid(element, &mut pid) == AX_SUCCESS && pid == libc::getpid() {
            return info;
        }
        if pid > 0 {
            let mut buf = [0u8; 128];
            let n = proc_name(pid, buf.as_mut_ptr() as *mut c_void, buf.len() as u32);
            if n > 0 {
                info.proc_name = std::str::from_utf8(&buf[..n as usize])
                    .ok()
                    .map(String::from);
            }
        }

        if Instant::now() >= deadline {
            return info;
        }
        let mut range_value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            &mut range_value,
        );
        info.hung = err == AX_ERROR_CANNOT_COMPLETE;
        if err == AX_SUCCESS && !range_value.is_null() {
            let range_guard = CFType::wrap_under_create_rule(range_value);
            let mut range = CFRange {
                location: 0,
                length: 0,
            };
            if AXValueGetValue(
                range_guard.as_CFTypeRef(),
                K_AX_VALUE_TYPE_CFRANGE,
                &mut range as *mut CFRange as *mut c_void,
            ) != 0
            {
                info.selection_len = Some(range.length);
            }
        }
    }
    info
}
