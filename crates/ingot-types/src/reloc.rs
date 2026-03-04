/// ARM64 relocation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelocKind {
    /// 26-bit PC-relative branch offset (B, BL).
    Branch26,
    /// 21-bit page address (ADRP).
    Page21,
    /// 12-bit page offset (ADD/LDR with :lo12:).
    PageOff12,
    /// GOT page address.
    GotLoadPage21,
    /// GOT page offset.
    GotLoadPageOff12,
    /// 19-bit PC-relative offset (B.cond, CBZ, CBNZ, LDR literal).
    Pcrel19,
    /// 21-bit PC-relative offset for ADR, split across immhi[23:5] and immlo[30:29].
    Adr21,
}

/// A relocation that the linker must resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRelocation {
    pub kind: RelocKind,
    pub symbol: String,
    pub addend: i64,
}

impl PendingRelocation {
    /// Create a relocation with addend 0.
    pub fn simple(kind: RelocKind, symbol: String) -> Self {
        Self {
            kind,
            symbol,
            addend: 0,
        }
    }
}
