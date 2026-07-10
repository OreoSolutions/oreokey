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
}

impl Engine {
    pub fn new(cfg: EngineConfig) -> Engine {
        Engine {
            cfg,
            macros: macros::MacroTable::default(),
            raw: String::new(),
            last_render: String::new(),
            raw_mode: false,
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
        // Trạng thái TRƯỚC khi thêm c đã "sạch đã chốt" chưa: đang hiển thị
        // bình thường (không ở raw_mode) và không mang biến đổi nào — dấu
        // đã bị hủy hết hoặc chưa từng có. Ví dụ "mas" (đã hủy sắc của má).
        let was_settled = !self.raw_mode && !self.is_transformed(&self.raw);
        self.raw.push(c);
        let new_render = if self.raw_mode {
            self.raw.clone()
        } else {
            let (text, restored) = self.render_word(&self.raw);
            if restored {
                // Chỉ khóa khi từ CHẾT hẳn (cụm bất khả). Trạng thái CÒN
                // SỐNG (tiền tố hợp lệ, vd nhân âm dở chờ dấu mũ) giữ raw
                // hiển thị nhưng KHÔNG khóa — phím sau còn cơ hội hoàn
                // thiện âm tiết (issue #4).
                let state = self.build_state(&self.raw);
                if !spell::is_live_prefix(&state) {
                    self.raw_mode = true;
                }
                // Từ đang sạch mà thêm một phím dấu làm nó không hợp lệ:
                // chỉ nên rơi phím đó xuống thành ký tự thường, KHÔNG bung
                // lại raw (raw còn giữ cả phím dấu ĐÃ HỦY — bung ra sẽ làm
                // chữ đã hủy hiện lại: "mas" + f phải là "masf", không phải
                // "massf"). Khác hẳn khôi phục từ tiếng Anh ("asdf") vốn
                // hoàn tác một dấu ĐANG hoạt động — lúc đó was_settled=false.
                if was_settled {
                    let mut kept = self.last_render.clone();
                    kept.push(c);
                    // Đồng bộ raw với đúng phần đang hiển thị để backspace
                    // không lệch (bỏ lịch sử phím dấu đã hủy).
                    self.raw = kept.clone();
                    kept
                } else {
                    text
                }
            } else {
                text
            }
        };
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
                self.raw.clone()
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
                    let (text, restored) = self.render_word(&self.raw);
                    if !restored && text == self.last_render {
                        self.raw_mode = false;
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

    /// Chuỗi phím gốc có mang biến đổi nào không (dấu thanh / dấu phụ).
    fn is_transformed(&self, raw: &str) -> bool {
        spell::is_transformed(&self.build_state(raw))
    }

    /// Render chuỗi phím gốc thành văn bản hiển thị. Cờ thứ hai báo
    /// spell check đã phải khôi phục phím gốc (từ không phải tiếng Việt).
    fn render_word(&self, raw: &str) -> (String, bool) {
        if raw.is_empty() {
            return (String::new(), false);
        }
        let state = self.build_state(raw);
        // Từ bị biến đổi nhưng không phải âm tiết chấp nhận được → trả phím gốc.
        // Thoải mái (Loose) không bao giờ khôi phục; Chặt/Thường dùng gate.
        if self.cfg.spell_mode != SpellMode::Loose
            && spell::is_transformed(&state)
            && !spell::is_acceptable(&state, self.cfg.spell_mode == SpellMode::Standard)
        {
            return (raw.to_string(), true);
        }
        (render_letters(&state, self.cfg.modern_tone), false)
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
