//! Luật gõ Telex: s f r x j → thanh; z → xóa thanh; aa ee oo → mũ;
//! w → móc/trăng (đứng một mình → ư); dd → đ. Gõ lặp modifier để hủy —
//! sau khi hủy, phím đó thành chữ thường cho tới hết từ.

use super::spell;
use super::syllable::vowel_indices;
use super::{Letter, Tone, WordState};

pub fn apply_key(state: &mut WordState, c: char, flexible_marks: bool) {
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
            state.apply_tone_key(tone, c);
        }
        'z' => state.apply_tone_clear(c),
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
            // Gõ mũ muộn: phím lặp sau phụ âm cuối vẫn tìm về nguyên âm
            // cùng loại (nanag → nâng, viete → viêt).
            if flexible_marks {
                let vidx = vowel_indices(&state.letters);
                if let Some(&i) = vidx
                    .iter()
                    .rev()
                    .find(|&&i| state.letters[i].base == lower)
                {
                    if state.letters[i].circ {
                        // Hủy muộn, đối xứng với hủy liền kề.
                        state.letters[i].circ = false;
                        state.dead.push(lower);
                        state.letters.push(Letter::plain(c));
                        return;
                    }
                    if !state.letters[i].has_mark() {
                        // Chỉ giữ nếu ra âm tiết hợp lệ — tránh biến từ
                        // tiếng Anh (banana) thành tiếng Việt nửa mùa.
                        state.letters[i].circ = true;
                        // Luôn kiểm strict (false) kể cả ở chế độ loose: mũ
                        // muộn chỉ nên áp khi ra âm tiết TV hợp lệ hẳn.
                        if spell::is_acceptable(state, false) {
                            return;
                        }
                        state.letters[i].circ = false;
                    }
                }
            }
            state.letters.push(Letter::plain(c));
        }
        'w' => {
            // Từ bắt đầu bằng w thường là Latin (web, Windows...), nên giữ
            // cả chuỗi w nguyên văn thay vì diễn giải w thứ hai là tạo ư.
            // Người dùng vẫn gõ ư đầu từ bằng `uw`.
            if state.letters.first().is_none_or(|first| {
                first.base == 'w' && !first.w_origin
            }) {
                state.letters.push(Letter::plain(c));
                return;
            }
            let n = state.letters.len();
            // Hủy cặp ươ → uo: quét cặp chữ u(móc)+o(móc) liền kề bất kỳ, kể
            // cả khi còn nguyên âm cuối theo sau (đối xứng chiều áp dụng
            // ươi/ươu). Trước đây chỉ xét hai chữ cuối nên bấm w để hủy ươ
            // giữa cụm ("cười") sinh ra ký tự ư thừa.
            if let Some(k) = (0..n.saturating_sub(1)).find(|&k| {
                state.letters[k].base == 'u'
                    && state.letters[k].horn
                    && state.letters[k + 1].base == 'o'
                    && state.letters[k + 1].horn
            }) {
                state.letters[k].horn = false;
                state.letters[k + 1].horn = false;
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
            // Áp dụng: cặp uo liền kề bất kỳ trong cụm nguyên âm → ươ. Quét
            // cả cụm (không chỉ hai nguyên âm cuối) để ươi/ươu — "người",
            // "cười", "rượu" — móc được cả cặp dù còn nguyên âm cuối theo sau.
            let vidx = vowel_indices(&state.letters);
            for k in 0..vidx.len().saturating_sub(1) {
                let (i, j) = (vidx[k], vidx[k + 1]);
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
            // Gạch ngang muộn: phím d sau khi đã có nguyên âm vẫn tìm về chữ
            // d đầu từ (did → đi), song song mũ muộn. Chỉ áp khi ra âm tiết TV
            // hợp lệ để không biến từ tiếng Anh (dryad giữ nguyên).
            if flexible_marks
                && state.letters.first().is_some_and(|l| l.base == 'd' && !l.stroke)
            {
                state.letters[0].stroke = true;
                if spell::is_acceptable(state, false) {
                    return;
                }
                state.letters[0].stroke = false;
            }
            state.letters.push(Letter::plain(c));
        }
        _ => state.letters.push(Letter::plain(c)),
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::testutil::type_str;
    use crate::engine::{Engine, EngineConfig, SpellMode, TypingMethod};

    fn engine(modern: bool) -> Engine {
        Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Standard,
            modern_tone: modern,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        })
    }

    fn t(keys: &str) -> String {
        crate::engine::testutil::raw_render(TypingMethod::Telex, keys, false, true)
    }

    fn t_modern(keys: &str) -> String {
        crate::engine::testutil::raw_render(TypingMethod::Telex, keys, true, true)
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
        assert_eq!(t("w"), "w");
        assert_eq!(t("W"), "W");
        assert_eq!(t("tw"), "tư");
        assert_eq!(t("aww"), "aw");
        assert_eq!(t("ww"), "ww");
        assert_eq!(t("www"), "www");
        assert_eq!(t("duongw"), "dương");
        assert_eq!(t("dduongwf"), "đường");
        assert_eq!(t("uwowng"), "ương");
        assert_eq!(t("khoawn"), "khoăn");
        assert_eq!(t("quow"), "quơ"); // u sau q không nhận móc
    }

    #[test]
    fn uo_horn_with_final_vowel() {
        // Cặp uo + nguyên âm cuối (ươi/ươu): một chữ w móc cả cặp, kể cả khi
        // uo không nằm ở hai nguyên âm cuối của từ.
        assert_eq!(t("nguoiwf"), "người");
        assert_eq!(t("cuoiwf"), "cười");
        assert_eq!(t("tuoiw"), "tươi");
        assert_eq!(t("ruouwj"), "rượu");
        // Không hồi quy: cặp uo ở cuối vẫn chạy.
        assert_eq!(t("duongw"), "dương");
        assert_eq!(t("uwowng"), "ương");
        // Bất biến: u sau q không thuộc cụm; uô (đã mang mũ) không phải cặp uo.
        assert_eq!(t("quow"), "quơ");
        assert_eq!(t("buonw"), "bươn");
    }

    #[test]
    fn uo_horn_cancel_with_final_vowel() {
        // Đối xứng chiều hủy: bấm w sau ươ giữa cụm gỡ móc cả hai, không rác.
        assert_eq!(t("cuoiww"), "cuoiw");
        assert_eq!(t("nguoiww"), "nguoiw");
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
    fn late_stroke_d() {
        // Gạch ngang muộn: phím d sau nguyên âm vẫn tìm về chữ d đầu từ
        // (did → đi), song song với mũ muộn (nanag → nâng). Cách cũ dd vẫn chạy.
        assert_eq!(t("did"), "đi");
        assert_eq!(t("ddi"), "đi");
        assert_eq!(t("dangd"), "đang");
        assert_eq!(t("dungwd"), "đưng");
        // Rào chắn: chỉ áp khi ra âm tiết TV hợp lệ — từ tiếng Anh giữ nguyên.
        assert_eq!(t("dryad"), "dryad");
    }

    #[test]
    fn late_stroke_d_respects_flexible_marks() {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Standard,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: false,
            censor_enabled: false,
        });
        // Tắt "dấu linh hoạt" → gạch muộn không áp; dd liền kề vẫn được.
        assert_eq!(type_str(&mut e, "did"), "did");
        e.reset();
        assert_eq!(type_str(&mut e, "ddi"), "đi");
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
    fn late_circumflex() {
        // Gõ mũ muộn: phím a/e/o lặp lại sau phụ âm cuối vẫn áp vào
        // nguyên âm cùng loại trong từ.
        assert_eq!(t("nanag"), "nâng");
        assert_eq!(t("nangas"), "nấng");
        assert_eq!(t("viete"), "viêt");
        assert_eq!(t("vietej"), "việt");
        // Rào chắn: chỉ áp khi ra âm tiết hợp lệ — "oâ" không hợp lệ.
        assert_eq!(t("hoana"), "hoana");
    }

    #[test]
    fn late_circumflex_respects_spell_mode() {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        // bân + n không hợp lệ → về raw mode → chữ tiếng Anh nguyên vẹn.
        assert_eq!(type_str(&mut e, "banana"), "banana");
    }

    #[test]
    fn flexible_marks_off_disables_late_circumflex() {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Standard,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: false,
            censor_enabled: false,
        });
        assert_eq!(type_str(&mut e, "nanag"), "nanag");
        e.reset(); // hai từ riêng biệt — ngắt từ (như gõ dấu cách thật).
        // Không reset thì raw gộp "nanagnaang" (nguyên âm không liền) → loose
        // khôi phục raw, đúng hành vi; test cũ chỉ đúng nhờ đặt-dấu-mù.
        // Dạng liền kề cổ điển vẫn hoạt động.
        assert_eq!(type_str(&mut e, "naang"), "nâng");
    }

    fn t_spell(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        type_str(&mut e, keys)
    }

    #[test]
    fn cancel_then_tone_key_no_phantom() {
        // Bug thực địa: gõ "má" (mas) → hủy sắc bằng s (mas) → thêm f. Phím
        // f làm "màs" không hợp lệ; KHÔNG được bung raw để hiện lại chữ s
        // đã hủy ("massf"). Chỉ rơi f xuống ký tự thường → "masf".
        assert_eq!(t_spell("massf"), "masf");
        assert_eq!(t_spell("assf"), "asf");
        // Backspace sau đó phải khớp đúng phần hiển thị (raw đã đồng bộ).
        assert_eq!(t_spell("massf\u{8}"), "mas");
    }

    #[test]
    fn english_restore_still_reverts_active_marks() {
        // Rào chắn ngược: khôi phục từ tiếng Anh phải hoàn tác dấu ĐANG hoạt
        // động (không dính nhầm nhánh "sạch đã chốt").
        assert_eq!(t_spell("asdf"), "asdf");
        assert_eq!(t_spell("dds"), "dds"); // đ + s vô lệ → bung raw như cũ
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
