//! Lọc "phím bóng ma": bản sao keydown do WindowServer giao lại khi
//! callback của event tap xử lý chậm.
//!
//! Cơ chế gốc (đã kiểm chứng bằng log thực địa): mỗi phím kích hoạt một
//! `Action::Replace` (gõ dấu, hủy dấu) khiến callback phải bơm phím / gọi
//! AX — chậm tới mức macOS coi như tap treo và GIAO LẠI đúng keydown đó.
//! Bản sao mang `srcpid=0`, `magic=false` (không phân biệt được với phím
//! thật), nhưng timestamp PHẦN CỨNG của nó chỉ chênh bản gốc vài–vài chục
//! ms, trong khi người gõ lặp cùng một phím nhanh nhất cũng cách ≥130ms
//! (dữ liệu thực địa: bóng ma ≤33ms, gõ lặp có chủ đích 130–222ms). Vì
//! vậy chỉ cần một cửa sổ thời gian đủ hẹp là tách sạch hai loại.
//!
//! BÀI HỌC XƯƠNG MÁU: `CGEventGetTimestamp` trả về mach absolute time
//! TICK, KHÔNG phải nanosecond. Trên Apple Silicon 1 tick ≈ 41,67ns nên
//! một cửa sổ "30 triệu" tưởng là 30ms hóa ra ~1,25 GIÂY — từng nuốt oan
//! cả phím hủy dấu ("gõ 3 lần s mới hủy được"). Struct này cố tình KHÔNG
//! quy đổi đơn vị: nó nhận cửa sổ và timestamp cùng một đơn vị do lời gọi
//! quyết định (tick ngoài đời, hoặc "ms giả lập" trong test), nên logic
//! thời gian kiểm thử được độc lập với phần cứng.

use std::collections::VecDeque;

/// Cửa sổ nhận diện bóng ma, tính bằng nano-giây: 70ms. Dữ liệu thực địa:
/// bóng ma đến ≤33ms sau bản gốc, gõ lặp có chủ đích ≥130ms. 70ms nằm
/// giữa với biên an toàn cả hai phía.
const WINDOW_NS: u64 = 70_000_000;

/// Quy 70ms ra đơn vị của `CGEventGetTimestamp` (mach absolute time tick)
/// bằng `mach_timebase_info`. `ns = tick * numer / denom` ⇒
/// `tick = ns * denom / numer`. Trên Apple Silicon numer/denom cho
/// ~41,67ns mỗi tick nên 70ms ≈ 1,68 triệu tick.
#[cfg(target_os = "macos")]
pub fn window_ticks() -> u64 {
    #[repr(C)]
    struct MachTimebaseInfo {
        numer: u32,
        denom: u32,
    }
    extern "C" {
        fn mach_timebase_info(info: *mut MachTimebaseInfo) -> libc::c_int;
    }
    let mut info = MachTimebaseInfo { numer: 0, denom: 0 };
    if unsafe { mach_timebase_info(&mut info) } != 0 || info.numer == 0 {
        // Không lấy được timebase: giả định 1 tick = 1ns (cửa sổ = 70ms
        // theo ns) — an toàn hơn là để cửa sổ khổng lồ.
        return WINDOW_NS;
    }
    WINDOW_NS * u64::from(info.denom) / u64::from(info.numer)
}

/// Số phím bị nuốt gần đây cần nhớ. Phải >1: khi gõ `ss` hủy dấu, bóng ma
/// của `s` thứ nhất có thể đến SAU `s` thứ hai thật — nhớ một phím là lọt.
const RING_CAP: usize = 16;

/// Bộ lọc bóng ma: ghi lại các phím ĐÃ BỊ NUỐT (Replace → Drop) rồi nhận
/// diện bản sao đến ngay sau đó với cùng keycode.
#[derive(Debug)]
pub struct GhostGuard {
    /// (keycode, timestamp phần cứng) của các phím vừa bị nuốt.
    recent: VecDeque<(u16, u64)>,
    /// Khoảng cách tối đa (cùng đơn vị với timestamp) để coi là bản sao.
    window: u64,
}

impl GhostGuard {
    pub fn new(window: u64) -> GhostGuard {
        GhostGuard {
            recent: VecDeque::with_capacity(RING_CAP),
            window,
        }
    }

    /// Ghi nhận một phím vừa bị nuốt (đã trả `Drop` vì Replace).
    pub fn record_drop(&mut self, keycode: u16, ts: u64) {
        self.recent.push_back((keycode, ts));
        if self.recent.len() > RING_CAP {
            self.recent.pop_front();
        }
    }

    /// `ts` có phải bản sao của một phím vừa bị nuốt không? Bản sao luôn
    /// đến SAU bản gốc (`ts >= dropped_ts`) và trong cửa sổ hẹp. Chỉ so
    /// một chiều để không bao giờ nuốt nhầm một phím đến TRƯỚC.
    pub fn is_ghost(&self, keycode: u16, ts: u64) -> bool {
        self.recent.iter().any(|&(code, dropped_ts)| {
            code == keycode && ts >= dropped_ts && ts - dropped_ts <= self.window
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Đơn vị test: quy ước "1 đơn vị = 1ms" để đọc cho dễ; cửa sổ 70ms.
    const WINDOW_MS: u64 = 70;
    fn guard() -> GhostGuard {
        GhostGuard::new(WINDOW_MS)
    }

    #[test]
    fn ghost_ngay_sau_ban_goc_bi_nuot() {
        // Dữ liệu thực địa: bóng ma đến +10,5ms sau phím bị nuốt.
        let mut g = guard();
        g.record_drop(1, 1000);
        assert!(g.is_ghost(1, 1010), "bóng ma +10ms phải bị bắt");
    }

    #[test]
    fn go_lap_co_chu_dich_khong_bi_nuot() {
        // Dữ liệu thực địa: phím hủy dấu thật đến +222ms — KHÔNG phải bóng ma.
        let mut g = guard();
        g.record_drop(1, 1000);
        assert!(!g.is_ghost(1, 1222), "gõ lặp 222ms là chủ đích, phải cho qua");
    }

    #[test]
    fn ranh_gioi_cua_so() {
        let mut g = guard();
        g.record_drop(1, 1000);
        assert!(g.is_ghost(1, 1000 + WINDOW_MS), "đúng mép cửa sổ vẫn là bóng ma");
        assert!(!g.is_ghost(1, 1000 + WINDOW_MS + 1), "quá mép thì cho qua");
    }

    #[test]
    fn khac_keycode_khong_lien_quan() {
        let mut g = guard();
        g.record_drop(1, 1000);
        assert!(!g.is_ghost(17, 1005), "phím khác (t) không bị nuốt vì s");
    }

    #[test]
    fn khong_nuot_phim_den_truoc() {
        // Bản sao chỉ đến sau; một phím có ts nhỏ hơn không thể là bóng ma.
        let mut g = guard();
        g.record_drop(1, 2000);
        assert!(!g.is_ghost(1, 1990));
    }

    #[test]
    fn ring_giu_bong_ma_cua_phim_dau_khi_go_ss() {
        // Kịch bản `ss` hủy dấu: s#1 nuốt, s#2 thật nuốt, rồi bóng ma của
        // s#1 đến muộn — ring nhiều slot phải vẫn bắt được.
        let mut g = guard();
        g.record_drop(1, 1000); // s#1
        g.record_drop(1, 1222); // s#2 thật (đã cho qua ở tầng trên, cũng bị nuốt vì Replace)
        // bóng ma của s#1, timestamp ~ +12ms so với s#1:
        assert!(g.is_ghost(1, 1012), "bóng ma của s#1 vẫn phải bị bắt sau khi s#2 vào ring");
    }

    #[test]
    fn ring_khong_tran_qua_16() {
        let mut g = guard();
        for i in 0..20u64 {
            g.record_drop(2, 100 + i * 1000);
        }
        assert_eq!(g.recent.len(), RING_CAP);
    }

    #[test]
    fn chuoi_thuc_dia_master_va_huy_dau() {
        // Replay đúng chuỗi 's' từ log: gốc, thật(+222ms), ma(+10,5ms).
        // Đơn vị test đổi sang "phần chục micro giây" để giữ số nguyên gần
        // giống tick thật mà vẫn đọc được — dùng ns cho rõ, cửa sổ 70ms.
        let mut g = GhostGuard::new(70_000_000); // 70ms tính bằng ns
        let s1 = 1_313_375_026_515u64;
        let s2 = 1_313_597_026_989u64; // +222,000,474 ns
        let ghost = 1_313_607_558_963u64; // +10,531,974 ns so với s2
        g.record_drop(1, s1);
        assert!(!g.is_ghost(1, s2), "s#2 (+222ms) là gõ thật");
        g.record_drop(1, s2);
        assert!(g.is_ghost(1, ghost), "s#3 (+10,5ms) là bóng ma");
    }
}
