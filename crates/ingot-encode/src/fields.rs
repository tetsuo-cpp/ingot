//! Bit-field manipulation helpers for ARM64 instruction encoding.

/// Set bits `[hi:lo]` of `inst` to `value`.
///
/// # Panics
/// Debug-asserts that `value` fits in the field width.
#[inline]
pub fn set_field(inst: u32, hi: u32, lo: u32, value: u32) -> u32 {
    let width = hi - lo + 1;
    debug_assert!(
        value < (1 << width),
        "value {value:#x} does not fit in {width}-bit field [{hi}:{lo}]"
    );
    let mask = ((1u32 << width) - 1) << lo;
    (inst & !mask) | ((value << lo) & mask)
}

/// Set Rd field (bits [4:0]).
#[inline]
pub fn set_rd(inst: u32, rd: u8) -> u32 {
    set_field(inst, 4, 0, rd as u32)
}

/// Set Rn field (bits [9:5]).
#[inline]
pub fn set_rn(inst: u32, rn: u8) -> u32 {
    set_field(inst, 9, 5, rn as u32)
}

/// Set Rm field (bits [20:16]).
#[inline]
pub fn set_rm(inst: u32, rm: u8) -> u32 {
    set_field(inst, 20, 16, rm as u32)
}

/// Set sf bit (bit 31) — 1 for 64-bit, 0 for 32-bit.
#[inline]
pub fn set_sf(inst: u32, is_64: bool) -> u32 {
    if is_64 {
        inst | (1 << 31)
    } else {
        inst & !(1 << 31)
    }
}

/// Set Rt field (bits [4:0]) — same position as Rd, used for load/store.
#[inline]
pub fn set_rt(inst: u32, rt: u8) -> u32 {
    set_rd(inst, rt)
}

/// Set Rt2 field (bits [14:10]) — second register for LDP/STP.
#[inline]
pub fn set_rt2(inst: u32, rt2: u8) -> u32 {
    set_field(inst, 14, 10, rt2 as u32)
}

/// Truncate a signed value to `width` bits (mask off upper bits).
#[inline]
pub fn truncate_imm(value: i64, width: u32) -> u32 {
    (value as u32) & ((1u32 << width) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_field_basic() {
        let inst = set_field(0, 4, 0, 0b11111);
        assert_eq!(inst, 0b11111);
    }

    #[test]
    fn set_field_middle() {
        let inst = set_field(0, 9, 5, 0b11111);
        assert_eq!(inst, 0b11111 << 5);
    }

    #[test]
    fn set_field_preserves_other_bits() {
        let inst = set_field(0xFFFF_FFFF, 4, 0, 0);
        assert_eq!(inst, 0xFFFF_FFE0);
    }

    #[test]
    fn set_rd_rn_rm() {
        let inst = set_rd(0, 5);
        assert_eq!(inst & 0x1F, 5);

        let inst = set_rn(0, 10);
        assert_eq!((inst >> 5) & 0x1F, 10);

        let inst = set_rm(0, 20);
        assert_eq!((inst >> 16) & 0x1F, 20);
    }

    #[test]
    fn truncate_imm_negative() {
        // -1 truncated to 9 bits = 0x1FF
        assert_eq!(truncate_imm(-1, 9), 0x1FF);
        // -16 truncated to 7 bits = 0x70 (112)
        assert_eq!(truncate_imm(-16, 7), (-16i64 as u32) & 0x7F);
    }
}
