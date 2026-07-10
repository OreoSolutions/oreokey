//! Che từ tục tĩu: khi chốt từ, nếu từ nằm trong danh sách thì thay
//! bằng dấu sao cùng độ dài. So khớp không phân biệt hoa thường.

/// Danh sách từ bị che. Chọn lọc bảo thủ — chỉ những từ gần như không
/// có nghĩa vô hại trong ngữ cảnh thường — để tránh che nhầm.
const PROFANITY: &[&str] = &[
    // Tiếng Việt
    "đụ", "địt", "đéo", "lồn", "cặc", "buồi", "đĩ", "cứt",
    "đm", "đcm", "đmm", "đjt", "vcl", "vkl", "cmm", "clm", "cặk", "cak",
    // Tiếng Anh
    "fuck", "fucking", "fucker", "shit", "bitch", "cunt", "asshole", "dick",
];

pub fn is_profane(word: &str) -> bool {
    let lower = word.to_lowercase();
    PROFANITY.contains(&lower.as_str())
}

/// Chuỗi thay thế: dấu * cùng số ký tự.
pub fn mask(word: &str) -> String {
    "*".repeat(word.chars().count())
}

#[cfg(test)]
mod tests {
    use crate::engine::testutil::type_str;
    use crate::engine::{Action, Engine, EngineConfig, KeyInput, SpellMode, TypingMethod};

    fn engine() -> Engine {
        Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: true,
        })
    }

    #[test]
    fn profane_word_masked_on_break() {
        let mut e = engine();
        type_str(&mut e, "ddeos"); // → đéo
        assert_eq!(
            e.on_key(KeyInput::WordBreak(Some(' '))),
            Action::Replace {
                old: "đéo".to_string(),
                text: "*** ".to_string()
            }
        );
    }

    #[test]
    fn case_insensitive() {
        let mut e = engine();
        type_str(&mut e, "FUCK");
        assert_eq!(
            e.on_key(KeyInput::WordBreak(Some('!'))),
            Action::Replace {
                old: "FUCK".to_string(),
                text: "****!".to_string()
            }
        );
    }

    #[test]
    fn normal_words_untouched() {
        let mut e = engine();
        type_str(&mut e, "vieetj");
        assert_eq!(e.on_key(KeyInput::WordBreak(Some(' '))), Action::PassThrough);
        // "đó" chứa "đ" nhưng không nằm trong danh sách.
        type_str(&mut e, "ddos");
        assert_eq!(e.on_key(KeyInput::WordBreak(Some(' '))), Action::PassThrough);
    }

    #[test]
    fn disabled_by_default_config() {
        let mut e = Engine::new(EngineConfig::default());
        type_str(&mut e, "ddeos");
        assert_eq!(e.on_key(KeyInput::WordBreak(Some(' '))), Action::PassThrough);
    }
}
