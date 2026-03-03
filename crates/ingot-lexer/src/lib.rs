//! ARM64 tokenizer using the `logos` crate.

pub mod token;

pub use token::Token;

use ingot_types::{AsmError, Span};
use logos::Logos;

/// A lexer that wraps `logos::Lexer` and produces `(Token, Span)` pairs.
pub struct Lexer<'src> {
    inner: logos::SpannedIter<'src, Token>,
    source: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            inner: Token::lexer(source).spanned(),
            source,
        }
    }

    /// The original source text.
    pub fn source(&self) -> &'src str {
        self.source
    }

    /// Lex all tokens, separating successes from errors.
    pub fn collect_all(source: &str) -> (Vec<(Token, Span)>, Vec<AsmError>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        for result in Lexer::new(source) {
            match result {
                Ok(pair) => tokens.push(pair),
                Err(e) => errors.push(e),
            }
        }
        (tokens, errors)
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<(Token, Span), AsmError>;

    fn next(&mut self) -> Option<Self::Item> {
        let (result, range) = self.inner.next()?;
        let span = Span::new(range.start as u32, (range.end - range.start) as u32);
        Some(match result {
            Ok(token) => Ok((token, span)),
            Err(()) => {
                let text = &self.source[range.start..range.end];
                let detail = if text == "/*" {
                    "unterminated block comment".to_string()
                } else {
                    format!("unexpected character `{text}`")
                };
                Err(AsmError::LexError { detail, span })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unterminated_block_comment_error() {
        let errors: Vec<_> = Lexer::new("/* no end").filter_map(|r| r.err()).collect();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            AsmError::LexError { detail, .. } => {
                assert_eq!(detail, "unterminated block comment");
            }
            other => panic!("expected LexError, got {other:?}"),
        }
    }
}
