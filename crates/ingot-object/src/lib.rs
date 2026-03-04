//! Mach-O object file emitter.
//!
//! Wraps the `object` crate to emit Mach-O `.o` files for ARM64 macOS.
//! Core type is [`ObjectBuilder`], which manages sections, symbols,
//! relocations, and metadata for a single relocatable object file.

mod reloc;

use std::collections::HashMap;

use ingot_types::directive::{BuildVersion, Platform, SectionSpec};
use ingot_types::{PendingRelocation, RelocKind};
use object::write::{self, Mangling, Object, Symbol};
use object::{Architecture, BinaryFormat, Endianness, SectionKind, SymbolKind, SymbolScope};
use thiserror::Error;

pub use object::write::{SectionId, SymbolId};

/// Errors from the object file emitter.
#[derive(Debug, Error)]
pub enum ObjectError {
    #[error("object write error: {detail}")]
    Write { detail: String },

    #[error("no current section selected")]
    NoCurrentSection,

    #[error("unsupported relocation kind {kind:?}: {detail}")]
    UnsupportedRelocation { kind: RelocKind, detail: String },
}

impl From<write::Error> for ObjectError {
    fn from(e: write::Error) -> Self {
        ObjectError::Write {
            detail: e.to_string(),
        }
    }
}

type Result<T> = std::result::Result<T, ObjectError>;

/// Builder for Mach-O ARM64 relocatable object files.
pub struct ObjectBuilder {
    obj: Object<'static>,
    section_map: HashMap<(String, String), SectionId>,
    section_kinds: HashMap<SectionId, SectionKind>,
    symbol_map: HashMap<String, SymbolId>,
    current_section: Option<SectionId>,
}

impl ObjectBuilder {
    /// Create a new builder targeting ARM64 macOS.
    pub fn new() -> Self {
        let mut obj = Object::new(
            BinaryFormat::MachO,
            Architecture::Aarch64,
            Endianness::Little,
        );
        obj.mangling = Mangling::None;
        Self {
            obj,
            section_map: HashMap::new(),
            section_kinds: HashMap::new(),
            symbol_map: HashMap::new(),
            current_section: None,
        }
    }

    // --- Section management ---

    /// Get or create a section, deduplicating by (segment, section) pair.
    pub fn get_or_create_section(&mut self, segment: &str, section: &str) -> SectionId {
        let key = (segment.to_string(), section.to_string());
        if let Some(&id) = self.section_map.get(&key) {
            return id;
        }
        let kind = infer_section_kind(segment, section);
        let id = self.obj.add_section(
            segment.as_bytes().to_vec(),
            section.as_bytes().to_vec(),
            kind,
        );
        self.section_map.insert(key, id);
        self.section_kinds.insert(id, kind);
        id
    }

    /// Switch the current section, creating it if needed.
    pub fn switch_section(&mut self, segment: &str, section: &str) -> SectionId {
        let id = self.get_or_create_section(segment, section);
        self.current_section = Some(id);
        id
    }

    /// Switch to a section from a `.section` directive.
    pub fn switch_section_spec(&mut self, spec: &SectionSpec) -> SectionId {
        self.switch_section(&spec.segment, &spec.section)
    }

    /// Return the current section, if any.
    pub fn current_section(&self) -> Option<SectionId> {
        self.current_section
    }

    /// Return the current size of the given section.
    pub fn section_offset(&self, section: SectionId) -> u64 {
        self.obj.section(section).data().len() as u64
    }

    // --- Data emission ---

    /// Emit a 32-bit instruction to the current section (4-byte aligned).
    pub fn emit_instruction(&mut self, bits: u32) -> Result<u64> {
        let section = self.current_section.ok_or(ObjectError::NoCurrentSection)?;
        let bytes = bits.to_le_bytes();
        let offset = self.obj.append_section_data(section, &bytes, 4);
        Ok(offset)
    }

    /// Emit raw data to the current section.
    pub fn emit_data(&mut self, data: &[u8], align: u64) -> Result<u64> {
        let section = self.current_section.ok_or(ObjectError::NoCurrentSection)?;
        let offset = self.obj.append_section_data(section, data, align);
        Ok(offset)
    }

    /// Pad the current section to the given alignment.
    pub fn emit_align(&mut self, align: u64) -> Result<()> {
        let section = self.current_section.ok_or(ObjectError::NoCurrentSection)?;
        self.obj.append_section_data(section, &[], align);
        Ok(())
    }

    /// Return the current write offset in the current section.
    pub fn current_offset(&self) -> Result<u64> {
        let section = self.current_section.ok_or(ObjectError::NoCurrentSection)?;
        Ok(self.section_offset(section))
    }

    // --- Symbol management ---

    /// Define a symbol at the given offset in the given section.
    pub fn define_symbol(
        &mut self,
        name: &str,
        offset: u64,
        section: SectionId,
    ) -> Result<SymbolId> {
        let kind = match self.section_kinds.get(&section) {
            Some(SectionKind::Text) => SymbolKind::Text,
            _ => SymbolKind::Data,
        };
        let id = self.obj.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: offset,
            size: 0,
            kind,
            scope: SymbolScope::Compilation,
            weak: false,
            section: write::SymbolSection::Section(section),
            flags: object::SymbolFlags::None,
        });
        self.symbol_map.insert(name.to_string(), id);
        Ok(id)
    }

    /// Upgrade a symbol to global (dynamic) scope.
    /// Returns `true` if the symbol was found and upgraded, `false` if unknown.
    pub fn set_global(&mut self, name: &str) -> bool {
        if let Some(&id) = self.symbol_map.get(name) {
            self.obj.symbol_mut(id).scope = SymbolScope::Dynamic;
            true
        } else {
            false
        }
    }

    /// Declare an external (undefined) symbol. Returns existing ID if already known.
    pub fn declare_external(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.symbol_map.get(name) {
            return id;
        }
        let id = self.obj.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: 0,
            size: 0,
            kind: SymbolKind::Unknown,
            scope: SymbolScope::Unknown,
            weak: false,
            section: write::SymbolSection::Undefined,
            flags: object::SymbolFlags::None,
        });
        self.symbol_map.insert(name.to_string(), id);
        id
    }

    /// Get or create a symbol — returns existing if known, otherwise creates external.
    pub fn ensure_symbol(&mut self, name: &str) -> SymbolId {
        if let Some(&id) = self.symbol_map.get(name) {
            id
        } else {
            self.declare_external(name)
        }
    }

    /// Look up a symbol ID by name.
    pub fn symbol_id(&self, name: &str) -> Option<SymbolId> {
        self.symbol_map.get(name).copied()
    }

    // --- Relocations ---

    /// Add a relocation at the given offset in the given section.
    pub fn add_relocation(
        &mut self,
        section: SectionId,
        offset: u64,
        pending: &PendingRelocation,
    ) -> Result<()> {
        let symbol = self.ensure_symbol(&pending.symbol);
        let flags = reloc::reloc_flags(pending.kind)?;
        self.obj
            .add_relocation(
                section,
                write::Relocation {
                    offset,
                    symbol,
                    addend: pending.addend,
                    flags,
                },
            )
            .map_err(ObjectError::from)
    }

    // --- Metadata ---

    /// Set the `MH_SUBSECTIONS_VIA_SYMBOLS` flag.
    pub fn set_subsections_via_symbols(&mut self) {
        self.obj.flags = object::FileFlags::MachO {
            flags: object::macho::MH_SUBSECTIONS_VIA_SYMBOLS,
        };
    }

    /// Set the build version from a `.build_version` directive.
    pub fn set_build_version(&mut self, bv: &BuildVersion) {
        let platform = match bv.platform {
            Platform::MacOS => object::macho::PLATFORM_MACOS,
            Platform::IOS => object::macho::PLATFORM_IOS,
            Platform::TvOS => object::macho::PLATFORM_TVOS,
            Platform::WatchOS => object::macho::PLATFORM_WATCHOS,
        };
        let minos = parse_version(&bv.min_version);
        let mut build_ver = object::write::MachOBuildVersion::default();
        build_ver.platform = platform;
        build_ver.minos = minos;
        build_ver.sdk = 0;
        self.obj.set_macho_build_version(build_ver);
    }

    // --- Output ---

    /// Consume the builder and emit the Mach-O object file bytes.
    pub fn finish(self) -> Result<Vec<u8>> {
        self.obj.write().map_err(ObjectError::from)
    }
}

impl Default for ObjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a version string "X.Y" or "X.Y.Z" into packed u32: `X<<16 | Y<<8 | Z`.
fn parse_version(s: &str) -> u32 {
    let mut parts = s.split('.');
    let major = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    ((major & 0xFFFF) << 16) | ((minor & 0xFF) << 8) | (patch & 0xFF)
}

/// Infer `SectionKind` from segment and section names.
fn infer_section_kind(segment: &str, section: &str) -> SectionKind {
    match (segment, section) {
        ("__TEXT", "__text") => SectionKind::Text,
        ("__TEXT", "__const") => SectionKind::ReadOnlyData,
        ("__TEXT", "__cstring") => SectionKind::ReadOnlyString,
        ("__DATA", "__data") => SectionKind::Data,
        ("__DATA", "__bss") => SectionKind::UninitializedData,
        ("__DATA", "__common") => SectionKind::Common,
        ("__DATA", "__thread_data") => SectionKind::Tls,
        ("__DATA", "__thread_bss") => SectionKind::UninitializedTls,
        ("__DATA", "__thread_vars") => SectionKind::TlsVariables,
        _ => SectionKind::Data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::RelocKind;

    #[test]
    fn reloc_mapping() {
        let cases = [
            (
                RelocKind::Branch26,
                object::macho::ARM64_RELOC_BRANCH26,
                true,
            ),
            (RelocKind::Page21, object::macho::ARM64_RELOC_PAGE21, true),
            (
                RelocKind::PageOff12,
                object::macho::ARM64_RELOC_PAGEOFF12,
                false,
            ),
            (
                RelocKind::GotLoadPage21,
                object::macho::ARM64_RELOC_GOT_LOAD_PAGE21,
                true,
            ),
            (
                RelocKind::GotLoadPageOff12,
                object::macho::ARM64_RELOC_GOT_LOAD_PAGEOFF12,
                false,
            ),
        ];
        for (kind, expected_type, expected_pcrel) in cases {
            let flags = reloc::reloc_flags(kind).unwrap();
            assert_eq!(
                flags,
                object::RelocationFlags::MachO {
                    r_type: expected_type,
                    r_pcrel: expected_pcrel,
                    r_length: 2,
                }
            );
        }
    }

    #[test]
    fn unsupported_reloc_kinds() {
        assert!(reloc::reloc_flags(RelocKind::Pcrel19).is_err());
        assert!(reloc::reloc_flags(RelocKind::Adr21).is_err());
    }

    #[test]
    fn section_dedup() {
        let mut builder = ObjectBuilder::new();
        let id1 = builder.get_or_create_section("__TEXT", "__text");
        let id2 = builder.get_or_create_section("__TEXT", "__text");
        assert_eq!(id1, id2);
    }

    #[test]
    fn symbol_define_and_global() {
        let mut builder = ObjectBuilder::new();
        let section = builder.switch_section("__TEXT", "__text");
        builder.define_symbol("_foo", 0, section).unwrap();
        assert!(builder.symbol_id("_foo").is_some());

        let id = builder.symbol_id("_foo").unwrap();
        assert_eq!(builder.obj.symbol(id).scope, SymbolScope::Compilation);

        assert!(builder.set_global("_foo"));
        assert_eq!(builder.obj.symbol(id).scope, SymbolScope::Dynamic);
    }

    #[test]
    fn set_global_unknown_symbol() {
        let mut builder = ObjectBuilder::new();
        assert!(!builder.set_global("_nonexistent"));
    }

    #[test]
    fn declare_external_dedup() {
        let mut builder = ObjectBuilder::new();
        let id1 = builder.declare_external("_ext");
        let id2 = builder.declare_external("_ext");
        assert_eq!(id1, id2);
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("14.0"), 0x000E_0000);
        assert_eq!(parse_version("11.3.1"), 0x000B_0301);
        assert_eq!(parse_version("15.2.0"), 0x000F_0200);
        assert_eq!(parse_version("12.0.0"), 0x000C_0000);
    }

    #[test]
    fn version_parsing_overflow() {
        // Components exceeding their field width should be masked
        let v = parse_version("65537.256.256"); // 0x10001, 0x100, 0x100
        assert_eq!(v >> 16, 1); // major masked to 16 bits: 0x10001 & 0xFFFF = 1
        assert_eq!((v >> 8) & 0xFF, 0); // minor masked to 8 bits: 0x100 & 0xFF = 0
        assert_eq!(v & 0xFF, 0); // patch masked to 8 bits: 0x100 & 0xFF = 0
    }

    #[test]
    fn roundtrip_nop() {
        use object::Object as _;
        use object::ObjectSection as _;
        use object::ObjectSymbol as _;

        let mut builder = ObjectBuilder::new();
        builder.switch_section("__TEXT", "__text");
        builder.emit_instruction(0xD503201F).unwrap(); // NOP
        let section = builder.current_section().unwrap();
        builder.define_symbol("_main", 0, section).unwrap();
        builder.set_global("_main");

        let bytes = builder.finish().unwrap();

        let file = object::File::parse(&*bytes).unwrap();
        assert_eq!(file.architecture(), Architecture::Aarch64);

        let text_section = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        assert_eq!(text_section.data().unwrap(), &0xD503201Fu32.to_le_bytes());

        let main_sym = file.symbols().find(|s| s.name() == Ok("_main")).unwrap();
        assert!(main_sym.is_global());
    }

    #[test]
    fn roundtrip_with_relocation() {
        use object::Object as _;
        use object::ObjectSection as _;

        let mut builder = ObjectBuilder::new();
        let section = builder.switch_section("__TEXT", "__text");

        // ADRP x0, _foo@PAGE
        let adrp_offset = builder.emit_instruction(0x90000000).unwrap();
        builder
            .add_relocation(
                section,
                adrp_offset,
                &PendingRelocation::simple(RelocKind::Page21, "_foo".into()),
            )
            .unwrap();

        // ADD x0, x0, _foo@PAGEOFF
        let add_offset = builder.emit_instruction(0x91000000).unwrap();
        builder
            .add_relocation(
                section,
                add_offset,
                &PendingRelocation::simple(RelocKind::PageOff12, "_foo".into()),
            )
            .unwrap();

        let bytes = builder.finish().unwrap();

        let file = object::File::parse(&*bytes).unwrap();
        let text_section = file.sections().find(|s| s.name() == Ok("__text")).unwrap();

        let relocs: Vec<_> = text_section.relocations().collect();
        assert_eq!(relocs.len(), 2);
    }
}
