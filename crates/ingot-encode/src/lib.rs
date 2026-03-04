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
pub use ingot_types::{PendingRelocation, RelocKind};

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
