use ingot_lexer::Token;
use ingot_types::expr::{BinOp, Expr, RelocModifier, UnaryOp};

use crate::parser::Parser;

impl Parser<'_> {
    /// Parse an expression (Pratt parser entry point).
    pub(crate) fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_expr_bp(0)
    }

    /// Pratt parser with binding power.
    fn parse_expr_bp(&mut self, min_bp: u8) -> Option<Expr> {
        let mut lhs = self.parse_expr_atom()?;

        while let Some(tok) = self.peek() {
            let (op, l_bp, r_bp) = match tok {
                Token::Plus => (BinOp::Add, 1, 2),
                Token::Minus => (BinOp::Sub, 1, 2),
                Token::Star => (BinOp::Mul, 3, 4),
                Token::Slash => (BinOp::Div, 3, 4),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.advance(); // consume operator

            let rhs = match self.parse_expr_bp(r_bp) {
                Some(r) => r,
                None => {
                    self.error("expected expression after operator");
                    return Some(lhs);
                }
            };

            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Some(lhs)
    }

    /// Parse an atom in an expression.
    fn parse_expr_atom(&mut self) -> Option<Expr> {
        match self.peek()? {
            Token::Integer(_) => {
                let (n, _) = self.expect_integer("expected integer")?;
                Some(Expr::Literal(n))
            }
            Token::Minus => {
                self.advance();
                let inner = self.parse_expr_atom()?;
                Some(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(inner),
                })
            }
            Token::Tilde => {
                self.advance();
                let inner = self.parse_expr_atom()?;
                Some(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(inner),
                })
            }
            Token::LParen => {
                self.advance(); // consume (
                let inner = self.parse_expr_bp(0)?;
                self.expect(&Token::RParen, "expected `)`")?;
                Some(inner)
            }
            Token::Dot => {
                self.advance();
                Some(Expr::CurrentAddress)
            }
            Token::Ident => {
                let (_, span) = self.advance().unwrap();
                let name = self.slice(span).to_string();
                Some(Expr::Symbol(name))
            }
            Token::Colon => {
                // Relocation modifier: :lo12:sym, :pg_hi21:sym, etc.
                self.parse_reloc_expr()
            }
            _ => {
                self.error("expected expression");
                None
            }
        }
    }

    /// Parse a relocation expression like `:lo12:symbol` or `:lo12:symbol+addend`.
    fn parse_reloc_expr(&mut self) -> Option<Expr> {
        let start_span = self.peek_span();
        self.expect(&Token::Colon, "expected `:`")?;

        // Read modifier name
        let (_, mod_span) = self.expect(&Token::Ident, "expected relocation modifier")?;
        let mod_name = self.slice(mod_span);

        let modifier = match RelocModifier::parse(mod_name) {
            Some(m) => m,
            None => {
                self.errors.push(ingot_types::AsmError::ParseError {
                    detail: format!("unknown relocation modifier `{mod_name}`"),
                    span: start_span.merge(mod_span),
                });
                return None;
            }
        };

        self.expect(&Token::Colon, "expected `:` after relocation modifier")?;

        let (_, sym_span) = self.expect(&Token::Ident, "expected symbol name")?;
        let symbol = self.slice(sym_span).to_string();

        // Optional addend: +N or -N
        let addend = if self.check(&Token::Plus) || self.check(&Token::Minus) {
            let negate = self.check(&Token::Minus);
            self.advance();
            let (n, _) = self.expect_integer("expected integer addend")?;
            if negate {
                -n
            } else {
                n
            }
        } else {
            0
        };

        Some(Expr::Relocated {
            modifier,
            symbol,
            addend,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_lexer::Lexer;
    use ingot_types::expr::{BinOp, Expr, UnaryOp};

    fn parse_expr(source: &str) -> Option<Expr> {
        let (tokens, errors) = Lexer::collect_all(source);
        let mut parser = Parser::new(source, tokens, errors);
        parser.parse_expr()
    }

    #[test]
    fn literal() {
        assert_eq!(parse_expr("42"), Some(Expr::Literal(42)));
    }

    #[test]
    fn hex_literal() {
        assert_eq!(parse_expr("0xff"), Some(Expr::Literal(255)));
    }

    #[test]
    fn symbol() {
        assert_eq!(parse_expr("foo"), Some(Expr::Symbol("foo".into())));
    }

    #[test]
    fn current_address() {
        assert_eq!(parse_expr("."), Some(Expr::CurrentAddress));
    }

    #[test]
    fn unary_neg() {
        assert_eq!(
            parse_expr("-5"),
            Some(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(Expr::Literal(5)),
            })
        );
    }

    #[test]
    fn unary_not() {
        assert_eq!(
            parse_expr("~0xff"),
            Some(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(Expr::Literal(255)),
            })
        );
    }

    #[test]
    fn binary_add() {
        assert_eq!(
            parse_expr("1 + 2"),
            Some(Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(Expr::Literal(1)),
                rhs: Box::new(Expr::Literal(2)),
            })
        );
    }

    #[test]
    fn precedence_mul_over_add() {
        // 1 + 2 * 3 => 1 + (2 * 3)
        let expr = parse_expr("1 + 2 * 3").unwrap();
        match expr {
            Expr::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                assert_eq!(*lhs, Expr::Literal(1));
                match *rhs {
                    Expr::Binary {
                        op: BinOp::Mul,
                        lhs,
                        rhs,
                    } => {
                        assert_eq!(*lhs, Expr::Literal(2));
                        assert_eq!(*rhs, Expr::Literal(3));
                    }
                    other => panic!("expected Mul, got {other:?}"),
                }
            }
            other => panic!("expected Add, got {other:?}"),
        }
    }

    #[test]
    fn left_associative() {
        // 1 - 2 - 3 => (1 - 2) - 3
        let expr = parse_expr("1 - 2 - 3").unwrap();
        match expr {
            Expr::Binary {
                op: BinOp::Sub,
                lhs,
                rhs,
            } => {
                assert_eq!(*rhs, Expr::Literal(3));
                match *lhs {
                    Expr::Binary {
                        op: BinOp::Sub,
                        lhs,
                        rhs,
                    } => {
                        assert_eq!(*lhs, Expr::Literal(1));
                        assert_eq!(*rhs, Expr::Literal(2));
                    }
                    other => panic!("expected inner Sub, got {other:?}"),
                }
            }
            other => panic!("expected Sub, got {other:?}"),
        }
    }

    #[test]
    fn parenthesized() {
        // (1 + 2) * 3 => Mul(Add(1, 2), 3)
        let expr = parse_expr("(1 + 2) * 3").unwrap();
        assert_eq!(expr.as_literal(), Some(9));
    }

    #[test]
    fn reloc_lo12() {
        assert_eq!(
            parse_expr(":lo12:msg"),
            Some(Expr::Relocated {
                modifier: RelocModifier::Lo12,
                symbol: "msg".into(),
                addend: 0,
            })
        );
    }

    #[test]
    fn reloc_with_addend() {
        assert_eq!(
            parse_expr(":lo12:msg+4"),
            Some(Expr::Relocated {
                modifier: RelocModifier::Lo12,
                symbol: "msg".into(),
                addend: 4,
            })
        );
    }

    #[test]
    fn reloc_with_negative_addend() {
        assert_eq!(
            parse_expr(":lo12:msg-8"),
            Some(Expr::Relocated {
                modifier: RelocModifier::Lo12,
                symbol: "msg".into(),
                addend: -8,
            })
        );
    }

    #[test]
    fn symbol_plus_offset() {
        // foo + 8
        let expr = parse_expr("foo + 8").unwrap();
        match expr {
            Expr::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                assert_eq!(*lhs, Expr::Symbol("foo".into()));
                assert_eq!(*rhs, Expr::Literal(8));
            }
            other => panic!("expected Binary Add, got {other:?}"),
        }
    }
}
