use ingot_lexer::Token;
use ingot_types::operand::{MemoryOperand, Operand};

use crate::parser::Parser;

impl Parser<'_> {
    /// Parse a single operand.
    pub(crate) fn parse_operand(&mut self) -> Option<Operand> {
        let tok = self.peek()?;

        // Register operand
        if let Some(reg) = tok.as_register() {
            self.advance();
            return Some(Operand::Register(reg));
        }

        // Memory operand: [...]
        if matches!(tok, Token::LBracket) {
            return self.parse_memory_operand().map(Operand::Memory);
        }

        // Immediate with #
        if matches!(tok, Token::Hash) {
            self.advance(); // consume #
            let expr = self.parse_expr()?;
            return Some(Operand::Immediate(expr));
        }

        // Shift modifier: lsl, lsr, asr, ror
        if let Some(op) = tok.as_shift_op() {
            self.advance();
            // Consume optional # before shift amount
            self.eat(&Token::Hash);
            let amount = self.parse_expr()?;
            return Some(Operand::Shift { op, amount });
        }

        // Extend modifier: uxtb, uxth, etc.
        if let Some(op) = tok.as_extend_op() {
            self.advance();
            // Optional amount: #expr or bare expr
            let has_hash = self.eat(&Token::Hash).is_some();
            let amount = if has_hash
                || (!self.at_eol() && !self.check(&Token::Comma) && !self.check(&Token::RBracket))
            {
                Some(self.parse_expr()?)
            } else {
                None
            };
            return Some(Operand::Extend { op, amount });
        }

        // Label reference (identifier not matched as a register/mnemonic)
        if matches!(tok, Token::Ident) {
            let (_, span) = self.advance().unwrap();
            return Some(Operand::Label(self.slice(span).to_string()));
        }

        // Bare integer (without #) as immediate
        if matches!(tok, Token::Integer(_)) {
            let expr = self.parse_expr()?;
            return Some(Operand::Immediate(expr));
        }

        // Bare minus sign starting a negative immediate
        if matches!(tok, Token::Minus) {
            let expr = self.parse_expr()?;
            return Some(Operand::Immediate(expr));
        }

        // Relocation (:lo12:sym)
        if matches!(tok, Token::Colon) {
            let expr = self.parse_expr()?;
            return Some(Operand::Immediate(expr));
        }

        self.error("expected operand");
        None
    }

    /// Parse a memory operand: `[base]`, `[base, offset]`, `[base, offset]!`, `[base], offset`
    fn parse_memory_operand(&mut self) -> Option<MemoryOperand> {
        self.expect(&Token::LBracket, "expected `[`")?;

        // Base register
        let base = match self.peek()?.as_register() {
            Some(reg) => {
                self.advance();
                reg
            }
            None => {
                self.error("expected base register");
                return None;
            }
        };

        // [base] — simple base
        if self.eat(&Token::RBracket).is_some() {
            // Check for post-index: [base], #offset
            if self.eat(&Token::Comma).is_some() {
                if self.eat(&Token::Hash).is_some() {
                    let offset = self.parse_expr()?;
                    return Some(MemoryOperand::PostIndex { reg: base, offset });
                }
                self.error("expected `#` after post-index comma");
                return None;
            }
            return Some(MemoryOperand::Base { reg: base });
        }

        // Must be comma after base
        self.expect(&Token::Comma, "expected `,` or `]`")?;

        // Check what follows the comma
        let next = self.peek()?;

        // Register index: [base, Rm{, extend {#amount}}]
        if let Some(index) = next.as_register() {
            self.advance();

            let mut extend = None;
            let mut amount = None;

            // Optional extend/shift
            if self.eat(&Token::Comma).is_some() {
                if let Some(tok) = self.peek() {
                    if let Some(ext) = tok.as_extend_op() {
                        self.advance();
                        extend = Some(ext);
                    } else if let Some(shift) = tok.as_shift_op() {
                        self.advance();
                        // Convert LSL to UXTX for register index addressing
                        extend = match shift {
                            ingot_types::operand::ShiftOp::Lsl => {
                                Some(ingot_types::operand::ExtendOp::Uxtx)
                            }
                            _ => {
                                self.error("only LSL is valid for register index");
                                return None;
                            }
                        };
                    }
                }
                // Optional #amount after extend/shift
                if extend.is_some() && self.eat(&Token::Hash).is_some() {
                    let expr = self.parse_expr()?;
                    amount = expr.as_literal().map(|v| v as u8);
                }
            }

            self.expect(&Token::RBracket, "expected `]`")?;

            return Some(MemoryOperand::BaseRegister {
                base,
                index,
                extend,
                amount,
            });
        }

        // Immediate offset: [base, #imm] or [base, #imm]!
        if self.eat(&Token::Hash).is_some() {
            let offset = self.parse_expr()?;
            self.expect(&Token::RBracket, "expected `]`")?;

            // Pre-index: [base, #imm]!
            if self.eat(&Token::Exclaim).is_some() {
                return Some(MemoryOperand::PreIndex { reg: base, offset });
            }

            return Some(MemoryOperand::BaseOffset { reg: base, offset });
        }

        self.error("expected register, `#`, or `]`");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_lexer::Lexer;
    use ingot_types::expr::Expr;
    use ingot_types::operand::{ExtendOp, MemoryOperand, Operand, ShiftOp};
    use ingot_types::register::{GpReg, RegWidth, Register, SpecialReg};

    fn parse_operand(source: &str) -> (Option<Operand>, Vec<ingot_types::AsmError>) {
        let (tokens, errors) = Lexer::collect_all(source);
        let mut parser = Parser::new(source, tokens, errors);
        let result = parser.parse_operand();
        (result, parser.errors)
    }

    #[test]
    fn register_operand() {
        let (op, e) = parse_operand("x0");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Register(Register::Gp(GpReg::new(
                0,
                RegWidth::X64
            ))))
        );
    }

    #[test]
    fn immediate_with_hash() {
        let (op, e) = parse_operand("#42");
        assert!(e.is_empty());
        assert_eq!(op, Some(Operand::Immediate(Expr::Literal(42))));
    }

    #[test]
    fn immediate_negative() {
        let (op, e) = parse_operand("#-5");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Immediate(Expr::Unary {
                op: ingot_types::expr::UnaryOp::Neg,
                expr: Box::new(Expr::Literal(5)),
            }))
        );
    }

    #[test]
    fn label_operand() {
        let (op, e) = parse_operand("_loop");
        assert!(e.is_empty());
        assert_eq!(op, Some(Operand::Label("_loop".into())));
    }

    #[test]
    fn shift_operand() {
        let (op, e) = parse_operand("lsl #2");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Shift {
                op: ShiftOp::Lsl,
                amount: Expr::Literal(2),
            })
        );
    }

    #[test]
    fn extend_operand() {
        let (op, e) = parse_operand("sxtw #3");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Extend {
                op: ExtendOp::Sxtw,
                amount: Some(Expr::Literal(3)),
            })
        );
    }

    #[test]
    fn extend_without_amount() {
        let (op, e) = parse_operand("uxtw");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Extend {
                op: ExtendOp::Uxtw,
                amount: None,
            })
        );
    }

    #[test]
    fn memory_base() {
        let (op, e) = parse_operand("[x0]");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::Base {
                reg: Register::Gp(GpReg::new(0, RegWidth::X64)),
            }))
        );
    }

    #[test]
    fn memory_base_sp() {
        let (op, e) = parse_operand("[sp]");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::Base {
                reg: Register::Special(SpecialReg::Sp),
            }))
        );
    }

    #[test]
    fn memory_base_offset() {
        let (op, e) = parse_operand("[x1, #8]");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::BaseOffset {
                reg: Register::Gp(GpReg::new(1, RegWidth::X64)),
                offset: Expr::Literal(8),
            }))
        );
    }

    #[test]
    fn memory_pre_index() {
        let (op, e) = parse_operand("[sp, #-16]!");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::PreIndex {
                reg: Register::Special(SpecialReg::Sp),
                offset: Expr::Unary {
                    op: ingot_types::expr::UnaryOp::Neg,
                    expr: Box::new(Expr::Literal(16)),
                },
            }))
        );
    }

    #[test]
    fn memory_post_index() {
        let (op, e) = parse_operand("[sp], #16");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::PostIndex {
                reg: Register::Special(SpecialReg::Sp),
                offset: Expr::Literal(16),
            }))
        );
    }

    #[test]
    fn memory_base_register() {
        let (op, e) = parse_operand("[x0, x1]");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::BaseRegister {
                base: Register::Gp(GpReg::new(0, RegWidth::X64)),
                index: Register::Gp(GpReg::new(1, RegWidth::X64)),
                extend: None,
                amount: None,
            }))
        );
    }

    #[test]
    fn memory_base_register_with_extend() {
        let (op, e) = parse_operand("[x0, w1, sxtw #2]");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::BaseRegister {
                base: Register::Gp(GpReg::new(0, RegWidth::X64)),
                index: Register::Gp(GpReg::new(1, RegWidth::W32)),
                extend: Some(ExtendOp::Sxtw),
                amount: Some(2),
            }))
        );
    }

    #[test]
    fn memory_base_register_lsl() {
        let (op, e) = parse_operand("[x0, x1, lsl #3]");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Memory(MemoryOperand::BaseRegister {
                base: Register::Gp(GpReg::new(0, RegWidth::X64)),
                index: Register::Gp(GpReg::new(1, RegWidth::X64)),
                extend: Some(ExtendOp::Uxtx),
                amount: Some(3),
            }))
        );
    }

    #[test]
    fn bare_integer_immediate() {
        let (op, e) = parse_operand("42");
        assert!(e.is_empty());
        assert_eq!(op, Some(Operand::Immediate(Expr::Literal(42))));
    }

    #[test]
    fn relocation_operand() {
        let (op, e) = parse_operand(":lo12:msg");
        assert!(e.is_empty());
        assert_eq!(
            op,
            Some(Operand::Immediate(Expr::Relocated {
                modifier: ingot_types::expr::RelocModifier::Lo12,
                symbol: "msg".into(),
                addend: 0,
            }))
        );
    }
}
