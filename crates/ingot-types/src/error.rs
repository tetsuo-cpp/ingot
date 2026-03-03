use miette::Diagnostic;
use thiserror::Error;

use crate::span::Span;

/// A single assembler error with source location.
#[derive(Debug, Error, Diagnostic)]
#[allow(unused_assignments)] // thiserror 2.x derive triggers this on some toolchains
pub enum AsmError {
    #[error("unknown mnemonic `{mnemonic}`")]
    UnknownMnemonic {
        mnemonic: String,
        #[label("not a recognized instruction")]
        span: Span,
    },

    #[error("invalid operand")]
    InvalidOperand {
        detail: String,
        #[label("{detail}")]
        span: Span,
    },

    #[error("immediate value {value} out of range")]
    ImmediateOutOfRange {
        value: i64,
        #[label("value does not fit")]
        span: Span,
    },

    #[error("undefined symbol `{name}`")]
    UndefinedSymbol {
        name: String,
        #[label("not defined")]
        span: Span,
    },

    #[error("duplicate symbol `{name}`")]
    DuplicateSymbol {
        name: String,
        #[label("already defined")]
        span: Span,
    },

    #[error("branch target out of range")]
    BranchOutOfRange {
        #[label("target too far")]
        span: Span,
    },

    #[error("invalid register `{name}`")]
    InvalidRegister {
        name: String,
        #[label("not a valid register")]
        span: Span,
    },

    #[error("wrong number of operands: expected {expected}, got {got}")]
    WrongOperandCount {
        expected: usize,
        got: usize,
        #[label("wrong operand count")]
        span: Span,
    },

    #[error("invalid directive")]
    InvalidDirective {
        detail: String,
        #[label("{detail}")]
        span: Span,
    },

    #[error("lexer error")]
    LexError {
        detail: String,
        #[label("{detail}")]
        span: Span,
    },

    #[error("parse error")]
    ParseError {
        detail: String,
        #[label("{detail}")]
        span: Span,
    },

    #[error("invalid alignment")]
    InvalidAlignment {
        #[label("alignment must be a power of two")]
        span: Span,
    },

    #[error("unresolved expression")]
    UnresolvedExpression {
        #[label("cannot resolve at assembly time")]
        span: Span,
    },
}

/// A collection of assembler errors for multi-error reporting.
#[derive(Debug, Error, Diagnostic)]
#[error("assembly failed with {} error(s)", .errors.len())]
pub struct AsmErrors {
    #[related]
    pub errors: Vec<AsmError>,
}

impl AsmErrors {
    pub fn new(errors: Vec<AsmError>) -> Self {
        Self { errors }
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = AsmError::UnknownMnemonic {
            mnemonic: "foobar".into(),
            span: Span::new(0, 6),
        };
        assert_eq!(e.to_string(), "unknown mnemonic `foobar`");
    }

    #[test]
    fn errors_collection() {
        let errs = AsmErrors::new(vec![
            AsmError::UnknownMnemonic {
                mnemonic: "bad1".into(),
                span: Span::new(0, 4),
            },
            AsmError::UndefinedSymbol {
                name: "missing".into(),
                span: Span::new(10, 7),
            },
        ]);
        assert!(!errs.is_empty());
        assert_eq!(errs.errors.len(), 2);
        assert_eq!(errs.to_string(), "assembly failed with 2 error(s)");
    }
}
