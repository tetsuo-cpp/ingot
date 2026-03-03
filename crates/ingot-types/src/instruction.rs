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
        // Bucket by length to reduce comparisons, avoid allocation.
        match s.len() {
            1 => {
                if s.eq_ignore_ascii_case("b") {
                    return Some(Mnemonic::B);
                }
                None
            }
            2 => {
                if s.eq_ignore_ascii_case("bl") {
                    return Some(Mnemonic::Bl);
                }
                if s.eq_ignore_ascii_case("br") {
                    return Some(Mnemonic::Br);
                }
                None
            }
            3 => {
                if s.eq_ignore_ascii_case("add") {
                    return Some(Mnemonic::Add);
                }
                if s.eq_ignore_ascii_case("sub") {
                    return Some(Mnemonic::Sub);
                }
                if s.eq_ignore_ascii_case("mov") {
                    return Some(Mnemonic::Mov);
                }
                if s.eq_ignore_ascii_case("cmp") {
                    return Some(Mnemonic::Cmp);
                }
                if s.eq_ignore_ascii_case("cmn") {
                    return Some(Mnemonic::Cmn);
                }
                if s.eq_ignore_ascii_case("tst") {
                    return Some(Mnemonic::Tst);
                }
                if s.eq_ignore_ascii_case("and") {
                    return Some(Mnemonic::And);
                }
                if s.eq_ignore_ascii_case("orr") {
                    return Some(Mnemonic::Orr);
                }
                if s.eq_ignore_ascii_case("eor") {
                    return Some(Mnemonic::Eor);
                }
                if s.eq_ignore_ascii_case("blr") {
                    return Some(Mnemonic::Blr);
                }
                if s.eq_ignore_ascii_case("ret") {
                    return Some(Mnemonic::Ret);
                }
                if s.eq_ignore_ascii_case("cbz") {
                    return Some(Mnemonic::Cbz);
                }
                if s.eq_ignore_ascii_case("adr") {
                    return Some(Mnemonic::Adr);
                }
                if s.eq_ignore_ascii_case("ldr") {
                    return Some(Mnemonic::Ldr);
                }
                if s.eq_ignore_ascii_case("str") {
                    return Some(Mnemonic::Str);
                }
                if s.eq_ignore_ascii_case("ldp") {
                    return Some(Mnemonic::Ldp);
                }
                if s.eq_ignore_ascii_case("stp") {
                    return Some(Mnemonic::Stp);
                }
                if s.eq_ignore_ascii_case("nop") {
                    return Some(Mnemonic::Nop);
                }
                if s.eq_ignore_ascii_case("svc") {
                    return Some(Mnemonic::Svc);
                }
                if s.eq_ignore_ascii_case("brk") {
                    return Some(Mnemonic::Brk);
                }
                None
            }
            4 => {
                if s.eq_ignore_ascii_case("adds") {
                    return Some(Mnemonic::Adds);
                }
                if s.eq_ignore_ascii_case("subs") {
                    return Some(Mnemonic::Subs);
                }
                if s.eq_ignore_ascii_case("movz") {
                    return Some(Mnemonic::Movz);
                }
                if s.eq_ignore_ascii_case("movn") {
                    return Some(Mnemonic::Movn);
                }
                if s.eq_ignore_ascii_case("movk") {
                    return Some(Mnemonic::Movk);
                }
                if s.eq_ignore_ascii_case("ands") {
                    return Some(Mnemonic::Ands);
                }
                if s.eq_ignore_ascii_case("cbnz") {
                    return Some(Mnemonic::Cbnz);
                }
                if s.eq_ignore_ascii_case("adrp") {
                    return Some(Mnemonic::Adrp);
                }
                if s.eq_ignore_ascii_case("ldrb") {
                    return Some(Mnemonic::Ldrb);
                }
                if s.eq_ignore_ascii_case("ldrh") {
                    return Some(Mnemonic::Ldrh);
                }
                if s.eq_ignore_ascii_case("strb") {
                    return Some(Mnemonic::Strb);
                }
                if s.eq_ignore_ascii_case("strh") {
                    return Some(Mnemonic::Strh);
                }
                None
            }
            _ => None,
        }
    }
}

/// A parsed instruction: mnemonic + operands + source span.
#[derive(Debug, Clone, PartialEq, Eq)]
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
