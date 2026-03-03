use ingot_types::operand::{ExtendOp, ShiftOp};
use ingot_types::register::{FpReg, FpRegWidth, GpReg, RegWidth, Register, SpecialReg};
use logos::Logos;

// ── Callback helpers ────────────────────────────────────────────────

fn parse_gp_x(lex: &mut logos::Lexer<'_, Token>) -> Option<GpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 30 {
        return None;
    }
    Some(GpReg::new(n, RegWidth::X64))
}

fn parse_gp_w(lex: &mut logos::Lexer<'_, Token>) -> Option<GpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 30 {
        return None;
    }
    Some(GpReg::new(n, RegWidth::W32))
}

fn parse_fp_b(lex: &mut logos::Lexer<'_, Token>) -> Option<FpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 31 {
        return None;
    }
    Some(FpReg::new(n, FpRegWidth::B))
}

fn parse_fp_h(lex: &mut logos::Lexer<'_, Token>) -> Option<FpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 31 {
        return None;
    }
    Some(FpReg::new(n, FpRegWidth::H))
}

fn parse_fp_s(lex: &mut logos::Lexer<'_, Token>) -> Option<FpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 31 {
        return None;
    }
    Some(FpReg::new(n, FpRegWidth::S))
}

fn parse_fp_d(lex: &mut logos::Lexer<'_, Token>) -> Option<FpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 31 {
        return None;
    }
    Some(FpReg::new(n, FpRegWidth::D))
}

fn parse_fp_q(lex: &mut logos::Lexer<'_, Token>) -> Option<FpReg> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 31 {
        return None;
    }
    Some(FpReg::new(n, FpRegWidth::Q))
}

fn parse_vec_number(lex: &mut logos::Lexer<'_, Token>) -> Option<u8> {
    let n: u8 = lex.slice()[1..].parse().ok()?;
    if n > 31 {
        return None;
    }
    Some(n)
}

fn parse_decimal(lex: &mut logos::Lexer<'_, Token>) -> Option<i64> {
    let s = lex.slice().replace('_', "");
    s.parse().ok()
}

fn parse_hex(lex: &mut logos::Lexer<'_, Token>) -> Option<i64> {
    let s = lex.slice().replace('_', "");
    i64::from_str_radix(&s[2..], 16).ok()
}

fn parse_octal(lex: &mut logos::Lexer<'_, Token>) -> Option<i64> {
    let s = lex.slice().replace('_', "");
    i64::from_str_radix(&s[2..], 8).ok()
}

fn parse_binary(lex: &mut logos::Lexer<'_, Token>) -> Option<i64> {
    let s = lex.slice().replace('_', "");
    i64::from_str_radix(&s[2..], 2).ok()
}

fn parse_string(lex: &mut logos::Lexer<'_, Token>) -> Option<String> {
    let slice = lex.slice();
    // Strip surrounding quotes
    let inner = &slice[1..slice.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                '0' => result.push('\0'),
                _ => return None,
            }
        } else {
            result.push(c);
        }
    }
    Some(result)
}

fn skip_block_comment(lex: &mut logos::Lexer<'_, Token>) -> logos::FilterResult<(), ()> {
    let remainder = lex.remainder();
    match remainder.find("*/") {
        Some(pos) => {
            lex.bump(pos + 2);
            logos::FilterResult::Skip
        }
        None => logos::FilterResult::Error(()),
    }
}

// ── Token enum ──────────────────────────────────────────────────────

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = ())]
#[logos(skip r"[ \t\r]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r";[^\n]*")]
pub enum Token {
    // ── Block comment ───────────────────────────────────────────
    #[token("/*", skip_block_comment)]
    BlockComment,

    // ── GP registers ────────────────────────────────────────────
    #[regex(r"[xX]([0-9]|[12][0-9]|30)", parse_gp_x, priority = 3)]
    GpReg(GpReg),

    // W registers — need separate variant to avoid ambiguity with wsp/wzr
    #[regex(r"[wW]([0-9]|[12][0-9]|30)", parse_gp_w, priority = 3)]
    WReg(GpReg),

    // ── Special registers ───────────────────────────────────────
    #[token("sp", ignore(ascii_case))]
    Sp,

    #[token("wsp", ignore(ascii_case))]
    Wsp,

    #[token("xzr", ignore(ascii_case))]
    Xzr,

    #[token("wzr", ignore(ascii_case))]
    Wzr,

    // ── FP/SIMD scalar registers ────────────────────────────────
    #[regex(r"[bB]([0-9]|[12][0-9]|3[01])", parse_fp_b, priority = 3)]
    FpRegB(FpReg),

    #[regex(r"[hH]([0-9]|[12][0-9]|3[01])", parse_fp_h, priority = 3)]
    FpRegH(FpReg),

    #[regex(r"[sS]([0-9]|[12][0-9]|3[01])", parse_fp_s, priority = 3)]
    FpRegS(FpReg),

    #[regex(r"[dD]([0-9]|[12][0-9]|3[01])", parse_fp_d, priority = 3)]
    FpRegD(FpReg),

    #[regex(r"[qQ]([0-9]|[12][0-9]|3[01])", parse_fp_q, priority = 3)]
    FpRegQ(FpReg),

    // ── Vector register base ────────────────────────────────────
    #[regex(r"[vV]([0-9]|[12][0-9]|3[01])", parse_vec_number, priority = 3)]
    VecReg(u8),

    // ── Integer literals ────────────────────────────────────────
    #[regex(r"0[xX][0-9a-fA-F][0-9a-fA-F_]*", parse_hex, priority = 2)]
    #[regex(r"0[oO][0-7][0-7_]*", parse_octal, priority = 2)]
    #[regex(r"0[bB][01][01_]*", parse_binary, priority = 2)]
    #[regex(r"[0-9][0-9_]*", parse_decimal, priority = 2)]
    Integer(i64),

    // ── String literal ──────────────────────────────────────────
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    StringLiteral(String),

    // ── Shift keywords ──────────────────────────────────────────
    #[token("lsl", ignore(ascii_case))]
    Lsl,
    #[token("lsr", ignore(ascii_case))]
    Lsr,
    #[token("asr", ignore(ascii_case))]
    Asr,
    #[token("ror", ignore(ascii_case))]
    Ror,

    // ── Extend keywords ─────────────────────────────────────────
    #[token("uxtb", ignore(ascii_case))]
    Uxtb,
    #[token("uxth", ignore(ascii_case))]
    Uxth,
    #[token("uxtw", ignore(ascii_case))]
    Uxtw,
    #[token("uxtx", ignore(ascii_case))]
    Uxtx,
    #[token("sxtb", ignore(ascii_case))]
    Sxtb,
    #[token("sxth", ignore(ascii_case))]
    Sxth,
    #[token("sxtw", ignore(ascii_case))]
    Sxtw,
    #[token("sxtx", ignore(ascii_case))]
    Sxtx,

    // ── Directive ───────────────────────────────────────────────
    #[regex(r"\.[a-zA-Z_][a-zA-Z0-9_]*")]
    Directive,

    // ── Identifier (lowest priority — mnemonics, labels, symbols) ─
    #[regex(r"[a-zA-Z_$][a-zA-Z0-9_$]*")]
    Ident,

    // ── Punctuation ─────────────────────────────────────────────
    #[token(",")]
    Comma,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("!")]
    Exclaim,
    #[token(":")]
    Colon,
    #[token("#")]
    Hash,
    #[token(".")]
    Dot,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("~")]
    Tilde,
    #[token("=")]
    Eq,

    // ── Newline (statement separator) ───────────────────────────
    #[token("\n")]
    Newline,
}

impl Token {
    /// Convert register tokens to the unified `Register` type.
    pub fn as_register(&self) -> Option<Register> {
        match self {
            Token::GpReg(r) | Token::WReg(r) => Some(Register::Gp(*r)),
            Token::Sp => Some(Register::Special(SpecialReg::Sp)),
            Token::Wsp => Some(Register::Special(SpecialReg::Wsp)),
            Token::Xzr => Some(Register::Special(SpecialReg::Xzr)),
            Token::Wzr => Some(Register::Special(SpecialReg::Wzr)),
            Token::FpRegB(r)
            | Token::FpRegH(r)
            | Token::FpRegS(r)
            | Token::FpRegD(r)
            | Token::FpRegQ(r) => Some(Register::Fp(*r)),
            _ => None,
        }
    }

    /// Convert shift keyword tokens to `ShiftOp`.
    pub fn as_shift_op(&self) -> Option<ShiftOp> {
        match self {
            Token::Lsl => Some(ShiftOp::Lsl),
            Token::Lsr => Some(ShiftOp::Lsr),
            Token::Asr => Some(ShiftOp::Asr),
            Token::Ror => Some(ShiftOp::Ror),
            _ => None,
        }
    }

    /// Convert extend keyword tokens to `ExtendOp`.
    pub fn as_extend_op(&self) -> Option<ExtendOp> {
        match self {
            Token::Uxtb => Some(ExtendOp::Uxtb),
            Token::Uxth => Some(ExtendOp::Uxth),
            Token::Uxtw => Some(ExtendOp::Uxtw),
            Token::Uxtx => Some(ExtendOp::Uxtx),
            Token::Sxtb => Some(ExtendOp::Sxtb),
            Token::Sxth => Some(ExtendOp::Sxth),
            Token::Sxtw => Some(ExtendOp::Sxtw),
            Token::Sxtx => Some(ExtendOp::Sxtx),
            _ => None,
        }
    }

    /// Check if this token is a newline.
    pub fn is_newline(&self) -> bool {
        matches!(self, Token::Newline)
    }

    /// Check if this token represents any GP register (X or W).
    pub fn as_gp_reg(&self) -> Option<GpReg> {
        match self {
            Token::GpReg(r) | Token::WReg(r) => Some(*r),
            _ => None,
        }
    }

    /// Check if this token represents any FP register.
    pub fn as_fp_reg(&self) -> Option<FpReg> {
        match self {
            Token::FpRegB(r)
            | Token::FpRegH(r)
            | Token::FpRegS(r)
            | Token::FpRegD(r)
            | Token::FpRegQ(r) => Some(*r),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::Span;
    use pretty_assertions::assert_eq;

    /// Helper: lex source and return all Ok tokens (ignoring spans).
    fn lex_tokens(source: &str) -> Vec<Token> {
        Token::lexer(source).filter_map(|r| r.ok()).collect()
    }

    /// Helper: lex source and return tokens with spans.
    fn lex_with_spans(source: &str) -> Vec<(Token, Span)> {
        let lexer = Token::lexer(source);
        lexer
            .spanned()
            .filter_map(|(r, span)| {
                r.ok().map(|tok| {
                    (
                        tok,
                        Span::new(span.start as u32, (span.end - span.start) as u32),
                    )
                })
            })
            .collect()
    }

    // ── GP register tests ───────────────────────────────────────

    #[test]
    fn gp_x_registers() {
        assert_eq!(
            lex_tokens("x0"),
            vec![Token::GpReg(GpReg::new(0, RegWidth::X64))]
        );
        assert_eq!(
            lex_tokens("x30"),
            vec![Token::GpReg(GpReg::new(30, RegWidth::X64))]
        );
        assert_eq!(
            lex_tokens("X15"),
            vec![Token::GpReg(GpReg::new(15, RegWidth::X64))]
        );
    }

    #[test]
    fn gp_w_registers() {
        assert_eq!(
            lex_tokens("w0"),
            vec![Token::WReg(GpReg::new(0, RegWidth::W32))]
        );
        assert_eq!(
            lex_tokens("w30"),
            vec![Token::WReg(GpReg::new(30, RegWidth::W32))]
        );
        assert_eq!(
            lex_tokens("W7"),
            vec![Token::WReg(GpReg::new(7, RegWidth::W32))]
        );
    }

    #[test]
    fn x31_is_ident() {
        // x31 is not a valid GP register — should fall through to Ident
        assert_eq!(lex_tokens("x31"), vec![Token::Ident]);
    }

    #[test]
    fn register_prefix_in_longer_ident() {
        // x0abc should be Ident because longest-match wins
        assert_eq!(lex_tokens("x0abc"), vec![Token::Ident]);
    }

    // ── Special register tests ──────────────────────────────────

    #[test]
    fn special_registers() {
        assert_eq!(lex_tokens("sp"), vec![Token::Sp]);
        assert_eq!(lex_tokens("SP"), vec![Token::Sp]);
        assert_eq!(lex_tokens("wsp"), vec![Token::Wsp]);
        assert_eq!(lex_tokens("WSP"), vec![Token::Wsp]);
        assert_eq!(lex_tokens("xzr"), vec![Token::Xzr]);
        assert_eq!(lex_tokens("XZR"), vec![Token::Xzr]);
        assert_eq!(lex_tokens("wzr"), vec![Token::Wzr]);
        assert_eq!(lex_tokens("WZR"), vec![Token::Wzr]);
    }

    // ── FP register tests ───────────────────────────────────────

    #[test]
    fn fp_registers() {
        assert_eq!(
            lex_tokens("b0"),
            vec![Token::FpRegB(FpReg::new(0, FpRegWidth::B))]
        );
        assert_eq!(
            lex_tokens("h15"),
            vec![Token::FpRegH(FpReg::new(15, FpRegWidth::H))]
        );
        assert_eq!(
            lex_tokens("s31"),
            vec![Token::FpRegS(FpReg::new(31, FpRegWidth::S))]
        );
        assert_eq!(
            lex_tokens("d0"),
            vec![Token::FpRegD(FpReg::new(0, FpRegWidth::D))]
        );
        assert_eq!(
            lex_tokens("Q31"),
            vec![Token::FpRegQ(FpReg::new(31, FpRegWidth::Q))]
        );
    }

    #[test]
    fn fp_boundary_b32_invalid() {
        // b32 is not valid — should be Ident
        assert_eq!(lex_tokens("b32"), vec![Token::Ident]);
    }

    // ── Vector register tests ───────────────────────────────────

    #[test]
    fn vec_registers() {
        assert_eq!(lex_tokens("v0"), vec![Token::VecReg(0)]);
        assert_eq!(lex_tokens("v31"), vec![Token::VecReg(31)]);
        assert_eq!(lex_tokens("V16"), vec![Token::VecReg(16)]);
    }

    #[test]
    fn vec_v32_invalid() {
        assert_eq!(lex_tokens("v32"), vec![Token::Ident]);
    }

    // ── Integer tests ───────────────────────────────────────────

    #[test]
    fn decimal_integers() {
        assert_eq!(lex_tokens("0"), vec![Token::Integer(0)]);
        assert_eq!(lex_tokens("42"), vec![Token::Integer(42)]);
        assert_eq!(lex_tokens("1_000_000"), vec![Token::Integer(1_000_000)]);
    }

    #[test]
    fn hex_integers() {
        assert_eq!(lex_tokens("0xff"), vec![Token::Integer(255)]);
        assert_eq!(lex_tokens("0xFF"), vec![Token::Integer(255)]);
        assert_eq!(lex_tokens("0x1_0000"), vec![Token::Integer(0x1_0000)]);
    }

    #[test]
    fn octal_integers() {
        assert_eq!(lex_tokens("0o77"), vec![Token::Integer(63)]);
        assert_eq!(lex_tokens("0o1_0"), vec![Token::Integer(8)]);
    }

    #[test]
    fn binary_integers() {
        assert_eq!(lex_tokens("0b1010"), vec![Token::Integer(10)]);
        assert_eq!(lex_tokens("0b1111_0000"), vec![Token::Integer(240)]);
    }

    // ── String literal tests ────────────────────────────────────

    #[test]
    fn string_basic() {
        assert_eq!(
            lex_tokens(r#""hello""#),
            vec![Token::StringLiteral("hello".into())]
        );
    }

    #[test]
    fn string_escapes() {
        assert_eq!(
            lex_tokens(r#""a\nb\t""#),
            vec![Token::StringLiteral("a\nb\t".into())]
        );
        assert_eq!(
            lex_tokens(r#""\\\"""#),
            vec![Token::StringLiteral("\\\"".into())]
        );
        assert_eq!(
            lex_tokens(r#""\0""#),
            vec![Token::StringLiteral("\0".into())]
        );
    }

    // ── Comment tests ───────────────────────────────────────────

    #[test]
    fn line_comment_double_slash() {
        assert_eq!(
            lex_tokens("add // comment\nx0"),
            vec![
                Token::Ident,
                Token::Newline,
                Token::GpReg(GpReg::new(0, RegWidth::X64))
            ]
        );
    }

    #[test]
    fn line_comment_semicolon() {
        assert_eq!(
            lex_tokens("add ; comment\nx0"),
            vec![
                Token::Ident,
                Token::Newline,
                Token::GpReg(GpReg::new(0, RegWidth::X64))
            ]
        );
    }

    #[test]
    fn block_comment() {
        assert_eq!(
            lex_tokens("add /* block comment */ x0"),
            vec![Token::Ident, Token::GpReg(GpReg::new(0, RegWidth::X64))]
        );
    }

    #[test]
    fn block_comment_multiline() {
        assert_eq!(
            lex_tokens("add /* multi\nline */ x0"),
            vec![Token::Ident, Token::GpReg(GpReg::new(0, RegWidth::X64))]
        );
    }

    #[test]
    fn newline_preserved_after_comment() {
        let tokens = lex_tokens("add // comment\nmov");
        assert!(tokens.contains(&Token::Newline));
    }

    // ── Shift/extend keyword tests ──────────────────────────────

    #[test]
    fn shift_keywords() {
        assert_eq!(lex_tokens("lsl"), vec![Token::Lsl]);
        assert_eq!(lex_tokens("LSR"), vec![Token::Lsr]);
        assert_eq!(lex_tokens("Asr"), vec![Token::Asr]);
        assert_eq!(lex_tokens("ROR"), vec![Token::Ror]);
    }

    #[test]
    fn extend_keywords() {
        assert_eq!(lex_tokens("uxtb"), vec![Token::Uxtb]);
        assert_eq!(lex_tokens("SXTX"), vec![Token::Sxtx]);
    }

    // ── Directive tests ─────────────────────────────────────────

    #[test]
    fn directives() {
        assert_eq!(lex_tokens(".global"), vec![Token::Directive]);
        assert_eq!(lex_tokens(".text"), vec![Token::Directive]);
        assert_eq!(lex_tokens(".p2align"), vec![Token::Directive]);
    }

    // ── Punctuation tests ───────────────────────────────────────

    #[test]
    fn punctuation() {
        let tokens = lex_tokens(", [ ] ! : # . ( ) + - * / ~ =");
        assert_eq!(
            tokens,
            vec![
                Token::Comma,
                Token::LBracket,
                Token::RBracket,
                Token::Exclaim,
                Token::Colon,
                Token::Hash,
                Token::Dot,
                Token::LParen,
                Token::RParen,
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Tilde,
                Token::Eq,
            ]
        );
    }

    // ── Full instruction line tests ─────────────────────────────

    #[test]
    fn add_instruction() {
        let tokens = lex_tokens("add x0, x1, #42");
        assert_eq!(
            tokens,
            vec![
                Token::Ident, // add
                Token::GpReg(GpReg::new(0, RegWidth::X64)),
                Token::Comma,
                Token::GpReg(GpReg::new(1, RegWidth::X64)),
                Token::Comma,
                Token::Hash,
                Token::Integer(42),
            ]
        );
    }

    #[test]
    fn label_line() {
        let tokens = lex_tokens("_main:");
        assert_eq!(tokens, vec![Token::Ident, Token::Colon]);
    }

    #[test]
    fn ldr_memory_operand() {
        let tokens = lex_tokens("ldr x0, [x1, #8]");
        assert_eq!(
            tokens,
            vec![
                Token::Ident, // ldr
                Token::GpReg(GpReg::new(0, RegWidth::X64)),
                Token::Comma,
                Token::LBracket,
                Token::GpReg(GpReg::new(1, RegWidth::X64)),
                Token::Comma,
                Token::Hash,
                Token::Integer(8),
                Token::RBracket,
            ]
        );
    }

    #[test]
    fn global_directive() {
        let tokens = lex_tokens(".global _main");
        assert_eq!(tokens, vec![Token::Directive, Token::Ident]);
    }

    #[test]
    fn negative_immediate() {
        let tokens = lex_tokens("#-42");
        assert_eq!(tokens, vec![Token::Hash, Token::Minus, Token::Integer(42)]);
    }

    // ── Span accuracy tests ─────────────────────────────────────

    #[test]
    fn span_positions() {
        let items = lex_with_spans("add x0, x1");
        assert_eq!(items.len(), 4);
        // "add" at 0..3
        assert_eq!(items[0], (Token::Ident, Span::new(0, 3)));
        // "x0" at 4..6
        assert_eq!(
            items[1],
            (Token::GpReg(GpReg::new(0, RegWidth::X64)), Span::new(4, 2))
        );
        // "," at 6..7
        assert_eq!(items[2], (Token::Comma, Span::new(6, 1)));
        // "x1" at 8..10
        assert_eq!(
            items[3],
            (Token::GpReg(GpReg::new(1, RegWidth::X64)), Span::new(8, 2))
        );
    }

    // ── Error tests ─────────────────────────────────────────────

    #[test]
    fn unrecognized_char_produces_error() {
        let results: Vec<_> = Token::lexer("@").collect();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    // ── Helper method tests ─────────────────────────────────────

    #[test]
    fn as_register_gp() {
        let tok = Token::GpReg(GpReg::new(5, RegWidth::X64));
        assert_eq!(
            tok.as_register(),
            Some(Register::Gp(GpReg::new(5, RegWidth::X64)))
        );
    }

    #[test]
    fn as_register_w() {
        let tok = Token::WReg(GpReg::new(10, RegWidth::W32));
        assert_eq!(
            tok.as_register(),
            Some(Register::Gp(GpReg::new(10, RegWidth::W32)))
        );
    }

    #[test]
    fn as_register_special() {
        assert_eq!(
            Token::Sp.as_register(),
            Some(Register::Special(SpecialReg::Sp))
        );
        assert_eq!(
            Token::Xzr.as_register(),
            Some(Register::Special(SpecialReg::Xzr))
        );
    }

    #[test]
    fn as_register_fp() {
        let tok = Token::FpRegD(FpReg::new(7, FpRegWidth::D));
        assert_eq!(
            tok.as_register(),
            Some(Register::Fp(FpReg::new(7, FpRegWidth::D)))
        );
    }

    #[test]
    fn as_register_non_register() {
        assert_eq!(Token::Ident.as_register(), None);
        assert_eq!(Token::Integer(42).as_register(), None);
    }

    #[test]
    fn as_shift_op_test() {
        assert_eq!(Token::Lsl.as_shift_op(), Some(ShiftOp::Lsl));
        assert_eq!(Token::Lsr.as_shift_op(), Some(ShiftOp::Lsr));
        assert_eq!(Token::Asr.as_shift_op(), Some(ShiftOp::Asr));
        assert_eq!(Token::Ror.as_shift_op(), Some(ShiftOp::Ror));
        assert_eq!(Token::Ident.as_shift_op(), None);
    }

    #[test]
    fn as_extend_op_test() {
        assert_eq!(Token::Uxtb.as_extend_op(), Some(ExtendOp::Uxtb));
        assert_eq!(Token::Sxtx.as_extend_op(), Some(ExtendOp::Sxtx));
        assert_eq!(Token::Ident.as_extend_op(), None);
    }

    #[test]
    fn is_newline_test() {
        assert!(Token::Newline.is_newline());
        assert!(!Token::Ident.is_newline());
    }
}
