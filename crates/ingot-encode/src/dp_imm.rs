//! Data Processing — Immediate instruction encoders.

use ingot_types::expr::RelocModifier;
use ingot_types::{AsmError, Expr, Mnemonic, Operand, Register, Span};

use crate::encode::{add_sub_op_s, expect_imm, expect_reg, logical_opc, reg_is_64};
use crate::fields::{set_field, set_rd, set_rn, set_sf};
use crate::{EncodedInst, PendingRelocation, RelocKind};

/// Encode ADD/SUB/ADDS/SUBS immediate.
///
/// `sf op S 100010 sh imm12 Rn Rd`
pub fn encode_add_sub_imm(
    mnemonic: Mnemonic,
    rd: &Register,
    rn: &Register,
    imm_expr: &Expr,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let is_64 = reg_is_64(rd, span)?;
    let (op, s_bit) = add_sub_op_s(mnemonic);

    // ADD/SUB imm base (sf=0, op=0, S=0): 0x11000000
    let mut inst: u32 = 0x1100_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 30, op);
    inst = set_field(inst, 29, 29, s_bit);
    inst = set_rd(inst, rd.encoding());
    inst = set_rn(inst, rn.encoding());

    match imm_expr {
        Expr::Relocated {
            modifier: RelocModifier::Lo12,
            symbol,
            addend,
        } => Ok(EncodedInst::with_reloc(
            inst,
            PendingRelocation {
                kind: RelocKind::PageOff12,
                symbol: symbol.clone(),
                addend: *addend,
            },
        )),
        _ => {
            let value = imm_expr
                .as_literal()
                .ok_or_else(|| AsmError::InvalidOperand {
                    detail: "ADD/SUB immediate requires a constant or :lo12: relocation".into(),
                    span,
                })?;

            if value < 0 {
                return Err(AsmError::ImmediateOutOfRange { value, span });
            }

            let (sh, imm12) = if (value as u64) <= 0xFFF {
                (0u32, value as u32)
            } else if (value as u64) <= 0xFFF_000 && (value & 0xFFF) == 0 {
                (1u32, (value >> 12) as u32)
            } else {
                return Err(AsmError::ImmediateOutOfRange { value, span });
            };

            inst = set_field(inst, 22, 22, sh);
            inst = set_field(inst, 21, 10, imm12);
            Ok(EncodedInst::new(inst))
        }
    }
}

/// Encode ADD/SUB immediate with explicit shift operand.
pub fn encode_add_sub_imm_with_shift(
    mnemonic: Mnemonic,
    rd: &Register,
    rn: &Register,
    imm_expr: &Expr,
    shift_amount: i64,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let is_64 = reg_is_64(rd, span)?;
    let (op, s_bit) = add_sub_op_s(mnemonic);

    let value = imm_expr
        .as_literal()
        .ok_or_else(|| AsmError::InvalidOperand {
            detail: "shifted immediate requires a literal value".into(),
            span,
        })?;

    if shift_amount != 0 && shift_amount != 12 {
        return Err(AsmError::InvalidOperand {
            detail: "ADD/SUB immediate shift must be 0 or 12".into(),
            span,
        });
    }

    if !(0..=0xFFF).contains(&value) {
        return Err(AsmError::ImmediateOutOfRange { value, span });
    }

    let sh = if shift_amount == 12 { 1u32 } else { 0u32 };

    let mut inst: u32 = 0x1100_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 30, op);
    inst = set_field(inst, 29, 29, s_bit);
    inst = set_rd(inst, rd.encoding());
    inst = set_rn(inst, rn.encoding());
    inst = set_field(inst, 22, 22, sh);
    inst = set_field(inst, 21, 10, value as u32);
    Ok(EncodedInst::new(inst))
}

/// Encode MOVZ, MOVN, or MOVK.
///
/// `sf opc 100101 hw imm16 Rd`
pub fn encode_mov_wide(
    mnemonic: Mnemonic,
    operands: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if operands.len() < 2 || operands.len() > 3 {
        return Err(AsmError::WrongOperandCount {
            expected: 2,
            got: operands.len(),
            span,
        });
    }

    let rd = expect_reg(&operands[0], span)?;
    let is_64 = reg_is_64(rd, span)?;
    let imm_expr = expect_imm(&operands[1], span)?;

    let value = imm_expr
        .as_literal()
        .ok_or_else(|| AsmError::InvalidOperand {
            detail: "MOV wide requires a literal immediate".into(),
            span,
        })?;

    if !(0..=0xFFFF).contains(&value) {
        return Err(AsmError::ImmediateOutOfRange { value, span });
    }

    // Get hw from optional LSL shift
    let hw: u32 = if operands.len() == 3 {
        match &operands[2] {
            Operand::Shift {
                op: ingot_types::operand::ShiftOp::Lsl,
                amount,
            } => {
                let shift = amount
                    .as_literal()
                    .ok_or_else(|| AsmError::InvalidOperand {
                        detail: "shift amount must be a literal".into(),
                        span,
                    })?;
                match shift {
                    0 => 0,
                    16 => 1,
                    32 => {
                        if !is_64 {
                            return Err(AsmError::InvalidOperand {
                                detail: "LSL #32 not valid for 32-bit register".into(),
                                span,
                            });
                        }
                        2
                    }
                    48 => {
                        if !is_64 {
                            return Err(AsmError::InvalidOperand {
                                detail: "LSL #48 not valid for 32-bit register".into(),
                                span,
                            });
                        }
                        3
                    }
                    _ => {
                        return Err(AsmError::InvalidOperand {
                            detail: "MOVZ/MOVN/MOVK shift must be 0, 16, 32, or 48".into(),
                            span,
                        })
                    }
                }
            }
            _ => {
                return Err(AsmError::InvalidOperand {
                    detail: "expected LSL shift".into(),
                    span,
                })
            }
        }
    } else {
        0
    };

    let opc: u32 = match mnemonic {
        Mnemonic::Movz => 0b10,
        Mnemonic::Movn => 0b00,
        Mnemonic::Movk => 0b11,
        _ => unreachable!(),
    };

    // MOVZ/MOVN/MOVK base (sf=0, opc=00/MOVN): 0x12800000
    let mut inst: u32 = 0x1280_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 29, opc);
    inst = set_field(inst, 22, 21, hw);
    inst = set_field(inst, 20, 5, value as u32);
    inst = set_rd(inst, rd.encoding());
    Ok(EncodedInst::new(inst))
}

/// Encode ADR or ADRP.
///
/// ADR:  `0 immlo 10000 immhi Rd`
/// ADRP: `1 immlo 10000 immhi Rd`
pub fn encode_adr_adrp(
    mnemonic: Mnemonic,
    operands: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if operands.len() != 2 {
        return Err(AsmError::WrongOperandCount {
            expected: 2,
            got: operands.len(),
            span,
        });
    }

    let rd = expect_reg(&operands[0], span)?;
    let is_adrp = mnemonic == Mnemonic::Adrp;

    // ADR base: 0x10000000, ADRP base: 0x90000000
    let mut inst: u32 = if is_adrp { 0x9000_0000 } else { 0x1000_0000 };
    inst = set_rd(inst, rd.encoding());

    let reloc_kind = if is_adrp {
        RelocKind::Page21
    } else {
        RelocKind::Adr21
    };

    match &operands[1] {
        Operand::Label(name) => Ok(EncodedInst::with_reloc(
            inst,
            PendingRelocation::simple(reloc_kind, name.clone()),
        )),
        Operand::Immediate(expr) => match expr {
            Expr::Symbol(name) => Ok(EncodedInst::with_reloc(
                inst,
                PendingRelocation::simple(reloc_kind, name.clone()),
            )),
            Expr::Relocated {
                modifier,
                symbol,
                addend,
            } => {
                let kind = match modifier {
                    RelocModifier::PgHi21 => RelocKind::Page21,
                    RelocModifier::Got => RelocKind::GotLoadPage21,
                    _ => {
                        return Err(AsmError::InvalidOperand {
                            detail: format!("invalid relocation modifier for {mnemonic:?}"),
                            span,
                        })
                    }
                };
                Ok(EncodedInst::with_reloc(
                    inst,
                    PendingRelocation {
                        kind,
                        symbol: symbol.clone(),
                        addend: *addend,
                    },
                ))
            }
            Expr::Literal(value) => {
                let imm = *value;
                let immlo = (imm as u32) & 0b11;
                let immhi = ((imm as u32) >> 2) & 0x7FFFF;
                inst = set_field(inst, 30, 29, immlo);
                inst = set_field(inst, 23, 5, immhi);
                Ok(EncodedInst::new(inst))
            }
            _ => Err(AsmError::InvalidOperand {
                detail: "ADR/ADRP requires a symbol or label".into(),
                span,
            }),
        },
        _ => Err(AsmError::InvalidOperand {
            detail: "expected label or symbol".into(),
            span,
        }),
    }
}

/// Encode logical immediate (AND, ORR, EOR, ANDS).
///
/// `sf opc 100100 N immr imms Rn Rd`
pub fn encode_logical_imm(
    mnemonic: Mnemonic,
    rd: &Register,
    rn: &Register,
    imm_expr: &Expr,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let is_64 = reg_is_64(rd, span)?;
    let opc = logical_opc(mnemonic);

    let value = imm_expr
        .as_literal()
        .ok_or_else(|| AsmError::InvalidOperand {
            detail: "logical immediate requires a constant".into(),
            span,
        })?;

    let (n, immr, imms) =
        encode_bitmask_imm(value as u64, is_64).ok_or_else(|| AsmError::InvalidOperand {
            detail: format!("value {value:#x} cannot be encoded as a logical immediate"),
            span,
        })?;

    if !is_64 && n != 0 {
        return Err(AsmError::InvalidOperand {
            detail: "N=1 not valid for 32-bit logical immediate".into(),
            span,
        });
    }

    // Logical imm base (sf=0, opc=00): 0x12000000
    let mut inst: u32 = 0x1200_0000;
    inst = set_sf(inst, is_64);
    inst = set_field(inst, 30, 29, opc);
    inst = set_field(inst, 22, 22, n);
    inst = set_field(inst, 21, 16, immr);
    inst = set_field(inst, 15, 10, imms);
    inst = set_rn(inst, rn.encoding());
    inst = set_rd(inst, rd.encoding());
    Ok(EncodedInst::new(inst))
}

/// Encode a 64-bit value as an ARM64 logical immediate (N, immr, imms).
///
/// Returns `None` if the value cannot be represented as a logical immediate.
pub fn encode_bitmask_imm(value: u64, is_64: bool) -> Option<(u32, u32, u32)> {
    let value = if is_64 { value } else { value & 0xFFFF_FFFF };

    if value == 0 {
        return None;
    }
    let full_mask = if is_64 { u64::MAX } else { 0xFFFF_FFFF };
    if value == full_mask {
        return None;
    }

    let reg_size: u32 = if is_64 { 64 } else { 32 };

    for element_size in [2u32, 4, 8, 16, 32, 64] {
        if element_size > reg_size {
            break;
        }

        let element_mask: u64 = if element_size == 64 {
            u64::MAX
        } else {
            (1u64 << element_size) - 1
        };

        let element = value & element_mask;
        let mut is_repeating = true;
        let mut pos = element_size;
        while pos < reg_size {
            if ((value >> pos) & element_mask) != element {
                is_repeating = false;
                break;
            }
            pos += element_size;
        }
        if !is_repeating {
            continue;
        }

        for rotation in 0..element_size {
            let rotated = if rotation == 0 {
                element
            } else {
                ((element >> rotation) | (element << (element_size - rotation))) & element_mask
            };

            if rotated == 0 {
                continue;
            }
            let ones = rotated.trailing_ones();
            let remaining = (rotated >> ones) & element_mask.checked_shr(ones).unwrap_or(0);
            if remaining != 0 {
                continue;
            }

            let (n, imms) = if element_size == 64 {
                (1u32, ones - 1)
            } else {
                let size_encoding = (!(element_size - 1)) & 0x3F;
                (0u32, (size_encoding | (ones - 1)) & 0x3F)
            };
            let immr = rotation;

            return Some((n, immr, imms));
        }
    }

    None
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
    fn add_x0_x1_42() {
        // add x0, x1, #42 → 0x9100A820
        let enc = encode_add_sub_imm(
            Mnemonic::Add,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &Expr::Literal(42),
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x9100_A820);
    }

    #[test]
    fn sub_w0_w1_1() {
        let enc = encode_add_sub_imm(
            Mnemonic::Sub,
            &gp(0, RegWidth::W32),
            &gp(1, RegWidth::W32),
            &Expr::Literal(1),
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x5100_0420);
    }

    #[test]
    fn adds_x0_x1_0() {
        let enc = encode_add_sub_imm(
            Mnemonic::Adds,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &Expr::Literal(0),
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0xB100_0020);
    }

    #[test]
    fn add_x0_x1_imm12_shifted() {
        // add x0, x1, #4096 → 0x91400420
        let enc = encode_add_sub_imm(
            Mnemonic::Add,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &Expr::Literal(4096),
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x9140_0420);
    }

    #[test]
    fn movz_x0_42() {
        let ops = vec![
            Operand::Register(gp(0, RegWidth::X64)),
            Operand::Immediate(Expr::Literal(42)),
        ];
        let enc = encode_mov_wide(Mnemonic::Movz, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xD280_0540);
    }

    #[test]
    fn movz_x0_42_lsl16() {
        let ops = vec![
            Operand::Register(gp(0, RegWidth::X64)),
            Operand::Immediate(Expr::Literal(42)),
            Operand::Shift {
                op: ingot_types::operand::ShiftOp::Lsl,
                amount: Expr::Literal(16),
            },
        ];
        let enc = encode_mov_wide(Mnemonic::Movz, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xD2A0_0540);
    }

    #[test]
    fn movn_w0_0() {
        let ops = vec![
            Operand::Register(gp(0, RegWidth::W32)),
            Operand::Immediate(Expr::Literal(0)),
        ];
        let enc = encode_mov_wide(Mnemonic::Movn, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0x1280_0000);
    }

    #[test]
    fn bitmask_all_ones_invalid() {
        assert!(encode_bitmask_imm(u64::MAX, true).is_none());
        assert!(encode_bitmask_imm(0xFFFF_FFFF, false).is_none());
    }

    #[test]
    fn bitmask_zero_invalid() {
        assert!(encode_bitmask_imm(0, true).is_none());
        assert!(encode_bitmask_imm(0, false).is_none());
    }

    #[test]
    fn bitmask_0xff() {
        let (n, immr, imms) = encode_bitmask_imm(0xFF, true).unwrap();
        assert_eq!(n, 1);
        assert_eq!(immr, 0);
        assert_eq!(imms, 7);
    }

    #[test]
    fn bitmask_0x5555_5555_5555_5555() {
        let result = encode_bitmask_imm(0x5555_5555_5555_5555, true);
        assert!(result.is_some());
    }

    #[test]
    fn and_x0_x1_0xff() {
        // and x0, x1, #0xff — N=1, immr=0, imms=7
        let enc = encode_logical_imm(
            Mnemonic::And,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &Expr::Literal(0xFF),
            span(),
        )
        .unwrap();
        assert_eq!(enc.bits, 0x9240_1C20);
    }

    #[test]
    fn adrp_x0_label() {
        let ops = vec![
            Operand::Register(gp(0, RegWidth::X64)),
            Operand::Label("sym".into()),
        ];
        let enc = encode_adr_adrp(Mnemonic::Adrp, &ops, span()).unwrap();
        assert_eq!(enc.bits & 0x9F00_0000, 0x9000_0000);
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::Page21);
    }

    #[test]
    fn adr_x0_label() {
        let ops = vec![
            Operand::Register(gp(0, RegWidth::X64)),
            Operand::Label("sym".into()),
        ];
        let enc = encode_adr_adrp(Mnemonic::Adr, &ops, span()).unwrap();
        assert_eq!(enc.bits & 0x9F00_0000, 0x1000_0000);
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::Adr21);
    }

    #[test]
    fn add_imm_out_of_range() {
        let err = encode_add_sub_imm(
            Mnemonic::Add,
            &gp(0, RegWidth::X64),
            &gp(1, RegWidth::X64),
            &Expr::Literal(0x1_000_001),
            span(),
        )
        .unwrap_err();
        assert!(matches!(err, AsmError::ImmediateOutOfRange { .. }));
    }

    #[test]
    fn movz_wrong_operand_count() {
        let ops = vec![Operand::Register(gp(0, RegWidth::X64))];
        let err = encode_mov_wide(Mnemonic::Movz, &ops, span()).unwrap_err();
        assert!(matches!(
            err,
            AsmError::WrongOperandCount {
                expected: 2,
                got: 1,
                ..
            }
        ));
    }
}
