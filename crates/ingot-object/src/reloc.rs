use ingot_types::RelocKind;
use object::macho;
use object::RelocationFlags;

/// Map an assembler `RelocKind` to Mach-O relocation flags.
///
/// Uses `RelocationFlags::MachO` to bypass the generic translation layer
/// (which only supports UNSIGNED and BRANCH26 for AArch64).
///
/// Returns `Err` for `Pcrel19` and `Adr21` which have no dedicated Mach-O
/// relocation types and must be resolved at assembly time by `ingot-asm`.
pub fn reloc_flags(kind: RelocKind) -> Result<RelocationFlags, super::ObjectError> {
    let (r_type, r_pcrel) = match kind {
        RelocKind::Branch26 => (macho::ARM64_RELOC_BRANCH26, true),
        RelocKind::Page21 => (macho::ARM64_RELOC_PAGE21, true),
        RelocKind::PageOff12 => (macho::ARM64_RELOC_PAGEOFF12, false),
        RelocKind::GotLoadPage21 => (macho::ARM64_RELOC_GOT_LOAD_PAGE21, true),
        RelocKind::GotLoadPageOff12 => (macho::ARM64_RELOC_GOT_LOAD_PAGEOFF12, false),
        RelocKind::Pcrel19 => {
            return Err(super::ObjectError::UnsupportedRelocation {
                kind,
                detail: "19-bit PC-relative branches must be resolved at assembly time".into(),
            });
        }
        RelocKind::Adr21 => {
            return Err(super::ObjectError::UnsupportedRelocation {
                kind,
                detail: "ADR immediates must be resolved at assembly time".into(),
            });
        }
    };
    Ok(RelocationFlags::MachO {
        r_type,
        r_pcrel,
        r_length: 2, // 32-bit (all ARM64 instructions)
    })
}
