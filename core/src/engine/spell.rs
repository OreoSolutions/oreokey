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

/// `cluster` có phải TIỀN TỐ của `valid` không (cho phép nguyên âm trơn
/// khớp bản có dấu: "ie" là tiền tố của "iê", "iêu"). Khác `can_become`
/// ở chỗ không đòi cùng độ dài.
fn can_become_prefix(cluster: &str, valid: &str) -> bool {
    let cc: Vec<char> = cluster.chars().collect();
    let vc: Vec<char> = valid.chars().collect();
    if cc.len() > vc.len() {
        return false;
    }
    cc.iter()
        .zip(vc.iter())
        .all(|(&c, &v)| c == v || (c == base_of(c) && base_of(v) == c))
}

/// Cụm hiện tại còn có thể trở thành âm tiết hợp lệ bằng cách gõ THÊM
/// dấu/chữ không (tiền tố hợp lệ)? Dùng để KHÔNG khóa `raw_mode` quá sớm:
/// trạng thái "còn sống" khác "chết hẳn" (cụm bất khả như `cl`, nguyên âm
/// rời). Cho phép nhân âm dở kể cả khi ĐÃ có thanh (khác `is_acceptable`).
pub fn is_live_prefix(state: &WordState) -> bool {
    let letters = &state.letters;
    if letters.iter().any(|l| !l.base.is_ascii_alphabetic()) {
        return false;
    }
    if letters.iter().skip(1).any(|l| l.stroke) {
        return false;
    }
    let vidx = vowel_indices(letters);
    let Some(&run_start) = vidx.first() else {
        // Chưa có nguyên âm: phụ âm đầu phải là tiền tố của một INITIAL.
        let initial = letters.iter().map(marked_lower).collect::<String>();
        return INITIALS.iter().any(|i| i.starts_with(&initial));
    };
    // Cụm nguyên âm phải liên tục.
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
    // Đã có nguyên âm → phụ âm đầu phải khớp HẲN một INITIAL.
    let initial = render_range(0, run_start);
    if !INITIALS.contains(&initial.as_str()) {
        return false;
    }
    let nucleus = render_range(run_start, run_end + 1);
    if !NUCLEI.iter().any(|n| can_become_prefix(&nucleus, n)) {
        return false;
    }
    // Phụ âm cuối đang gõ dở phải là tiền tố của một FINAL.
    let final_c = render_range(run_end + 1, letters.len());
    FINALS.iter().any(|f| f.starts_with(&final_c))
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
    let final_c = render_range(run_end + 1, letters.len());
    // "oo" chỉ có trong từ mượn đóng bằng c/ng (xoong, boong, soóc) —
    // không bao giờ kết thúc âm tiết, nên "lóo" bất khả ở cả hai mức.
    if nucleus == "oo" && !matches!(final_c.as_str(), "c" | "ng") {
        return false;
    }
    // Loose: thả tự do phụ âm cuối và bỏ luật thanh trên phụ âm tắc.
    if loose {
        return true;
    }
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
    use crate::engine::{Engine, EngineConfig, SpellMode, TypingMethod};

    fn t(keys: &str) -> String {
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
    fn oo_nucleus_requires_c_or_ng_final() {
        // "oo" chỉ có trong từ mượn đóng bằng c/ng (xoong, boong, soóc,
        // goòng) — không bao giờ kết thúc âm tiết, nên "lóo" là bất khả.
        // Phím o thứ ba đã tiêu vào việc hủy mũ nên nguyên văn là "loos"
        // (cùng thiết kế với `ass` → `as`).
        assert_eq!(t("looos"), "loos");
        assert_eq!(standard("looos"), "loos");
        // Từ mượn hợp lệ vẫn gõ được, kể cả đặt thanh trước phụ âm cuối.
        assert_eq!(t("sooocs"), "soóc");
        assert_eq!(t("sooosc"), "soóc");
        assert_eq!(t("gooongf"), "goòng");
        // Backspace sau khôi phục còn-sống: raw giữ lịch sử phím thật
        // ("sooos"), xóa một ký tự phải về đúng "soo".
        assert_eq!(t("sooos\u{8}"), "soo");
        // Từ chết MUỘN (sau khi đã hủy mũ) cũng không được bung lại phím
        // hủy: người dùng thấy "loos" rồi gõ e thì phải ra "loose",
        // không phải "looose".
        assert_eq!(t("looose"), "loose");
        assert_eq!(standard("looose"), "loose");
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

    fn standard(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Standard,
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
        assert_eq!(standard("ddc"), "đc"); // đ + c, không nguyên âm
        assert_eq!(standard("nefk"), "nèk"); // đuôi k không hợp lệ vẫn cho
    }

    #[test]
    fn loose_still_restores_english() {
        // Cụm bất khả (phụ âm đầu / nguyên âm / nguyên âm không liền) vẫn bắt.
        assert_eq!(standard("clear"), "clear"); // cl đầu bất khả
        assert_eq!(standard("sound"), "sound"); // ou bất khả
        assert_eq!(standard("for"), "for"); // f đầu bất khả
        assert_eq!(standard("class"), "class");
        assert_eq!(standard("dies"), "dies"); // ie + thanh → bất khả
        assert_eq!(standard("status"), "status"); // a…u không liên tục
    }

    #[test]
    fn loose_keeps_valid_vietnamese() {
        assert_eq!(standard("vieetj"), "việt");
        assert_eq!(standard("dduongwf"), "đường");
        assert_eq!(standard("toans"), "toán");
    }

    #[test]
    fn loose_transforms_ambiguous_english_by_design() {
        // Đánh đổi đã chấp nhận: cùng cấu trúc với nèk nên bị đặt dấu.
        assert_eq!(standard("mask"), "mák");
        assert_eq!(standard("task"), "ták");
    }

    fn standard_vni(keys: &str) -> String {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Vni,
            spell_mode: SpellMode::Standard,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        type_str(&mut e, keys)
    }

    #[test]
    fn loose_applies_to_vni() {
        // Bộ lọc loose chạy trên WordState nên áp cho cả VNI (đ tạo bằng d9).
        assert_eq!(standard_vni("d9c"), "đc"); // đ + c, không nguyên âm
        assert_eq!(standard_vni("vie65t"), "việt"); // âm tiết hợp lệ vẫn giữ
    }

    #[test]
    fn tone_right_after_qu_onset_stays_live() {
        // Bug sweep toàn từ điển: thanh gõ NGAY sau "qu" (trước nguyên âm
        // chính) khiến 'u' bị tính là nhân âm ở trạng thái gõ dở, initial
        // suy ra còn "q" trơ (không có trong INITIALS) → is_live_prefix
        // false → raw_mode khóa vĩnh viễn, gõ đủ vần cũng không hồi phục.
        assert_eq!(standard_vni("qu1an"), "quán");
        assert_eq!(standard_vni("qu2ang"), "quàng");
        assert_eq!(standard_vni("qu1a6y"), "quấy");
        assert_eq!(standard("qusan"), "quán");
        assert_eq!(standard("qufang"), "quàng");
        assert_eq!(standard("qusaya"), "quấy");
        // Strict cũng phải sống — cùng đường is_live_prefix.
        assert_eq!(t("qusan"), "quán");
        // Thứ tự bình thường không được hồi quy.
        assert_eq!(standard("quans"), "quán");
        assert_eq!(standard_vni("quan1"), "quán");
    }

    #[test]
    fn tone_right_after_gi_onset_still_works() {
        // Đối chứng "gi": chữ i sau g CÓ THỂ là nhân âm thật ("gì") nên
        // không được áp cùng cách sửa như "qu"; hành vi hiện tại đúng.
        assert_eq!(standard("gifa"), "già");
        assert_eq!(standard_vni("gi2a"), "già");
        assert_eq!(standard("gif"), "gì");
    }

    #[test]
    fn live_prefix_recognizes_incomplete_toned_nucleus() {
        use crate::engine::vni;
        use crate::engine::WordState;
        let build = |keys: &str| {
            let mut s = WordState::default();
            for c in keys.chars() {
                vni::apply_key(&mut s, c);
            }
            s
        };
        // "thie1" (thié): nhân âm "ie" + thanh — CÒN SỐNG (chờ mũ 6).
        assert!(super::is_live_prefix(&build("thie1")));
        // "die" + s tương đương telex là live, nhưng dead-cluster thì không:
        // "cla" (cl không phải phụ âm đầu tiếng Việt) → CHẾT.
        let cla = {
            let mut s = WordState::default();
            for c in "cla".chars() { vni::apply_key(&mut s, c); }
            s
        };
        assert!(!super::is_live_prefix(&cla));
    }
}
