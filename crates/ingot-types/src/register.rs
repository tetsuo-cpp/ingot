use std::fmt;

/// Width of a general-purpose register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegWidth {
    /// 32-bit (W0-W30)
    W32,
    /// 64-bit (X0-X30)
    X64,
}

/// A general-purpose register: number (0-30) + width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpReg {
    pub number: u8,
    pub width: RegWidth,
}

impl GpReg {
    pub fn new(number: u8, width: RegWidth) -> Self {
        debug_assert!(number <= 30, "GP register number must be 0-30");
        Self { number, width }
    }

    /// The 5-bit encoding (same as number for GP regs).
    pub fn encoding(&self) -> u8 {
        self.number
    }
}

impl fmt::Display for GpReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.width {
            RegWidth::W32 => 'w',
            RegWidth::X64 => 'x',
        };
        write!(f, "{prefix}{}", self.number)
    }
}

/// Special registers that share encoding 31 with different semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialReg {
    /// Stack pointer (64-bit context)
    Sp,
    /// Stack pointer (32-bit context)
    Wsp,
    /// Zero register (64-bit)
    Xzr,
    /// Zero register (32-bit)
    Wzr,
}

impl SpecialReg {
    /// All special registers encode as 31.
    pub fn encoding(&self) -> u8 {
        31
    }
}

impl fmt::Display for SpecialReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecialReg::Sp => write!(f, "sp"),
            SpecialReg::Wsp => write!(f, "wsp"),
            SpecialReg::Xzr => write!(f, "xzr"),
            SpecialReg::Wzr => write!(f, "wzr"),
        }
    }
}

/// Width of a floating-point/SIMD scalar register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FpRegWidth {
    /// 8-bit (B0-B31)
    B,
    /// 16-bit (H0-H31)
    H,
    /// 32-bit (S0-S31)
    S,
    /// 64-bit (D0-D31)
    D,
    /// 128-bit (Q0-Q31)
    Q,
}

/// A floating-point/SIMD scalar register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FpReg {
    pub number: u8,
    pub width: FpRegWidth,
}

impl FpReg {
    pub fn new(number: u8, width: FpRegWidth) -> Self {
        debug_assert!(number <= 31, "FP register number must be 0-31");
        Self { number, width }
    }

    pub fn encoding(&self) -> u8 {
        self.number
    }
}

impl fmt::Display for FpReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.width {
            FpRegWidth::B => 'b',
            FpRegWidth::H => 'h',
            FpRegWidth::S => 's',
            FpRegWidth::D => 'd',
            FpRegWidth::Q => 'q',
        };
        write!(f, "{prefix}{}", self.number)
    }
}

/// SIMD vector arrangement specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VecArrangement {
    /// 8 bytes (8B)
    B8,
    /// 16 bytes (16B)
    B16,
    /// 4 half-words (4H)
    H4,
    /// 8 half-words (8H)
    H8,
    /// 2 single-words (2S)
    S2,
    /// 4 single-words (4S)
    S4,
    /// 1 double-word (1D)
    D1,
    /// 2 double-words (2D)
    D2,
}

/// A SIMD vector register with arrangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VecReg {
    pub number: u8,
    pub arrangement: VecArrangement,
}

impl VecReg {
    pub fn new(number: u8, arrangement: VecArrangement) -> Self {
        debug_assert!(number <= 31, "Vector register number must be 0-31");
        Self {
            number,
            arrangement,
        }
    }

    pub fn encoding(&self) -> u8 {
        self.number
    }
}

impl fmt::Display for VecReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arr = match self.arrangement {
            VecArrangement::B8 => "8b",
            VecArrangement::B16 => "16b",
            VecArrangement::H4 => "4h",
            VecArrangement::H8 => "8h",
            VecArrangement::S2 => "2s",
            VecArrangement::S4 => "4s",
            VecArrangement::D1 => "1d",
            VecArrangement::D2 => "2d",
        };
        write!(f, "v{}.{arr}", self.number)
    }
}

/// Unified register type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    Gp(GpReg),
    Special(SpecialReg),
    Fp(FpReg),
    Vec(VecReg),
}

impl Register {
    /// The 5-bit register encoding.
    pub fn encoding(&self) -> u8 {
        match self {
            Register::Gp(r) => r.encoding(),
            Register::Special(r) => r.encoding(),
            Register::Fp(r) => r.encoding(),
            Register::Vec(r) => r.encoding(),
        }
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::Gp(r) => write!(f, "{r}"),
            Register::Special(r) => write!(f, "{r}"),
            Register::Fp(r) => write!(f, "{r}"),
            Register::Vec(r) => write!(f, "{r}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gp_encoding() {
        let r = GpReg::new(5, RegWidth::X64);
        assert_eq!(r.encoding(), 5);
        assert_eq!(r.to_string(), "x5");
    }

    #[test]
    fn w_register_display() {
        let r = GpReg::new(30, RegWidth::W32);
        assert_eq!(r.to_string(), "w30");
    }

    #[test]
    fn special_encoding_is_31() {
        assert_eq!(SpecialReg::Sp.encoding(), 31);
        assert_eq!(SpecialReg::Wsp.encoding(), 31);
        assert_eq!(SpecialReg::Xzr.encoding(), 31);
        assert_eq!(SpecialReg::Wzr.encoding(), 31);
    }

    #[test]
    fn fp_display() {
        let r = FpReg::new(0, FpRegWidth::D);
        assert_eq!(r.to_string(), "d0");
    }

    #[test]
    fn vec_display() {
        let r = VecReg::new(3, VecArrangement::S4);
        assert_eq!(r.to_string(), "v3.4s");
    }

    #[test]
    fn unified_encoding() {
        let gp = Register::Gp(GpReg::new(10, RegWidth::X64));
        let sp = Register::Special(SpecialReg::Sp);
        assert_eq!(gp.encoding(), 10);
        assert_eq!(sp.encoding(), 31);
    }

    #[test]
    fn register_display() {
        assert_eq!(Register::Gp(GpReg::new(0, RegWidth::X64)).to_string(), "x0");
        assert_eq!(
            Register::Gp(GpReg::new(30, RegWidth::W32)).to_string(),
            "w30"
        );
        assert_eq!(Register::Special(SpecialReg::Sp).to_string(), "sp");
        assert_eq!(Register::Special(SpecialReg::Wsp).to_string(), "wsp");
        assert_eq!(Register::Special(SpecialReg::Xzr).to_string(), "xzr");
        assert_eq!(Register::Special(SpecialReg::Wzr).to_string(), "wzr");
        assert_eq!(Register::Fp(FpReg::new(7, FpRegWidth::S)).to_string(), "s7");
        assert_eq!(
            Register::Vec(VecReg::new(2, VecArrangement::D2)).to_string(),
            "v2.2d"
        );
    }
}
