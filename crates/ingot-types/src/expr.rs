/// Relocation modifier for symbol references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelocModifier {
    /// `:lo12:sym` — page offset, bits [11:0]
    Lo12,
    /// `:pg_hi21:sym` — page address, bits [32:12]
    PgHi21,
    /// `:got:sym` — GOT page
    Got,
    /// `:got_lo12:sym` — GOT page offset
    GotLo12,
}

/// Binary operators in assembly expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Unary operators in assembly expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// An expression that can appear as an immediate operand.
///
/// Assembly immediates aren't always literal integers — they can reference
/// symbols, apply relocation modifiers, or use arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// An integer literal.
    Literal(i64),
    /// A symbol reference.
    Symbol(String),
    /// A relocated symbol: `modifier(symbol + addend)`.
    Relocated {
        modifier: RelocModifier,
        symbol: String,
        addend: i64,
    },
    /// Binary operation.
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// Unary operation.
    Unary { op: UnaryOp, expr: Box<Expr> },
    /// `.` — current address.
    CurrentAddress,
}

impl Expr {
    /// If this expression is a compile-time integer constant, return it.
    pub fn as_literal(&self) -> Option<i64> {
        match self {
            Expr::Literal(v) => Some(*v),
            Expr::Unary {
                op: UnaryOp::Neg,
                expr,
            } => expr.as_literal().map(|v| -v),
            Expr::Unary {
                op: UnaryOp::Not,
                expr,
            } => expr.as_literal().map(|v| !v),
            Expr::Binary { op, lhs, rhs } => {
                let l = lhs.as_literal()?;
                let r = rhs.as_literal()?;
                Some(match op {
                    BinOp::Add => l.wrapping_add(r),
                    BinOp::Sub => l.wrapping_sub(r),
                    BinOp::Mul => l.wrapping_mul(r),
                    BinOp::Div => {
                        if r == 0 {
                            return None;
                        }
                        l.wrapping_div(r)
                    }
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_folds() {
        assert_eq!(Expr::Literal(42).as_literal(), Some(42));
    }

    #[test]
    fn symbol_does_not_fold() {
        assert_eq!(Expr::Symbol("foo".into()).as_literal(), None);
    }

    #[test]
    fn neg_folds() {
        let e = Expr::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(Expr::Literal(5)),
        };
        assert_eq!(e.as_literal(), Some(-5));
    }

    #[test]
    fn binary_folds() {
        let e = Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::Literal(10)),
            rhs: Box::new(Expr::Literal(32)),
        };
        assert_eq!(e.as_literal(), Some(42));
    }

    #[test]
    fn div_by_zero_returns_none() {
        let e = Expr::Binary {
            op: BinOp::Div,
            lhs: Box::new(Expr::Literal(10)),
            rhs: Box::new(Expr::Literal(0)),
        };
        assert_eq!(e.as_literal(), None);
    }

    #[test]
    fn nested_binary_folds() {
        // (3 + 4) * 2 = 14
        let e = Expr::Binary {
            op: BinOp::Mul,
            lhs: Box::new(Expr::Binary {
                op: BinOp::Add,
                lhs: Box::new(Expr::Literal(3)),
                rhs: Box::new(Expr::Literal(4)),
            }),
            rhs: Box::new(Expr::Literal(2)),
        };
        assert_eq!(e.as_literal(), Some(14));
    }

    #[test]
    fn symbol_in_binary_blocks_folding() {
        let e = Expr::Binary {
            op: BinOp::Add,
            lhs: Box::new(Expr::Symbol("x".into())),
            rhs: Box::new(Expr::Literal(1)),
        };
        assert_eq!(e.as_literal(), None);
    }
}
