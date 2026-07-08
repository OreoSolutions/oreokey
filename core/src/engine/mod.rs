//! Engine gõ tiếng Việt thuần túy: không import gì ngoài `std`.
//!
//! Kiến trúc re-render + diff: giữ chuỗi phím gốc (`raw`) của từ đang gõ,
//! mỗi phím render lại toàn bộ từ rồi so với lần render trước để tạo
//! `Action` sửa chữ tối thiểu.

pub mod encoding;
pub mod macros;
pub mod spell;
pub mod syllable;
pub mod telex;
pub mod vni;

use syllable::render_letters;

/// Hành động engine yêu cầu tầng platform thực hiện.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Cho phím gốc đi qua, không can thiệp.
    PassThrough,
    /// Nuốt phím gốc, thay `old` (đoạn cuối từ đang hiển thị) bằng `text`.
    /// `old` cho phép tầng platform XÁC MINH văn bản trước caret trước
    /// khi sửa qua AX — chống race khi ký tự passthrough chưa kịp vào app.
    Replace { old: String, text: String },
}

impl Action {
    /// Số ký tự cần xóa khi sửa bằng backspace.
    pub fn backspaces(&self) -> usize {
        match self {
            Action::PassThrough => 0,
            Action::Replace { old, .. } => old.chars().count(),
        }
    }
}

/// Phím đầu vào đã được tầng platform chuẩn hóa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    /// Ký tự in được (a-z, A-Z, 0-9...).
    Char(char),
    Backspace,
    /// Ký tự ngắt từ (space, dấu câu, Enter) hoặc None nếu là di chuyển
    /// con trỏ / click chuột (chỉ reset, không có ký tự đi kèm).
    WordBreak(Option<char>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingMethod {
    Telex,
    Vni,
}

/// Thanh điệu (không tính thanh ngang).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Acute, // sắc
    Grave, // huyền
    Hook,  // hỏi
    Tilde, // ngã
    Dot,   // nặng
}

/// Một chữ cái trong từ đang gõ cùng các dấu phụ (trừ thanh điệu —
/// thanh là thuộc tính cấp từ).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Letter {
    /// Chữ cái gốc ascii thường (a-z, 0-9).
    pub base: char,
    pub upper: bool,
    /// Dấu mũ: â ê ô.
    pub circ: bool,
    /// Dấu móc: ơ ư.
    pub horn: bool,
    /// Dấu trăng: ă.
    pub breve: bool,
    /// Gạch ngang: đ.
    pub stroke: bool,
    /// Chữ `ư` sinh từ phím `w` đứng một mình (Telex) — khi hủy phải
    /// hoàn về `w` chứ không phải `u`.
    pub w_origin: bool,
}

impl Letter {
    pub fn plain(c: char) -> Letter {
        Letter {
            base: c.to_ascii_lowercase(),
            upper: c.is_ascii_uppercase(),
            circ: false,
            horn: false,
            breve: false,
            stroke: false,
            w_origin: false,
        }
    }

    pub fn is_vowel(&self) -> bool {
        matches!(self.base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
    }

    pub fn has_mark(&self) -> bool {
        self.circ || self.horn || self.breve
    }
}

/// Trạng thái từ đang gõ, dựng lại từ đầu sau mỗi phím.
#[derive(Debug, Clone, Default)]
pub struct WordState {
    pub letters: Vec<Letter>,
    pub tone: Option<Tone>,
    /// Các phím modifier đã bị hủy bằng cách gõ lặp — trở thành chữ
    /// thường cho tới hết từ (vd `ass` → `as`, gõ thêm `s` → `ass`).
    pub dead: Vec<char>,
}

impl WordState {
    pub fn is_dead(&self, c: char) -> bool {
        self.dead.contains(&c)
    }

    pub fn has_vowel(&self) -> bool {
        self.letters.iter().any(|l| l.is_vowel())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub method: TypingMethod,
    pub spell_check: bool,
    /// Kiểu đặt dấu mới (`hoà`) thay vì kiểu cũ (`hòa`).
    pub modern_tone: bool,
    pub macros_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            method: TypingMethod::Telex,
            spell_check: true,
            modern_tone: false,
            macros_enabled: true,
        }
    }
}

pub struct Engine {
    cfg: EngineConfig,
    macros: macros::MacroTable,
    /// Chuỗi phím gốc của từ hiện tại.
    raw: String,
    /// Văn bản từ hiện tại như app đích đang hiển thị.
    last_render: String,
    /// Từ hiện tại đã bị phát hiện không phải tiếng Việt — hiển thị
    /// nguyên phím gốc cho tới khi ngắt từ (tránh nuốt phím dấu khi
    /// người dùng gõ tiếng nước ngoài).
    raw_mode: bool,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Engine {
        Engine {
            cfg,
            macros: macros::MacroTable::default(),
            raw: String::new(),
            last_render: String::new(),
            raw_mode: false,
        }
    }

    pub fn set_config(&mut self, cfg: EngineConfig) {
        self.cfg = cfg;
        self.reset();
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    pub fn set_macros(&mut self, table: macros::MacroTable) {
        self.macros = table;
    }

    /// Chốt/hủy từ hiện tại (đổi app, click chuột, di chuyển con trỏ...).
    pub fn reset(&mut self) {
        self.raw.clear();
        self.last_render.clear();
        self.raw_mode = false;
    }

    /// Từ đang gõ như đang hiển thị (phục vụ test/debug).
    pub fn current_word(&self) -> &str {
        &self.last_render
    }

    pub fn on_key(&mut self, k: KeyInput) -> Action {
        match k {
            KeyInput::Char(c) => self.on_char(c),
            KeyInput::Backspace => self.on_backspace(),
            KeyInput::WordBreak(ch) => {
                let action = match ch {
                    // Chỉ mở rộng gõ tắt khi chốt từ bằng một ký tự thật
                    // (space, dấu câu, Enter) — không phải di chuyển con trỏ.
                    Some(break_ch)
                        if self.cfg.macros_enabled && !self.last_render.is_empty() =>
                    {
                        match self.macros.expand(&self.last_render) {
                            Some(expansion) => {
                                let mut text = expansion.to_string();
                                text.push(break_ch);
                                Action::Replace {
                                    old: self.last_render.clone(),
                                    text,
                                }
                            }
                            None => Action::PassThrough,
                        }
                    }
                    _ => Action::PassThrough,
                };
                self.reset();
                action
            }
        }
    }

    fn on_char(&mut self, c: char) -> Action {
        self.raw.push(c);
        let new_render = if self.raw_mode {
            self.raw.clone()
        } else {
            let (text, restored) = self.render_word(&self.raw);
            if restored {
                self.raw_mode = true;
            }
            text
        };
        let action = diff_action(&self.last_render, &new_render, c);
        self.last_render = new_render;
        action
    }

    fn on_backspace(&mut self) -> Action {
        if self.raw.is_empty() {
            return Action::PassThrough;
        }
        // App sẽ tự xóa 1 ký tự hiển thị; đồng bộ lại raw cho khớp.
        let mut target: Vec<char> = self.last_render.chars().collect();
        target.pop();
        let target: String = target.into_iter().collect();
        while !self.raw.is_empty() {
            self.raw.pop();
            let rendered = if self.raw_mode {
                self.raw.clone()
            } else {
                self.render_word(&self.raw).0
            };
            if rendered == target {
                self.last_render = target;
                return Action::PassThrough;
            }
        }
        // Không khớp được (không nên xảy ra) — bỏ theo dõi từ này.
        self.reset();
        Action::PassThrough
    }

    /// Render chuỗi phím gốc thành văn bản hiển thị. Cờ thứ hai báo
    /// spell check đã phải khôi phục phím gốc (từ không phải tiếng Việt).
    fn render_word(&self, raw: &str) -> (String, bool) {
        if raw.is_empty() {
            return (String::new(), false);
        }
        let mut state = WordState::default();
        for c in raw.chars() {
            match self.cfg.method {
                TypingMethod::Telex => telex::apply_key(&mut state, c),
                TypingMethod::Vni => vni::apply_key(&mut state, c),
            }
        }
        // Từ bị biến đổi nhưng không phải âm tiết tiếng Việt → trả phím gốc.
        if self.cfg.spell_check
            && spell::is_transformed(&state)
            && !spell::is_acceptable(&state)
        {
            return (raw.to_string(), true);
        }
        (render_letters(&state, self.cfg.modern_tone), false)
    }
}

/// So hai lần render, tạo action tối thiểu. `typed` là phím vừa gõ —
/// nếu kết quả đúng bằng "thêm phím đó vào cuối" thì cho đi qua.
fn diff_action(last: &str, new: &str, typed: char) -> Action {
    let last_chars: Vec<char> = last.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let mut p = 0;
    while p < last_chars.len() && p < new_chars.len() && last_chars[p] == new_chars[p] {
        p += 1;
    }
    let old: String = last_chars[p..].iter().collect();
    let text: String = new_chars[p..].iter().collect();
    if old.is_empty() && text.chars().count() == 1 && text.chars().next() == Some(typed) {
        return Action::PassThrough;
    }
    Action::Replace { old, text }
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;

    /// Chạy chuỗi phím, trả về văn bản cuối như app đích thấy.
    /// `\u{8}` trong `keys` được hiểu là Backspace.
    pub fn type_str(engine: &mut Engine, keys: &str) -> String {
        let mut screen = String::new();
        for c in keys.chars() {
            let action = if c == '\u{8}' {
                engine.on_key(KeyInput::Backspace)
            } else {
                engine.on_key(KeyInput::Char(c))
            };
            match action {
                Action::PassThrough => {
                    if c == '\u{8}' {
                        screen.pop();
                    } else {
                        screen.push(c);
                    }
                }
                Action::Replace { old, text } => {
                    // Bất biến: phần bị thay phải đúng là đuôi màn hình —
                    // đây chính là điều AX verify dựa vào ngoài đời thật.
                    assert!(
                        screen.ends_with(&old),
                        "Replace old={old:?} không khớp đuôi màn hình {screen:?}"
                    );
                    for _ in 0..old.chars().count() {
                        screen.pop();
                    }
                    screen.push_str(&text);
                }
            }
        }
        screen
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::type_str;
    use super::*;

    fn telex_no_spell() -> Engine {
        Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_check: false,
            modern_tone: false,
            macros_enabled: false,
        })
    }

    #[test]
    fn plain_letters_pass_through() {
        let mut e = telex_no_spell();
        for c in ['h', 'n'] {
            assert_eq!(e.on_key(KeyInput::Char(c)), Action::PassThrough);
        }
        assert_eq!(e.current_word(), "hn");
    }

    #[test]
    fn word_break_resets() {
        let mut e = telex_no_spell();
        e.on_key(KeyInput::Char('a'));
        e.on_key(KeyInput::WordBreak(Some(' ')));
        assert_eq!(e.current_word(), "");
    }

    #[test]
    fn backspace_syncs_buffer() {
        let mut e = telex_no_spell();
        let screen = type_str(&mut e, "vieet\u{8}\u{8}");
        assert_eq!(screen, "vi");
        assert_eq!(e.current_word(), "vi");
    }

    #[test]
    fn backspace_on_empty_buffer_passes() {
        let mut e = telex_no_spell();
        assert_eq!(e.on_key(KeyInput::Backspace), Action::PassThrough);
    }
}
