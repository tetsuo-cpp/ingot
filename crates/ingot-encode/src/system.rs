//! System instruction encoders.

use crate::EncodedInst;

/// Encode NOP: `0xD503201F`.
pub fn encode_nop() -> EncodedInst {
    EncodedInst::new(0xD503_201F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop() {
        assert_eq!(encode_nop().bits, 0xD503_201F);
        assert!(encode_nop().relocation.is_none());
    }
}
