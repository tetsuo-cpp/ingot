use std::fmt::Write;

use ingot_lexer::Token;
use ingot_types::directive::{BuildVersion, Directive, Platform, SectionSpec};
use ingot_types::Span;

use crate::parser::Parser;

impl Parser<'_> {
    /// Parse directive arguments after the directive token has been identified.
    /// `name` is the directive name without the leading dot, `dir_span` is the span of the directive token.
    ///
    /// Note: `name` borrows from `self.source` (the `&'src str`), not from `&mut self`,
    /// so it remains valid even as we mutate parser state throughout this method.
    pub(crate) fn parse_directive(&mut self, name: &str, dir_span: Span) -> Option<Directive> {
        match name {
            s if s.eq_ignore_ascii_case("text") => Some(Directive::Text),
            s if s.eq_ignore_ascii_case("data") => Some(Directive::Data),
            s if s.eq_ignore_ascii_case("subsections_via_symbols") => {
                Some(Directive::SubsectionsViaSymbols)
            }

            s if s.eq_ignore_ascii_case("global") || s.eq_ignore_ascii_case("globl") => {
                let (_, span) = self.expect(&Token::Ident, "expected symbol name")?;
                Some(Directive::Global(self.slice(span).to_string()))
            }

            s if s.eq_ignore_ascii_case("local") => {
                let (_, span) = self.expect(&Token::Ident, "expected symbol name")?;
                Some(Directive::Local(self.slice(span).to_string()))
            }

            s if s.eq_ignore_ascii_case("align") => {
                let expr = self.parse_expr()?;
                Some(Directive::Align(expr))
            }

            s if s.eq_ignore_ascii_case("p2align") => {
                let expr = self.parse_expr()?;
                Some(Directive::P2Align(expr))
            }

            s if s.eq_ignore_ascii_case("byte") => {
                let exprs = self.parse_expr_list()?;
                Some(Directive::Byte(exprs))
            }

            s if s.eq_ignore_ascii_case("short") || s.eq_ignore_ascii_case("hword") => {
                let exprs = self.parse_expr_list()?;
                Some(Directive::Short(exprs))
            }

            s if s.eq_ignore_ascii_case("word") || s.eq_ignore_ascii_case("long") => {
                let exprs = self.parse_expr_list()?;
                Some(Directive::Word(exprs))
            }

            s if s.eq_ignore_ascii_case("quad") => {
                let exprs = self.parse_expr_list()?;
                Some(Directive::Quad(exprs))
            }

            s if s.eq_ignore_ascii_case("ascii") => {
                let (s, _) = self.expect_string("expected string literal")?;
                Some(Directive::Ascii(s))
            }

            s if s.eq_ignore_ascii_case("asciz") || s.eq_ignore_ascii_case("string") => {
                let (s, _) = self.expect_string("expected string literal")?;
                Some(Directive::Asciz(s))
            }

            s if s.eq_ignore_ascii_case("space") || s.eq_ignore_ascii_case("skip") => {
                let size = self.parse_expr()?;
                let fill = if self.eat(&Token::Comma).is_some() {
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                Some(Directive::Space { size, fill })
            }

            s if s.eq_ignore_ascii_case("section") => {
                let (_, seg_span) = self.expect(&Token::Ident, "expected segment name")?;
                let segment = self.slice(seg_span).to_string();
                self.expect(&Token::Comma, "expected `,`")?;
                let (_, sec_span) = self.expect(&Token::Ident, "expected section name")?;
                let section = self.slice(sec_span).to_string();
                Some(Directive::Section(SectionSpec { segment, section }))
            }

            s if s.eq_ignore_ascii_case("build_version") => self.parse_build_version(),

            _ => {
                self.errors.push(ingot_types::AsmError::InvalidDirective {
                    detail: format!("unknown directive `.{name}`"),
                    span: dir_span,
                });
                None
            }
        }
    }

    /// Parse comma-separated expression list (at least one).
    fn parse_expr_list(&mut self) -> Option<Vec<ingot_types::Expr>> {
        let mut exprs = vec![self.parse_expr()?];
        while self.eat(&Token::Comma).is_some() {
            exprs.push(self.parse_expr()?);
        }
        Some(exprs)
    }

    /// Parse `.build_version platform, major.minor.patch`
    fn parse_build_version(&mut self) -> Option<Directive> {
        let (_, plat_span) = self.expect(&Token::Ident, "expected platform name")?;
        let plat_name = self.slice(plat_span);
        let platform = match Platform::parse(plat_name) {
            Some(p) => p,
            None => {
                self.errors.push(ingot_types::AsmError::InvalidDirective {
                    detail: format!("unknown platform `{plat_name}`"),
                    span: plat_span,
                });
                return None;
            }
        };

        self.expect(&Token::Comma, "expected `,`")?;

        // Version string: major.minor or major.minor.patch
        let (major, _) = self.expect_integer("expected version number")?;
        let mut version = format!("{major}");

        if self.eat(&Token::Dot).is_some() {
            let (minor, _) = self.expect_integer("expected minor version")?;
            write!(version, ".{minor}").unwrap();

            if self.eat(&Token::Dot).is_some() {
                let (patch, _) = self.expect_integer("expected patch version")?;
                write!(version, ".{patch}").unwrap();
            }
        }

        Some(Directive::BuildVersion(BuildVersion {
            platform,
            min_version: version,
        }))
    }
}

#[cfg(test)]
mod tests {
    use ingot_types::directive::{BuildVersion, Directive, Platform, SectionSpec};
    use ingot_types::expr::Expr;

    fn parse_dir(source: &str) -> (Option<Directive>, Vec<ingot_types::AsmError>) {
        let src = format!(".{source}\n");
        let (stmts, errors) = crate::parse(&src);
        let directive = stmts.into_iter().find_map(|s| match s {
            ingot_types::Statement::Directive { directive, .. } => Some(directive),
            _ => None,
        });
        (directive, errors)
    }

    #[test]
    fn text_directive() {
        let (d, e) = parse_dir("text");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Text));
    }

    #[test]
    fn data_directive() {
        let (d, e) = parse_dir("data");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Data));
    }

    #[test]
    fn global_directive() {
        let (d, e) = parse_dir("global _main");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Global("_main".into())));
    }

    #[test]
    fn globl_directive() {
        let (d, e) = parse_dir("globl _start");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Global("_start".into())));
    }

    #[test]
    fn byte_directive() {
        let (d, e) = parse_dir("byte 1, 2, 3");
        assert!(e.is_empty());
        assert_eq!(
            d,
            Some(Directive::Byte(vec![
                Expr::Literal(1),
                Expr::Literal(2),
                Expr::Literal(3),
            ]))
        );
    }

    #[test]
    fn asciz_directive() {
        let (d, e) = parse_dir(r#"asciz "hello""#);
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Asciz("hello".into())));
    }

    #[test]
    fn space_directive() {
        let (d, e) = parse_dir("space 16");
        assert!(e.is_empty());
        assert_eq!(
            d,
            Some(Directive::Space {
                size: Expr::Literal(16),
                fill: None,
            })
        );
    }

    #[test]
    fn space_with_fill() {
        let (d, e) = parse_dir("space 16, 0xff");
        assert!(e.is_empty());
        assert_eq!(
            d,
            Some(Directive::Space {
                size: Expr::Literal(16),
                fill: Some(Expr::Literal(255)),
            })
        );
    }

    #[test]
    fn section_directive() {
        let (d, e) = parse_dir("section __TEXT, __text");
        assert!(e.is_empty());
        assert_eq!(
            d,
            Some(Directive::Section(SectionSpec {
                segment: "__TEXT".into(),
                section: "__text".into(),
            }))
        );
    }

    #[test]
    fn subsections_via_symbols() {
        let (d, e) = parse_dir("subsections_via_symbols");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::SubsectionsViaSymbols));
    }

    #[test]
    fn build_version_directive() {
        let (d, e) = parse_dir("build_version macos, 14.0");
        assert!(e.is_empty());
        assert_eq!(
            d,
            Some(Directive::BuildVersion(BuildVersion {
                platform: Platform::MacOS,
                min_version: "14.0".into(),
            }))
        );
    }

    #[test]
    fn build_version_with_patch() {
        let (d, e) = parse_dir("build_version macos, 14.0.0");
        assert!(e.is_empty());
        assert_eq!(
            d,
            Some(Directive::BuildVersion(BuildVersion {
                platform: Platform::MacOS,
                min_version: "14.0.0".into(),
            }))
        );
    }

    #[test]
    fn unknown_directive_error() {
        let (d, e) = parse_dir("foobar");
        assert!(d.is_none());
        assert_eq!(e.len(), 1);
    }

    #[test]
    fn align_directive() {
        let (d, e) = parse_dir("align 4");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Align(Expr::Literal(4))));
    }

    #[test]
    fn p2align_directive() {
        let (d, e) = parse_dir("p2align 2");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::P2Align(Expr::Literal(2))));
    }

    #[test]
    fn word_directive() {
        let (d, e) = parse_dir("word 0x1234");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Word(vec![Expr::Literal(0x1234)])));
    }

    #[test]
    fn quad_directive() {
        let (d, e) = parse_dir("quad 100");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Quad(vec![Expr::Literal(100)])));
    }

    #[test]
    fn local_directive() {
        let (d, e) = parse_dir("local _helper");
        assert!(e.is_empty());
        assert_eq!(d, Some(Directive::Local("_helper".into())));
    }
}
