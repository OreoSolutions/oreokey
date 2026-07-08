//! Luật gõ Telex: s f r x j → thanh; z → xóa thanh; aa ee oo → mũ;
//! w → móc/trăng (đứng một mình → ư); dd → đ. Gõ lặp modifier để hủy —
//! sau khi hủy, phím đó thành chữ thường cho tới hết từ.

use super::syllable::vowel_indices;
use super::{Letter, Tone, WordState};

pub fn apply_key(state: &mut WordState, c: char) {
    let lower = c.to_ascii_lowercase();
    if state.is_dead(lower) {
        state.letters.push(Letter::plain(c));
        return;
    }
    match lower {
        's' | 'f' | 'r' | 'x' | 'j' => {
            let tone = match lower {
                's' => Tone::Acute,
                'f' => Tone::Grave,
                'r' => Tone::Hook,
                'x' => Tone::Tilde,
                _ => Tone::Dot,
            };
            if !state.has_vowel() {
                state.letters.push(Letter::plain(c));
            } else if state.tone == Some(tone) {
                // Gõ lặp phím thanh → hủy, trả phím về dạng chữ.
                state.tone = None;
                state.dead.push(lower);
                state.letters.push(Letter::plain(c));
            } else {
                state.tone = Some(tone);
            }
        }
        'z' => {
            if state.tone.is_some() {
                state.tone = None;
            } else {
                state.letters.push(Letter::plain(c));
            }
        }
        'a' | 'e' | 'o' => {
            if let Some(last) = state.letters.last_mut() {
                if last.base == lower && last.circ {
                    // ââ → hủy mũ (aaa → aa).
                    last.circ = false;
                    state.dead.push(lower);
                    state.letters.push(Letter::plain(c));
                    return;
                }
                if last.base == lower && !last.has_mark() {
                    // aa → â, ee → ê, oo → ô.
                    last.circ = true;
                    return;
                }
            }
            state.letters.push(Letter::plain(c));
        }
        'w' => {
            let n = state.letters.len();
            // Hủy cặp ươ → uo (uoww hoặc sau khi w áp lên cả cặp).
            if n >= 2
                && state.letters[n - 2].base == 'u'
                && state.letters[n - 2].horn
                && state.letters[n - 1].base == 'o'
                && state.letters[n - 1].horn
            {
                state.letters[n - 2].horn = false;
                state.letters[n - 1].horn = false;
                state.dead.push('w');
                state.letters.push(Letter::plain(c));
                return;
            }
            // Hủy đơn: chữ cuối đang mang móc/trăng.
            if n >= 1 && (state.letters[n - 1].horn || state.letters[n - 1].breve) {
                if state.letters[n - 1].w_origin {
                    // ư sinh từ w → hoàn về đúng phím w, không thêm chữ.
                    state.letters[n - 1].base = 'w';
                    state.letters[n - 1].horn = false;
                    state.letters[n - 1].w_origin = false;
                    state.dead.push('w');
                    return;
                }
                state.letters[n - 1].horn = false;
                state.letters[n - 1].breve = false;
                state.dead.push('w');
                state.letters.push(Letter::plain(c));
                return;
            }
            // Áp dụng: cặp uo cuối cùng → ươ.
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
            // Nguyên âm áp dụng được gần cuối nhất: a → ă, o → ơ, u → ư.
            if let Some(&i) = vidx
                .iter()
                .rev()
                .find(|&&i| {
                    matches!(state.letters[i].base, 'a' | 'o' | 'u')
                        && !state.letters[i].has_mark()
                        && !state.letters[i].circ
                })
            {
                if state.letters[i].base == 'a' {
                    state.letters[i].breve = true;
                } else {
                    state.letters[i].horn = true;
                }
                return;
            }
            // w đứng một mình / sau phụ âm → ư.
            let mut l = Letter::plain(c);
            l.base = 'u';
            l.horn = true;
            l.w_origin = true;
            state.letters.push(l);
        }
        'd' => {
            if let Some(last) = state.letters.last_mut() {
                if last.base == 'd' {
                    if last.stroke {
                        // đd → hủy (ddd → dd).
                        last.stroke = false;
                        state.dead.push('d');
                        state.letters.push(Letter::plain(c));
                    } else {
                        last.stroke = true;
                    }
                    return;
                }
            }
            state.letters.push(Letter::plain(c));
        }
        _ => state.letters.push(Letter::plain(c)),
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::testutil::type_str;
    use crate::engine::{Engine, EngineConfig, TypingMethod};

    fn engine(modern: bool) -> Engine {
        Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_check: false,
            modern_tone: modern,
            macros_enabled: false,
        })
    }

    fn t(keys: &str) -> String {
        type_str(&mut engine(false), keys)
    }

    fn t_modern(keys: &str) -> String {
        type_str(&mut engine(true), keys)
    }

    #[test]
    fn tones() {
        assert_eq!(t("as"), "á");
        assert_eq!(t("af"), "à");
        assert_eq!(t("ar"), "ả");
        assert_eq!(t("ax"), "ã");
        assert_eq!(t("aj"), "ạ");
        assert_eq!(t("vietj"), "viẹt"); // không gõ ee thì không có mũ
        assert_eq!(t("vieetj"), "việt");
        assert_eq!(t("viejt"), "viẹt"); // thanh dời theo khi cụm từ mở rộng
        assert_eq!(t("toans"), "toán");
        assert_eq!(t("hoif"), "hòi");
        assert_eq!(t("muaf"), "mùa");
        assert_eq!(t("ngoafi"), "ngoài"); // dấu tự dời khi cụm nguyên âm mở rộng
        assert_eq!(t("nguyeenx"), "nguyễn");
    }

    #[test]
    fn tone_cancel_and_dead_key() {
        assert_eq!(t("ass"), "as");
        assert_eq!(t("asss"), "ass"); // sau hủy, s là chữ thường
        assert_eq!(t("asf"), "à"); // thanh mới thay thanh cũ
        assert_eq!(t("classs"), "class");
    }

    #[test]
    fn remove_tone_z() {
        assert_eq!(t("asz"), "a");
        assert_eq!(t("az"), "az"); // không có thanh → z là chữ
    }

    #[test]
    fn circumflex() {
        assert_eq!(t("aa"), "â");
        assert_eq!(t("aas"), "ấ");
        assert_eq!(t("aaa"), "aa");
        assert_eq!(t("ee"), "ê");
        assert_eq!(t("oo"), "ô");
        assert_eq!(t("xooong"), "xoong"); // ooo hủy mũ để gõ xoong
        assert_eq!(t("ddoongf"), "đồng");
    }

    #[test]
    fn horn_breve_w() {
        assert_eq!(t("aw"), "ă");
        assert_eq!(t("ow"), "ơ");
        assert_eq!(t("uw"), "ư");
        assert_eq!(t("w"), "ư");
        assert_eq!(t("tw"), "tư");
        assert_eq!(t("aww"), "aw");
        assert_eq!(t("ww"), "w"); // ư hoàn về w
        assert_eq!(t("www"), "ww"); // sau hủy, w chết
        assert_eq!(t("duongw"), "dương");
        assert_eq!(t("dduongwf"), "đường");
        assert_eq!(t("uwowng"), "ương");
        assert_eq!(t("khoawn"), "khoăn");
        assert_eq!(t("quow"), "quơ"); // u sau q không nhận móc
    }

    #[test]
    fn word_do_actions_carry_old_text() {
        // Bug thực địa "đó → óo": engine phải khai báo đúng đoạn bị thay
        // (old) để tầng AX xác minh trước khi ghi đè.
        use crate::engine::{Action, KeyInput};
        let mut e = engine(false);
        e.on_key(KeyInput::Char('d'));
        assert_eq!(
            e.on_key(KeyInput::Char('d')),
            Action::Replace { old: "d".into(), text: "đ".into() }
        );
        e.on_key(KeyInput::Char('o'));
        assert_eq!(
            e.on_key(KeyInput::Char('s')),
            Action::Replace { old: "o".into(), text: "ó".into() }
        );
        assert_eq!(e.current_word(), "đó");
    }

    #[test]
    fn d_stroke() {
        assert_eq!(t("dd"), "đ");
        assert_eq!(t("ddd"), "dd");
        assert_eq!(t("dddd"), "ddd");
        assert_eq!(t("ddi"), "đi");
    }

    #[test]
    fn tone_placement_styles() {
        assert_eq!(t("hoaf"), "hòa");
        assert_eq!(t_modern("hoaf"), "hoà");
        assert_eq!(t("thuys"), "thúy");
        assert_eq!(t_modern("thuys"), "thuý");
        assert_eq!(t("khoer"), "khỏe");
        assert_eq!(t_modern("khoer"), "khoẻ");
        // qu/gi: bán nguyên âm thuộc phụ âm đầu
        assert_eq!(t("quas"), "quá");
        assert_eq!(t("quys"), "quý");
        assert_eq!(t("gias"), "giá");
        assert_eq!(t("gif"), "gì");
        assert_eq!(t("giwowngf"), "giường");
        // 3 nguyên âm mở → dấu giữa
        assert_eq!(t("khuyru"), "khuỷu");
    }

    #[test]
    fn case_preserved() {
        assert_eq!(t("VIEETJ"), "VIỆT");
        assert_eq!(t("Vieetj"), "Việt");
        assert_eq!(t("DDaij"), "Đại");
    }

    #[test]
    fn non_vietnamese_untouched() {
        assert_eq!(t("viet"), "viet");
        assert_eq!(t("2026"), "2026");
        assert_eq!(t("ang"), "ang");
    }
}
