//! Thực hiện `Action::Replace` vào app đích — trái tim của việc chống
//! dính/nháy chữ. Thứ tự ưu tiên: AX API (nguyên tử, không nháy) →
//! key injection với diff tối thiểu, gộp chuỗi vào ít event nhất.

use std::collections::HashMap;
use std::time::Duration;

use core_graphics::event::{CGEvent, CGEventTapProxy, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use super::ax;
use super::profiles::ResolvedProfile;
use crate::config::FixMode;

/// Đánh dấu event do OreoKey bơm ra để tap bỏ qua ("OREO").
pub const MAGIC: i64 = 0x4F52_454F;

const KEY_BACKSPACE: u16 = 51;
const KEY_SPACE: u16 = 49;
/// Số đơn vị UTF-16 tối đa mỗi event chữ — một số app bỏ ký tự khi
/// nhận chuỗi quá dài trong một event.
const CHUNK_UTF16: usize = 8;

pub fn apply(
    proxy: CGEventTapProxy,
    old: &str,
    text: &str,
    profile: &ResolvedProfile,
    bundle: &str,
    ax_ok: &mut HashMap<String, bool>,
) {
    let backspaces = old.chars().count();
    // Chèn thuần (không xóa gì): bơm phím không thể nháy, và AX lúc này
    // dễ dính race với ký tự passthrough đang trên đường đến app → bơm.
    let ax_worth_it = !old.is_empty();

    super::dlog(&format!(
        "apply bundle={bundle} mode={:?} old={old:?} text={text:?} browser_fix={}",
        profile.mode, profile.browser_fix
    ));
    match profile.mode {
        FixMode::Auto => {
            // AX trước nếu app này chưa từng fail hẳn.
            if ax_worth_it && *ax_ok.get(bundle).unwrap_or(&true) {
                match ax::replace_tail(old, text) {
                    Ok(()) => {
                        super::dlog("  -> AX ok");
                        ax_ok.insert(bundle.to_string(), true);
                        return;
                    }
                    // Mismatch = văn bản trước caret chưa ổn định (ký tự
                    // passthrough chưa vào app) — KHÔNG phải app không hỗ
                    // trợ AX, đừng cache là fail, chỉ fallback lần này.
                    Err(ax::AxFail::Mismatch) => {
                        super::dlog("  -> AX mismatch, fallback inject");
                    }
                    Err(ax::AxFail::Unsupported) => {
                        super::dlog("  -> AX unsupported, cache fail + inject");
                        ax_ok.insert(bundle.to_string(), false);
                    }
                }
            } else {
                super::dlog("  -> inject (AX cached fail / insert-only)");
            }
            key_inject(proxy, backspaces, text, profile);
        }
        FixMode::AxOnly => {
            let _ = ax::replace_tail(old, text);
        }
        FixMode::InjectFast | FixMode::InjectSlow => {
            key_inject(proxy, backspaces, text, profile);
        }
    }
}

fn key_inject(proxy: CGEventTapProxy, backspaces: usize, text: &str, profile: &ResolvedProfile) {
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
        return;
    };
    let delay = if profile.mode == FixMode::InjectSlow {
        Some(Duration::from_millis(profile.delay_ms.max(1)))
    } else {
        None
    };

    // Trình duyệt có thể đang bôi đen phần gợi ý autocomplete — backspace
    // đầu sẽ nuốt phần bôi đen thay vì ký tự thật (bug thực địa: "bận"
    // thành "baận" trên Chrome). Gõ một dấu cách rồi xóa ngay: có gợi ý
    // thì dấu cách thay thế phần bôi đen (hủy gợi ý), không có thì cặp
    // gõ-xóa tự triệt tiêu.
    //
    // Chỉ làm khi THỰC SỰ có vùng bôi đen (đọc qua AX) — gõ chay cặp
    // space+backspace mỗi lần bỏ dấu gây nháy con trỏ thấy rõ. Không
    // đọc được selection thì thà an toàn: vẫn gửi.
    if profile.browser_fix && backspaces > 0 {
        let has_selection = ax::selection_length().map(|len| len > 0).unwrap_or(true);
        if has_selection {
            post_key(&source, proxy, KEY_SPACE, " ", delay);
            post_key(&source, proxy, KEY_BACKSPACE, "", delay);
        }
    }

    for _ in 0..backspaces {
        post_key(&source, proxy, KEY_BACKSPACE, "", delay);
    }

    let chars: Vec<char> = text.chars().collect();
    for chunk in chars.chunks(CHUNK_UTF16) {
        let s: String = chunk.iter().collect();
        post_key(&source, proxy, 0, &s, delay);
    }
}

/// Gửi một cặp keydown/keyup được đánh dấu MAGIC. `text` không rỗng thì
/// gắn chuỗi unicode vào keydown (app đọc chuỗi, không quan tâm keycode).
fn post_key(
    source: &CGEventSource,
    proxy: CGEventTapProxy,
    keycode: u16,
    text: &str,
    delay: Option<Duration>,
) {
    for down in [true, false] {
        let Ok(ev) = CGEvent::new_keyboard_event(source.clone(), keycode, down) else {
            continue;
        };
        if !text.is_empty() && down {
            ev.set_string(text);
        }
        ev.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, MAGIC);
        ev.post_from_tap(proxy);
    }
    if let Some(d) = delay {
        std::thread::sleep(d);
    }
}
