//! ARM64 assembly parser.
//!
//! Hand-written recursive descent parser that consumes the lexer's token stream
//! and produces `Vec<Statement>`. Line-oriented: each line may have an optional
//! label, then an instruction or directive.

mod directive;
mod expr;
mod instruction;
mod operand;
mod parser;
mod statement;

use ingot_lexer::Lexer;
use ingot_types::{AsmError, Statement};

/// Parse ARM64 assembly source into statements.
///
/// Returns both statements and errors (not `Result`) — follows the project's
/// "collect all errors" philosophy.
pub fn parse(source: &str) -> (Vec<Statement>, Vec<AsmError>) {
    let (tokens, lex_errors) = Lexer::collect_all(source);
    let mut parser = parser::Parser::new(source, tokens, lex_errors);
    let stmts = parser.parse_all();
    (stmts, parser.errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::directive::Directive;
    use ingot_types::expr::Expr;
    use ingot_types::instruction::Mnemonic;
    use ingot_types::operand::{Condition, Operand};
    use ingot_types::register::{GpReg, RegWidth, Register};
    use pretty_assertions::assert_eq;

    #[test]
    fn hello_world_program() {
        let src = ".global _main\n_main:\n    mov x0, #0\n    mov x16, #1\n    svc #0x80\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 5);

        // .global _main
        assert!(matches!(&stmts[0], Statement::Directive {
            directive: Directive::Global(name), ..
        } if name == "_main"));

        // _main:
        assert!(matches!(&stmts[1], Statement::Label { name, .. } if name == "_main"));

        // mov x0, #0
        match &stmts[2] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Mov);
                assert_eq!(
                    instr.operands[0],
                    Operand::Register(Register::Gp(GpReg::new(0, RegWidth::X64)))
                );
                assert_eq!(instr.operands[1], Operand::Immediate(Expr::Literal(0)));
            }
            other => panic!("expected Instruction, got {other:?}"),
        }

        // mov x16, #1
        match &stmts[3] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Mov);
            }
            other => panic!("expected Instruction, got {other:?}"),
        }

        // svc #0x80
        match &stmts[4] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Svc);
                assert_eq!(instr.operands[0], Operand::Immediate(Expr::Literal(0x80)));
            }
            other => panic!("expected Instruction, got {other:?}"),
        }
    }

    #[test]
    fn conditional_branch_program() {
        let src = "    cmp x0, #0\n    b.eq done\n    sub x0, x0, #1\ndone:\n    ret\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 5); // cmp, b.eq, sub, done:, ret

        match &stmts[1] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::BCond);
                assert_eq!(instr.operands[0], Operand::Condition(Condition::Eq));
                assert_eq!(instr.operands[1], Operand::Label("done".into()));
            }
            other => panic!("expected BCond instruction, got {other:?}"),
        }
    }

    #[test]
    fn load_store_program() {
        let src = "    stp x29, x30, [sp, #-16]!\n    ldr x0, [x1, #8]\n    ldp x29, x30, [sp], #16\n    ret\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 4);

        // stp x29, x30, [sp, #-16]!
        match &stmts[0] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Stp);
                assert_eq!(instr.operands.len(), 3);
            }
            other => panic!("expected Instruction, got {other:?}"),
        }
    }

    #[test]
    fn multiple_directives() {
        let src = ".text\n.global _main\n.p2align 2\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 3);
    }

    #[test]
    fn data_section() {
        let src = ".data\nmsg:\n    .asciz \"hello\\n\"\n.text\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 4); // .data, msg:, .asciz, .text
    }

    #[test]
    fn error_recovery_multi_line() {
        let src = "    mov x0, #1\n    badinstruction\n    mov x1, #2\n";
        let (stmts, errors) = parse(src);
        assert_eq!(errors.len(), 1);
        assert_eq!(stmts.len(), 2); // both mov instructions parsed
    }

    #[test]
    fn adrp_with_relocation() {
        let src = "    adrp x0, _msg@PAGE\n";
        // @PAGE isn't the :modifier: syntax, so this will error,
        // but the parser should not panic
        let (_stmts, _errors) = parse(src);
    }

    #[test]
    fn add_with_relocation() {
        let src = "    add x0, x0, :lo12:msg\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn label_on_same_line_as_directive() {
        let src = "msg: .asciz \"hello\"\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 2);
        assert!(matches!(&stmts[0], Statement::Label { name, .. } if name == "msg"));
        assert!(matches!(&stmts[1], Statement::Directive { .. }));
    }

    #[test]
    fn subsections_via_symbols() {
        let src = ".subsections_via_symbols\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            &stmts[0],
            Statement::Directive {
                directive: Directive::SubsectionsViaSymbols,
                ..
            }
        ));
    }

    #[test]
    fn build_version() {
        let src = ".build_version macos, 14.0\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn comments_are_ignored() {
        let src = "    mov x0, #1 // set exit code\n    ret ; return\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn cbz_instruction() {
        let (stmts, errors) = parse("cbz x0, target\n");
        assert!(errors.is_empty());
        match &stmts[0] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Cbz);
                assert_eq!(instr.operands.len(), 2);
            }
            other => panic!("expected Instruction, got {other:?}"),
        }
    }

    #[test]
    fn byte_directive_with_expressions() {
        let (stmts, errors) = parse(".byte 1, 2+3, 4*5\n");
        assert!(errors.is_empty());
        match &stmts[0] {
            Statement::Directive {
                directive: Directive::Byte(exprs),
                ..
            } => {
                assert_eq!(exprs.len(), 3);
                assert_eq!(exprs[0].as_literal(), Some(1));
                assert_eq!(exprs[1].as_literal(), Some(5));
                assert_eq!(exprs[2].as_literal(), Some(20));
            }
            other => panic!("expected Byte directive, got {other:?}"),
        }
    }
}
