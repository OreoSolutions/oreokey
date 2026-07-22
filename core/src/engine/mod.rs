//! Engine gõ tiếng Việt thuần túy: không import gì ngoài `std`.
//!
//! Kiến trúc re-render + diff: giữ chuỗi phím gốc (`raw`) của từ đang gõ,
//! mỗi phím render lại toàn bộ từ rồi so với lần render trước để tạo
//! `Action` sửa chữ tối thiểu.

pub mod censor;
pub mod encoding;
pub mod macros;
pub mod spell;
pub mod syllable;
pub mod telex;
pub mod vni;

use syllable::render_letters;

/// Hành động engine yêu cầu tầng platform thực hiện.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Cho phím gốc đi qua, không can thiệp.
    PassThrough,
    /// Nuốt phím gốc, thay `old` (đoạn cuối từ đang hiển thị) bằng `text`.
    /// `old` cho phép tầng platform XÁC MINH văn bản trước caret trước
    /// khi sửa qua AX — chống race khi ký tự passthrough chưa kịp vào app.
    Replace { old: String, text: String },
}

impl Action {
    /// Số ký tự cần xóa khi sửa bằng backspace.
    pub fn backspaces(&self) -> usize {
        match self {
            Action::PassThrough => 0,
            Action::Replace { old, .. } => old.chars().count(),
        }
    }
}

/// Phím đầu vào đã được tầng platform chuẩn hóa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyInput {
    /// Ký tự in được (a-z, A-Z, 0-9...).
    Char(char),
    Backspace,
    /// Ký tự ngắt từ (space, dấu câu, Enter) hoặc None nếu là di chuyển
    /// con trỏ / click chuột (chỉ reset, không có ký tự đi kèm).
    WordBreak(Option<char>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingMethod {
    Telex,
    Vni,
}

/// Mức kiểm tra chính tả: Chặt (bảo vệ tối đa tiếng Anh) → Thường (gõ
/// tắt, vẫn bắt cụm bất khả) → Thoải mái (không khôi phục, luôn đặt dấu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellMode {
    Strict,
    Standard,
    Loose,
}

/// Thanh điệu (không tính thanh ngang).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Acute, // sắc
    Grave, // huyền
    Hook,  // hỏi
    Tilde, // ngã
    Dot,   // nặng
}

/// Một chữ cái trong từ đang gõ cùng các dấu phụ (trừ thanh điệu —
/// thanh là thuộc tính cấp từ).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Letter {
    /// Chữ cái gốc ascii thường (a-z, 0-9).
    pub base: char,
    pub upper: bool,
    /// Dấu mũ: â ê ô.
    pub circ: bool,
    /// Dấu móc: ơ ư.
    pub horn: bool,
    /// Dấu trăng: ă.
    pub breve: bool,
    /// Gạch ngang: đ.
    pub stroke: bool,
    /// Chữ `ư` sinh từ phím `w` đứng một mình (Telex) — khi hủy phải
    /// hoàn về `w` chứ không phải `u`.
    pub w_origin: bool,
}

impl Letter {
    pub fn plain(c: char) -> Letter {
        Letter {
            base: c.to_ascii_lowercase(),
            upper: c.is_ascii_uppercase(),
            circ: false,
            horn: false,
            breve: false,
            stroke: false,
            w_origin: false,
        }
    }

    pub fn is_vowel(&self) -> bool {
        matches!(self.base, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
    }

    pub fn has_mark(&self) -> bool {
        self.circ || self.horn || self.breve
    }
}

/// Trạng thái từ đang gõ, dựng lại từ đầu sau mỗi phím.
#[derive(Debug, Clone, Default)]
pub struct WordState {
    pub letters: Vec<Letter>,
    pub tone: Option<Tone>,
    /// Các phím modifier đã bị hủy bằng cách gõ lặp — trở thành chữ
    /// thường cho tới hết từ (vd `ass` → `as`, gõ thêm `s` → `ass`).
    pub dead: Vec<char>,
}

impl WordState {
    pub fn is_dead(&self, c: char) -> bool {
        self.dead.contains(&c)
    }

    /// Phím thanh — logic chung Telex (s f r x j) và VNI (1-5):
    /// chưa có nguyên âm → phím rơi thành chữ thường; gõ lặp cùng thanh
    /// → hủy và phím đó chết tới hết từ; khác → thay thanh cũ.
    pub(crate) fn apply_tone_key(&mut self, tone: Tone, c: char) {
        if !self.has_vowel() {
            self.letters.push(Letter::plain(c));
        } else if self.tone == Some(tone) {
            self.tone = None;
            self.dead.push(c.to_ascii_lowercase());
            self.letters.push(Letter::plain(c));
        } else {
            self.tone = Some(tone);
        }
    }

    /// Phím xóa thanh (Telex `z`, VNI `0`): không có thanh → chữ thường.
    pub(crate) fn apply_tone_clear(&mut self, c: char) {
        if self.tone.is_some() {
            self.tone = None;
        } else {
            self.letters.push(Letter::plain(c));
        }
    }

    pub fn has_vowel(&self) -> bool {
        self.letters.iter().any(|l| l.is_vowel())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub method: TypingMethod,
    pub spell_mode: SpellMode,
    /// Kiểu đặt dấu mới (`hoà`) thay vì kiểu cũ (`hòa`).
    pub modern_tone: bool,
    pub macros_enabled: bool,
    /// Gõ dấu mũ muộn (Telex): `nanag` → `nâng`, `viete` → `viêt`.
    /// Chỉ áp khi kết quả là âm tiết hợp lệ để không phá từ tiếng Anh.
    pub flexible_marks: bool,
    /// Che từ tục tĩu bằng dấu * khi chốt từ.
    pub censor_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: true,
            flexible_marks: true,
            censor_enabled: false,
        }
    }
}

pub struct Engine {
    cfg: EngineConfig,
    macros: macros::MacroTable,
    /// Chuỗi phím gốc của từ hiện tại.
    raw: String,
    /// Văn bản từ hiện tại như app đích đang hiển thị.
    last_render: String,
    /// Từ hiện tại đã bị phát hiện không phải tiếng Việt — hiển thị
    /// nguyên phím gốc cho tới khi ngắt từ (tránh nuốt phím dấu khi
    /// người dùng gõ tiếng nước ngoài).
    raw_mode: bool,
    /// Với mức Thường/Thoải mái: một âm tiết đã hợp lệ được giữ nguyên khi
    /// phần đuôi mới làm nó thành cụm bất khả. `raw` vẫn giữ phím thật để
    /// Backspace có thể quay lại engine bình thường ngay khi xóa hết đuôi.
    frozen_prefix: Option<FrozenPrefix>,
}

#[derive(Debug, Clone)]
struct FrozenPrefix {
    render: String,
    raw_len: usize,
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Engine {
        Engine {
            cfg,
            macros: macros::MacroTable::default(),
            raw: String::new(),
            last_render: String::new(),
            raw_mode: false,
            frozen_prefix: None,
        }
    }

    pub fn set_config(&mut self, cfg: EngineConfig) {
        self.cfg = cfg;
        self.reset();
    }

    pub fn config(&self) -> &EngineConfig {
        &self.cfg
    }

    pub fn set_macros(&mut self, table: macros::MacroTable) {
        self.macros = table;
    }

    /// Chốt/hủy từ hiện tại (đổi app, click chuột, di chuyển con trỏ...).
    pub fn reset(&mut self) {
        self.raw.clear();
        self.last_render.clear();
        self.raw_mode = false;
        self.frozen_prefix = None;
    }

    /// Từ đang gõ như đang hiển thị (phục vụ test/debug).
    pub fn current_word(&self) -> &str {
        &self.last_render
    }

    pub fn on_key(&mut self, k: KeyInput) -> Action {
        match k {
            KeyInput::Char(c) => self.on_char(c),
            KeyInput::Backspace => self.on_backspace(),
            KeyInput::WordBreak(ch) => {
                let action = match ch {
                    // Chỉ biến đổi khi chốt từ bằng một ký tự thật (space,
                    // dấu câu, Enter) — không phải di chuyển con trỏ.
                    Some(break_ch) if !self.last_render.is_empty() => {
                        // Gõ tắt trước, che từ tục sau.
                        let replacement = self
                            .cfg
                            .macros_enabled
                            .then(|| self.macros.expand(&self.last_render))
                            .flatten()
                            .map(str::to_string)
                            .or_else(|| {
                                (self.cfg.censor_enabled
                                    && censor::is_profane(&self.last_render))
                                .then(|| censor::mask(&self.last_render))
                            });
                        match replacement {
                            Some(mut text) => {
                                text.push(break_ch);
                                Action::Replace {
                                    old: self.last_render.clone(),
                                    text,
                                }
                            }
                            None => Action::PassThrough,
                        }
                    }
                    _ => Action::PassThrough,
                };
                self.reset();
                action
            }
        }
    }

    fn on_char(&mut self, c: char) -> Action {
        let raw_len_before = self.raw.chars().count();
        // Chặt giữ nguyên cơ chế cũ. Ở hai mức dưới, chỉ bảo toàn khi phần
        // trước đã là một âm tiết có biến đổi hợp lệ; nhờ vậy từ tiếng Anh
        // thuần không vô tình được "đóng băng".
        let keep_completed_prefix = !self.raw_mode && self.cfg.spell_mode != SpellMode::Strict && {
            let previous = self.build_state(&self.raw);
            // Phím a/e/o lặp lại vẫn phải được quyền hủy dấu mũ Telex, kể
            // cả khi trạng thái trước đó tình cờ là một âm tiết hợp lệ.
            // Ví dụ `data` đang render thành `dâta`; phím a tiếp theo phải
            // hủy mũ thành `dataa`, không được biến thành đuôi literal của
            // frozen-prefix. `yêu` + u không phải thao tác hủy nên vẫn giữ.
            let cancels_circumflex = self.cfg.spell_mode == SpellMode::Standard
                && self.cfg.method == TypingMethod::Telex
                && matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'o')
                && previous.letters.iter().any(|letter| {
                    letter.base == c.to_ascii_lowercase() && letter.circ
                });
            !cancels_circumflex
                && spell::is_transformed(&previous)
                && spell::is_acceptable(&previous, false)
        };
        self.raw.push(c);
        let new_render = if self.raw_mode {
            self.raw_mode_text()
        } else {
            let (text, restored, state) = self.render_word(&self.raw);
            if keep_completed_prefix && !spell::is_acceptable(&state, true) {
                // Ví dụ: "đô" + u → "đôu", "yêu" + u → "yêuu".
                // Sau đó phần đuôi đi qua nguyên văn, nhưng vẫn giữ raw để
                // Backspace về đúng trạng thái trước khi đóng băng.
                self.raw_mode = true;
                self.frozen_prefix = Some(FrozenPrefix {
                    render: self.last_render.clone(),
                    raw_len: raw_len_before,
                });
                self.raw_mode_text()
            } else {
                if restored {
                    // Chỉ khóa khi từ CHẾT hẳn (cụm bất khả). Trạng thái CÒN
                    // SỐNG (tiền tố hợp lệ, vd nhân âm dở chờ dấu mũ) giữ văn
                    // bản khôi phục nhưng KHÔNG khóa — phím sau còn cơ hội hoàn
                    // thiện âm tiết (issue #4).
                    // is_live_prefix cố ý KHÔNG xét thanh điệu (tone-blind) — nhờ
                    // vậy một nhân âm đã có thanh nhưng chưa xong dấu mũ/móc vẫn
                    // được coi là còn sống. Đừng thêm kiểm tra thanh ở đây.
                    if !spell::is_live_prefix(&state) {
                        self.raw_mode = true;
                        self.frozen_prefix = None;
                        // Từ đây raw_mode hiển thị self.raw nguyên văn — đồng bộ
                        // raw với văn bản khôi phục (đã rút phím hủy) để phím sau
                        // và backspace không làm chữ đã hủy hiện lại. Từ còn sống
                        // thì KHÔNG đồng bộ: mất lịch sử phím hủy sẽ khiến replay
                        // tự áp lại dấu đã hủy ("sooos" + c phải ra "soóc").
                        self.raw = text.clone();
                    }
                }
                text
            }
        };
        // Chống bệnh lý: token alnum liền dài (hex/base64 gõ tay, không
        // space) không thể là âm tiết tiếng Việt — khóa raw để mỗi phím
        // sau là O(1) thay vì replay O(n). Đồng bộ raw = văn bản hiển thị
        // (cùng chính sách với nhánh khóa phía trên: giữ bất biến phím đã
        // hủy không hiện lại). Từ tiếng Việt thật không bao giờ chạm cap.
        const RAW_CAP: usize = 64;
        if !self.raw_mode && new_render.chars().count() >= RAW_CAP {
            self.raw_mode = true;
            self.frozen_prefix = None;
            self.raw = new_render.clone();
        }
        let action = diff_action(&self.last_render, &new_render, c);
        self.last_render = new_render;
        action
    }

    fn on_backspace(&mut self) -> Action {
        if self.raw.is_empty() {
            return Action::PassThrough;
        }
        // App sẽ tự xóa 1 ký tự hiển thị; đồng bộ lại raw cho khớp.
        let mut target: Vec<char> = self.last_render.chars().collect();
        target.pop();
        let target: String = target.into_iter().collect();
        while !self.raw.is_empty() {
            self.raw.pop();
            let rendered = if self.raw_mode {
                self.raw_mode_text()
            } else {
                self.render_word(&self.raw).0
            };
            if rendered == target {
                self.last_render = target;
                // Từ đang bị khóa raw (spell check phán không phải tiếng
                // Việt): nếu phần còn lại render sạch và khớp đúng màn
                // hình thì gỡ khóa — người dùng xóa để gõ lại không phải
                // bấm space mới có dấu.
                if self.raw_mode {
                    let frozen_done = self
                        .frozen_prefix
                        .as_ref()
                        .is_some_and(|prefix| self.raw.chars().count() <= prefix.raw_len);
                    if frozen_done {
                        self.raw_mode = false;
                        self.frozen_prefix = None;
                    } else if self.frozen_prefix.is_none() {
                        let (text, restored, _) = self.render_word(&self.raw);
                        if !restored && text == self.last_render {
                            self.raw_mode = false;
                        }
                    }
                }
                return Action::PassThrough;
            }
        }
        // Không khớp được (không nên xảy ra) — bỏ theo dõi từ này.
        self.reset();
        Action::PassThrough
    }

    /// Dựng lại `WordState` từ chuỗi phím gốc theo bộ gõ đang chọn.
    fn build_state(&self, raw: &str) -> WordState {
        let mut state = WordState::default();
        for c in raw.chars() {
            match self.cfg.method {
                TypingMethod::Telex => {
                    telex::apply_key(&mut state, c, self.cfg.flexible_marks)
                }
                TypingMethod::Vni => vni::apply_key(&mut state, c),
            }
        }
        state
    }

    /// Văn bản hiển thị khi đã khóa raw. Với khóa thông thường, nguyên văn
    /// là đúng; với prefix bảo toàn, chỉ phần đuôi sau âm tiết hợp lệ mới là
    /// nguyên văn.
    fn raw_mode_text(&self) -> String {
        let Some(prefix) = &self.frozen_prefix else {
            return self.raw.clone();
        };
        let tail: String = self.raw.chars().skip(prefix.raw_len).collect();
        format!("{}{}", prefix.render, tail)
    }

    /// Văn bản khôi phục cho từ không phải tiếng Việt: phần đầu dài nhất
    /// còn "sạch" (chưa mang biến đổi) hiển thị theo dạng đã render — nhờ
    /// vậy phím hủy dấu đã tiêu KHÔNG bung trở lại ("looo" là "loo") —
    /// phần phím gõ sau đó giữ nguyên văn ("looos" + e → "loose", không
    /// phải "looose"; "mas" + f → "masf", không phải "massf").
    fn restore_text(&self, raw: &str) -> String {
        let chars: Vec<char> = raw.chars().collect();
        let mut state = WordState::default();
        let mut settled = String::new();
        let mut settled_len = 0;
        for (i, &c) in chars.iter().enumerate() {
            match self.cfg.method {
                TypingMethod::Telex => {
                    telex::apply_key(&mut state, c, self.cfg.flexible_marks)
                }
                TypingMethod::Vni => vni::apply_key(&mut state, c),
            }
            if !spell::is_transformed(&state) {
                settled = render_letters(&state, self.cfg.modern_tone);
                settled_len = i + 1;
            }
        }
        settled.extend(&chars[settled_len..]);
        settled
    }

    /// Render chuỗi phím gốc thành văn bản hiển thị. Cờ thứ hai báo
    /// spell check đã phải khôi phục phím gốc (từ không phải tiếng Việt).
    /// Trả kèm `WordState` đã dựng để caller khỏi replay lần nữa.
    fn render_word(&self, raw: &str) -> (String, bool, WordState) {
        if raw.is_empty() {
            return (String::new(), false, WordState::default());
        }
        let state = self.build_state(raw);
        // Từ bị biến đổi nhưng không phải âm tiết chấp nhận được → trả phím gốc.
        // Thoải mái (Loose) không bao giờ khôi phục; Chặt/Thường dùng gate.
        if self.cfg.spell_mode != SpellMode::Loose
            && spell::is_transformed(&state)
            && !spell::is_acceptable(&state, self.cfg.spell_mode == SpellMode::Standard)
        {
            return (self.restore_text(raw), true, state);
        }
        let text = render_letters(&state, self.cfg.modern_tone);
        (text, false, state)
    }
}

/// So hai lần render, tạo action tối thiểu. `typed` là phím vừa gõ —
/// nếu kết quả đúng bằng "thêm phím đó vào cuối" thì cho đi qua.
fn diff_action(last: &str, new: &str, typed: char) -> Action {
    let last_chars: Vec<char> = last.chars().collect();
    let new_chars: Vec<char> = new.chars().collect();
    let mut p = 0;
    while p < last_chars.len() && p < new_chars.len() && last_chars[p] == new_chars[p] {
        p += 1;
    }
    let old: String = last_chars[p..].iter().collect();
    let text: String = new_chars[p..].iter().collect();
    if old.is_empty() && text.chars().count() == 1 && text.chars().next() == Some(typed) {
        return Action::PassThrough;
    }
    Action::Replace { old, text }
}

#[cfg(test)]
pub(crate) mod testutil {
    use super::*;

    /// Chạy chuỗi phím, trả về văn bản cuối như app đích thấy.
    /// `\u{8}` trong `keys` được hiểu là Backspace.
    pub fn type_str(engine: &mut Engine, keys: &str) -> String {
        let mut screen = String::new();
        for c in keys.chars() {
            let action = if c == '\u{8}' {
                engine.on_key(KeyInput::Backspace)
            } else {
                engine.on_key(KeyInput::Char(c))
            };
            match action {
                Action::PassThrough => {
                    if c == '\u{8}' {
                        screen.pop();
                    } else {
                        screen.push(c);
                    }
                }
                Action::Replace { old, text } => {
                    // Bất biến: phần bị thay phải đúng là đuôi màn hình —
                    // đây chính là điều AX verify dựa vào ngoài đời thật.
                    assert!(
                        screen.ends_with(&old),
                        "Replace old={old:?} không khớp đuôi màn hình {screen:?}"
                    );
                    for _ in 0..old.chars().count() {
                        screen.pop();
                    }
                    screen.push_str(&text);
                }
            }
        }
        screen
    }

    /// Dựng `WordState` từ chuỗi phím rồi render, KHÔNG qua tầng spell.
    /// Dùng cho test cơ chế biến đổi telex/vni (chỉ chuỗi phím tiến).
    pub(crate) fn raw_render(
        method: TypingMethod,
        keys: &str,
        modern_tone: bool,
        flexible_marks: bool,
    ) -> String {
        let mut state = WordState::default();
        for c in keys.chars() {
            match method {
                TypingMethod::Telex => telex::apply_key(&mut state, c, flexible_marks),
                TypingMethod::Vni => vni::apply_key(&mut state, c),
            }
        }
        render_letters(&state, modern_tone)
    }
}

#[cfg(test)]
mod tests {
    use super::testutil::type_str;
    use super::*;

    fn telex_no_spell() -> Engine {
        Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: SpellMode::Standard,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        })
    }

    #[test]
    fn plain_letters_pass_through() {
        let mut e = telex_no_spell();
        for c in ['h', 'n'] {
            assert_eq!(e.on_key(KeyInput::Char(c)), Action::PassThrough);
        }
        assert_eq!(e.current_word(), "hn");
    }

    #[test]
    fn word_break_resets() {
        let mut e = telex_no_spell();
        e.on_key(KeyInput::Char('a'));
        e.on_key(KeyInput::WordBreak(Some(' ')));
        assert_eq!(e.current_word(), "");
    }

    #[test]
    fn backspace_syncs_buffer() {
        let mut e = telex_no_spell();
        let screen = type_str(&mut e, "vieet\u{8}\u{8}");
        assert_eq!(screen, "vi");
        assert_eq!(e.current_word(), "vi");
    }

    #[test]
    fn backspace_on_empty_buffer_passes() {
        let mut e = telex_no_spell();
        assert_eq!(e.on_key(KeyInput::Backspace), Action::PassThrough);
    }

    fn engine_mode(mode: SpellMode) -> Engine {
        Engine::new(EngineConfig {
            method: TypingMethod::Telex,
            spell_mode: mode,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        })
    }

    #[test]
    fn loose_never_restores_english() {
        // Thoải mái: KHÔNG khôi phục — từ có dấu vẫn giữ dấu.
        let mut e = engine_mode(SpellMode::Loose);
        assert_eq!(type_str(&mut e, "mask"), "mák"); // strict sẽ bung "mask"
        let mut e = engine_mode(SpellMode::Loose);
        assert_eq!(type_str(&mut e, "class"), "clas"); // s hủy s (ass→as), không bung raw
    }

    #[test]
    fn strict_still_restores_english() {
        let mut e = engine_mode(SpellMode::Strict);
        assert_eq!(type_str(&mut e, "mask"), "mask");
        assert_eq!(type_str(&mut e, "class"), "class");
    }

    #[test]
    fn strict_keeps_telex_mark_cancellation() {
        let mut e = engine_mode(SpellMode::Strict);
        // `goo` tạo "gô"; thêm o là thao tác hủy mũ Telex → "goo".
        assert_eq!(type_str(&mut e, "gooo"), "goo");
    }

    #[test]
    fn relaxed_modes_keep_a_completed_syllable_before_literal_tail() {
        for mode in [SpellMode::Standard, SpellMode::Loose] {
            let mut e = engine_mode(mode);
            assert_eq!(type_str(&mut e, "ddoouuuu"), "đôuuuu");

            let mut e = engine_mode(mode);
            assert_eq!(type_str(&mut e, "ddoonguuuu"), "đônguuuu");

            let mut e = engine_mode(mode);
            assert_eq!(type_str(&mut e, "yeeuuuuu"), "yêuuuuu");

            let mut e = engine_mode(mode);
            assert_eq!(type_str(&mut e, "chaofuuuu"), "chàouuuu");

            let mut e = engine_mode(mode);
            assert_eq!(type_str(&mut e, "chafoooooo"), "chàoooooo");
        }
    }

    #[test]
    fn standard_cancels_circumflex_but_keeps_a_literal_vowel_tail() {
        let mut e = engine_mode(SpellMode::Standard);
        assert_eq!(type_str(&mut e, "dataaaa"), "dataaa");

        let mut e = engine_mode(SpellMode::Standard);
        assert_eq!(type_str(&mut e, "yeeuuu"), "yêuuu");
    }

    #[test]
    fn deleting_literal_tail_reenables_relaxed_typing() {
        let mut e = engine_mode(SpellMode::Standard);
        assert_eq!(type_str(&mut e, "ddoou\u{8}ng"), "đông");
    }

    #[test]
    fn relaxed_literal_tail_also_works_in_vni() {
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Vni,
            spell_mode: SpellMode::Standard,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        assert_eq!(type_str(&mut e, "d9o6uuuu"), "đôuuuu");
    }

    #[test]
    fn vni_tone_before_circumflex_midsyllable() {
        // Issue #4: gõ số thanh trước số mũ giữa âm tiết không được kẹt raw.
        let mut e = Engine::new(EngineConfig {
            method: TypingMethod::Vni,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        assert_eq!(type_str(&mut e, "thie16u"), "thiếu");
        let mut e2 = Engine::new(EngineConfig {
            method: TypingMethod::Vni,
            spell_mode: SpellMode::Strict,
            modern_tone: false,
            macros_enabled: false,
            flexible_marks: true,
            censor_enabled: false,
        });
        assert_eq!(type_str(&mut e2, "tie61t"), "tiết");
    }

    #[test]
    fn english_still_restored_after_live_prefix_fix() {
        // Trạng thái còn-sống KHÔNG được phá auto-restore tiếng Anh (telex).
        let mut e = engine_mode(SpellMode::Strict);
        assert_eq!(type_str(&mut e, "dies"), "dies");
        let mut e = engine_mode(SpellMode::Strict);
        assert_eq!(type_str(&mut e, "lies"), "lies");
        let mut e = engine_mode(SpellMode::Strict);
        assert_eq!(type_str(&mut e, "class"), "class"); // dead-cluster latch ngay
    }

    #[test]
    fn issue4_fix_behavior_is_uniform_across_methods() {
        // Issue #4 sửa cho CẢ hai bộ gõ (không phân biệt VNI/Telex): trạng
        // thái còn-sống (tiền tố hợp lệ chờ hoàn thiện âm tiết) không bị
        // khóa raw_mode nữa. Test này khóa lại quyết định: giữ nguyên đồng
        // nhất giữa hai bộ gõ, chấp nhận đánh đổi hẹp ở tiếng Anh (telex).
        let strict = |method: TypingMethod| {
            Engine::new(EngineConfig {
                method,
                spell_mode: SpellMode::Strict,
                modern_tone: false,
                macros_enabled: false,
                flexible_marks: true,
                censor_enabled: false,
            })
        };

        // THẮNG (cả hai bộ gõ phải qua): số thanh trước số mũ (VNI) và dấu
        // mũ muộn (Telex) đều hoàn thiện đúng âm tiết, không kẹt raw.
        let mut vni_engine = strict(TypingMethod::Vni);
        assert_eq!(type_str(&mut vni_engine, "thie16u"), "thiếu");
        let mut telex_engine = strict(TypingMethod::Telex);
        assert_eq!(type_str(&mut telex_engine, "thieesu"), "thiếu");

        // CÁI GIÁ ĐÃ CHẤP NHẬN (KHÔNG PHẢI BUG — đừng "sửa" các assert dưới
        // đây): vì fix áp dụng đồng nhất cho cả hai bộ gõ, một số từ tiếng
        // Anh hiếm gặp trong telex ("diese", "liese") giờ hoàn thiện thành
        // âm tiết tiếng Việt hợp lệ thay vì được khôi phục nguyên phím gốc.
        // Đây là đánh đổi đã được người dùng chấp nhận có ý thức để đổi lấy
        // việc sửa VNI "thie16u" → "thiếu". Từ thông dụng không bị ảnh hưởng.
        let mut telex_engine = strict(TypingMethod::Telex);
        assert_eq!(type_str(&mut telex_engine, "diese"), "diế");
        let mut telex_engine = strict(TypingMethod::Telex);
        assert_eq!(type_str(&mut telex_engine, "liese"), "liế");
    }
}
