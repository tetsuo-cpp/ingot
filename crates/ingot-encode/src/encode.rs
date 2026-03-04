//! Top-level instruction encoding dispatch.

use ingot_types::operand::{ExtendOp, ShiftOp};
use ingot_types::register::{RegWidth, SpecialReg};
use ingot_types::{AsmError, Expr, Instruction, Mnemonic, Operand, Register, Span};

use crate::branch;
use crate::dp_imm;
use crate::dp_reg;
use crate::ldst;
use crate::system;
use crate::{EncodedInst, PendingRelocation, RelocKind};

/// Encode a single ARM64 instruction.
pub fn encode(inst: &Instruction) -> Result<EncodedInst, AsmError> {
    let ops = &inst.operands;
    let span = inst.span;

    match inst.mnemonic {
        // System
        Mnemonic::Nop => {
            if !ops.is_empty() {
                return Err(AsmError::WrongOperandCount {
                    expected: 0,
                    got: ops.len(),
                    span,
                });
            }
            Ok(system::encode_nop())
        }

        // Branch
        Mnemonic::B | Mnemonic::Bl => branch::encode_b_bl(inst.mnemonic, ops, span),
        Mnemonic::Br | Mnemonic::Blr | Mnemonic::Ret => {
            branch::encode_br_blr_ret(inst.mnemonic, ops, span)
        }
        Mnemonic::BCond => branch::encode_bcond(ops, span),
        Mnemonic::Cbz | Mnemonic::Cbnz => branch::encode_cbz_cbnz(inst.mnemonic, ops, span),
        Mnemonic::Svc | Mnemonic::Brk => branch::encode_svc_brk(inst.mnemonic, ops, span),

        // Move wide
        Mnemonic::Movz | Mnemonic::Movn | Mnemonic::Movk => {
            dp_imm::encode_mov_wide(inst.mnemonic, ops, span)
        }

        // ADR/ADRP
        Mnemonic::Adr | Mnemonic::Adrp => dp_imm::encode_adr_adrp(inst.mnemonic, ops, span),

        // ADD/SUB — dispatch between immediate and register forms
        Mnemonic::Add | Mnemonic::Adds | Mnemonic::Sub | Mnemonic::Subs => {
            encode_add_sub(inst.mnemonic, ops, span)
        }

        // CMP/CMN — aliases for SUBS/ADDS with ZR destination
        Mnemonic::Cmp => encode_cmp_cmn(Mnemonic::Subs, ops, span),
        Mnemonic::Cmn => encode_cmp_cmn(Mnemonic::Adds, ops, span),

        // Logical — dispatch between immediate and register
        Mnemonic::And | Mnemonic::Ands | Mnemonic::Orr | Mnemonic::Eor => {
            encode_logical(inst.mnemonic, ops, span)
        }

        // TST — alias for ANDS with ZR destination
        Mnemonic::Tst => encode_tst(ops, span),

        // MOV — complex alias
        Mnemonic::Mov => encode_mov(ops, span),

        // Load/Store single register
        Mnemonic::Ldr
        | Mnemonic::Str
        | Mnemonic::Ldrb
        | Mnemonic::Strb
        | Mnemonic::Ldrh
        | Mnemonic::Strh => ldst::encode_ldst(inst.mnemonic, ops, span),

        // Load/Store pair
        Mnemonic::Ldp | Mnemonic::Stp => ldst::encode_ldp_stp(inst.mnemonic, ops, span),
    }
}

/// Dispatch ADD/SUB between immediate and register forms.
fn encode_add_sub(
    mnemonic: Mnemonic,
    ops: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if ops.len() < 3 || ops.len() > 4 {
        return Err(AsmError::WrongOperandCount {
            expected: 3,
            got: ops.len(),
            span,
        });
    }

    let rd = expect_reg(&ops[0], span)?;
    let rn = expect_reg(&ops[1], span)?;

    match &ops[2] {
        Operand::Immediate(expr) => {
            // Check for trailing shift operand (LSL #0 or LSL #12)
            if ops.len() == 4 {
                match &ops[3] {
                    Operand::Shift {
                        op: ShiftOp::Lsl,
                        amount,
                    } => {
                        let shift =
                            amount
                                .as_literal()
                                .ok_or_else(|| AsmError::InvalidOperand {
                                    detail: "shift amount must be a literal".into(),
                                    span,
                                })?;
                        return dp_imm::encode_add_sub_imm_with_shift(
                            mnemonic, rd, rn, expr, shift, span,
                        );
                    }
                    _ => {
                        return Err(AsmError::InvalidOperand {
                            detail: "expected LSL shift after immediate".into(),
                            span,
                        })
                    }
                }
            }
            dp_imm::encode_add_sub_imm(mnemonic, rd, rn, expr, span)
        }
        Operand::Register(rm) => {
            // Check for shift or extend modifier
            if ops.len() == 4 {
                match &ops[3] {
                    Operand::Shift { op, amount } => {
                        let shift_amt =
                            amount
                                .as_literal()
                                .ok_or_else(|| AsmError::InvalidOperand {
                                    detail: "shift amount must be a literal".into(),
                                    span,
                                })?;
                        dp_reg::encode_add_sub_shifted(
                            mnemonic,
                            rd,
                            rn,
                            rm,
                            Some(*op),
                            shift_amt as u32,
                            span,
                        )
                    }
                    Operand::Extend { op, amount } => {
                        let ext_amt = match amount {
                            Some(expr) => {
                                expr.as_literal().ok_or_else(|| AsmError::InvalidOperand {
                                    detail: "extend amount must be a literal".into(),
                                    span,
                                })? as u32
                            }
                            None => 0,
                        };
                        dp_reg::encode_add_sub_extended(mnemonic, rd, rn, rm, *op, ext_amt, span)
                    }
                    _ => Err(AsmError::InvalidOperand {
                        detail: "expected shift or extend modifier".into(),
                        span,
                    }),
                }
            } else {
                // No shift — check if it should be extended register form
                // When SP is involved, ARM64 uses extended register form
                if is_sp(rd) || is_sp(rn) {
                    // Use extended register form: ADD Rd, Rn, Rm, UXTX #0 (or UXTW for 32-bit)
                    let ext = if reg_is_64(rm, span)? {
                        ExtendOp::Uxtx
                    } else {
                        ExtendOp::Uxtw
                    };
                    dp_reg::encode_add_sub_extended(mnemonic, rd, rn, rm, ext, 0, span)
                } else {
                    dp_reg::encode_add_sub_shifted(mnemonic, rd, rn, rm, None, 0, span)
                }
            }
        }
        _ => Err(AsmError::InvalidOperand {
            detail: "expected immediate or register".into(),
            span,
        }),
    }
}

/// Encode CMP/CMN — aliases for SUBS/ADDS XZR, Rn, op2.
fn encode_cmp_cmn(
    underlying: Mnemonic,
    ops: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if ops.len() < 2 || ops.len() > 3 {
        return Err(AsmError::WrongOperandCount {
            expected: 2,
            got: ops.len(),
            span,
        });
    }

    let rn = expect_reg(&ops[0], span)?;
    let is_64 = reg_is_64(rn, span)?;
    let zr = zero_reg(is_64);

    // Reconstruct as: SUBS/ADDS ZR, Rn, op2 [, shift/extend]
    let mut new_ops = vec![Operand::Register(zr), ops[0].clone()];
    new_ops.extend_from_slice(&ops[1..]);
    encode_add_sub(underlying, &new_ops, span)
}

/// Encode TST — alias for ANDS XZR, Rn, op2.
fn encode_tst(ops: &[Operand], span: Span) -> Result<EncodedInst, AsmError> {
    if ops.len() < 2 || ops.len() > 3 {
        return Err(AsmError::WrongOperandCount {
            expected: 2,
            got: ops.len(),
            span,
        });
    }

    let rn = expect_reg(&ops[0], span)?;
    let is_64 = reg_is_64(rn, span)?;
    let zr = zero_reg(is_64);

    // Reconstruct as: ANDS ZR, Rn, op2
    let mut new_ops = vec![Operand::Register(zr), ops[0].clone()];
    new_ops.extend_from_slice(&ops[1..]);
    encode_logical(Mnemonic::Ands, &new_ops, span)
}

/// Dispatch logical between immediate and register forms.
fn encode_logical(
    mnemonic: Mnemonic,
    ops: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if ops.len() < 3 || ops.len() > 4 {
        return Err(AsmError::WrongOperandCount {
            expected: 3,
            got: ops.len(),
            span,
        });
    }

    let rd = expect_reg(&ops[0], span)?;
    let rn = expect_reg(&ops[1], span)?;

    match &ops[2] {
        Operand::Immediate(expr) => dp_imm::encode_logical_imm(mnemonic, rd, rn, expr, span),
        Operand::Register(rm) => {
            let (shift_op, shift_amt) = if ops.len() == 4 {
                match &ops[3] {
                    Operand::Shift { op, amount } => {
                        let amt = amount
                            .as_literal()
                            .ok_or_else(|| AsmError::InvalidOperand {
                                detail: "shift amount must be a literal".into(),
                                span,
                            })?;
                        (Some(*op), amt as u32)
                    }
                    _ => {
                        return Err(AsmError::InvalidOperand {
                            detail: "expected shift modifier".into(),
                            span,
                        })
                    }
                }
            } else {
                (None, 0)
            };
            dp_reg::encode_logical_shifted(mnemonic, rd, rn, rm, shift_op, shift_amt, span)
        }
        _ => Err(AsmError::InvalidOperand {
            detail: "expected immediate or register".into(),
            span,
        }),
    }
}

/// Encode MOV — complex alias.
///
/// - MOV Rd, Rn (register): ORR Rd, XZR, Rn (or ADD Rd, Rn, #0 when SP involved)
/// - MOV Rd, #imm: try MOVZ, then MOVN, then ORR (bitmask)
fn encode_mov(ops: &[Operand], span: Span) -> Result<EncodedInst, AsmError> {
    if ops.len() != 2 {
        return Err(AsmError::WrongOperandCount {
            expected: 2,
            got: ops.len(),
            span,
        });
    }

    let rd = expect_reg(&ops[0], span)?;
    let is_64 = reg_is_64(rd, span)?;

    match &ops[1] {
        Operand::Register(rn) => {
            // MOV Rd, Rn
            if is_sp(rd) || is_sp(rn) {
                // When SP is involved, use ADD Rd, Rn, #0
                dp_imm::encode_add_sub_imm(Mnemonic::Add, rd, rn, &Expr::Literal(0), span)
            } else {
                // Use ORR Rd, XZR, Rn
                let zr = zero_reg(is_64);
                dp_reg::encode_logical_shifted(Mnemonic::Orr, rd, &zr, rn, None, 0, span)
            }
        }
        Operand::Immediate(expr) => {
            let value = expr.as_literal().ok_or_else(|| AsmError::InvalidOperand {
                detail: "MOV immediate requires a constant".into(),
                span,
            })?;

            // Try MOVZ (value fits in 16 bits at some hw position)
            if let Some(enc) = try_movz(rd, value, is_64) {
                return Ok(enc);
            }

            // Try MOVN (bitwise NOT of value fits in 16 bits at some hw position)
            if let Some(enc) = try_movn(rd, value, is_64) {
                return Ok(enc);
            }

            // Try ORR bitmask immediate
            let zr = zero_reg(is_64);
            dp_imm::encode_logical_imm(Mnemonic::Orr, rd, &zr, expr, span)
        }
        _ => Err(AsmError::InvalidOperand {
            detail: "expected register or immediate".into(),
            span,
        }),
    }
}

/// Core logic for encoding a wide move (MOVZ or MOVN).
fn try_mov_wide(rd: &Register, uval: u64, is_64: bool, base: u32) -> Option<EncodedInst> {
    let hw_positions = if is_64 { 4 } else { 2 };
    for hw in 0..hw_positions {
        let imm16 = (uval >> (hw * 16)) & 0xFFFF;
        if uval == (imm16 << (hw * 16)) {
            let mut inst = base;
            inst = crate::fields::set_sf(inst, is_64);
            inst = crate::fields::set_field(inst, 22, 21, hw as u32);
            inst = crate::fields::set_field(inst, 20, 5, imm16 as u32);
            inst = crate::fields::set_rd(inst, rd.encoding());
            return Some(EncodedInst::new(inst));
        }
    }
    None
}

/// Try to encode a MOV as MOVZ.
fn try_movz(rd: &Register, value: i64, is_64: bool) -> Option<EncodedInst> {
    let uval = if is_64 {
        value as u64
    } else {
        (value as u64) & 0xFFFF_FFFF
    };
    try_mov_wide(rd, uval, is_64, 0x5280_0000)
}

/// Try to encode a MOV as MOVN.
fn try_movn(rd: &Register, value: i64, is_64: bool) -> Option<EncodedInst> {
    let inverted = if is_64 {
        !(value as u64)
    } else {
        (!(value as u64)) & 0xFFFF_FFFF
    };
    try_mov_wide(rd, inverted, is_64, 0x1280_0000)
}

// ─── Shared Helpers ───────────────────────────────────────────────────────────

/// Extract a Register reference from an operand.
pub fn expect_reg(op: &Operand, span: Span) -> Result<&Register, AsmError> {
    match op {
        Operand::Register(r) => Ok(r),
        _ => Err(AsmError::InvalidOperand {
            detail: "expected register".into(),
            span,
        }),
    }
}

/// Extract an Expr reference from an immediate operand.
pub fn expect_imm(op: &Operand, span: Span) -> Result<&Expr, AsmError> {
    match op {
        Operand::Immediate(e) => Ok(e),
        _ => Err(AsmError::InvalidOperand {
            detail: "expected immediate".into(),
            span,
        }),
    }
}

/// Extract a label name, a literal, or a symbol expression from an operand.
pub fn expect_label_or_imm<'a>(
    op: &'a Operand,
    span: Span,
) -> Result<LabelOrImm<'a>, AsmError> {
    match op {
        Operand::Label(name) => Ok(LabelOrImm::Label(name)),
        Operand::Immediate(expr) => match expr.as_literal() {
            Some(v) => Ok(LabelOrImm::Literal(v)),
            None => Ok(LabelOrImm::SymbolExpr(expr)),
        },
        _ => Err(AsmError::InvalidOperand {
            detail: "expected label or immediate".into(),
            span,
        }),
    }
}

/// Determine if a register is 64-bit.
pub fn reg_is_64(reg: &Register, span: Span) -> Result<bool, AsmError> {
    match reg {
        Register::Gp(gp) => Ok(gp.width == RegWidth::X64),
        Register::Special(SpecialReg::Sp | SpecialReg::Xzr) => Ok(true),
        Register::Special(SpecialReg::Wsp | SpecialReg::Wzr) => Ok(false),
        _ => Err(AsmError::InvalidOperand {
            detail: "expected general-purpose or special register".into(),
            span,
        }),
    }
}

/// Check if a register is SP or WSP.
fn is_sp(reg: &Register) -> bool {
    matches!(reg, Register::Special(SpecialReg::Sp | SpecialReg::Wsp))
}

/// Extract (op, S) bits for ADD/SUB/ADDS/SUBS.
pub fn add_sub_op_s(mnemonic: Mnemonic) -> (u32, u32) {
    match mnemonic {
        Mnemonic::Add => (0, 0),
        Mnemonic::Adds => (0, 1),
        Mnemonic::Sub => (1, 0),
        Mnemonic::Subs => (1, 1),
        _ => unreachable!(),
    }
}

/// Extract opc for AND/ORR/EOR/ANDS.
pub fn logical_opc(mnemonic: Mnemonic) -> u32 {
    match mnemonic {
        Mnemonic::And => 0b00,
        Mnemonic::Orr => 0b01,
        Mnemonic::Eor => 0b10,
        Mnemonic::Ands => 0b11,
        _ => unreachable!(),
    }
}

/// Return the zero register matching the given width.
fn zero_reg(is_64: bool) -> Register {
    if is_64 {
        Register::Special(SpecialReg::Xzr)
    } else {
        Register::Special(SpecialReg::Wzr)
    }
}

/// Helper: extract relocation info from a non-literal Expr.
pub(crate) fn label_reloc_from_expr(
    expr: &Expr,
    base: u32,
    kind: RelocKind,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    match expr {
        Expr::Symbol(name) => Ok(EncodedInst::with_reloc(
            base,
            PendingRelocation::simple(kind, name.clone()),
        )),
        _ => Err(AsmError::InvalidOperand {
            detail: "expected label or literal immediate".into(),
            span,
        }),
    }
}

/// Classified result from `expect_label_or_imm`.
pub enum LabelOrImm<'a> {
    Label(&'a str),
    Literal(i64),
    SymbolExpr(&'a Expr),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::operand::Condition;
    use ingot_types::register::GpReg;

    fn make_inst(m: Mnemonic, ops: Vec<Operand>) -> Instruction {
        Instruction {
            mnemonic: m,
            operands: ops,
            span: Span::dummy(),
        }
    }

    fn gp(n: u8, w: RegWidth) -> Operand {
        Operand::Register(Register::Gp(GpReg::new(n, w)))
    }

    fn sp() -> Operand {
        Operand::Register(Register::Special(SpecialReg::Sp))
    }

    fn imm(v: i64) -> Operand {
        Operand::Immediate(Expr::Literal(v))
    }

    fn label(s: &str) -> Operand {
        Operand::Label(s.into())
    }

    // ─── System ───
    #[test]
    fn nop() {
        let enc = encode(&make_inst(Mnemonic::Nop, vec![])).unwrap();
        assert_eq!(enc.bits, 0xD503_201F);
    }

    // ─── Branch ───
    #[test]
    fn ret_default() {
        let enc = encode(&make_inst(Mnemonic::Ret, vec![])).unwrap();
        assert_eq!(enc.bits, 0xD65F_03C0);
    }

    #[test]
    fn b_label() {
        let enc = encode(&make_inst(Mnemonic::B, vec![label("target")])).unwrap();
        assert!(enc.relocation.is_some());
    }

    #[test]
    fn svc_0x80() {
        let enc = encode(&make_inst(Mnemonic::Svc, vec![imm(0x80)])).unwrap();
        assert_eq!(enc.bits, 0xD400_1001);
    }

    // ─── Data Processing Immediate ───
    #[test]
    fn add_x0_x1_42() {
        let enc = encode(&make_inst(
            Mnemonic::Add,
            vec![gp(0, RegWidth::X64), gp(1, RegWidth::X64), imm(42)],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0x9100_A820);
    }

    #[test]
    fn movz_x0_42() {
        let enc = encode(&make_inst(
            Mnemonic::Movz,
            vec![gp(0, RegWidth::X64), imm(42)],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0xD280_0540);
    }

    // ─── Data Processing Register ───
    #[test]
    fn add_x0_x1_x2() {
        let enc = encode(&make_inst(
            Mnemonic::Add,
            vec![
                gp(0, RegWidth::X64),
                gp(1, RegWidth::X64),
                gp(2, RegWidth::X64),
            ],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0x8B02_0020);
    }

    #[test]
    fn add_x0_x1_x2_lsl3() {
        let enc = encode(&make_inst(
            Mnemonic::Add,
            vec![
                gp(0, RegWidth::X64),
                gp(1, RegWidth::X64),
                gp(2, RegWidth::X64),
                Operand::Shift {
                    op: ShiftOp::Lsl,
                    amount: Expr::Literal(3),
                },
            ],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0x8B02_0C20);
    }

    // ─── Aliases ───
    #[test]
    fn cmp_x0_42() {
        // cmp x0, #42 → subs xzr, x0, #42 → 0xF100A81F
        let enc = encode(&make_inst(
            Mnemonic::Cmp,
            vec![gp(0, RegWidth::X64), imm(42)],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0xF100_A81F);
    }

    #[test]
    fn mov_x0_x1() {
        // mov x0, x1 → orr x0, xzr, x1 → 0xAA0103E0
        let enc = encode(&make_inst(
            Mnemonic::Mov,
            vec![gp(0, RegWidth::X64), gp(1, RegWidth::X64)],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0xAA01_03E0);
    }

    #[test]
    fn mov_sp_x0() {
        // mov sp, x0 → add sp, x0, #0 → 0x910003FF
        let enc = encode(&make_inst(Mnemonic::Mov, vec![sp(), gp(0, RegWidth::X64)])).unwrap();
        assert_eq!(enc.bits, 0x9100_001F);
    }

    #[test]
    fn mov_x0_minus1() {
        // mov x0, #-1 → movn x0, #0 → 0x92800000
        let enc = encode(&make_inst(
            Mnemonic::Mov,
            vec![gp(0, RegWidth::X64), imm(-1)],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0x9280_0000);
    }

    #[test]
    fn tst_x0_0xff() {
        // tst x0, #0xff → ands xzr, x0, #0xff
        let enc = encode(&make_inst(
            Mnemonic::Tst,
            vec![gp(0, RegWidth::X64), imm(0xFF)],
        ))
        .unwrap();
        // sf=1 opc=11 100100 N=1 immr=0 imms=000111 Rn=0 Rd=31
        assert_eq!(enc.bits, 0xF240_1C1F);
    }

    // ─── Load/Store ───
    #[test]
    fn ldr_x0_x1_8() {
        let enc = encode(&make_inst(
            Mnemonic::Ldr,
            vec![
                gp(0, RegWidth::X64),
                Operand::Memory(ingot_types::MemoryOperand::BaseOffset {
                    reg: Register::Gp(GpReg::new(1, RegWidth::X64)),
                    offset: Expr::Literal(8),
                }),
            ],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0xF940_0420);
    }

    #[test]
    fn stp_x29_x30_sp_pre_neg16() {
        let enc = encode(&make_inst(
            Mnemonic::Stp,
            vec![
                gp(29, RegWidth::X64),
                gp(30, RegWidth::X64),
                Operand::Memory(ingot_types::MemoryOperand::PreIndex {
                    reg: Register::Special(SpecialReg::Sp),
                    offset: Expr::Literal(-16),
                }),
            ],
        ))
        .unwrap();
        assert_eq!(enc.bits, 0xA9BF_7BFD);
    }

    #[test]
    fn bcond_eq() {
        let enc = encode(&make_inst(
            Mnemonic::BCond,
            vec![Operand::Condition(Condition::Eq), label("target")],
        ))
        .unwrap();
        assert_eq!(enc.bits & 0xFF00_000F, 0x5400_0000);
        assert!(enc.relocation.is_some());
    }
}
