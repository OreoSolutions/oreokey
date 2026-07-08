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

/// Thay `backspaces` ký tự ngay trước con trỏ bằng `text`.
pub fn replace_tail(backspaces: usize, text: &str) -> Result<(), ()> {
    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return Err(());
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
            return Err(());
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
            return Err(());
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
            || caret.length != 0
            || (caret.location as usize) < backspaces
        {
            return Err(());
        }

        // Chọn đúng N ký tự cần thay...
        let target = CFRange {
            location: caret.location - backspaces as isize,
            length: backspaces as isize,
        };
        let target_value =
            AXValueCreate(K_AX_VALUE_TYPE_CFRANGE, &target as *const CFRange as *const c_void);
        if target_value.is_null() {
            return Err(());
        }
        let target_guard = CFType::wrap_under_create_rule(target_value);
        if AXUIElementSetAttributeValue(
            element,
            attr("AXSelectedTextRange").as_concrete_TypeRef(),
            target_guard.as_CFTypeRef(),
        ) != AX_SUCCESS
        {
            return Err(());
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
            return Err(());
        }
        Ok(())
    }
}

/// Đã được cấp quyền Accessibility chưa (Swift dùng cho onboarding).
pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}
