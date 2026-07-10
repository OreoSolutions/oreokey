//! Gõ tắt: bảng cụm tắt do người dùng định nghĩa, khớp chính xác
//! (phân biệt hoa thường) với từ vừa hiển thị khi chốt từ.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct MacroTable {
    map: HashMap<String, String>,
}

impl MacroTable {
    pub fn new(entries: impl IntoIterator<Item = (String, String)>) -> MacroTable {
        MacroTable {
            map: entries.into_iter().collect(),
        }
    }

    pub fn expand(&self, word: &str) -> Option<&str> {
        self.map.get(word).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::testutil::type_str;
    use crate::engine::{Action, Engine, EngineConfig, KeyInput, SpellMode, TypingMethod};

    fn engine_with_macros() -> Engine {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: true,
            flexible_marks: true,
            censor_enabled: false,
        });
        e.set_macros(super::MacroTable::new([
            ("vn".to_string(), "Việt Nam".to_string()),
            ("email".to_string(), "red.diephoang@gmail.com".to_string()),
        ]));
        e
    }

    #[test]
    fn expands_on_word_break() {
        let mut e = engine_with_macros();
        type_str(&mut e, "vn");
        let action = e.on_key(KeyInput::WordBreak(Some(' ')));
        assert_eq!(
            action,
            Action::Replace {
                old: "vn".to_string(),
                text: "Việt Nam ".to_string()
            }
        );
        assert_eq!(e.current_word(), "");
    }

    #[test]
    fn no_match_passes_through() {
        let mut e = engine_with_macros();
        type_str(&mut e, "vietj");
        assert_eq!(e.on_key(KeyInput::WordBreak(Some(' '))), Action::PassThrough);
    }

    #[test]
    fn case_sensitive() {
        let mut e = engine_with_macros();
        type_str(&mut e, "VN");
        assert_eq!(e.on_key(KeyInput::WordBreak(Some(' '))), Action::PassThrough);
    }

    #[test]
    fn cursor_move_does_not_expand() {
        let mut e = engine_with_macros();
        type_str(&mut e, "vn");
        // Click chuột / mũi tên: chỉ reset, không chèn gì.
        assert_eq!(e.on_key(KeyInput::WordBreak(None)), Action::PassThrough);
    }

    #[test]
    fn disabled_macros_do_nothing() {
        let mut e = engine_with_macros();
        let mut cfg = e.config().clone();
        cfg.macros_enabled = false;
        e.set_config(cfg);
        type_str(&mut e, "vn");
        assert_eq!(e.on_key(KeyInput::WordBreak(Some(' '))), Action::PassThrough);
    }
}
