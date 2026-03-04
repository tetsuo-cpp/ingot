use ingot_lexer::Token;
use ingot_types::Statement;

use crate::parser::Parser;

impl Parser<'_> {
    /// Parse all statements from the token stream.
    pub(crate) fn parse_all(&mut self) -> Vec<Statement> {
        let mut stmts = Vec::new();

        while self.peek().is_some() {
            self.parse_line(&mut stmts);
            self.consume_newline();
        }

        stmts
    }

    /// Parse a single line, which may produce 0-2 statements (label + instruction/directive).
    fn parse_line(&mut self, stmts: &mut Vec<Statement>) {
        // Skip blank lines
        if self.at_eol() {
            return;
        }

        // Check for label: Ident followed by Colon
        if matches!(self.peek(), Some(Token::Ident)) && matches!(self.peek2(), Some(Token::Colon)) {
            let (_, name_span) = self.advance().unwrap(); // consume ident
            let (_, colon_span) = self.advance().unwrap(); // consume colon
            let label_name = self.slice(name_span).to_string();
            let span = name_span.merge(colon_span);
            stmts.push(Statement::Label {
                name: label_name,
                span,
            });

            // After label, there may be an instruction/directive on the same line
            if self.at_eol() {
                return;
            }
        }

        // Dispatch on current token
        match self.peek() {
            Some(Token::Directive) => {
                let (_, dir_span) = self.advance().unwrap();
                let name = &self.slice(dir_span)[1..]; // strip leading dot
                match self.parse_directive(name, dir_span) {
                    Some(directive) => {
                        stmts.push(Statement::Directive {
                            directive,
                            span: dir_span,
                        });
                    }
                    None => {
                        self.skip_to_newline();
                    }
                }
            }
            Some(Token::Ident) | Some(Token::FpRegB(_)) => {
                // FpRegB covers `b0`..`b31` but `b` alone is Ident.
                // Mnemonics come through as Ident.
                let (_, mnem_span) = self.advance().unwrap();
                let name = self.slice(mnem_span);
                match self.parse_instruction(name, mnem_span) {
                    Some(instr) => {
                        stmts.push(Statement::Instruction(instr));
                    }
                    None => {
                        self.skip_to_newline();
                    }
                }
            }
            Some(Token::Newline) | None => {
                // Empty line, already handled above
            }
            _ => {
                self.error("expected instruction, directive, or label");
                self.skip_to_newline();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;
    use ingot_types::directive::Directive;
    use ingot_types::instruction::Mnemonic;
    use ingot_types::Span;
    use pretty_assertions::assert_eq;

    #[test]
    fn empty_source() {
        let (stmts, errors) = parse("");
        assert!(errors.is_empty());
        assert!(stmts.is_empty());
    }

    #[test]
    fn single_instruction() {
        let (stmts, errors) = parse("nop\n");
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Nop);
            }
            other => panic!("expected Instruction, got {other:?}"),
        }
    }

    #[test]
    fn label_only() {
        let (stmts, errors) = parse("_main:\n");
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::Label { name, .. } => assert_eq!(name, "_main"),
            other => panic!("expected Label, got {other:?}"),
        }
    }

    #[test]
    fn label_with_instruction() {
        let (stmts, errors) = parse("_main: nop\n");
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 2);
        assert!(matches!(&stmts[0], Statement::Label { name, .. } if name == "_main"));
        assert!(matches!(&stmts[1], Statement::Instruction(i) if i.mnemonic == Mnemonic::Nop));
    }

    #[test]
    fn directive_line() {
        let (stmts, errors) = parse(".global _main\n");
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::Directive { directive, .. } => {
                assert_eq!(*directive, Directive::Global("_main".into()));
            }
            other => panic!("expected Directive, got {other:?}"),
        }
    }

    #[test]
    fn multiple_lines() {
        let src = ".global _main\n_main:\n    mov x0, #0\n    ret\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 4); // global, label, mov, ret
    }

    #[test]
    fn error_recovery_continues() {
        let src = "foobar x0\nmov x0, #1\n";
        let (stmts, errors) = parse(src);
        assert_eq!(errors.len(), 1); // foobar is unknown
        assert_eq!(stmts.len(), 1); // mov still parsed
    }

    #[test]
    fn blank_lines_ignored() {
        let src = "\n\nnop\n\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn label_span_includes_colon() {
        let (stmts, _) = parse("foo:\n");
        match &stmts[0] {
            Statement::Label { span, .. } => {
                assert_eq!(*span, Span::new(0, 4)); // "foo:"
            }
            other => panic!("expected Label, got {other:?}"),
        }
    }

    #[test]
    fn full_program() {
        let src = ".global _main\n_main:\n    mov x0, #0\n    mov x16, #1\n    svc #0x80\n";
        let (stmts, errors) = parse(src);
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 5); // global, label, mov, mov, svc
    }

    #[test]
    fn instruction_with_memory_operand() {
        let (stmts, errors) = parse("ldr x0, [x1, #8]\n");
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::Ldr);
                assert_eq!(instr.operands.len(), 2);
            }
            other => panic!("expected Instruction, got {other:?}"),
        }
    }

    #[test]
    fn b_cond_statement() {
        let (stmts, errors) = parse("b.eq target\n");
        assert!(errors.is_empty());
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            Statement::Instruction(instr) => {
                assert_eq!(instr.mnemonic, Mnemonic::BCond);
            }
            other => panic!("expected Instruction, got {other:?}"),
        }
    }
}
