//! Chuyển mã văn bản tiếng Việt giữa 3 bảng mã: Unicode (chuẩn dựng sẵn —
//! precomposed NFC), VNI-Windows, và TCVN3 (ABC). Module thuần `std`,
//! không import gì từ module khác trong crate.
//!
//! Cách hoạt động: pivot qua Unicode — `convert` luôn giải mã `from` về
//! Unicode trước rồi mã hoá sang `to`.
//!
//! # Nguồn bảng mã (dữ kiện tra cứu, không copy code)
//!
//! * **VNI-Windows**: bảng "VNI 'ANSI' Encoding (Windows/Unix)" trong bài
//!   Wikipedia tiếng Anh "VNI" (`en.wikipedia.org/wiki/VNI`, mục
//!   "Character encodings" → "VNI Encoding (Windows/Unix)"), dẫn nguồn từ
//!   Vietnamese Unicode FAQs (`vietunicode.sourceforge.net/charset/vni.html`).
//!   Bảng liệt kê byte 0xC0-0xFF: mỗi ô hoặc là 1 ký tự dựng sẵn (vd 0xF1 =
//!   đ), hoặc là 1 "dấu" (combining mark) áp sau chữ cái ASCII gốc (vd 0xE2
//!   = dấu mũ chữ thường). Byte 0x80-0xBF không dùng.
//! * **TCVN3 (ABC)**: phần *không tô màu* (chú thích "VSCII-3") của bảng
//!   "VSCII-1" trong bài Wikipedia tiếng Anh "VSCII"
//!   (`en.wikipedia.org/wiki/VSCII`), dẫn nguồn TCVN 5712:1993. TCVN3 là
//!   tập con của VSCII-1: 67 chữ thường có dấu (byte 0xA1-0xFF, trừ các ô
//!   tô màu là phần mở rộng riêng của VSCII-1/2) + 7 chữ hoa "gốc" không
//!   dấu thanh (Ă Â Ê Ô Ơ Ư Đ, byte 0xA1-0xA7).
//!
//! Vì byte 0xA0-0xFF của Windows-1252/Latin-1 trùng đúng giá trị code
//! point Unicode U+00A0-U+00FF, module này biểu diễn mỗi byte 8-bit của
//! VNI-Windows/TCVN3 trực tiếp bằng ký tự Unicode có cùng giá trị (vd byte
//! 0xE4 ↔ `'\u{E4}'` = 'ä'). Đây chính là hiện tượng "mojibake" quen thuộc
//! khi mở văn bản VNI/TCVN3 cũ bằng font/encoding sai — ví dụ "Ngày" →
//! "Ngaøy", "Thứ Năm" → "Thöù Naêm" — nên các giá trị trong bảng dưới đây
//! có thể đối chiếu trực tiếp với các mẫu mojibake quen thuộc đó.
//!
//! # Quyết định best-effort: chữ hoa có dấu thanh trong TCVN3
//!
//! TCVN3 KHÔNG có byte riêng cho chữ hoa có dấu thanh (vd Ệ, Ắ, Ớ...) —
//! theo đúng Wikipedia: "Tone marks on uppercase vowels is accomplished in
//! TCVN3 by switching to an all-capital font" (tức là file TCVN3 thật thể
//! hiện chữ hoa bằng cách đổi font toàn khối, không phải bằng byte khác).
//! Vì đây là chuyển mã văn bản thuần (không có khái niệm "font"), quyết
//! định best-effort ở đây — theo đúng cách các converter phổ biến (Unikey
//! Toolkit và tương tự) xử lý — là: dùng byte của chữ thường có dấu tương
//! ứng (mất thông tin hoa/thường cho riêng phần dấu thanh đó). 7 chữ hoa
//! "gốc" không dấu thanh (Ă Â Ê Ô Ơ Ư Đ) có byte riêng nên giữ hoa chính
//! xác. Hệ quả: TCVN3 → Unicode luôn trả chữ thường cho các âm có dấu
//! thanh; đây là giới hạn cố hữu của TCVN3, không phải lỗi của module.

use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Unicode,
    VniWindows,
    Tcvn3,
}

pub fn convert(text: &str, from: Encoding, to: Encoding) -> String {
    let unicode = match from {
        Encoding::Unicode => text.to_string(),
        Encoding::VniWindows => vni_to_unicode(text),
        Encoding::Tcvn3 => tcvn3_to_unicode(text),
    };
    match to {
        Encoding::Unicode => unicode,
        Encoding::VniWindows => unicode_to_vni(&unicode),
        Encoding::Tcvn3 => unicode_to_tcvn3(&unicode),
    }
}

/// (ký tự Unicode dựng sẵn, chuỗi VNI-Windows tương ứng — 1 hoặc 2 ký tự).
/// 67 dòng chữ thường + 67 dòng chữ hoa = 134 âm tiếng Việt dựng sẵn.
#[rustfmt::skip]
const VNI_TABLE: &[(char, &str)] = &[
    // --- a / ă / â (chữ thường) ---
    ('á', "aù"), ('à', "aø"), ('ả', "aû"), ('ã', "aõ"), ('ạ', "aï"),
    ('â', "aâ"), ('ấ', "aá"), ('ầ', "aà"), ('ẩ', "aå"), ('ẫ', "aã"), ('ậ', "aä"),
    ('ă', "aê"), ('ắ', "aé"), ('ằ', "aè"), ('ẳ', "aú"), ('ẵ', "aü"), ('ặ', "aë"),
    // --- e / ê ---
    ('é', "eù"), ('è', "eø"), ('ẻ', "eû"), ('ẽ', "eõ"), ('ẹ', "eï"),
    ('ê', "eâ"), ('ế', "eá"), ('ề', "eà"), ('ể', "eå"), ('ễ', "eã"), ('ệ', "eä"),
    // --- i (dựng sẵn 1 ký tự, không tiền tố 'i') ---
    ('í', "í"), ('ì', "ì"), ('ỉ', "æ"), ('ĩ', "ó"), ('ị', "ò"),
    // --- đ (dựng sẵn 1 ký tự) ---
    ('đ', "ñ"),
    // --- o / ô / ơ ---
    ('ó', "où"), ('ò', "oø"), ('ỏ', "oû"), ('õ', "oõ"), ('ọ', "oï"),
    ('ô', "oâ"), ('ố', "oá"), ('ồ', "oà"), ('ổ', "oå"), ('ỗ', "oã"), ('ộ', "oä"),
    ('ơ', "ô"), ('ớ', "ôù"), ('ờ', "ôø"), ('ở', "ôû"), ('ỡ', "ôõ"), ('ợ', "ôï"),
    // --- u / ư ---
    ('ú', "uù"), ('ù', "uø"), ('ủ', "uû"), ('ũ', "uõ"), ('ụ', "uï"),
    ('ư', "ö"), ('ứ', "öù"), ('ừ', "öø"), ('ử', "öû"), ('ữ', "öõ"), ('ự', "öï"),
    // --- y (ỵ dựng sẵn 1 ký tự, còn lại tiền tố 'y') ---
    ('ý', "yù"), ('ỳ', "yø"), ('ỷ', "yû"), ('ỹ', "yõ"), ('ỵ', "î"),

    // --- A / Ă / Â (chữ hoa) ---
    ('Á', "AÙ"), ('À', "AØ"), ('Ả', "AÛ"), ('Ã', "AÕ"), ('Ạ', "AÏ"),
    ('Â', "AÂ"), ('Ấ', "AÁ"), ('Ầ', "AÀ"), ('Ẩ', "AÅ"), ('Ẫ', "AÃ"), ('Ậ', "AÄ"),
    ('Ă', "AÊ"), ('Ắ', "AÉ"), ('Ằ', "AÈ"), ('Ẳ', "AÚ"), ('Ẵ', "AÜ"), ('Ặ', "AË"),
    // --- E / Ê ---
    ('É', "EÙ"), ('È', "EØ"), ('Ẻ', "EÛ"), ('Ẽ', "EÕ"), ('Ẹ', "EÏ"),
    ('Ê', "EÂ"), ('Ế', "EÁ"), ('Ề', "EÀ"), ('Ể', "EÅ"), ('Ễ', "EÃ"), ('Ệ', "EÄ"),
    // --- I ---
    ('Í', "Í"), ('Ì', "Ì"), ('Ỉ', "Æ"), ('Ĩ', "Ó"), ('Ị', "Ò"),
    // --- Đ ---
    ('Đ', "Ñ"),
    // --- O / Ô / Ơ ---
    ('Ó', "OÙ"), ('Ò', "OØ"), ('Ỏ', "OÛ"), ('Õ', "OÕ"), ('Ọ', "OÏ"),
    ('Ô', "OÂ"), ('Ố', "OÁ"), ('Ồ', "OÀ"), ('Ổ', "OÅ"), ('Ỗ', "OÃ"), ('Ộ', "OÄ"),
    ('Ơ', "Ô"), ('Ớ', "ÔÙ"), ('Ờ', "ÔØ"), ('Ở', "ÔÛ"), ('Ỡ', "ÔÕ"), ('Ợ', "ÔÏ"),
    // --- U / Ư ---
    ('Ú', "UÙ"), ('Ù', "UØ"), ('Ủ', "UÛ"), ('Ũ', "UÕ"), ('Ụ', "UÏ"),
    ('Ư', "Ö"), ('Ứ', "ÖÙ"), ('Ừ', "ÖØ"), ('Ử', "ÖÛ"), ('Ữ', "ÖÕ"), ('Ự', "ÖÏ"),
    // --- Y ---
    ('Ý', "YÙ"), ('Ỳ', "YØ"), ('Ỷ', "YÛ"), ('Ỹ', "YÕ"), ('Ỵ', "Î"),
];

/// (ký tự Unicode dựng sẵn, byte TCVN3 tương ứng biểu diễn bằng ký tự
/// Unicode cùng giá trị code point — vd byte 0xE4 ↔ '\u{E4}'). 67 dòng
/// chữ thường + 7 dòng chữ hoa "gốc" không dấu thanh.
#[rustfmt::skip]
const TCVN3_TABLE: &[(char, char)] = &[
    // 7 chữ thường "gốc" (byte 0xA8-0xAE)
    ('ă', '\u{A8}'), ('â', '\u{A9}'), ('ê', '\u{AA}'), ('ô', '\u{AB}'),
    ('ơ', '\u{AC}'), ('ư', '\u{AD}'), ('đ', '\u{AE}'),
    // nhóm a (byte 0xB5-0xBE)
    ('à', '\u{B5}'), ('ả', '\u{B6}'), ('ã', '\u{B7}'), ('á', '\u{B8}'), ('ạ', '\u{B9}'),
    ('ằ', '\u{BB}'), ('ẳ', '\u{BC}'), ('ẵ', '\u{BD}'), ('ắ', '\u{BE}'),
    // nhóm â / e (byte 0xC6-0xCF)
    ('ặ', '\u{C6}'), ('ầ', '\u{C7}'), ('ẩ', '\u{C8}'), ('ẫ', '\u{C9}'), ('ấ', '\u{CA}'),
    ('ậ', '\u{CB}'), ('è', '\u{CC}'), ('ẻ', '\u{CE}'), ('ẽ', '\u{CF}'),
    // nhóm ê / i (byte 0xD0-0xDF)
    ('é', '\u{D0}'), ('ẹ', '\u{D1}'), ('ề', '\u{D2}'), ('ể', '\u{D3}'), ('ễ', '\u{D4}'),
    ('ế', '\u{D5}'), ('ệ', '\u{D6}'), ('ì', '\u{D7}'), ('ỉ', '\u{D8}'),
    ('ĩ', '\u{DC}'), ('í', '\u{DD}'), ('ị', '\u{DE}'), ('ò', '\u{DF}'),
    // nhóm o / ô (byte 0xE1-0xEF)
    ('ỏ', '\u{E1}'), ('õ', '\u{E2}'), ('ó', '\u{E3}'), ('ọ', '\u{E4}'), ('ồ', '\u{E5}'),
    ('ổ', '\u{E6}'), ('ỗ', '\u{E7}'), ('ố', '\u{E8}'), ('ộ', '\u{E9}'), ('ờ', '\u{EA}'),
    ('ở', '\u{EB}'), ('ỡ', '\u{EC}'), ('ớ', '\u{ED}'), ('ợ', '\u{EE}'), ('ù', '\u{EF}'),
    // nhóm u / ư / y (byte 0xF1-0xFE)
    ('ủ', '\u{F1}'), ('ũ', '\u{F2}'), ('ú', '\u{F3}'), ('ụ', '\u{F4}'), ('ừ', '\u{F5}'),
    ('ử', '\u{F6}'), ('ữ', '\u{F7}'), ('ứ', '\u{F8}'), ('ự', '\u{F9}'), ('ỳ', '\u{FA}'),
    ('ỷ', '\u{FB}'), ('ỹ', '\u{FC}'), ('ý', '\u{FD}'), ('ỵ', '\u{FE}'),
    // 7 chữ hoa "gốc" duy nhất có byte riêng (byte 0xA1-0xA7)
    ('Ă', '\u{A1}'), ('Â', '\u{A2}'), ('Ê', '\u{A3}'), ('Ô', '\u{A4}'),
    ('Ơ', '\u{A5}'), ('Ư', '\u{A6}'), ('Đ', '\u{A7}'),
];

fn unicode_to_vni(s: &str) -> String {
    static MAP: OnceLock<HashMap<char, &'static str>> = OnceLock::new();
    let map = MAP.get_or_init(|| VNI_TABLE.iter().copied().collect());
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match map.get(&c) {
            Some(v) => out.push_str(v),
            None => out.push(c),
        }
    }
    out
}

fn vni_to_unicode(s: &str) -> String {
    // Tách bảng thành 2 tầng tra cứu: chuỗi 1 ký tự (chữ dựng sẵn như
    // "ñ", "ô", "í"...) và chuỗi 2 ký tự (chữ gốc ASCII + ký tự dấu).
    type VniMaps = (HashMap<char, char>, HashMap<(char, char), char>);
    static MAPS: OnceLock<VniMaps> = OnceLock::new();
    let (single, pair) = MAPS.get_or_init(|| {
        let mut single: HashMap<char, char> = HashMap::new();
        let mut pair: HashMap<(char, char), char> = HashMap::new();
        for &(uni, vni) in VNI_TABLE {
            let mut it = vni.chars();
            let first = it.next().expect("VNI_TABLE entry rỗng");
            match it.next() {
                None => {
                    single.insert(first, uni);
                }
                Some(second) => {
                    pair.insert((first, second), uni);
                }
            }
        }
        (single, pair)
    });

    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() {
            if let Some(&uni) = pair.get(&(chars[i], chars[i + 1])) {
                out.push(uni);
                i += 2;
                continue;
            }
        }
        match single.get(&chars[i]) {
            Some(&uni) => out.push(uni),
            None => out.push(chars[i]),
        }
        i += 1;
    }
    out
}

fn unicode_to_tcvn3(s: &str) -> String {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    let map = MAP.get_or_init(|| TCVN3_TABLE.iter().copied().collect());
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if let Some(&b) = map.get(&c) {
            out.push(b);
            continue;
        }
        if c.is_uppercase() {
            // Chữ hoa có dấu thanh không có byte riêng trong TCVN3 —
            // fallback best-effort dùng byte của chữ thường tương ứng
            // (xem doc comment đầu file).
            if let Some(lower) = c.to_lowercase().next() {
                if let Some(&b) = map.get(&lower) {
                    out.push(b);
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

fn tcvn3_to_unicode(s: &str) -> String {
    static REV: OnceLock<HashMap<char, char>> = OnceLock::new();
    let rev = REV.get_or_init(|| TCVN3_TABLE.iter().map(|&(uni, b)| (b, uni)).collect());
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        out.push(*rev.get(&c).unwrap_or(&c));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vni_roundtrip_sentences() {
        for text in [
            "Tôi yêu tiếng nước tôi từ khi mới ra đời",
            "Việt Nam Nguyễn Đông trường đại học",
        ] {
            let vni = convert(text, Encoding::Unicode, Encoding::VniWindows);
            let back = convert(&vni, Encoding::VniWindows, Encoding::Unicode);
            assert_eq!(back, text, "VNI round-trip lệch cho: {text}");
        }
    }

    #[test]
    fn tcvn3_roundtrip_lowercase_sentence() {
        let text = "tôi yêu tiếng nước tôi từ khi mới ra đời";
        let tcvn3 = convert(text, Encoding::Unicode, Encoding::Tcvn3);
        let back = convert(&tcvn3, Encoding::Tcvn3, Encoding::Unicode);
        assert_eq!(back, text);
    }

    /// Đối chiếu trực tiếp với nguồn: bảng "VNI 'ANSI' Encoding
    /// (Windows/Unix)" — Wikipedia "VNI" — cho ra các mẫu mojibake quen
    /// thuộc khi mở văn bản VNI cũ sai encoding.
    #[test]
    fn vni_known_samples() {
        assert_eq!(convert("Việt", Encoding::Unicode, Encoding::VniWindows), "Vieät");
        assert_eq!(convert("Nguyễn", Encoding::Unicode, Encoding::VniWindows), "Nguyeãn");
        assert_eq!(convert("trường", Encoding::Unicode, Encoding::VniWindows), "tröôøng");
        assert_eq!(convert("đồng", Encoding::Unicode, Encoding::VniWindows), "ñoàng");
        // Mẫu quen thuộc khác, độc lập với đề bài, đối chiếu cùng nguồn.
        assert_eq!(convert("Ngày", Encoding::Unicode, Encoding::VniWindows), "Ngaøy");
        assert_eq!(convert("Thứ Năm", Encoding::Unicode, Encoding::VniWindows), "Thöù Naêm");
    }

    #[test]
    fn non_vietnamese_chars_untouched() {
        let text = "Hello, World! 123 - test_case@example.com <tag/>";
        for (from, to) in [
            (Encoding::Unicode, Encoding::VniWindows),
            (Encoding::Unicode, Encoding::Tcvn3),
            (Encoding::VniWindows, Encoding::Unicode),
            (Encoding::Tcvn3, Encoding::Unicode),
        ] {
            assert_eq!(convert(text, from, to), text);
        }
    }

    #[test]
    fn tcvn3_uppercase_shape_letters_roundtrip() {
        // 7 chữ hoa duy nhất có byte riêng trong TCVN3 — round-trip chính xác.
        let text = "Ă Â Ê Ô Ơ Ư Đ";
        let tcvn3 = convert(text, Encoding::Unicode, Encoding::Tcvn3);
        assert_eq!(convert(&tcvn3, Encoding::Tcvn3, Encoding::Unicode), text);
    }

    #[test]
    fn tcvn3_uppercase_tone_marked_is_lossy_best_effort() {
        // Chữ hoa có dấu thanh (Ệ) không có byte riêng trong TCVN3 —
        // fallback dùng byte chữ thường tương ứng (mất hoa/thường cho
        // riêng phần dấu thanh, đúng như doc comment). Đ thì có byte hoa
        // riêng (1 trong 7 chữ "gốc") nên round-trip chính xác.
        let text = "ĐIỆN";
        let tcvn3 = convert(text, Encoding::Unicode, Encoding::Tcvn3);
        assert_eq!(convert(&tcvn3, Encoding::Tcvn3, Encoding::Unicode), "ĐIệN");
    }
}
