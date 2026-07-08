//! Sửa chữ trực tiếp qua Accessibility API: chọn N ký tự trước con trỏ
//! và ghi đè bằng chuỗi mới trong MỘT thao tác — không backspace, không
//! thể nháy hay dính chữ. Không phải app nào cũng hỗ trợ; mọi lỗi trả
//! Err để caller rơi về key injection.

use std::ffi::c_void;

use core_foundation::base::{CFRange, CFType, CFTypeRef, TCFType};
use core_foundation::string::{CFString, CFStringRef};

type AXUIElementRef = *const c_void;
type AXError = i32;

const K_AX_VALUE_TYPE_CFRANGE: u32 = 4;
const AX_SUCCESS: AXError = 0;

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
    fn AXValueCreate(value_type: u32, value_ptr: *const c_void) -> CFTypeRef;
    fn AXValueGetValue(value: CFTypeRef, value_type: u32, out: *mut c_void) -> u8;
    pub fn AXIsProcessTrusted() -> u8;
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
}

/// Thay đoạn `old` ngay trước con trỏ bằng `text` — CHỈ khi xác minh
/// được app thực sự đang hiển thị `old` tại đó. Không xác minh mù:
/// ký tự passthrough đến app bất đồng bộ, ghi đè mù sẽ nuốt nhầm chữ
/// (bug thực địa: gõ nhanh "đó" thành "óo").
pub fn replace_tail(old: &str, text: &str) -> Result<(), AxFail> {
    // AX làm việc theo đơn vị UTF-16.
    let old_len = old.encode_utf16().count() as isize;

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
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
        super::dlog(&format!(
            "  ax caret loc={} len={} old_len={old_len}",
            caret.location, caret.length
        ));
        if caret.length != 0 || caret.location < old_len {
            return Err(AxFail::Mismatch);
        }

        let target = CFRange {
            location: caret.location - old_len,
            length: old_len,
        };

        // XÁC MINH: app phải đang thực sự hiển thị `old` ngay trước caret.
        // Nếu ký tự passthrough trước đó chưa vào app (gõ nhanh), đoạn
        // này sẽ khác → fallback bơm phím (được xếp hàng đúng thứ tự).
        match read_string_for_range(element, &target) {
            Some(actual) if actual == old => {}
            Some(actual) => {
                super::dlog(&format!("  ax verify FAIL actual={actual:?} vs old={old:?}"));
                return Err(AxFail::Mismatch);
            }
            // App không hỗ trợ đọc theo range → không xác minh được →
            // không đủ an toàn để ghi đè.
            None => {
                super::dlog("  ax StringForRange unsupported");
                return Err(AxFail::Unsupported);
            }
        }
        // Chọn đúng đoạn cần thay...
        let target_value =
            AXValueCreate(K_AX_VALUE_TYPE_CFRANGE, &target as *const CFRange as *const c_void);
        if target_value.is_null() {
            return Err(AxFail::Unsupported);
        }
        let target_guard = CFType::wrap_under_create_rule(target_value);
        if AXUIElementSetAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            target_guard.as_CFTypeRef(),
        ) != AX_SUCCESS
        {
            return Err(AxFail::Unsupported);
        }

        // Một số app (Chrome...) trả thành công nhưng lờ lệnh chọn vùng —
        // nếu ghi text lúc đó sẽ CHÈN thay vì THAY. Đọc lại để chắc chắn.
        let mut check: CFTypeRef = std::ptr::null();
        if AXUIElementCopyAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            &mut check,
        ) != AX_SUCCESS
            || check.is_null()
        {
            return Err(AxFail::Unsupported);
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
            super::dlog(&format!(
                "  ax range set ignored: applied=({},{}) target=({},{})",
                applied.location, applied.length, target.location, target.length
            ));
            return Err(AxFail::Unsupported);
        }

        // ...và ghi đè bằng chuỗi mới trong một thao tác nguyên tử.
        let replacement = CFString::new(text);
        if AXUIElementSetAttributeValue(
            element,
            attr("AXSelectedText").as_concrete_TypeRef(),
            replacement.as_CFTypeRef(),
        ) != AX_SUCCESS
        {
            // App nhận range nhưng từ chối ghi text: khôi phục con trỏ
            // về vị trí cũ trước khi báo lỗi để fallback không phá vùng chọn.
            let restore = CFRange {
                location: caret.location,
                length: 0,
            };
            let restore_value = AXValueCreate(
                K_AX_VALUE_TYPE_CFRANGE,
                &restore as *const CFRange as *const c_void,
            );
            if !restore_value.is_null() {
                let restore_guard = CFType::wrap_under_create_rule(restore_value);
                AXUIElementSetAttributeValue(
                    element,
                    attr("AXSelectedTextRange").as_concrete_TypeRef(),
                    restore_guard.as_CFTypeRef(),
                );
            }
            return Err(AxFail::Unsupported);
        }
        Ok(())
    }
}

/// Đọc chuỗi app đang hiển thị tại `range` (AXStringForRange).
unsafe fn read_string_for_range(element: AXUIElementRef, range: &CFRange) -> Option<String> {
    let range_value =
        AXValueCreate(K_AX_VALUE_TYPE_CFRANGE, range as *const CFRange as *const c_void);
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
