//! `sweep` — bộ sinh test toàn từ điển cho engine gõ tiếng Việt.
//!
//! Với mỗi âm tiết trong file từ điển, sinh ra mọi cách gõ hợp lệ (Telex
//! và/hoặc VNI), chạy từng chuỗi phím qua `Engine` thật, và ghi lại mọi
//! trường hợp kết quả hiển thị khác với âm tiết gốc.
//!
//! Thiết kế then chốt: bộ sinh KHÔNG tự mô phỏng lại toàn bộ luật tinh vi
//! của telex.rs/vni.rs (chọn nguyên âm "gần cuối nhất", cặp ươ...). Thay
//! vào đó nó sinh một siêu-tập ứng viên theo các trục mô tả trong đề bài,
//! rồi lọc lại bằng chính `telex::apply_key`/`vni::apply_key` (oracle) —
//! chỉ giữ chuỗi phím nào tái tạo ĐÚNG âm tiết mục tiêu khi render thô
//! (chưa qua tầng chính tả/khôi phục). Nhờ vậy tính đúng đắn của việc
//! *sinh* không phụ thuộc vào việc mô hình hoá tay các luật ưu tiên nội
//! bộ — nó luôn nhất quán với chính engine. Bài kiểm thực sự (mục đích
//! của sweep) là chạy các chuỗi đã lọc qua `Engine` ĐẦY ĐỦ (có tầng
//! spell/raw_mode) và so kết quả cuối với âm tiết mục tiêu.

use oreokey_core::engine::{
    syllable, telex, vni, Engine, EngineConfig, KeyInput, Letter, SpellMode, Tone, TypingMethod,
    WordState,
};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;

const CAP_PER_WORD_METHOD: usize = 5000;
const CARTESIAN_HARD_CAP: usize = 20_000;

// ---------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MethodSel {
    Telex,
    Vni,
    Both,
}

struct Args {
    dict_path: String,
    out_path: String,
    method: MethodSel,
    modern: bool,
    limit: Option<usize>,
}

fn parse_args() -> Result<Args, String> {
    let mut positional: Vec<String> = Vec::new();
    let mut out_path: Option<String> = None;
    let mut method = MethodSel::Both;
    let mut modern = false;
    let mut limit: Option<usize> = None;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" => {
                out_path = Some(it.next().ok_or("--out cần một đường dẫn")?);
            }
            "--method" => {
                let v = it.next().ok_or("--method cần telex|vni|both")?;
                method = match v.as_str() {
                    "telex" => MethodSel::Telex,
                    "vni" => MethodSel::Vni,
                    "both" => MethodSel::Both,
                    other => return Err(format!("--method không hợp lệ: {other}")),
                };
            }
            "--modern" => modern = true,
            "--limit" => {
                let v = it.next().ok_or("--limit cần một số")?;
                limit = Some(v.parse::<usize>().map_err(|e| e.to_string())?);
            }
            other if !other.starts_with("--") => positional.push(other.to_string()),
            other => return Err(format!("cờ không rõ: {other}")),
        }
    }

    let dict_path = positional
        .into_iter()
        .next()
        .ok_or("thiếu đường dẫn <syllables.txt>")?;
    let out_path = out_path.ok_or("thiếu --out <failures.jsonl>")?;

    Ok(Args { dict_path, out_path, method, modern, limit })
}

// ---------------------------------------------------------------------
// Bước 2: decomposer — NFC-ish, tách chữ + dấu phụ + thanh của cả từ.
// ---------------------------------------------------------------------

/// (chữ trơn, base ascii, circ, horn, breve)
const VOWEL_TABLE: &[(char, char, bool, bool, bool)] = &[
    ('a', 'a', false, false, false),
    ('ă', 'a', false, false, true),
    ('â', 'a', true, false, false),
    ('e', 'e', false, false, false),
    ('ê', 'e', true, false, false),
    ('i', 'i', false, false, false),
    ('o', 'o', false, false, false),
    ('ô', 'o', true, false, false),
    ('ơ', 'o', false, true, false),
    ('u', 'u', false, false, false),
    ('ư', 'u', false, true, false),
    ('y', 'y', false, false, false),
];

/// Bảng nguyên âm có dấu (sao y từ syllable.rs — cần thiết để giải mã
/// theo chiều ngược, không thể tái dùng vì hàm gốc là private).
const TONE_ROWS: &[(char, [char; 5])] = &[
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

fn vowel_flags(plain: char) -> Option<(char, bool, bool, bool)> {
    VOWEL_TABLE
        .iter()
        .find(|&&(p, ..)| p == plain)
        .map(|&(_, b, c, h, br)| (b, c, h, br))
}

fn tone_from_index(k: usize) -> Tone {
    match k {
        0 => Tone::Acute,
        1 => Tone::Grave,
        2 => Tone::Hook,
        3 => Tone::Tilde,
        _ => Tone::Dot,
    }
}

/// Giải mã một ký tự Unicode (dạng precomposed/NFC) thành
/// (base ascii, upper, circ, horn, breve, stroke, thanh).
fn decompose_char(c: char) -> Option<(char, bool, bool, bool, bool, bool, Option<Tone>)> {
    if c == 'đ' || c == 'Đ' {
        return Some(('d', c.is_uppercase(), false, false, false, true, None));
    }
    if c.is_ascii_alphabetic() {
        return Some((c.to_ascii_lowercase(), c.is_ascii_uppercase(), false, false, false, false, None));
    }
    let upper = c.is_uppercase();
    let lower = c.to_lowercase().next()?;
    if let Some((b, ci, ho, br)) = vowel_flags(lower) {
        return Some((b, upper, ci, ho, br, false, None));
    }
    for &(base_plain, tones) in TONE_ROWS {
        if let Some(k) = tones.iter().position(|&t| t == lower) {
            let (b, ci, ho, br) = vowel_flags(base_plain)?;
            return Some((b, upper, ci, ho, br, false, Some(tone_from_index(k))));
        }
    }
    None
}

/// Dấu phụ dạng combining mark (đề phòng dữ liệu NFD chứ không phải
/// precomposed hoàn toàn — macOS hay sinh NFD).
fn is_combining_mark(c: char) -> bool {
    matches!(
        c,
        '\u{0300}' | '\u{0301}' | '\u{0303}' | '\u{0309}' | '\u{0323}' | '\u{0302}' | '\u{0306}' | '\u{031B}'
    )
}

fn combining_tone(c: char) -> Option<Tone> {
    match c {
        '\u{0301}' => Some(Tone::Acute),
        '\u{0300}' => Some(Tone::Grave),
        '\u{0309}' => Some(Tone::Hook),
        '\u{0303}' => Some(Tone::Tilde),
        '\u{0323}' => Some(Tone::Dot),
        _ => None,
    }
}

/// Tách một âm tiết thành danh sách `Letter` (kiểu engine thật) + thanh
/// điệu cấp từ (tối đa 1 thanh/âm tiết — dictionary chỉ có 1 âm tiết/dòng).
fn decompose(word: &str) -> Option<(Vec<Letter>, Option<Tone>)> {
    let chars: Vec<char> = word.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let mut letters = Vec::new();
    let mut tone: Option<Tone> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_combining_mark(c) {
            return None; // combining mark mồ côi — dữ liệu hỏng, bỏ qua.
        }
        let (base, upper, mut circ, mut horn, mut breve, stroke, mut ctone) = decompose_char(c)?;
        i += 1;
        while i < chars.len() && is_combining_mark(chars[i]) {
            match chars[i] {
                '\u{0302}' => circ = true,
                '\u{031B}' => horn = true,
                '\u{0306}' => breve = true,
                m => {
                    if let Some(t) = combining_tone(m) {
                        ctone = Some(t);
                    }
                }
            }
            i += 1;
        }
        if let Some(t) = ctone {
            tone = Some(t);
        }
        letters.push(Letter { base, upper, circ, horn, breve, stroke, w_origin: false });
    }
    if !letters.iter().any(|l| l.is_vowel()) {
        return None;
    }
    Some((letters, tone))
}

// ---------------------------------------------------------------------
// Bước 3: variant generator.
// ---------------------------------------------------------------------

/// Một "sự kiện chèn phím": (vị trí — số chữ-gốc đã gõ, phím, nhãn trục).
type Event = (usize, char, &'static str);

#[derive(Clone)]
struct AxisChoice {
    /// Mỗi phần tử là MỘT cách đạt trục này (1 hoặc nhiều sự kiện, vd
    /// kiểu tách "uw...ow" cần 2 sự kiện).
    alts: Vec<Vec<Event>>,
}

fn base_char_for(l: &Letter) -> char {
    if l.upper {
        l.base.to_ascii_uppercase()
    } else {
        l.base
    }
}

fn tone_key(method: TypingMethod, t: Tone) -> char {
    match method {
        TypingMethod::Telex => match t {
            Tone::Acute => 's',
            Tone::Grave => 'f',
            Tone::Hook => 'r',
            Tone::Tilde => 'x',
            Tone::Dot => 'j',
        },
        TypingMethod::Vni => match t {
            Tone::Acute => '1',
            Tone::Grave => '2',
            Tone::Hook => '3',
            Tone::Tilde => '4',
            Tone::Dot => '5',
        },
    }
}

fn horn_key(method: TypingMethod) -> char {
    match method {
        TypingMethod::Telex => 'w',
        TypingMethod::Vni => '7',
    }
}

fn breve_key(method: TypingMethod) -> char {
    match method {
        TypingMethod::Telex => 'w',
        TypingMethod::Vni => '8',
    }
}

fn dstroke_key(method: TypingMethod) -> char {
    match method {
        TypingMethod::Telex => 'd',
        TypingMethod::Vni => '9',
    }
}

/// Sinh danh sách "kế hoạch" (base_tokens, axes). Bình thường chỉ có một
/// kế hoạch; khi có thể gõ `ư`/`ươ` bằng phím `w` đứng một mình (Telex,
/// case (c) trong đề bài) thì thêm một kế hoạch thứ hai với vị trí chữ
/// `u` đó bị "khuyết" (None — không gõ ký tự gốc, để nguyên cho trục móc
/// tự lấp bằng chính sự kiện `w` của nó).
fn build_generation_plans(
    letters: &[Letter],
    tone: Option<Tone>,
    method: TypingMethod,
) -> Vec<(Vec<Option<char>>, Vec<AxisChoice>)> {
    let n = letters.len();
    let base_tokens: Vec<Option<char>> = letters.iter().map(|l| Some(base_char_for(l))).collect();
    let mut axes: Vec<AxisChoice> = Vec::new();

    // Axis: thanh điệu.
    if let Some(t) = tone {
        if let Some(fv) = letters.iter().position(|l| l.is_vowel()) {
            let key = tone_key(method, t);
            let alts = ((fv + 1)..=n).map(|g| vec![(g, key, "tone_pos")]).collect();
            axes.push(AxisChoice { alts });
        }
    }

    // Phát hiện cặp ươ liền kề (u-horn, o-horn).
    let pair_idx = (0..n.saturating_sub(1)).find(|&i| {
        letters[i].base == 'u' && letters[i].horn && letters[i + 1].base == 'o' && letters[i + 1].horn
    });

    // (index chữ, có phải cặp không) — khi có thể gõ bằng w đứng một mình.
    let mut worigin_plan: Option<usize> = None;

    if let Some(i) = pair_idx {
        let key = horn_key(method);
        let mut alts: Vec<Vec<Event>> = Vec::new();
        for g in (i + 2)..=n {
            alts.push(vec![(g, key, "w_pair_single")]);
        }
        for g2 in (i + 2)..=n {
            alts.push(vec![(i + 1, key, "w_pair_split"), (g2, key, "w_pair_split")]);
        }
        axes.push(AxisChoice { alts });

        if method == TypingMethod::Telex {
            let prev_ok = i == 0 || (!letters[i - 1].is_vowel() && letters[i - 1].base != 'q');
            if prev_ok {
                worigin_plan = Some(i);
            }
        }
    } else {
        for i in 0..n {
            let l = letters[i];
            if l.horn && matches!(l.base, 'u' | 'o') {
                let key = horn_key(method);
                let alts = ((i + 1)..=n).map(|g| vec![(g, key, "horn_single")]).collect();
                axes.push(AxisChoice { alts });

                if method == TypingMethod::Telex && l.base == 'u' {
                    let prev_ok = i == 0 || (!letters[i - 1].is_vowel() && letters[i - 1].base != 'q');
                    if prev_ok {
                        worigin_plan = Some(i);
                    }
                }
            }
            if l.breve && l.base == 'a' {
                let key = breve_key(method);
                let alts = ((i + 1)..=n).map(|g| vec![(g, key, "breve_single")]).collect();
                axes.push(AxisChoice { alts });
            }
        }
    }

    // Axis: mũ (â/ê/ô) — độc lập với móc/trăng.
    for i in 0..n {
        if letters[i].circ {
            let key = match method {
                TypingMethod::Telex => letters[i].base,
                TypingMethod::Vni => '6',
            };
            let alts = ((i + 1)..=n)
                .map(|g| {
                    let label = if method == TypingMethod::Telex && g == i + 1 {
                        "circ_adjacent"
                    } else {
                        "circ_late"
                    };
                    vec![(g, key, label)]
                })
                .collect();
            axes.push(AxisChoice { alts });
        }
    }

    // Axis: đ.
    if letters[0].base == 'd' && letters[0].stroke {
        let key = dstroke_key(method);
        let mut alts: Vec<Vec<Event>> = Vec::new();
        match method {
            TypingMethod::Telex => {
                alts.push(vec![(1, key, "d_adjacent")]);
                for g in 2..=n {
                    alts.push(vec![(g, key, "d_late")]);
                }
            }
            TypingMethod::Vni => {
                for g in 1..=n {
                    alts.push(vec![(g, key, "d_late")]);
                }
            }
        }
        axes.push(AxisChoice { alts });
    }

    // Axis: chữ lặp literal (oo/aa/ee thật) — chỉ Telex (VNI không tự
    // động ghép mũ khi gõ lặp chữ, chỉ ghép qua phím số 6).
    if method == TypingMethod::Telex {
        for i in 0..n.saturating_sub(1) {
            let (l1, l2) = (letters[i], letters[i + 1]);
            if l1.base == l2.base && matches!(l1.base, 'a' | 'e' | 'o') && !l1.has_mark() && !l2.has_mark() {
                let alts = ((i + 2)..=n).map(|g| vec![(g, l1.base, "oo_literal")]).collect();
                axes.push(AxisChoice { alts });
            }
        }
    }

    let mut plans = vec![(base_tokens.clone(), axes.clone())];
    if let Some(i) = worigin_plan {
        let mut alt_tokens = base_tokens;
        alt_tokens[i] = None;
        plans.push((alt_tokens, axes));
    }
    plans
}

fn permutations<T: Clone>(items: Vec<T>) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items];
    }
    let mut out = Vec::new();
    for i in 0..items.len() {
        let mut rest = items.clone();
        let item = rest.remove(i);
        for mut p in permutations(rest) {
            p.insert(0, item.clone());
            out.push(p);
        }
    }
    out
}

/// Tích Đề-các các lựa chọn của mỗi trục — mỗi kết quả là một danh sách
/// sự kiện phẳng (chưa sắp xếp hoán vị theo vị trí trùng nhau).
fn cartesian_combos(axes: &[AxisChoice]) -> Vec<Vec<Event>> {
    let mut result: Vec<Vec<Event>> = vec![Vec::new()];
    for axis in axes {
        let mut next = Vec::new();
        'build: for combo in &result {
            for alt in &axis.alts {
                let mut c = combo.clone();
                c.extend(alt.iter().copied());
                next.push(c);
                if next.len() >= CARTESIAN_HARD_CAP {
                    break 'build;
                }
            }
        }
        result = next;
        if result.len() >= CARTESIAN_HARD_CAP {
            break;
        }
    }
    result
}

/// Khi nhiều sự kiện rơi cùng một vị trí chèn: sinh mọi hoán vị thứ tự
/// giữa chúng (spec bước 3). Vị trí khác nhau giữ nguyên thứ tự tăng dần.
fn expand_permutations(events: &[Event]) -> Vec<Vec<Event>> {
    let mut by_gap: BTreeMap<usize, Vec<(char, &'static str)>> = BTreeMap::new();
    for &(g, c, l) in events {
        by_gap.entry(g).or_default().push((c, l));
    }
    let mut result: Vec<Vec<Event>> = vec![Vec::new()];
    for (&g, items) in &by_gap {
        let orderings = permutations(items.clone());
        let mut next = Vec::new();
        for combo in &result {
            for order in &orderings {
                let mut c = combo.clone();
                for &(ch, lab) in order {
                    c.push((g, ch, lab));
                }
                next.push(c);
            }
        }
        result = next;
    }
    result
}

/// Ráp `base_tokens` (chữ gốc, `None` = khuyết — trục móc tự lấp) với
/// danh sách sự kiện (đã sắp theo vị trí tăng dần) thành chuỗi phím.
fn materialize(base_tokens: &[Option<char>], events: &[Event]) -> String {
    let mut s = String::new();
    let mut ei = 0;
    let n = base_tokens.len();
    for g in 0..=n {
        while ei < events.len() && events[ei].0 == g {
            s.push(events[ei].1);
            ei += 1;
        }
        if g < n {
            if let Some(c) = base_tokens[g] {
                s.push(c);
            }
        }
    }
    s
}

fn dedup_preserve(v: &mut Vec<&'static str>) {
    let mut out = Vec::new();
    for x in v.drain(..) {
        if !out.contains(&x) {
            out.push(x);
        }
    }
    *v = out;
}

/// Oracle: render thô (không qua tầng chính tả) của một chuỗi phím theo
/// đúng luật engine thật (`telex::apply_key`/`vni::apply_key`).
fn raw_render(method: TypingMethod, keys: &str, modern_tone: bool) -> String {
    let mut state = WordState::default();
    for c in keys.chars() {
        match method {
            TypingMethod::Telex => telex::apply_key(&mut state, c, true),
            TypingMethod::Vni => vni::apply_key(&mut state, c),
        }
    }
    syllable::render_letters(&state, modern_tone)
}

/// Sinh + lọc (bằng oracle) toàn bộ chuỗi phím hợp lệ cho một âm tiết.
fn generate_variants(
    letters: &[Letter],
    tone: Option<Tone>,
    method: TypingMethod,
    modern_tone: bool,
    expected: &str,
) -> (Vec<(String, Vec<&'static str>)>, bool) {
    let plans = build_generation_plans(letters, tone, method);
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<(String, Vec<&'static str>)> = Vec::new();
    let mut capped = false;

    'outer: for (tokens, axes) in &plans {
        for combo in cartesian_combos(axes) {
            let mut labels: Vec<&'static str> = combo.iter().map(|e| e.2).collect();
            dedup_preserve(&mut labels);
            for ordering in expand_permutations(&combo) {
                let keys = materialize(tokens, &ordering);
                if !seen.insert(keys.clone()) {
                    continue;
                }
                // Oracle: chỉ giữ chuỗi phím thực sự tái tạo đúng âm tiết
                // mục tiêu ở tầng render thô.
                if raw_render(method, &keys, modern_tone) != expected {
                    continue;
                }
                out.push((keys, labels.clone()));
                if out.len() >= CAP_PER_WORD_METHOD {
                    capped = true;
                    break 'outer;
                }
            }
        }
    }
    (out, capped)
}

// ---------------------------------------------------------------------
// Bước 4: runner + output.
// ---------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn write_failure(
    w: &mut impl Write,
    word: &str,
    expected: &str,
    method: &str,
    style: &str,
    keys: &str,
    got: &str,
    axes: &[&'static str],
) -> std::io::Result<()> {
    let axes_json = axes
        .iter()
        .map(|a| format!("\"{}\"", json_escape(a)))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(
        w,
        "{{\"word\":\"{}\",\"expected\":\"{}\",\"method\":\"{}\",\"style\":\"{}\",\"keys\":\"{}\",\"got\":\"{}\",\"axes\":[{}]}}",
        json_escape(word),
        json_escape(expected),
        method,
        style,
        json_escape(keys),
        json_escape(got),
        axes_json
    )
}

fn run() -> Result<(), String> {
    let args = parse_args()?;

    let file = File::open(&args.dict_path).map_err(|e| format!("mở {} lỗi: {e}", args.dict_path))?;
    let reader = BufReader::new(file);
    let out_file = File::create(&args.out_path).map_err(|e| format!("tạo {} lỗi: {e}", args.out_path))?;
    let mut out = BufWriter::new(out_file);

    let methods: Vec<TypingMethod> = match args.method {
        MethodSel::Telex => vec![TypingMethod::Telex],
        MethodSel::Vni => vec![TypingMethod::Vni],
        MethodSel::Both => vec![TypingMethod::Telex, TypingMethod::Vni],
    };
    let style = if args.modern { "modern" } else { "old" };

    let mut engines: Vec<(TypingMethod, Engine)> = methods
        .iter()
        .map(|&m| {
            (
                m,
                Engine::new(EngineConfig {
                    method: m,
                    spell_mode: SpellMode::Standard,
                    modern_tone: args.modern,
                    macros_enabled: false,
                    flexible_marks: true,
                    censor_enabled: false,
                }),
            )
        })
        .collect();

    let mut words_tested: usize = 0;
    let mut attempted: usize = 0;
    let mut total_variants: usize = 0;
    let mut total_failures: usize = 0;
    let mut by_axis: BTreeMap<String, usize> = BTreeMap::new();
    let mut cap_hits: usize = 0;

    'lines: for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        for token in trimmed.split_whitespace() {
            if let Some(limit) = args.limit {
                if attempted >= limit {
                    break 'lines;
                }
            }
            attempted += 1;
            if attempted % 1000 == 0 {
                eprintln!("[sweep] progress: {attempted} âm tiết đã xử lý...");
            }

            let Some((letters, tone)) = decompose(token) else {
                continue;
            };
            // Sanity: dictionary ở kiểu cũ — decompose phải tái tạo đúng
            // nguyên văn khi render lại kiểu cũ. Nếu lệch, bỏ qua (dữ
            // liệu không phải âm tiết tiếng Việt chuẩn, vd số/tiếng Anh).
            let old_state = WordState { letters: letters.clone(), tone, dead: Vec::new() };
            let reconstructed_old = syllable::render_letters(&old_state, false);
            if reconstructed_old != token {
                eprintln!("[sweep] bỏ qua token không tái tạo được: {token:?} -> {reconstructed_old:?}");
                continue;
            }
            let expected = syllable::render_letters(&old_state, args.modern);

            words_tested += 1;

            for (method, engine) in engines.iter_mut() {
                let method_name = match method {
                    TypingMethod::Telex => "telex",
                    TypingMethod::Vni => "vni",
                };
                let (variants, capped) =
                    generate_variants(&letters, tone, *method, args.modern, &expected);
                if capped {
                    cap_hits += 1;
                    eprintln!(
                        "[sweep] cap {CAP_PER_WORD_METHOD} variant chạm cho từ={token:?} method={method_name}"
                    );
                }
                total_variants += variants.len();

                for (keys, axes) in &variants {
                    engine.reset();
                    for c in keys.chars() {
                        engine.on_key(KeyInput::Char(c));
                    }
                    let got = engine.current_word();
                    if got != expected {
                        total_failures += 1;
                        for &axis in axes {
                            *by_axis.entry(axis.to_string()).or_insert(0) += 1;
                        }
                        write_failure(&mut out, token, &expected, method_name, style, keys, got, axes)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }

    out.flush().map_err(|e| e.to_string())?;

    if cap_hits > 0 {
        eprintln!("[sweep] tổng số lần chạm cap: {cap_hits}");
    }

    let by_axis_json = by_axis
        .iter()
        .map(|(k, v)| format!("\"{}\":{}", json_escape(k), v))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"words\":{},\"variants\":{},\"failures\":{},\"by_axis\":{{{}}}}}",
        words_tested, total_variants, total_failures, by_axis_json
    );

    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lỗi: {e}");
            eprintln!(
                "cách dùng: sweep <syllables.txt> --out <failures.jsonl> [--method telex|vni|both] [--modern] [--limit N]"
            );
            ExitCode::FAILURE
        }
    }
}
