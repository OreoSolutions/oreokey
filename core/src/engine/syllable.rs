//! Dựng văn bản hiển thị từ `WordState`: ánh xạ chữ cái + dấu phụ sang
//! Unicode precomposed và đặt thanh điệu đúng vị trí.

use super::{Letter, Tone, WordState};

/// Vị trí các nguyên âm trong từ, đã loại `u` trong `qu` và `i` trong
/// `gi` khi chúng đóng vai trò phụ âm đầu.
pub fn vowel_indices(letters: &[Letter]) -> Vec<usize> {
    let mut idx = Vec::new();
    for (i, l) in letters.iter().enumerate() {
        if !l.is_vowel() {
            continue;
        }
        // "qu": u ngay sau q là một phần của phụ âm đầu nếu sau nó còn nguyên âm.
        if l.base == 'u'
            && !l.has_mark()
            && i > 0
            && letters[i - 1].base == 'q'
            && letters[i + 1..].iter().any(|x| x.is_vowel())
        {
            continue;
        }
        // "gi": i sau g đầu từ là một phần của phụ âm đầu nếu sau nó còn nguyên âm.
        if l.base == 'i'
            && !l.has_mark()
            && i == 1
            && letters[0].base == 'g'
            && letters[2..].iter().any(|x| x.is_vowel())
        {
            continue;
        }
        idx.push(i);
    }
    idx
}

/// Cụm nguyên âm chính: chuỗi nguyên âm liên tiếp đầu tiên, kèm cờ "có
/// phụ âm cuối" (còn chữ nào đứng sau cụm).
fn main_cluster(letters: &[Letter]) -> Option<(Vec<usize>, bool)> {
    let vidx = vowel_indices(letters);
    let first = *vidx.first()?;
    let mut run = vec![first];
    for &i in &vidx[1..] {
        if i == run.last().unwrap() + 1 {
            run.push(i);
        } else {
            break;
        }
    }
    let has_final = run.last().unwrap() + 1 < letters.len();
    Some((run, has_final))
}

/// Chỉ số chữ cái nhận thanh điệu, theo luật chính tả:
/// 1. Nguyên âm mang dấu phụ (ă â ê ô ơ ư) — lấy cái cuối cùng (ươ → ơ).
/// 2. Có phụ âm cuối → nguyên âm cuối của cụm.
/// 3. Cụm mở: 1 nguyên âm → chính nó; 2 → nguyên âm đầu, riêng oa/oe/uy
///    kiểu mới đặt nguyên âm sau; 3 → nguyên âm giữa.
fn tone_index(letters: &[Letter], modern: bool) -> Option<usize> {
    let (run, has_final) = main_cluster(letters)?;
    if let Some(&i) = run.iter().filter(|&&i| letters[i].has_mark()).last() {
        return Some(i);
    }
    if has_final {
        return Some(*run.last().unwrap());
    }
    match run.len() {
        1 => Some(run[0]),
        2 => {
            let pair = (letters[run[0]].base, letters[run[1]].base);
            let glide = matches!(pair, ('o', 'a') | ('o', 'e') | ('u', 'y'));
            Some(if glide && modern { run[1] } else { run[0] })
        }
        _ => Some(run[1]),
    }
}

/// Chữ cái + dấu phụ → ký tự thường chưa có thanh.
pub(crate) fn marked_lower(l: &Letter) -> char {
    match l.base {
        'a' if l.circ => 'â',
        'a' if l.breve => 'ă',
        'e' if l.circ => 'ê',
        'o' if l.circ => 'ô',
        'o' if l.horn => 'ơ',
        'u' if l.horn => 'ư',
        'd' if l.stroke => 'đ',
        b => b,
    }
}

/// Nguyên âm (đã có dấu phụ) + thanh → ký tự precomposed.
fn toned(v: char, t: Tone) -> char {
    const ROWS: &[(char, [char; 5])] = &[
        ('a', ['á', 'à', 'ả', 'ã', 'ạ']),
        ('ă', ['ắ', 'ằ', 'ẳ', 'ẵ', 'ặ']),
        ('â', ['ấ', 'ầ', 'ẩ', 'ẫ', 'ậ']),
        ('e', ['é', 'è', 'ẻ', 'ẽ', 'ẹ']),
        ('ê', ['ế', 'ề', 'ể', 'ễ', 'ệ']),
        ('i', ['í', 'ì', 'ỉ', 'ĩ', 'ị']),
        ('o', ['ó', 'ò', 'ỏ', 'õ', 'ọ']),
        ('ô', ['ố', 'ồ', 'ổ', 'ỗ', 'ộ']),
        ('ơ', ['ớ', 'ờ', 'ở', 'ỡ', 'ợ']),
        ('u', ['ú', 'ù', 'ủ', 'ũ', 'ụ']),
        ('ư', ['ứ', 'ừ', 'ử', 'ữ', 'ự']),
        ('y', ['ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ']),
    ];
    let k = match t {
        Tone::Acute => 0,
        Tone::Grave => 1,
        Tone::Hook => 2,
        Tone::Tilde => 3,
        Tone::Dot => 4,
    };
    ROWS.iter()
        .find(|(b, _)| *b == v)
        .map(|(_, r)| r[k])
        .unwrap_or(v)
}

/// Render toàn bộ từ, giữ nguyên hoa/thường của từng phím gốc.
pub fn render_letters(state: &WordState, modern: bool) -> String {
    let ti = state
        .tone
        .and_then(|_| tone_index(&state.letters, modern));
    let mut out = String::new();
    for (i, l) in state.letters.iter().enumerate() {
        let mut ch = marked_lower(l);
        if Some(i) == ti {
            if let Some(t) = state.tone {
                ch = toned(ch, t);
            }
        }
        if l.upper {
            out.extend(ch.to_uppercase());
        } else {
            out.push(ch);
        }
    }
    out
}
