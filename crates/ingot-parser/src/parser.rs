use ingot_lexer::Token;
use ingot_types::{AsmError, Span};

/// Core parser state that drives the recursive descent.
///
/// Pre-lexes all tokens into a `Vec` for trivial lookahead.
pub(crate) struct Parser<'src> {
    pub(crate) source: &'src str,
    pub(crate) tokens: Vec<(Token, Span)>,
    pub(crate) pos: usize,
    pub(crate) errors: Vec<AsmError>,
}

impl<'src> Parser<'src> {
    pub(crate) fn new(
        source: &'src str,
        tokens: Vec<(Token, Span)>,
        lex_errors: Vec<AsmError>,
    ) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
            errors: lex_errors,
        }
    }

    /// Peek at the current token without consuming it.
    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    /// Peek at the span of the current token.
    pub(crate) fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|(_, s)| *s)
            .unwrap_or_else(|| {
                // EOF: point past the last character
                Span::new(self.source.len() as u32, 0)
            })
    }

    /// Return the span of the most recently consumed token.
    pub(crate) fn prev_span(&self) -> Span {
        assert!(self.pos > 0, "prev_span called before any advance");
        self.tokens[self.pos - 1].1
    }

    /// Peek at the token two positions ahead.
    pub(crate) fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|(t, _)| t)
    }

    /// Advance past the current token and return it with its span.
    pub(crate) fn advance(&mut self) -> Option<(Token, Span)> {
        if self.pos < self.tokens.len() {
            let pair = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(pair)
        } else {
            None
        }
    }

    /// Check if the current token matches the expected discriminant (ignoring payload).
    pub(crate) fn check(&self, expected: &Token) -> bool {
        self.peek()
            .map(|t| std::mem::discriminant(t) == std::mem::discriminant(expected))
            .unwrap_or(false)
    }

    /// Consume the current token if it matches the expected discriminant.
    /// Returns the consumed token+span, or `None` if it didn't match.
    pub(crate) fn eat(&mut self, expected: &Token) -> Option<(Token, Span)> {
        if self.check(expected) {
            self.advance()
        } else {
            None
        }
    }

    /// Consume the current token if it matches, otherwise record a parse error.
    pub(crate) fn expect(&mut self, expected: &Token, detail: &str) -> Option<(Token, Span)> {
        if let Some(pair) = self.eat(expected) {
            Some(pair)
        } else {
            self.error(detail);
            None
        }
    }

    /// True if we're at end-of-line (Newline token or EOF).
    pub(crate) fn at_eol(&self) -> bool {
        matches!(self.peek(), None | Some(Token::Newline))
    }

    /// Record a parse error at the current position.
    pub(crate) fn error(&mut self, detail: &str) {
        let span = self.peek_span();
        self.errors.push(AsmError::ParseError {
            detail: detail.to_string(),
            span,
        });
    }

    /// Skip tokens until the next newline or EOF (error recovery).
    pub(crate) fn skip_to_newline(&mut self) {
        while !self.at_eol() {
            self.advance();
        }
    }

    /// Consume a newline if present.
    pub(crate) fn consume_newline(&mut self) {
        self.eat(&Token::Newline);
    }

    /// Expect an integer token and return its value + span.
    pub(crate) fn expect_integer(&mut self, detail: &str) -> Option<(i64, Span)> {
        let (tok, span) = self.expect(&Token::Integer(0), detail)?;
        let Token::Integer(n) = tok else {
            unreachable!()
        };
        Some((n, span))
    }

    /// Expect a string literal token and return its value + span.
    pub(crate) fn expect_string(&mut self, detail: &str) -> Option<(String, Span)> {
        let (tok, span) = self.expect(&Token::StringLiteral(String::new()), detail)?;
        let Token::StringLiteral(s) = tok else {
            unreachable!()
        };
        Some((s, span))
    }

    /// Get the source text for a given span.
    pub(crate) fn slice(&self, span: Span) -> &'src str {
        let start = span.offset as usize;
        let end = start + span.len as usize;
        &self.source[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_lexer::Lexer;

    fn make_parser(source: &str) -> Parser<'_> {
        let (tokens, errors) = Lexer::collect_all(source);
        Parser::new(source, tokens, errors)
    }

    #[test]
    fn peek_and_advance() {
        let mut p = make_parser("add x0");
        assert_eq!(p.peek(), Some(&Token::Ident));
        let (tok, _) = p.advance().unwrap();
        assert_eq!(tok, Token::Ident);
        assert!(p.peek().unwrap().as_register().is_some());
    }

    #[test]
    fn peek2_works() {
        let p = make_parser("_main:");
        assert_eq!(p.peek(), Some(&Token::Ident));
        assert_eq!(p.peek2(), Some(&Token::Colon));
    }

    #[test]
    fn eat_matching() {
        let mut p = make_parser(",");
        assert!(p.eat(&Token::Comma).is_some());
        assert!(p.peek().is_none());
    }

    #[test]
    fn eat_non_matching() {
        let mut p = make_parser(",");
        assert!(p.eat(&Token::Hash).is_none());
        assert!(p.peek().is_some()); // not consumed
    }

    #[test]
    fn expect_failure_records_error() {
        let mut p = make_parser("x0");
        assert!(p.expect(&Token::Hash, "expected #").is_none());
        assert_eq!(p.errors.len(), 1);
    }

    #[test]
    fn at_eol_eof() {
        let p = make_parser("");
        assert!(p.at_eol());
    }

    #[test]
    fn at_eol_newline() {
        let p = make_parser("\n");
        assert!(p.at_eol());
    }

    #[test]
    fn skip_to_newline_advances() {
        let mut p = make_parser("add x0, x1\nmov");
        p.skip_to_newline();
        assert_eq!(p.peek(), Some(&Token::Newline));
    }

    #[test]
    fn slice_returns_source_text() {
        let src = "add x0";
        let p = make_parser(src);
        assert_eq!(p.slice(Span::new(0, 3)), "add");
    }
}
