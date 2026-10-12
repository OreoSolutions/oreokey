//! Gõ tắt: bảng cụm tắt do người dùng định nghĩa, khớp không phân biệt
//! hoa thường với từ vừa hiển thị khi chốt từ; kết quả theo cách gõ:
//! `ko` → không, `Ko` → Không, `KO` → KHÔNG.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct MacroTable {
    map: HashMap<String, String>,
}

impl MacroTable {
    pub fn new(entries: impl IntoIterator<Item = (String, String)>) -> MacroTable {
        MacroTable {
            map: entries
                .into_iter()
                .map(|(from, to)| (from.to_lowercase(), to))
                .collect(),
        }
    }

    pub fn expand(&self, word: &str) -> Option<String> {
        let to = self.map.get(&word.to_lowercase())?;
        let mut chars = word.chars();
        let first_upper = chars.next().is_some_and(char::is_uppercase);
        let rest_upper = chars.clone().any(char::is_uppercase);
        Some(if first_upper && rest_upper {
            to.to_uppercase()
        } else if first_upper {
            let mut out = String::with_capacity(to.len());
            let mut it = to.chars();
            if let Some(c) = it.next() {
                out.extend(c.to_uppercase());
            }
            out.extend(it);
            out
        } else {
            to.clone()
        })
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
            auto_capitalize: false,
            esc_revert: false,
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
    fn case_follows_how_the_trigger_was_typed() {
        let table = super::MacroTable::new([("ko".to_string(), "không".to_string())]);
        assert_eq!(table.expand("ko").as_deref(), Some("không"));
        assert_eq!(table.expand("Ko").as_deref(), Some("Không"));
        assert_eq!(table.expand("KO").as_deref(), Some("KHÔNG"));
        // Trộn hoa thường không theo mẫu nào: trả nguyên.
        assert_eq!(table.expand("kO").as_deref(), Some("không"));
        assert_eq!(table.expand("xo"), None);
    }

    #[test]
    fn uppercase_key_in_table_still_matches() {
        let table = super::MacroTable::new([("VN".to_string(), "Việt Nam".to_string())]);
        assert_eq!(table.expand("vn").as_deref(), Some("Việt Nam"));
        assert_eq!(table.expand("VN").as_deref(), Some("VIỆT NAM"));
    }

    #[test]
    fn single_letter_capital_is_title_case() {
        let table = super::MacroTable::new([("k".to_string(), "không".to_string())]);
        assert_eq!(table.expand("K").as_deref(), Some("Không"));
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
