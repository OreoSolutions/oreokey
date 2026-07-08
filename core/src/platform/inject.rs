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
/// Số đơn vị UTF-16 tối đa mỗi event chữ — một số app bỏ ký tự khi
/// nhận chuỗi quá dài trong một event.
const CHUNK_UTF16: usize = 8;

pub fn apply(
    proxy: CGEventTapProxy,
    backspaces: usize,
    text: &str,
    profile: &ResolvedProfile,
    bundle: &str,
    ax_ok: &mut HashMap<String, bool>,
) {
    match profile.mode {
        FixMode::Auto => {
            // AX trước nếu app này chưa từng fail.
            if *ax_ok.get(bundle).unwrap_or(&true) {
                if ax::replace_tail(backspaces, text).is_ok() {
                    ax_ok.insert(bundle.to_string(), true);
                    return;
                }
                ax_ok.insert(bundle.to_string(), false);
            }
            key_inject(proxy, backspaces, text, profile);
        }
        FixMode::AxOnly => {
            let _ = ax::replace_tail(backspaces, text);
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

    // Trình duyệt đang bôi đen phần gợi ý autocomplete: gửi một phím
    // "rỗng" vô hại để hủy bôi đen trước khi backspace, tránh xóa nhầm.
    if profile.browser_fix && backspaces > 0 {
        post_key(&source, proxy, 255, "", delay);
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
