use crate::directive::Directive;
use crate::instruction::Instruction;
use crate::span::Span;

/// A single parsed statement.
///
/// A line like `_main: nop` produces two statements (`Label` + `Instruction`),
/// not a compound type. This simplifies assembler pass1, which processes labels
/// and instructions independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Label { name: String, span: Span },
    Instruction(Instruction),
    Directive { directive: Directive, span: Span },
}
