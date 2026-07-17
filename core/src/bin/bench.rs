//! Micro-benchmark hot path của engine: ns/phím cho các kịch bản gõ
//! thực tế và bệnh lý. Chạy: cargo run --release --bin bench

use oreokey_core::engine::{Engine, EngineConfig, KeyInput, SpellMode, TypingMethod};
use std::time::Instant;

fn engine(method: TypingMethod, mode: SpellMode) -> Engine {
    Engine::new(EngineConfig {
        method,
        spell_mode: mode,
        modern_tone: false,
        macros_enabled: false,
        flexible_marks: true,
        censor_enabled: false,
    })
}

/// Gõ `keys` lặp `iters` lần (space = WordBreak), trả về ns/phím.
fn bench(label: &str, mut e: Engine, keys: &str, iters: u32) {
    let mut total_keys = 0u64;
    let t = Instant::now();
    for _ in 0..iters {
        for c in keys.chars() {
            let k = match c {
                ' ' => KeyInput::WordBreak(Some(' ')),
                '\u{8}' => KeyInput::Backspace,
                other => KeyInput::Char(other),
            };
            std::hint::black_box(e.on_key(k));
            total_keys += 1;
        }
        e.on_key(KeyInput::WordBreak(None));
    }
    let el = t.elapsed();
    println!(
        "{label:<44} {:>9.0} ns/phím  ({total_keys} phím / {el:?})",
        el.as_nanos() as f64 / total_keys as f64
    );
}

fn main() {
    let telex = || engine(TypingMethod::Telex, SpellMode::Standard);
    let telex_strict = || engine(TypingMethod::Telex, SpellMode::Strict);
    let vni = || engine(TypingMethod::Vni, SpellMode::Standard);

    // Kịch bản thực tế: câu tiếng Việt nhiều dấu.
    let vn = "dduongwf nguoiwf vieetj toans nguyeenx khuyru ";
    bench("telex câu TV nhiều dấu", telex(), vn, 30_000);
    bench("vni câu TV nhiều dấu", vni(), "d9uong72 nguoi72 vie6t5 toan1 ", 30_000);

    // Tiếng Anh (đường restore).
    bench("telex strict tiếng Anh (restore)", telex_strict(), "expression windows keyboard mask class ", 30_000);

    // Bệnh lý 1: từ dài 200 ký tự không space, chết raw sớm (URL-ish).
    let url: String = "httpsz".chars().chain("abcdefghij".chars().cycle().take(194)).collect();
    bench("bệnh lý: 200 ký tự raw-mode", telex(), &url, 2_000);

    // Bệnh lý 2: 200 ký tự KHÔNG bị khóa (phụ âm hợp lệ kéo dài "nnnn...").
    let ns: String = std::iter::repeat('n').take(200).collect();
    bench("bệnh lý: 200 ký tự không khóa (nnn…)", telex(), &ns, 2_000);

    // Bệnh lý 3: nguyên âm lặp dài (aa toggle mũ liên tục).
    let aa: String = std::iter::repeat('a').take(200).collect();
    bench("bệnh lý: 200 nguyên âm a (toggle mũ)", telex(), &aa, 2_000);

    // Backspace storm: gõ 100 rồi xóa 100.
    let mut bs = String::new();
    bs.extend("veryLongWordWithoutAnySpaceOrBreak".chars().cycle().take(100));
    bs.extend(std::iter::repeat('\u{8}').take(100));
    bench("backspace storm 100+100", telex(), &bs, 2_000);
}
