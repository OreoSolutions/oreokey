//! Luật gõ VNI: 1-5 → sắc huyền hỏi ngã nặng; 6 → mũ (â ê ô);
//! 7 → móc (ơ ư); 8 → trăng (ă); 9 → đ; 0 → xóa thanh.
//! Gõ lặp số để hủy — sau khi hủy, số đó thành ký tự thường tới hết từ.

use super::syllable::vowel_indices;
use super::{Letter, Tone, WordState};

pub fn apply_key(state: &mut WordState, c: char) {
    let lower = c.to_ascii_lowercase();
    if state.is_dead(lower) {
        state.letters.push(Letter::plain(c));
        return;
    }
    match lower {
        '1'..='5' => {
            let tone = match lower {
                '1' => Tone::Acute,
                '2' => Tone::Grave,
                '3' => Tone::Hook,
                '4' => Tone::Tilde,
                _ => Tone::Dot,
            };
            state.apply_tone_key(tone, c);
        }
        '0' => state.apply_tone_clear(c),
        '6' => {
            // Áp lên nguyên âm a/e/o chưa có dấu gần cuối nhất.
            if let Some(i) = rpos(state, |l| {
                matches!(l.base, 'a' | 'e' | 'o') && !l.has_mark()
            }) {
                state.letters[i].circ = true;
                return;
            }
            // Không còn chỗ áp → hủy mũ hiện có.
            if let Some(i) = rpos(state, |l| l.circ) {
                state.letters[i].circ = false;
                state.dead.push('6');
                state.letters.push(Letter::plain(c));
                return;
            }
            state.letters.push(Letter::plain(c));
        }
        '7' => {
            // Cặp uo cuối → ươ.
            let vidx = vowel_indices(&state.letters);
            if vidx.len() >= 2 {
                let (i, j) = (vidx[vidx.len() - 2], vidx[vidx.len() - 1]);
                if j == i + 1
                    && state.letters[i].base == 'u'
                    && state.letters[j].base == 'o'
                    && !state.letters[i].has_mark()
                    && !state.letters[j].has_mark()
                {
                    state.letters[i].horn = true;
                    state.letters[j].horn = true;
                    return;
                }
            }
            if let Some(i) = rpos(state, |l| {
                matches!(l.base, 'u' | 'o') && !l.has_mark()
            }) {
                state.letters[i].horn = true;
                return;
            }
            if state.letters.iter().any(|l| l.horn) {
                for l in state.letters.iter_mut() {
                    l.horn = false;
                }
                state.dead.push('7');
                state.letters.push(Letter::plain(c));
                return;
            }
            state.letters.push(Letter::plain(c));
        }
        '8' => {
            if let Some(i) = rpos(state, |l| l.base == 'a' && !l.has_mark()) {
                state.letters[i].breve = true;
                return;
            }
            if let Some(i) = rpos(state, |l| l.breve) {
                state.letters[i].breve = false;
                state.dead.push('8');
                state.letters.push(Letter::plain(c));
                return;
            }
            state.letters.push(Letter::plain(c));
        }
        '9' => {
            if let Some(i) = rpos(state, |l| l.base == 'd' && !l.stroke) {
                state.letters[i].stroke = true;
                return;
            }
            if let Some(i) = rpos(state, |l| l.stroke) {
                state.letters[i].stroke = false;
                state.dead.push('9');
                state.letters.push(Letter::plain(c));
                return;
            }
            state.letters.push(Letter::plain(c));
        }
        _ => state.letters.push(Letter::plain(c)),
    }
}

/// Quét MỌI chữ cái từ phải sang (không qua `vowel_indices` như Telex —
/// cố ý, không phải thiếu sót): lấy phải-nhất nghĩa là nguyên âm chính
/// luôn thắng u-trong-qu/i-trong-gi, còn các tổ hợp lệch (vd `qu7` khi
/// chưa có nguyên âm chính) sinh âm tiết bất khả và bị spell gate khôi
/// phục — hành vi cuối cùng vẫn đúng mà không cần lặp lại luật loại trừ.
fn rpos(state: &WordState, pred: impl Fn(&Letter) -> bool) -> Option<usize> {
    state.letters.iter().rposition(|l| pred(l))
}

#[cfg(test)]
mod tests {
    use crate::engine::TypingMethod;

    fn v(keys: &str) -> String {
        crate::engine::testutil::raw_render(TypingMethod::Vni, keys, false, true)
    }

    #[test]
    fn tones() {
        assert_eq!(v("a1"), "á");
        assert_eq!(v("a2"), "à");
        assert_eq!(v("a3"), "ả");
        assert_eq!(v("a4"), "ã");
        assert_eq!(v("a5"), "ạ");
        assert_eq!(v("viet5"), "viẹt"); // không gõ 6 thì không có mũ
        assert_eq!(v("toan1"), "toán");
        assert_eq!(v("quy1"), "quý");
    }

    #[test]
    fn tone_cancel_and_remove() {
        assert_eq!(v("a11"), "a1");
        assert_eq!(v("a111"), "a11");
        assert_eq!(v("a12"), "à");
        assert_eq!(v("a10"), "a");
        assert_eq!(v("a0"), "a0");
    }

    #[test]
    fn marks() {
        assert_eq!(v("a6"), "â");
        assert_eq!(v("e6"), "ê");
        assert_eq!(v("o6"), "ô");
        assert_eq!(v("a66"), "a6");
        assert_eq!(v("u7"), "ư");
        assert_eq!(v("o7"), "ơ");
        assert_eq!(v("u77"), "u7");
        assert_eq!(v("a8"), "ă");
        assert_eq!(v("a88"), "a8");
        assert_eq!(v("d9"), "đ");
        assert_eq!(v("d99"), "d9");
        assert_eq!(v("vie65t"), "việt");
    }

    #[test]
    fn compound_words() {
        assert_eq!(v("du7o7ng2"), "dường");
        assert_eq!(v("duong72"), "dường");
        assert_eq!(v("d9uo7ng2"), "đường");
        assert_eq!(v("nguye64n"), "nguyễn");
    }

    #[test]
    fn plain_numbers_untouched() {
        assert_eq!(v("2026"), "2026");
        assert_eq!(v("x1"), "x1");
    }

    #[test]
    fn issue4_tone_before_mark_full_engine() {
        use crate::engine::testutil::type_str;
        use crate::engine::{Engine, EngineConfig, SpellMode, TypingMethod};
        let mk = || {
            Engine::new(EngineConfig {
                method: TypingMethod::Vni,
                spell_mode: SpellMode::Strict,
                modern_tone: false,
                macros_enabled: false,
                flexible_marks: true,
                censor_enabled: false,
            })
        };
        assert_eq!(type_str(&mut mk(), "thie16u"), "thiếu");
        assert_eq!(type_str(&mut mk(), "thieu61"), "thiếu"); // thứ tự cũ vẫn đúng
        assert_eq!(type_str(&mut mk(), "thie61t"), "thiết");
    }
}
