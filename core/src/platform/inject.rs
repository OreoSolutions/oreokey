//! Thực hiện `Action::Replace` vào app đích — trái tim của việc chống
//! dính/nháy chữ. Thứ tự ưu tiên: AX API (nguyên tử, không nháy) →
//! key injection với diff tối thiểu, gộp chuỗi vào ít event nhất.

use std::collections::HashMap;
use std::time::Duration;

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapProxy, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use super::ax;
use super::profiles::ResolvedProfile;
use crate::config::FixMode;

/// Đánh dấu event do OreoKey bơm ra để tap bỏ qua ("OREO").
pub const MAGIC: i64 = 0x4F52_454F;

const KEY_BACKSPACE: u16 = 51;
const KEY_SPACE: u16 = 49;
const KEY_LEFT_ARROW: u16 = 123;
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
    selection_len: Option<isize>,
) -> bool {
    let backspaces = old.chars().count();
    // Chèn thuần (không xóa gì): bơm phím không thể nháy, và AX lúc này
    // dễ dính race với ký tự passthrough đang trên đường đến app → bơm.
    let ax_worth_it = !old.is_empty();

    if super::debug_enabled() {
        super::dlog(&format!(
            "apply bundle={bundle} mode={:?} replace_len={} text_len={} browser_fix={}",
            profile.mode,
            old.chars().count(),
            text.chars().count(),
            profile.browser_fix
        ));
    }
    match profile.mode {
        FixMode::Auto => {
            // AX trước nếu app này chưa từng fail hẳn.
            if ax_worth_it && *ax_ok.get(bundle).unwrap_or(&true) {
                match ax::replace_tail(old, text) {
                    Ok(()) => {
                        super::dlog("  -> AX ok");
                        ax_ok.insert(bundle.to_string(), true);
                        return true;
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
            key_inject(proxy, backspaces, text, profile, selection_len)
        }
        FixMode::AxOnly => ax::replace_tail(old, text).is_ok(),
        FixMode::InjectFast | FixMode::InjectSlow => {
            key_inject(proxy, backspaces, text, profile, selection_len)
        }
    }
}

fn key_inject(
    proxy: CGEventTapProxy,
    backspaces: usize,
    text: &str,
    profile: &ResolvedProfile,
    selection_len: Option<isize>,
) -> bool {
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
        return false;
    };
    let delay = if profile.mode == FixMode::InjectSlow {
        Some(Duration::from_millis(profile.delay_ms.max(1)))
    } else {
        None
    };

    // App điền lại ghost text sau MỌI event (Spotlight): backspace luôn
    // thua cuộc đua với autocomplete. Chọn ngược bằng Shift+← (tự hủy
    // ghost) rồi gõ đè lên vùng chọn — không có backspace nào để bị nuốt.
    if profile.select_replace && backspaces > 0 {
        if super::debug_enabled() {
            super::dlog(&format!("  select_replace bs={backspaces}"));
        }
        for _ in 0..backspaces {
            post_key_flags(
                &source,
                proxy,
                KEY_LEFT_ARROW,
                "",
                CGEventFlags::CGEventFlagShift,
                delay,
            );
        }
        post_text_chunks(&source, proxy, text, delay);
        return true;
    }

    // Ô nhập có autocomplete (thanh địa chỉ trình duyệt, Spotlight...)
    // bôi đen phần gợi ý — backspace đầu sẽ nuốt phần bôi đen thay vì ký
    // tự thật ("bận" thành "baận"). Khi engine đang giữa từ mà có vùng
    // chọn thì đó chắc chắn là autocomplete (click chuột / phím mũi tên
    // đều đã reset buffer), nên kiểm tra PHỔ QUÁT qua AX cho mọi app:
    // gõ một dấu cách thay thế phần bôi đen rồi xóa ngay.
    // Không đọc được selection (AX câm) → dựa vào cờ browser_fix của
    // profile: app được đánh dấu hay autocomplete thì thà gửi thừa.
    if backspaces > 0 {
        let clear = clear_strategy(profile, selection_len);
        if super::debug_enabled() {
            super::dlog(&format!(
                "  inject bs={backspaces} sel={selection_len:?} clear={clear:?}"
            ));
        }
        match clear {
            Clear::None => {}
            Clear::Backspace => post_key(&source, proxy, KEY_BACKSPACE, "", delay),
            Clear::SpaceBackspace => {
                post_key(&source, proxy, KEY_SPACE, " ", delay);
                post_key(&source, proxy, KEY_BACKSPACE, "", delay);
            }
        }
    }

    for _ in 0..backspaces {
        post_key(&source, proxy, KEY_BACKSPACE, "", delay);
    }

    post_text_chunks(&source, proxy, text, delay);
    true
}

/// Cách hủy phần gợi ý autocomplete đang bôi đen trước khi backspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clear {
    None,
    /// Vùng chọn chắc chắn có (AX đọc được): một backspace xóa đúng phần
    /// gợi ý. Vì là thao tác XÓA, omnibox không gợi ý lại.
    Backspace,
    /// Không biết có vùng chọn không (AX câm / app nói dối): space ghi
    /// đè nếu có gợi ý, backspace xóa lại space — trung hòa khi không có.
    SpaceBackspace,
}

/// `force_clear`: app có ghost text mà AX báo selection = 0 (Spotlight) —
/// kiểm tra selection vô nghĩa, luôn phải hủy.
///
/// Khi AX báo có vùng chọn thì KHÔNG dùng space+backspace: space là thao
/// tác chèn nên Chrome/Safari gợi ý lại ngay, backspace kế tiếp xóa gợi ý
/// mới thay vì xóa space → thiếu một backspace thật (bug thực địa: gõ
/// `thuowngr` trong thanh địa chỉ Chrome ra `thươởng`).
fn clear_strategy(profile: &ResolvedProfile, selection_len: Option<isize>) -> Clear {
    match selection_len {
        _ if profile.force_clear => Clear::SpaceBackspace,
        Some(len) if len > 0 => Clear::Backspace,
        Some(_) => Clear::None,
        None if profile.browser_fix => Clear::SpaceBackspace,
        None => Clear::None,
    }
}

/// Chunk by UTF-16 code units, matching the contract of `CHUNK_UTF16` even
/// for non-BMP characters in macro expansions.
fn post_text_chunks(
    source: &CGEventSource,
    proxy: CGEventTapProxy,
    text: &str,
    delay: Option<Duration>,
) {
    let mut chunk = String::new();
    let mut units = 0;
    for ch in text.chars() {
        let ch_units = ch.len_utf16();
        if !chunk.is_empty() && units + ch_units > CHUNK_UTF16 {
            post_key(source, proxy, 0, &chunk, delay);
            chunk.clear();
            units = 0;
        }
        chunk.push(ch);
        units += ch_units;
    }
    if !chunk.is_empty() {
        post_key(source, proxy, 0, &chunk, delay);
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
    post_key_flags(
        source,
        proxy,
        keycode,
        text,
        CGEventFlags::CGEventFlagNull,
        delay,
    );
}

fn post_key_flags(
    source: &CGEventSource,
    proxy: CGEventTapProxy,
    keycode: u16,
    text: &str,
    flags: CGEventFlags,
    delay: Option<Duration>,
) {
    for down in [true, false] {
        let Ok(ev) = CGEvent::new_keyboard_event(source.clone(), keycode, down) else {
            continue;
        };
        if flags != CGEventFlags::CGEventFlagNull {
            ev.set_flags(flags);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(browser_fix: bool, force_clear: bool) -> ResolvedProfile {
        ResolvedProfile {
            browser_fix,
            force_clear,
            ..ResolvedProfile::default()
        }
    }

    #[test]
    fn known_selection_uses_single_backspace() {
        // Chrome omnibox: "thương" + 20 ký tự gợi ý bôi đen. Space sẽ làm
        // omnibox gợi ý lại → phải dùng một backspace.
        assert_eq!(
            clear_strategy(&profile(true, false), Some(20)),
            Clear::Backspace
        );
        // Không phải app trình duyệt nhưng có vùng chọn giữa từ → vẫn là
        // autocomplete (click/mũi tên đều đã reset engine).
        assert_eq!(
            clear_strategy(&profile(false, false), Some(3)),
            Clear::Backspace
        );
    }

    #[test]
    fn unknown_selection_keeps_space_backspace_only_for_browsers() {
        assert_eq!(
            clear_strategy(&profile(true, false), None),
            Clear::SpaceBackspace
        );
        assert_eq!(clear_strategy(&profile(false, false), None), Clear::None);
        assert_eq!(clear_strategy(&profile(true, false), Some(0)), Clear::None);
    }

    #[test]
    fn force_clear_always_space_backspace() {
        assert_eq!(
            clear_strategy(&profile(false, true), Some(0)),
            Clear::SpaceBackspace
        );
    }
}
