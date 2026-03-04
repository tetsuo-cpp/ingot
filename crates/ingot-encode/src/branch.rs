//! Branch instruction encoders.

use ingot_types::{AsmError, Mnemonic, Operand, Span};

use crate::encode::{
    expect_imm, expect_label_or_imm, expect_reg, label_reloc_from_expr, reg_is_64, LabelOrImm,
};
use crate::fields::{set_field, set_rn, set_rt, set_sf, truncate_imm};
use crate::{EncodedInst, PendingRelocation, RelocKind};

/// Encode B or BL.
///
/// `B label`  → `0 00101 imm26`
/// `BL label` → `1 00101 imm26`
pub fn encode_b_bl(
    mnemonic: Mnemonic,
    operands: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if operands.len() != 1 {
        return Err(AsmError::WrongOperandCount {
            expected: 1,
            got: operands.len(),
            span,
        });
    }

    let op_bit: u32 = if mnemonic == Mnemonic::Bl { 1 } else { 0 };
    let base = (op_bit << 31) | (0b00101 << 26);

    match &operands[0] {
        Operand::Label(name) => Ok(EncodedInst::with_reloc(
            base,
            PendingRelocation::simple(RelocKind::Branch26, name.clone()),
        )),
        Operand::Immediate(expr) => match expr.as_literal() {
            Some(offset) => {
                let imm26 = truncate_imm(offset >> 2, 26);
                Ok(EncodedInst::new(set_field(base, 25, 0, imm26)))
            }
            None => label_reloc_from_expr(expr, base, RelocKind::Branch26, span),
        },
        _ => Err(AsmError::InvalidOperand {
            detail: "expected label or immediate".into(),
            span,
        }),
    }
}

/// Encode BR, BLR, or RET.
///
/// `1101011 0000 11111 000000 Rn 00000` (BR)
/// `1101011 0001 11111 000000 Rn 00000` (BLR)
/// `1101011 0010 11111 000000 Rn 00000` (RET)
///
/// RET defaults to X30 if no operand given.
pub fn encode_br_blr_ret(
    mnemonic: Mnemonic,
    operands: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let opc: u32 = match mnemonic {
        Mnemonic::Br => 0b00,
        Mnemonic::Blr => 0b01,
        Mnemonic::Ret => 0b10,
        _ => unreachable!(),
    };

    let rn = if operands.is_empty() {
        if mnemonic == Mnemonic::Ret {
            30u8 // X30 default
        } else {
            return Err(AsmError::WrongOperandCount {
                expected: 1,
                got: 0,
                span,
            });
        }
    } else if operands.len() == 1 {
        expect_reg(&operands[0], span)?.encoding()
    } else {
        return Err(AsmError::WrongOperandCount {
            expected: 1,
            got: operands.len(),
            span,
        });
    };

    // 1101011 0 0 opc 11111 000000 Rn 00000
    // BR base with opc=00: 0xD61F0000
    let base: u32 = 0xD61F_0000;
    let inst = set_field(base, 22, 21, opc);
    let inst = set_rn(inst, rn);
    Ok(EncodedInst::new(inst))
}

/// Encode B.cond.
///
/// `01010100 imm19 0 cond`
pub fn encode_bcond(operands: &[Operand], span: Span) -> Result<EncodedInst, AsmError> {
    // Operands: Condition, Label/Immediate
    if operands.len() != 2 {
        return Err(AsmError::WrongOperandCount {
            expected: 2,
            got: operands.len(),
            span,
        });
    }

    let cond = match &operands[0] {
        Operand::Condition(c) => c.encoding() as u32,
        _ => {
            return Err(AsmError::InvalidOperand {
                detail: "expected condition code".into(),
                span,
            })
        }
    };

    let base: u32 = 0b01010100 << 24 | cond;

    match &operands[1] {
        Operand::Label(name) => Ok(EncodedInst::with_reloc(
            base,
            PendingRelocation::simple(RelocKind::Pcrel19, name.clone()),
        )),
        Operand::Immediate(expr) => match expr.as_literal() {
            Some(offset) => {
                let imm19 = truncate_imm(offset >> 2, 19);
                Ok(EncodedInst::new(set_field(base, 23, 5, imm19)))
            }
            None => label_reloc_from_expr(expr, base, RelocKind::Pcrel19, span),
        },
        _ => Err(AsmError::InvalidOperand {
            detail: "expected label or immediate".into(),
            span,
        }),
    }
}

/// Encode CBZ or CBNZ.
///
/// `sf 011010 op imm19 Rt`
pub fn encode_cbz_cbnz(
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

    let reg = expect_reg(&operands[0], span)?;
    let is_64 = reg_is_64(reg, span)?;
    let rt = reg.encoding();

    let op: u32 = if mnemonic == Mnemonic::Cbnz { 1 } else { 0 };
    // sf 011010 op imm19 Rt — CBZ base (sf=0, op=0): 0x34000000
    let base = set_sf(0x3400_0000, is_64);
    let base = set_field(base, 24, 24, op);
    let base = set_rt(base, rt);

    match expect_label_or_imm(&operands[1], span)? {
        LabelOrImm::Label(name) => Ok(EncodedInst::with_reloc(
            base,
            PendingRelocation::simple(RelocKind::Pcrel19, name.to_string()),
        )),
        LabelOrImm::Literal(offset) => {
            let imm19 = truncate_imm(offset >> 2, 19);
            Ok(EncodedInst::new(set_field(base, 23, 5, imm19)))
        }
        LabelOrImm::SymbolExpr(expr) => label_reloc_from_expr(expr, base, RelocKind::Pcrel19, span),
    }
}

/// Encode SVC or BRK.
///
/// SVC: `0xD4000001 | (imm16 << 5)`
/// BRK: `0xD4200000 | (imm16 << 5)`
pub fn encode_svc_brk(
    mnemonic: Mnemonic,
    operands: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if operands.len() != 1 {
        return Err(AsmError::WrongOperandCount {
            expected: 1,
            got: operands.len(),
            span,
        });
    }

    let imm = expect_imm(&operands[0], span)?;
    let value = imm.as_literal().ok_or_else(|| AsmError::InvalidOperand {
        detail: "SVC/BRK requires a literal immediate".into(),
        span,
    })?;

    if !(0..=0xFFFF).contains(&value) {
        return Err(AsmError::ImmediateOutOfRange { value, span });
    }

    let base: u32 = match mnemonic {
        Mnemonic::Svc => 0xD400_0001,
        Mnemonic::Brk => 0xD420_0000,
        _ => unreachable!(),
    };

    let inst = set_field(base, 20, 5, value as u32);
    Ok(EncodedInst::new(inst))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::operand::Condition;
    use ingot_types::register::{GpReg, RegWidth};
    use ingot_types::{Expr, Register};

    fn gp(n: u8, w: RegWidth) -> Operand {
        Operand::Register(Register::Gp(GpReg::new(n, w)))
    }

    fn label(s: &str) -> Operand {
        Operand::Label(s.into())
    }

    fn imm(v: i64) -> Operand {
        Operand::Immediate(Expr::Literal(v))
    }

    fn span() -> Span {
        Span::dummy()
    }

    #[test]
    fn ret_default() {
        // ret (no operand) → 0xD65F03C0
        let enc = encode_br_blr_ret(Mnemonic::Ret, &[], span()).unwrap();
        assert_eq!(enc.bits, 0xD65F_03C0);
    }

    #[test]
    fn ret_x30() {
        let enc = encode_br_blr_ret(Mnemonic::Ret, &[gp(30, RegWidth::X64)], span()).unwrap();
        assert_eq!(enc.bits, 0xD65F_03C0);
    }

    #[test]
    fn br_x0() {
        let enc = encode_br_blr_ret(Mnemonic::Br, &[gp(0, RegWidth::X64)], span()).unwrap();
        assert_eq!(enc.bits, 0xD61F_0000);
    }

    #[test]
    fn blr_x8() {
        let enc = encode_br_blr_ret(Mnemonic::Blr, &[gp(8, RegWidth::X64)], span()).unwrap();
        assert_eq!(enc.bits, 0xD63F_0100);
    }

    #[test]
    fn b_label_reloc() {
        let enc = encode_b_bl(Mnemonic::B, &[label("target")], span()).unwrap();
        assert_eq!(enc.bits & 0xFC00_0000, 0b000101 << 26);
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::Branch26);
        assert_eq!(reloc.symbol, "target");
    }

    #[test]
    fn bl_label_reloc() {
        let enc = encode_b_bl(Mnemonic::Bl, &[label("func")], span()).unwrap();
        assert_eq!(enc.bits & 0xFC00_0000, (1 << 31) | (0b00101 << 26));
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::Branch26);
        assert_eq!(reloc.symbol, "func");
    }

    #[test]
    fn svc_0x80() {
        // svc #0x80 → 0xD4001001
        let enc = encode_svc_brk(Mnemonic::Svc, &[imm(0x80)], span()).unwrap();
        assert_eq!(enc.bits, 0xD400_1001);
    }

    #[test]
    fn brk_1() {
        // brk #1 → 0xD4200020
        let enc = encode_svc_brk(Mnemonic::Brk, &[imm(1)], span()).unwrap();
        assert_eq!(enc.bits, 0xD420_0020);
    }

    #[test]
    fn bcond_eq_label() {
        let ops = vec![Operand::Condition(Condition::Eq), label("target")];
        let enc = encode_bcond(&ops, span()).unwrap();
        // base: 01010100 << 24 | 0b0000 = 0x54000000
        assert_eq!(enc.bits, 0x5400_0000);
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::Pcrel19);
    }

    #[test]
    fn cbz_x0_label() {
        let ops = vec![gp(0, RegWidth::X64), label("target")];
        let enc = encode_cbz_cbnz(Mnemonic::Cbz, &ops, span()).unwrap();
        // sf=1, 0110100, op=0 => 0xB4000000
        assert_eq!(enc.bits & 0xFF00_0000, 0xB400_0000);
        assert!(enc.relocation.is_some());
    }

    #[test]
    fn cbnz_w1_label() {
        let ops = vec![gp(1, RegWidth::W32), label("loop")];
        let enc = encode_cbz_cbnz(Mnemonic::Cbnz, &ops, span()).unwrap();
        // sf=0, 0110101, op=1 => 0x35000001
        assert_eq!(enc.bits & 0xFF00_001F, 0x3500_0001);
        assert!(enc.relocation.is_some());
    }
}
