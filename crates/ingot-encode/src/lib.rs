//! ARM64 instruction encoder.
//!
//! Maps `Instruction { mnemonic, operands, span }` to `EncodedInst { bits, relocation }`.

mod branch;
mod dp_imm;
mod dp_reg;
mod encode;
mod fields;
mod ldst;
mod simd_fp;
mod system;

pub use encode::encode;

/// A successfully encoded ARM64 instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedInst {
    /// The 32-bit encoded instruction.
    pub bits: u32,
    /// Optional relocation to be resolved by the linker.
    pub relocation: Option<PendingRelocation>,
}

impl EncodedInst {
    pub fn new(bits: u32) -> Self {
        Self {
            bits,
            relocation: None,
        }
    }

    pub fn with_reloc(bits: u32, reloc: PendingRelocation) -> Self {
        Self {
            bits,
            relocation: Some(reloc),
        }
    }
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
