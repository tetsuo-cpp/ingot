use std::collections::{HashMap, HashSet};

use ingot_types::{AsmError, Span};

/// Information about a defined symbol.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SymbolInfo {
    pub section: (String, String),
    pub offset: u64,
    pub span: Span,
}

/// Tracks symbols, sections, and offsets during assembly.
pub struct AssemblyContext {
    pub symbols: HashMap<String, SymbolInfo>,
    pub globals: HashSet<String>,
    pub current_section: (String, String),
    pub section_offsets: HashMap<(String, String), u64>,
    pub errors: Vec<AsmError>,
}

impl AssemblyContext {
    pub fn new() -> Self {
        let initial = ("__TEXT".to_string(), "__text".to_string());
        let mut section_offsets = HashMap::new();
        section_offsets.insert(initial.clone(), 0);
        Self {
            symbols: HashMap::new(),
            globals: HashSet::new(),
            current_section: initial,
            section_offsets,
            errors: Vec::new(),
        }
    }

    /// Switch to a different section, creating it if it doesn't exist.
    pub fn switch_section(&mut self, segment: &str, section: &str) {
        let key = (segment.to_string(), section.to_string());
        self.section_offsets.entry(key.clone()).or_insert(0);
        self.current_section = key;
    }

    /// Current offset in the current section.
    pub fn current_offset(&self) -> u64 {
        self.section_offsets
            .get(&self.current_section)
            .copied()
            .unwrap_or(0)
    }

    /// Advance the current section offset by `n` bytes.
    pub fn advance(&mut self, n: u64) {
        // Use get_mut to avoid cloning current_section on every call.
        if let Some(offset) = self.section_offsets.get_mut(&self.current_section) {
            *offset += n;
        }
    }

    /// Align the current section offset to `alignment` (must be power of two).
    pub fn align_to(&mut self, alignment: u64) {
        let offset = self.current_offset();
        let aligned = (offset + alignment - 1) & !(alignment - 1);
        let padding = aligned - offset;
        if padding > 0 {
            self.advance(padding);
        }
    }

    /// Define a symbol at the current position.
    pub fn define_symbol(&mut self, name: &str, span: Span) {
        if self.symbols.contains_key(name) {
            self.errors.push(AsmError::DuplicateSymbol {
                name: name.to_string(),
                span,
            });
            return;
        }
        self.symbols.insert(
            name.to_string(),
            SymbolInfo {
                section: self.current_section.clone(),
                offset: self.current_offset(),
                span,
            },
        );
    }

    /// Mark a symbol as global.
    pub fn mark_global(&mut self, name: &str) {
        self.globals.insert(name.to_string());
    }

    /// Push an assembly error.
    pub fn push_error(&mut self, e: AsmError) {
        self.errors.push(e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_to_text_section() {
        let ctx = AssemblyContext::new();
        assert_eq!(
            ctx.current_section,
            ("__TEXT".to_string(), "__text".to_string())
        );
        assert_eq!(ctx.current_offset(), 0);
    }

    #[test]
    fn advance_and_offset() {
        let mut ctx = AssemblyContext::new();
        ctx.advance(4);
        assert_eq!(ctx.current_offset(), 4);
        ctx.advance(4);
        assert_eq!(ctx.current_offset(), 8);
    }

    #[test]
    fn switch_section_preserves_offsets() {
        let mut ctx = AssemblyContext::new();
        ctx.advance(8);
        ctx.switch_section("__DATA", "__data");
        assert_eq!(ctx.current_offset(), 0);
        ctx.advance(4);
        ctx.switch_section("__TEXT", "__text");
        assert_eq!(ctx.current_offset(), 8);
    }

    #[test]
    fn define_symbol_records_position() {
        let mut ctx = AssemblyContext::new();
        ctx.advance(12);
        ctx.define_symbol("_foo", Span::new(0, 4));
        let info = ctx.symbols.get("_foo").unwrap();
        assert_eq!(info.offset, 12);
        assert_eq!(info.section, ("__TEXT".to_string(), "__text".to_string()));
    }

    #[test]
    fn duplicate_symbol_pushes_error() {
        let mut ctx = AssemblyContext::new();
        ctx.define_symbol("_foo", Span::new(0, 4));
        ctx.define_symbol("_foo", Span::new(10, 4));
        assert_eq!(ctx.errors.len(), 1);
        assert!(matches!(
            &ctx.errors[0],
            AsmError::DuplicateSymbol { name, .. } if name == "_foo"
        ));
    }

    #[test]
    fn mark_global() {
        let mut ctx = AssemblyContext::new();
        ctx.mark_global("_main");
        assert!(ctx.globals.contains("_main"));
    }

    #[test]
    fn align_to_pads_correctly() {
        let mut ctx = AssemblyContext::new();
        ctx.advance(5);
        ctx.align_to(4);
        assert_eq!(ctx.current_offset(), 8);
    }

    #[test]
    fn align_to_noop_when_already_aligned() {
        let mut ctx = AssemblyContext::new();
        ctx.advance(8);
        ctx.align_to(4);
        assert_eq!(ctx.current_offset(), 8);
    }
}
