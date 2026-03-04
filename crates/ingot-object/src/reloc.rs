use ingot_types::RelocKind;
use object::macho;
use object::RelocationFlags;

/// Map an assembler `RelocKind` to Mach-O relocation flags.
///
/// Uses `RelocationFlags::MachO` to bypass the generic translation layer
/// (which only supports UNSIGNED and BRANCH26 for AArch64).
///
/// `Pcrel19` and `Adr21` don't have dedicated Mach-O reloc types — they map to
/// `BRANCH26` and `PAGE21` respectively, matching Apple's assembler behavior.
pub fn reloc_flags(kind: RelocKind) -> RelocationFlags {
    let (r_type, r_pcrel) = match kind {
        RelocKind::Branch26 => (macho::ARM64_RELOC_BRANCH26, true),
        RelocKind::Page21 => (macho::ARM64_RELOC_PAGE21, true),
        RelocKind::PageOff12 => (macho::ARM64_RELOC_PAGEOFF12, false),
        RelocKind::GotLoadPage21 => (macho::ARM64_RELOC_GOT_LOAD_PAGE21, true),
        RelocKind::GotLoadPageOff12 => (macho::ARM64_RELOC_GOT_LOAD_PAGEOFF12, false),
        RelocKind::Pcrel19 => (macho::ARM64_RELOC_BRANCH26, true),
        RelocKind::Adr21 => (macho::ARM64_RELOC_PAGE21, true),
    };
    RelocationFlags::MachO {
        r_type,
        r_pcrel,
        r_length: 2, // 32-bit (all ARM64 instructions)
    }
}
