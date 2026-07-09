//! Kiểm tra âm tiết tiếng Việt hợp lệ (phonotactics) để tự khôi phục
//! phím gốc khi người dùng gõ từ nước ngoài mà quên tắt tiếng Việt.
//!
//! Cho phép cả các trạng thái trung gian khi đang gõ dở (vd `duòng`
//! trước khi bấm `w` thành `dường`) — thà bỏ sót còn hơn phá từ đúng.

use super::syllable::{marked_lower, vowel_indices};
use super::WordState;
use super::Tone;

const INITIALS: &[&str] = &[
    "", "b", "c", "ch", "d", "đ", "g", "gh", "gi", "h", "k", "kh", "l", "m",
    "n", "ng", "ngh", "nh", "p", "ph", "qu", "r", "s", "t", "th", "tr", "v",
    "x",
];

const NUCLEI: &[&str] = &[
    // 1 nguyên âm
    "a", "ă", "â", "e", "ê", "i", "o", "ô", "ơ", "u", "ư", "y",
    // 2 nguyên âm
    "ai", "ao", "au", "ay", "âu", "ây", "eo", "êu", "ia", "iê", "iu", "oa",
    "oă", "oe", "oi", "ôi", "ơi", "oo", "ua", "uâ", "uê", "ui", "uô", "uơ",
    "uy", "ưa", "ưi", "ươ", "ưu", "yê",
    // 3 nguyên âm
    "iêu", "yêu", "oai", "oao", "oay", "oeo", "uây", "uôi", "ươi", "ươu",
    "uya", "uyê", "uyu",
    // Trạng thái trung gian phổ biến khi gõ dở (chờ w/7)
    "uo", "ưo",
];

const FINALS: &[&str] = &["", "c", "ch", "m", "n", "ng", "nh", "p", "t"];

/// Nguyên âm có dấu → dạng trơn: â ă → a, ê → e...
fn base_of(c: char) -> char {
    match c {
        'ă' | 'â' => 'a',
        'ê' => 'e',
        'ô' | 'ơ' => 'o',
        'ư' => 'u',
        other => other,
    }
}

/// `cluster` có thể trở thành vần hợp lệ `valid` chỉ bằng cách THÊM dấu
/// vào các nguyên âm còn trơn không? Ký tự đã mang dấu phải khớp chính
/// xác ("ie"→"iê" được; "oâ" không thể thành "oă").
fn can_become(cluster: &str, valid: &str) -> bool {
    if cluster.chars().count() != valid.chars().count() {
        return false;
    }
    cluster
        .chars()
        .zip(valid.chars())
        .all(|(c, v)| c == v || (c == base_of(c) && base_of(v) == c))
}

/// Từ có bị engine biến đổi không (có thanh hoặc dấu phụ). Chỉ những từ
/// bị biến đổi mới cần khôi phục — kết quả của việc hủy dấu (`ass`→`as`)
/// không được tính.
pub fn is_transformed(state: &WordState) -> bool {
    state.tone.is_some() || state.letters.iter().any(|l| l.has_mark() || l.stroke)
}

/// Âm tiết chấp nhận được (hợp lệ hoặc là tiền tố hợp lý của từ đang gõ).
/// `loose == true`: chế độ "gõ thoải mái" — thả kiểm tra phụ âm cuối và
/// luật thanh-trên-phụ-âm-tắc, nới trường hợp không nguyên âm cho từ có
/// `đ`. Vẫn giữ mọi phép chặn cụm bất khả (phụ âm đầu, cụm nguyên âm,
/// nguyên âm liên tục).
pub fn is_acceptable(state: &WordState, loose: bool) -> bool {
    let letters = &state.letters;
    if letters.iter().any(|l| !l.base.is_ascii_alphabetic()) {
        return false;
    }
    // đ chỉ đứng đầu từ.
    if letters.iter().skip(1).any(|l| l.stroke) {
        return false;
    }
    let vidx = vowel_indices(letters);
    let Some(&run_start) = vidx.first() else {
        // Không có nguyên âm.
        if loose {
            // Từ viết tắt kiểu đc/đk/đt: chấp nhận nếu chữ đầu mang gạch.
            return letters.first().is_some_and(|l| l.stroke);
        }
        // Strict: chỉ chấp nhận mỗi "đ" trơ (đang gõ dở "đi"...).
        return letters.len() == 1 && letters[0].stroke;
    };
    // Cụm nguyên âm phải liên tục; nguyên âm sau phụ âm cuối → không hợp lệ.
    let mut run_end = run_start;
    for &i in &vidx[1..] {
        if i == run_end + 1 {
            run_end = i;
        } else {
            return false;
        }
    }
    let render_range =
        |a: usize, b: usize| letters[a..b].iter().map(marked_lower).collect::<String>();

    let initial = render_range(0, run_start);
    if !INITIALS.contains(&initial.as_str()) {
        return false;
    }
    let nucleus = render_range(run_start, run_end + 1);
    let nucleus_ok = NUCLEI.contains(&nucleus.as_str())
        || (state.tone.is_none() && NUCLEI.iter().any(|n| can_become(&nucleus, n)));
    if !nucleus_ok {
        return false;
    }
    // Loose: thả tự do phụ âm cuối và bỏ luật thanh trên phụ âm tắc.
    if loose {
        return true;
    }
    let final_c = render_range(run_end + 1, letters.len());
    if !FINALS.contains(&final_c.as_str()) {
        return false;
    }
    // Âm tiết đóng bằng phụ âm tắc (p t c ch) chỉ mang thanh sắc/nặng.
    let stop_final = matches!(final_c.as_str(), "c" | "ch" | "p" | "t");
    if stop_final
        && matches!(
            state.tone,
            Some(Tone::Grave) | Some(Tone::Hook) | Some(Tone::Tilde)
        )
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::engine::testutil::type_str;
    use crate::engine::{Engine, EngineConfig, TypingMethod};

    fn t(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_check: true,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        type_str(&mut e, keys)
    }

    #[test]
    fn english_words_restored() {
        assert_eq!(t("mask"), "mask"); // má + k → phụ âm cuối k không hợp lệ
        assert_eq!(t("for"), "for"); // fỏ → phụ âm đầu f không hợp lệ
        assert_eq!(t("case"), "case"); // cáe → cụm nguyên âm ae không hợp lệ
        assert_eq!(t("mart"), "mart"); // mảt → hỏi + phụ âm tắc không hợp lệ
        assert_eq!(t("expression"), "expression");
        assert_eq!(t("windows"), "windows");
    }

    #[test]
    fn vietnamese_words_kept() {
        assert_eq!(t("vieetj"), "việt");
        assert_eq!(t("nguyeenx"), "nguyễn");
        assert_eq!(t("toans"), "toán");
        assert_eq!(t("muaf"), "mùa");
        assert_eq!(t("dduongwf"), "đường");
        assert_eq!(t("giwowngf"), "giường");
        assert_eq!(t("khuyru"), "khuỷu");
        assert_eq!(t("quaats"), "quất");
        assert_eq!(t("nghieng"), "nghieng");
        assert_eq!(t("nghieengs"), "nghiếng");
    }

    #[test]
    fn d_words_with_incomplete_nucleus_not_broken() {
        // Bug thực địa: "ddieen" bị khôi phục thành raw vì cụm trung
        // gian "ie" (chưa thành "iê") không có trong bảng vần.
        assert_eq!(t("ddieen"), "điên");
        assert_eq!(t("ddieenj"), "điện");
        assert_eq!(t("dduee"), "đuê");
        assert_eq!(t("ddieeuf"), "điều");
        assert_eq!(t("dduowngf"), "đường");
    }

    #[test]
    fn tone_on_incomplete_nucleus_still_restores_english() {
        // Cụm gốc chỉ được nới lỏng khi CHƯA có thanh — "dies" có thanh
        // sắc từ s nên vẫn phải trả về nguyên văn.
        assert_eq!(t("dies"), "dies");
        assert_eq!(t("lies"), "lies");
        assert_eq!(t("ties"), "ties");
    }

    #[test]
    fn intermediate_states_not_broken() {
        // duongf → duòng (chờ w), thêm w thành dường — không được khôi phục giữa chừng
        assert_eq!(t("duongf"), "duòng");
        assert_eq!(t("duongfw"), "dường");
        assert_eq!(t("dd"), "đ");
        assert_eq!(t("ddi"), "đi");
    }

    #[test]
    fn backspacing_out_of_raw_mode_reenables_typing() {
        // Bug thực địa: gõ sai → từ bị khóa raw; xóa đi gõ lại ngay
        // vẫn không ăn dấu, phải bấm space mới gõ được. Xóa về trạng
        // thái sạch phải tự gỡ khóa.
        assert_eq!(t("mart\u{8}\u{8}\u{8}\u{8}vieetj"), "việt");
        // Xóa một phần cũng đủ nếu phần còn lại render khớp màn hình.
        assert_eq!(t("mask\u{8}\u{8}s"), "má");
        // Chưa xóa về trạng thái sạch thì vẫn giữ nguyên văn.
        assert_eq!(t("mask\u{8}k"), "mask");
    }

    #[test]
    fn cancelled_keys_not_restored() {
        // Hủy dấu là chủ ý của người dùng, không phải từ sai.
        assert_eq!(t("ass"), "as");
        assert_eq!(t("xooong"), "xoong");
        // "cl" không phải phụ âm đầu tiếng Việt → vào raw mode ngay,
        // mọi phím sau hiện nguyên văn.
        assert_eq!(t("class"), "class");
    }

    #[test]
    fn spell_off_transforms_anyway() {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_check: false,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        assert_eq!(type_str(&mut e, "mask"), "mák");
    }

    fn loose(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_check: false, // false = chế độ gõ thoải mái (loose)
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        type_str(&mut e, keys)
    }

    #[test]
    fn loose_allows_abbreviations() {
        // Từ viết tắt có dấu, phụ âm cuối/không nguyên âm → được giữ.
        assert_eq!(loose("ddc"), "đc"); // đ + c, không nguyên âm
        assert_eq!(loose("nefk"), "nèk"); // đuôi k không hợp lệ vẫn cho
    }

    #[test]
    fn loose_still_restores_english() {
        // Cụm bất khả (phụ âm đầu / nguyên âm / nguyên âm không liền) vẫn bắt.
        assert_eq!(loose("clear"), "clear"); // cl đầu bất khả
        assert_eq!(loose("sound"), "sound"); // ou bất khả
        assert_eq!(loose("for"), "for"); // f đầu bất khả
        assert_eq!(loose("class"), "class");
        assert_eq!(loose("dies"), "dies"); // ie + thanh → bất khả
        assert_eq!(loose("status"), "status"); // a…u không liên tục
    }

    #[test]
    fn loose_keeps_valid_vietnamese() {
        assert_eq!(loose("vieetj"), "việt");
        assert_eq!(loose("dduongwf"), "đường");
        assert_eq!(loose("toans"), "toán");
    }

    #[test]
    fn loose_transforms_ambiguous_english_by_design() {
        // Đánh đổi đã chấp nhận: cùng cấu trúc với nèk nên bị đặt dấu.
        assert_eq!(loose("mask"), "mák");
        assert_eq!(loose("task"), "ták");
    }
}
