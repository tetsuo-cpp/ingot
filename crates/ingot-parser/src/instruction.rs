use ingot_lexer::Token;
use ingot_types::instruction::{Instruction, Mnemonic};
use ingot_types::operand::{Condition, Operand};
use ingot_types::Span;

use crate::parser::Parser;

impl Parser<'_> {
    /// Parse an instruction: mnemonic followed by operand list.
    /// The mnemonic identifier token has already been identified; `name` is its text, `mnem_span` its span.
    pub(crate) fn parse_instruction(&mut self, name: &str, mnem_span: Span) -> Option<Instruction> {
        let mut mnemonic = match Mnemonic::parse(name) {
            Some(m) => m,
            None => {
                self.errors.push(ingot_types::AsmError::UnknownMnemonic {
                    mnemonic: name.to_string(),
                    span: mnem_span,
                });
                return None;
            }
        };

        let mut operands = Vec::new();
        let mut end_span = mnem_span;

        // Handle b.cond: when mnemonic is B, peek for a Directive token (e.g., `.eq`)
        if mnemonic == Mnemonic::B {
            if let Some(Token::Directive) = self.peek() {
                let dir_span = self.peek_span();
                let dir_text = self.slice(dir_span);
                // Strip leading dot and check for condition code
                let cond_str = &dir_text[1..];
                if let Some(cond) = Condition::parse(cond_str) {
                    self.advance(); // consume the directive token
                    mnemonic = Mnemonic::BCond;
                    operands.push(Operand::Condition(cond));
                    end_span = end_span.merge(dir_span);
                }
            }
        }

        // Parse comma-separated operand list
        if !self.at_eol() {
            match self.parse_operand() {
                Some(op) => operands.push(op),
                None => return None,
            }
            end_span = self.prev_span();

            while self.eat(&Token::Comma).is_some() {
                match self.parse_operand() {
                    Some(op) => operands.push(op),
                    None => return None,
                }
                end_span = self.prev_span();
            }
        }

        Some(Instruction {
            mnemonic,
            operands,
            span: end_span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_lexer::Lexer;
    use ingot_types::expr::Expr;
    use ingot_types::instruction::Mnemonic;
    use ingot_types::operand::{Condition, Operand};
    use ingot_types::register::{GpReg, RegWidth, Register, SpecialReg};

    fn parse_instr(source: &str) -> (Option<Instruction>, Vec<ingot_types::AsmError>) {
        let (tokens, errors) = Lexer::collect_all(source);
        let mut parser = Parser::new(source, tokens, errors);
        // Consume the mnemonic ident
        let (_, mnem_span) = parser.advance().unwrap();
        let name = parser.slice(mnem_span).to_string();
        let result = parser.parse_instruction(&name, mnem_span);
        (result, parser.errors)
    }

    #[test]
    fn nop() {
        let (instr, e) = parse_instr("nop");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Nop);
        assert!(instr.operands.is_empty());
    }

    #[test]
    fn mov_reg_imm() {
        let (instr, e) = parse_instr("mov x0, #0");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Mov);
        assert_eq!(instr.operands.len(), 2);
        assert_eq!(
            instr.operands[0],
            Operand::Register(Register::Gp(GpReg::new(0, RegWidth::X64)))
        );
        assert_eq!(instr.operands[1], Operand::Immediate(Expr::Literal(0)));
    }

    #[test]
    fn add_three_regs() {
        let (instr, e) = parse_instr("add x0, x1, x2");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Add);
        assert_eq!(instr.operands.len(), 3);
    }

    #[test]
    fn b_cond() {
        let (instr, e) = parse_instr("b.eq _loop");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::BCond);
        assert_eq!(instr.operands.len(), 2);
        assert_eq!(instr.operands[0], Operand::Condition(Condition::Eq));
        assert_eq!(instr.operands[1], Operand::Label("_loop".into()));
    }

    #[test]
    fn b_cond_ne() {
        let (instr, e) = parse_instr("b.ne target");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::BCond);
        assert_eq!(instr.operands[0], Operand::Condition(Condition::Ne));
    }

    #[test]
    fn b_cond_cs_alias() {
        let (instr, e) = parse_instr("b.cs label");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::BCond);
        assert_eq!(instr.operands[0], Operand::Condition(Condition::Hs));
    }

    #[test]
    fn plain_branch() {
        let (instr, e) = parse_instr("b _start");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::B);
        assert_eq!(instr.operands.len(), 1);
        assert_eq!(instr.operands[0], Operand::Label("_start".into()));
    }

    #[test]
    fn ret_no_operand() {
        let (instr, e) = parse_instr("ret");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Ret);
        assert!(instr.operands.is_empty());
    }

    #[test]
    fn svc_immediate() {
        let (instr, e) = parse_instr("svc #0x80");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Svc);
        assert_eq!(instr.operands[0], Operand::Immediate(Expr::Literal(0x80)));
    }

    #[test]
    fn ldr_memory() {
        let (instr, e) = parse_instr("ldr x0, [x1, #8]");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Ldr);
        assert_eq!(instr.operands.len(), 2);
    }

    #[test]
    fn unknown_mnemonic_error() {
        let (instr, e) = parse_instr("foobar x0");
        assert!(instr.is_none());
        assert_eq!(e.len(), 1);
        match &e[0] {
            ingot_types::AsmError::UnknownMnemonic { mnemonic, .. } => {
                assert_eq!(mnemonic, "foobar");
            }
            other => panic!("expected UnknownMnemonic, got {other:?}"),
        }
    }

    #[test]
    fn add_with_shift() {
        let (instr, e) = parse_instr("add x0, x1, x2, lsl #3");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Add);
        assert_eq!(instr.operands.len(), 4);
    }

    #[test]
    fn stp_pre_index() {
        let (instr, e) = parse_instr("stp x29, x30, [sp, #-16]!");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(instr.mnemonic, Mnemonic::Stp);
        assert_eq!(instr.operands.len(), 3);
    }

    #[test]
    fn mov_sp() {
        let (instr, e) = parse_instr("mov sp, x0");
        assert!(e.is_empty());
        let instr = instr.unwrap();
        assert_eq!(
            instr.operands[0],
            Operand::Register(Register::Special(SpecialReg::Sp))
        );
    }
}
