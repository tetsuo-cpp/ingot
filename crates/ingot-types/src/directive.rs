use crate::expr::Expr;

/// A Mach-O section specifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSpec {
    pub segment: String,
    pub section: String,
}

/// Target platform for `.build_version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOS,
    IOS,
    TvOS,
    WatchOS,
}

/// Build version metadata from `.build_version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildVersion {
    pub platform: Platform,
    pub min_version: String,
}

/// Assembler directives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Directive {
    /// `.text`
    Text,
    /// `.data`
    Data,
    /// `.section segment,section`
    Section(SectionSpec),
    /// `.global symbol`
    Global(String),
    /// `.local symbol`
    Local(String),
    /// `.align boundary`
    Align(Expr),
    /// `.p2align power`
    P2Align(Expr),
    /// `.byte expr, ...`
    Byte(Vec<Expr>),
    /// `.short expr, ...` / `.hword`
    Short(Vec<Expr>),
    /// `.word expr, ...` / `.long`
    Word(Vec<Expr>),
    /// `.quad expr, ...`
    Quad(Vec<Expr>),
    /// `.ascii "string"`
    Ascii(String),
    /// `.asciz "string"` / `.string`
    Asciz(String),
    /// `.space size[, fill]` / `.skip`
    Space { size: Expr, fill: Option<Expr> },
    /// `.subsections_via_symbols`
    SubsectionsViaSymbols,
    /// `.build_version platform, version`
    BuildVersion(BuildVersion),
}
