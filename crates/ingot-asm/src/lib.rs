//! Two-pass assembler pipeline.
//!
//! Orchestrates lexing, parsing, symbol resolution, encoding, and Mach-O emission.

mod assembler;
mod context;
mod pass1;
mod pass2;

pub use assembler::assemble;
pub use ingot_types::{AsmError, AsmErrors};
