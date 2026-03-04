//! Data Processing — Register instruction encoders.

use ingot_types::operand::{ExtendOp, ShiftOp};
use ingot_types::{AsmError, Mnemonic, Register, Span};

use crate::encode::{add_sub_op_s, logical_opc, reg_is_64};
use crate::fields::{set_field, set_rd, set_rm, set_rn, set_sf};
use crate::EncodedInst;

/// Encode ADD/SUB/ADDS/SUBS shifted register.
///
/// `sf op S 01011 shift 0 Rm imm6 Rn Rd`
pub fn encode_add_sub_shifted(
    mnemonic: Mnemonic,
    rd: &Register,
    rn: &Register,
    rm: &Register,
    shift_op: Option<ShiftOp>,
    shift_amount: u32,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let is_64 = reg_is_64(rd, span)?;
    let (op, s_bit) = add_sub_op_s(mnemonic);

    let shift = shift_op.unwrap_or(ShiftOp::Lsl).encoding() as u32;

    let max_shift = if is_64 { 63 } else { 31 };
    if shift_amount > max_shift {
        return Err(AsmError::ImmediateOutOfRange {
            value: shift_amount as i64,
            span,
        });
    }

    // sf op S 01011 shift 0 Rm imm6 Rn Rd — base: 0x0B000000
    let mut inst: u32 = 0x0B00_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 30, op);
    inst = set_field(inst, 29, 29, s_bit);
    inst = set_field(inst, 23, 22, shift);
    inst = set_rm(inst, rm.encoding());
    inst = set_field(inst, 15, 10, shift_amount);
    inst = set_rn(inst, rn.encoding());
    inst = set_rd(inst, rd.encoding());
    Ok(EncodedInst::new(inst))
}

/// Encode ADD/SUB/ADDS/SUBS extended register.
///
/// `sf op S 01011 00 1 Rm option imm3 Rn Rd`
pub fn encode_add_sub_extended(
    mnemonic: Mnemonic,
    rd: &Register,
    rn: &Register,
    rm: &Register,
    extend_op: ExtendOp,
    amount: u32,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let is_64 = reg_is_64(rd, span)?;
    let (op, s_bit) = add_sub_op_s(mnemonic);

    if amount > 4 {
        return Err(AsmError::ImmediateOutOfRange {
            value: amount as i64,
            span,
        });
    }

    // sf op S 01011 00 1 Rm option imm3 Rn Rd — base: 0x0B200000
    let mut inst: u32 = 0x0B20_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 30, op);
    inst = set_field(inst, 29, 29, s_bit);
    inst = set_rm(inst, rm.encoding());
    inst = set_field(inst, 15, 13, extend_op.encoding() as u32);
    inst = set_field(inst, 12, 10, amount);
    inst = set_rn(inst, rn.encoding());
    inst = set_rd(inst, rd.encoding());
    Ok(EncodedInst::new(inst))
}

/// Encode logical shifted register (AND, ORR, EOR, ANDS).
///
/// `sf opc 01010 shift N Rm imm6 Rn Rd`
///
/// - AND:  opc=00, N=0
/// - ORR:  opc=01, N=0
/// - EOR:  opc=10, N=0
/// - ANDS: opc=11, N=0
pub fn encode_logical_shifted(
    mnemonic: Mnemonic,
    rd: &Register,
    rn: &Register,
    rm: &Register,
    shift_op: Option<ShiftOp>,
    shift_amount: u32,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let is_64 = reg_is_64(rd, span)?;
    let opc = logical_opc(mnemonic);

    let shift = shift_op.unwrap_or(ShiftOp::Lsl).encoding() as u32;

    let max_shift = if is_64 { 63 } else { 31 };
    if shift_amount > max_shift {
        return Err(AsmError::ImmediateOutOfRange {
            value: shift_amount as i64,
            span,
        });
    }

    // sf opc 01010 shift N(0) Rm imm6 Rn Rd — base: 0x0A000000
    let mut inst: u32 = 0x0A00_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 29, opc);
    inst = set_field(inst, 23, 22, shift);
    // N = 0 for non-inverted forms
    inst = set_rm(inst, rm.encoding());
    inst = set_field(inst, 15, 10, shift_amount);
    inst = set_rn(inst, rn.encoding());
    inst = set_rd(inst, rd.encoding());
    Ok(EncodedInst::new(inst))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::register::{GpReg, RegWidth};

    fn gp(n: u8, w: RegWidth) -> Register {
        Register::Gp(GpReg::new(n, w))
    }

    fn span() -> Span {
        Span::dummy()
    }

    #[test]
    fn add_x0_x1_x2() {
        // add x0, x1, x2 → 0x8B020020
        let enc = encode_add_sub_shifted(
            Mnemonic::Add,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &gp(2, RegWidth::X64),
            None,
            0,
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x8B02_0020);
    }

    #[test]
    fn add_x0_x1_x2_lsl3() {
        // add x0, x1, x2, lsl #3 → 0x8B020C20
        let enc = encode_add_sub_shifted(
            Mnemonic::Add,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &gp(2, RegWidth::X64),
            Some(ShiftOp::Lsl),
            3,
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x8B02_0C20);
    }

    #[test]
    fn sub_w0_w1_w2() {
        // sub w0, w1, w2 → 0x4B020020
        let enc = encode_add_sub_shifted(
            Mnemonic::Sub,
            &gp(0, RegWidth::W32),
            &gp(1, RegWidth::W32),
            &gp(2, RegWidth::W32),
            None,
            0,
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x4B02_0020);
    }

    #[test]
    fn orr_x0_x1_x2() {
        // orr x0, x1, x2 → 0xAA020020
        let enc = encode_logical_shifted(
            Mnemonic::Orr,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &gp(2, RegWidth::X64),
            None,
            0,
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0xAA02_0020);
    }

    #[test]
    fn add_x0_x1_w2_sxtw() {
        // add x0, x1, w2, sxtw → 0x8B22C020
        let enc = encode_add_sub_extended(
            Mnemonic::Add,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &gp(2, RegWidth::W32),
            ExtendOp::Sxtw,
            0,
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x8B22_C020);
    }
}
