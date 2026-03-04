use ingot_types::{AsmError, Directive, Span, Statement};

use crate::context::AssemblyContext;

/// Pass 1: walk statements to collect symbols and compute section sizes.
pub fn pass1(statements: &[Statement], ctx: &mut AssemblyContext) {
    for stmt in statements {
        match stmt {
            Statement::Label { name, span } => {
                ctx.define_symbol(name, *span);
            }
            Statement::Instruction(_) => {
                ctx.advance(4);
            }
            Statement::Directive { directive, span } => {
                handle_directive(directive, *span, ctx);
            }
        }
    }
}

fn handle_directive(directive: &Directive, span: Span, ctx: &mut AssemblyContext) {
    match directive {
        Directive::Text => ctx.switch_section("__TEXT", "__text"),
        Directive::Data => ctx.switch_section("__DATA", "__data"),
        Directive::Section(spec) => ctx.switch_section(&spec.segment, &spec.section),

        Directive::Global(name) => ctx.mark_global(name),
        Directive::Local(_) => {} // no-op for sizing

        Directive::Align(expr) | Directive::P2Align(expr) => {
            if let Some(val) = expr.as_literal() {
                // Apple `as` treats .align as power-of-two on ARM64
                if !(0..=30).contains(&val) {
                    ctx.push_error(AsmError::InvalidAlignment { span });
                } else {
                    let alignment = 1u64 << val;
                    ctx.align_to(alignment);
                }
            } else {
                ctx.push_error(AsmError::UnresolvedExpression { span });
            }
        }

        Directive::Byte(exprs) => ctx.advance(exprs.len() as u64),
        Directive::Short(exprs) => ctx.advance(exprs.len() as u64 * 2),
        Directive::Word(exprs) => ctx.advance(exprs.len() as u64 * 4),
        Directive::Quad(exprs) => ctx.advance(exprs.len() as u64 * 8),

        Directive::Ascii(s) => ctx.advance(s.len() as u64),
        Directive::Asciz(s) => ctx.advance(s.len() as u64 + 1),

        Directive::Space { size, fill: _ } => {
            if let Some(val) = size.as_literal() {
                if val >= 0 {
                    ctx.advance(val as u64);
                }
            } else {
                ctx.push_error(AsmError::UnresolvedExpression { span });
            }
        }

        Directive::SubsectionsViaSymbols | Directive::BuildVersion(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::expr::Expr;
    use ingot_types::instruction::{Instruction, Mnemonic};

    fn label(name: &str, offset: u32) -> Statement {
        Statement::Label {
            name: name.to_string(),
            span: Span::new(offset, name.len() as u32),
        }
    }

    fn nop_inst(offset: u32) -> Statement {
        Statement::Instruction(Instruction {
            mnemonic: Mnemonic::Nop,
            operands: vec![],
            span: Span::new(offset, 3),
        })
    }

    fn dir(d: Directive, offset: u32) -> Statement {
        Statement::Directive {
            directive: d,
            span: Span::new(offset, 1),
        }
    }

    #[test]
    fn empty_program() {
        let mut ctx = AssemblyContext::new();
        pass1(&[], &mut ctx);
        assert_eq!(ctx.current_offset(), 0);
        assert!(ctx.symbols.is_empty());
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn label_at_correct_offsets() {
        let stmts = vec![label("_a", 0), nop_inst(5), nop_inst(10), label("_b", 15)];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        assert_eq!(ctx.symbols["_a"].offset, 0);
        assert_eq!(ctx.symbols["_b"].offset, 8);
    }

    #[test]
    fn data_directive_sizing() {
        let stmts = vec![
            dir(Directive::Byte(vec![Expr::Literal(1), Expr::Literal(2)]), 0),
            dir(Directive::Short(vec![Expr::Literal(1)]), 10),
            dir(Directive::Word(vec![Expr::Literal(1)]), 20),
            dir(Directive::Quad(vec![Expr::Literal(1)]), 30),
        ];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        // 2 + 2 + 4 + 8 = 16
        assert_eq!(ctx.current_offset(), 16);
    }

    #[test]
    fn asciz_includes_null_terminator() {
        let stmts = vec![dir(Directive::Asciz("hello".to_string()), 0)];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        assert_eq!(ctx.current_offset(), 6); // "hello" + NUL
    }

    #[test]
    fn section_switch() {
        let stmts = vec![
            nop_inst(0),
            dir(Directive::Data, 10),
            dir(Directive::Byte(vec![Expr::Literal(0); 8]), 20),
            dir(Directive::Text, 30),
            label("_after_text", 40),
        ];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        assert_eq!(ctx.symbols["_after_text"].offset, 4);
        assert_eq!(
            *ctx.section_offsets
                .get(&("__DATA".to_string(), "__data".to_string()))
                .unwrap(),
            8
        );
    }

    #[test]
    fn align_directive() {
        let stmts = vec![
            dir(Directive::Byte(vec![Expr::Literal(0); 3]), 0),
            dir(Directive::Align(Expr::Literal(2)), 10), // 2^2 = 4-byte align
            label("_aligned", 20),
        ];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        assert_eq!(ctx.symbols["_aligned"].offset, 4);
    }

    #[test]
    fn space_directive() {
        let stmts = vec![dir(
            Directive::Space {
                size: Expr::Literal(16),
                fill: None,
            },
            0,
        )];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        assert_eq!(ctx.current_offset(), 16);
    }

    #[test]
    fn global_marks_name() {
        let stmts = vec![
            dir(Directive::Global("_main".to_string()), 0),
            label("_main", 10),
        ];
        let mut ctx = AssemblyContext::new();
        pass1(&stmts, &mut ctx);
        assert!(ctx.globals.contains("_main"));
        assert!(ctx.symbols.contains_key("_main"));
    }
}
