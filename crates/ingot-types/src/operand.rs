use crate::expr::Expr;
use crate::register::Register;

/// Shift operations for shifted register operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShiftOp {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

impl ShiftOp {
    /// 2-bit encoding used in ARM64 instructions.
    pub fn encoding(&self) -> u8 {
        match self {
            ShiftOp::Lsl => 0b00,
            ShiftOp::Lsr => 0b01,
            ShiftOp::Asr => 0b10,
            ShiftOp::Ror => 0b11,
        }
    }
}

/// Extend operations for extended register operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendOp {
    Uxtb,
    Uxth,
    Uxtw,
    Uxtx,
    Sxtb,
    Sxth,
    Sxtw,
    Sxtx,
}

impl ExtendOp {
    /// 3-bit encoding used in ARM64 instructions.
    pub fn encoding(&self) -> u8 {
        match self {
            ExtendOp::Uxtb => 0b000,
            ExtendOp::Uxth => 0b001,
            ExtendOp::Uxtw => 0b010,
            ExtendOp::Uxtx => 0b011,
            ExtendOp::Sxtb => 0b100,
            ExtendOp::Sxth => 0b101,
            ExtendOp::Sxtw => 0b110,
            ExtendOp::Sxtx => 0b111,
        }
    }
}

/// ARM64 condition codes (4-bit encoding).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Condition {
    Eq,
    Ne,
    Hs,
    Lo,
    Mi,
    Pl,
    Vs,
    Vc,
    Hi,
    Ls,
    Ge,
    Lt,
    Gt,
    Le,
    Al,
    Nv,
}

impl Condition {
    /// 4-bit condition code encoding.
    pub fn encoding(&self) -> u8 {
        match self {
            Condition::Eq => 0b0000,
            Condition::Ne => 0b0001,
            Condition::Hs => 0b0010,
            Condition::Lo => 0b0011,
            Condition::Mi => 0b0100,
            Condition::Pl => 0b0101,
            Condition::Vs => 0b0110,
            Condition::Vc => 0b0111,
            Condition::Hi => 0b1000,
            Condition::Ls => 0b1001,
            Condition::Ge => 0b1010,
            Condition::Lt => 0b1011,
            Condition::Gt => 0b1100,
            Condition::Le => 0b1101,
            Condition::Al => 0b1110,
            Condition::Nv => 0b1111,
        }
    }

    /// Parse a condition code from a string (case-insensitive).
    /// Handles aliases: `cs` -> `hs`, `cc` -> `lo`.
    pub fn parse(s: &str) -> Option<Condition> {
        if s.len() != 2 {
            return None;
        }
        if s.eq_ignore_ascii_case("eq") {
            return Some(Condition::Eq);
        }
        if s.eq_ignore_ascii_case("ne") {
            return Some(Condition::Ne);
        }
        if s.eq_ignore_ascii_case("hs") || s.eq_ignore_ascii_case("cs") {
            return Some(Condition::Hs);
        }
        if s.eq_ignore_ascii_case("lo") || s.eq_ignore_ascii_case("cc") {
            return Some(Condition::Lo);
        }
        if s.eq_ignore_ascii_case("mi") {
            return Some(Condition::Mi);
        }
        if s.eq_ignore_ascii_case("pl") {
            return Some(Condition::Pl);
        }
        if s.eq_ignore_ascii_case("vs") {
            return Some(Condition::Vs);
        }
        if s.eq_ignore_ascii_case("vc") {
            return Some(Condition::Vc);
        }
        if s.eq_ignore_ascii_case("hi") {
            return Some(Condition::Hi);
        }
        if s.eq_ignore_ascii_case("ls") {
            return Some(Condition::Ls);
        }
        if s.eq_ignore_ascii_case("ge") {
            return Some(Condition::Ge);
        }
        if s.eq_ignore_ascii_case("lt") {
            return Some(Condition::Lt);
        }
        if s.eq_ignore_ascii_case("gt") {
            return Some(Condition::Gt);
        }
        if s.eq_ignore_ascii_case("le") {
            return Some(Condition::Le);
        }
        if s.eq_ignore_ascii_case("al") {
            return Some(Condition::Al);
        }
        if s.eq_ignore_ascii_case("nv") {
            return Some(Condition::Nv);
        }
        None
    }
}

/// Memory addressing modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryOperand {
    /// `[Xn]`
    Base { reg: Register },
    /// `[Xn, #imm]`
    BaseOffset { reg: Register, offset: Expr },
    /// `[Xn, #imm]!`
    PreIndex { reg: Register, offset: Expr },
    /// `[Xn], #imm`
    PostIndex { reg: Register, offset: Expr },
    /// `[Xn, Rm{, extend {#amount}}]`
    BaseRegister {
        base: Register,
        index: Register,
        extend: Option<ExtendOp>,
        amount: Option<u8>,
    },
    /// PC-relative label reference (for literal loads)
    Label(String),
}

/// An instruction operand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Register(Register),
    Immediate(Expr),
    Label(String),
    Memory(MemoryOperand),
    Shift { op: ShiftOp, amount: Expr },
    Extend { op: ExtendOp, amount: Option<Expr> },
    Condition(Condition),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_encodings() {
        assert_eq!(ShiftOp::Lsl.encoding(), 0b00);
        assert_eq!(ShiftOp::Lsr.encoding(), 0b01);
        assert_eq!(ShiftOp::Asr.encoding(), 0b10);
        assert_eq!(ShiftOp::Ror.encoding(), 0b11);
    }

    #[test]
    fn extend_encodings() {
        assert_eq!(ExtendOp::Uxtb.encoding(), 0b000);
        assert_eq!(ExtendOp::Sxtx.encoding(), 0b111);
    }

    #[test]
    fn condition_encodings() {
        assert_eq!(Condition::Eq.encoding(), 0b0000);
        assert_eq!(Condition::Ne.encoding(), 0b0001);
        assert_eq!(Condition::Nv.encoding(), 0b1111);
    }

    #[test]
    fn condition_from_str() {
        assert_eq!(Condition::parse("eq"), Some(Condition::Eq));
        assert_eq!(Condition::parse("EQ"), Some(Condition::Eq));
        assert_eq!(Condition::parse("cs"), Some(Condition::Hs));
        assert_eq!(Condition::parse("cc"), Some(Condition::Lo));
        assert_eq!(Condition::parse("hs"), Some(Condition::Hs));
        assert_eq!(Condition::parse("lo"), Some(Condition::Lo));
        assert_eq!(Condition::parse("bad"), None);
    }
}
