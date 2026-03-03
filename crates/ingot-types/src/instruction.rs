use crate::operand::Operand;
use crate::span::Span;

/// ARM64 instruction mnemonics (MVP set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    // Arithmetic
    Add,
    Adds,
    Sub,
    Subs,

    // Move
    Mov,
    Movz,
    Movn,
    Movk,

    // Compare
    Cmp,
    Cmn,
    Tst,

    // Logical
    And,
    Ands,
    Orr,
    Eor,

    // Branch
    B,
    Bl,
    Br,
    Blr,
    Ret,
    BCond,
    Cbz,
    Cbnz,

    // PC-relative address
    Adr,
    Adrp,

    // Load/Store
    Ldr,
    Str,
    Ldp,
    Stp,
    Ldrb,
    Ldrh,
    Strb,
    Strh,

    // System
    Nop,
    Svc,
    Brk,
}

impl Mnemonic {
    /// Parse a mnemonic from a string (case-insensitive).
    ///
    /// Note: `b.cond` is handled specially by the parser — this only matches
    /// the plain `b` mnemonic, which the parser upgrades to `BCond` when
    /// a condition suffix is present.
    pub fn parse(s: &str) -> Option<Mnemonic> {
        match s.to_ascii_lowercase().as_str() {
            "add" => Some(Mnemonic::Add),
            "adds" => Some(Mnemonic::Adds),
            "sub" => Some(Mnemonic::Sub),
            "subs" => Some(Mnemonic::Subs),

            "mov" => Some(Mnemonic::Mov),
            "movz" => Some(Mnemonic::Movz),
            "movn" => Some(Mnemonic::Movn),
            "movk" => Some(Mnemonic::Movk),

            "cmp" => Some(Mnemonic::Cmp),
            "cmn" => Some(Mnemonic::Cmn),
            "tst" => Some(Mnemonic::Tst),

            "and" => Some(Mnemonic::And),
            "ands" => Some(Mnemonic::Ands),
            "orr" => Some(Mnemonic::Orr),
            "eor" => Some(Mnemonic::Eor),

            "b" => Some(Mnemonic::B),
            "bl" => Some(Mnemonic::Bl),
            "br" => Some(Mnemonic::Br),
            "blr" => Some(Mnemonic::Blr),
            "ret" => Some(Mnemonic::Ret),
            "cbz" => Some(Mnemonic::Cbz),
            "cbnz" => Some(Mnemonic::Cbnz),

            "adr" => Some(Mnemonic::Adr),
            "adrp" => Some(Mnemonic::Adrp),

            "ldr" => Some(Mnemonic::Ldr),
            "str" => Some(Mnemonic::Str),
            "ldp" => Some(Mnemonic::Ldp),
            "stp" => Some(Mnemonic::Stp),
            "ldrb" => Some(Mnemonic::Ldrb),
            "ldrh" => Some(Mnemonic::Ldrh),
            "strb" => Some(Mnemonic::Strb),
            "strh" => Some(Mnemonic::Strh),

            "nop" => Some(Mnemonic::Nop),
            "svc" => Some(Mnemonic::Svc),
            "brk" => Some(Mnemonic::Brk),

            _ => None,
        }
    }
}

/// A parsed instruction: mnemonic + operands + source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub mnemonic: Mnemonic,
    pub operands: Vec<Operand>,
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_from_str_basic() {
        assert_eq!(Mnemonic::parse("add"), Some(Mnemonic::Add));
        assert_eq!(Mnemonic::parse("ADD"), Some(Mnemonic::Add));
        assert_eq!(Mnemonic::parse("Add"), Some(Mnemonic::Add));
    }

    #[test]
    fn mnemonic_from_str_all_variants() {
        let cases = [
            ("add", Mnemonic::Add),
            ("adds", Mnemonic::Adds),
            ("sub", Mnemonic::Sub),
            ("subs", Mnemonic::Subs),
            ("mov", Mnemonic::Mov),
            ("movz", Mnemonic::Movz),
            ("movn", Mnemonic::Movn),
            ("movk", Mnemonic::Movk),
            ("cmp", Mnemonic::Cmp),
            ("cmn", Mnemonic::Cmn),
            ("tst", Mnemonic::Tst),
            ("and", Mnemonic::And),
            ("ands", Mnemonic::Ands),
            ("orr", Mnemonic::Orr),
            ("eor", Mnemonic::Eor),
            ("b", Mnemonic::B),
            ("bl", Mnemonic::Bl),
            ("br", Mnemonic::Br),
            ("blr", Mnemonic::Blr),
            ("ret", Mnemonic::Ret),
            ("cbz", Mnemonic::Cbz),
            ("cbnz", Mnemonic::Cbnz),
            ("adr", Mnemonic::Adr),
            ("adrp", Mnemonic::Adrp),
            ("ldr", Mnemonic::Ldr),
            ("str", Mnemonic::Str),
            ("ldp", Mnemonic::Ldp),
            ("stp", Mnemonic::Stp),
            ("ldrb", Mnemonic::Ldrb),
            ("ldrh", Mnemonic::Ldrh),
            ("strb", Mnemonic::Strb),
            ("strh", Mnemonic::Strh),
            ("nop", Mnemonic::Nop),
            ("svc", Mnemonic::Svc),
            ("brk", Mnemonic::Brk),
        ];
        for (s, expected) in cases {
            assert_eq!(Mnemonic::parse(s), Some(expected), "failed for {s:?}");
        }
    }

    #[test]
    fn mnemonic_unknown() {
        assert_eq!(Mnemonic::parse("xyz"), None);
        assert_eq!(Mnemonic::parse(""), None);
    }
}
