//! Load/Store instruction encoders.

use ingot_types::expr::RelocModifier;
use ingot_types::operand::ExtendOp;
use ingot_types::{AsmError, Expr, MemoryOperand, Mnemonic, Operand, Span};

use crate::encode::{expect_reg, reg_is_64};
use crate::fields::{set_field, set_rm, set_rn, set_rt, set_rt2, truncate_imm};
use crate::{EncodedInst, PendingRelocation, RelocKind};

/// Size/opc mapping for load/store instructions.
struct LdstParams {
    size: u32, // bits [31:30]
    v: u32,    // bit [26] — 0 for GP registers
    opc: u32,  // bits [23:22]
    is_load: bool,
}

fn ldst_params(mnemonic: Mnemonic, is_64: bool) -> LdstParams {
    match mnemonic {
        Mnemonic::Ldr => LdstParams {
            size: if is_64 { 0b11 } else { 0b10 },
            v: 0,
            opc: 0b01,
            is_load: true,
        },
        Mnemonic::Str => LdstParams {
            size: if is_64 { 0b11 } else { 0b10 },
            v: 0,
            opc: 0b00,
            is_load: false,
        },
        Mnemonic::Ldrb => LdstParams {
            size: 0b00,
            v: 0,
            opc: 0b01,
            is_load: true,
        },
        Mnemonic::Strb => LdstParams {
            size: 0b00,
            v: 0,
            opc: 0b00,
            is_load: false,
        },
        Mnemonic::Ldrh => LdstParams {
            size: 0b01,
            v: 0,
            opc: 0b01,
            is_load: true,
        },
        Mnemonic::Strh => LdstParams {
            size: 0b01,
            v: 0,
            opc: 0b00,
            is_load: false,
        },
        _ => unreachable!(),
    }
}

/// Access size in bytes for the given size encoding.
fn access_size_bytes(size: u32) -> u32 {
    1 << size // 00→1, 01→2, 10→4, 11→8
}

/// Encode LDR/STR/LDRB/STRB/LDRH/STRH.
///
/// Dispatches by addressing mode in the Memory operand.
pub fn encode_ldst(
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
    let is_64 = match mnemonic {
        Mnemonic::Ldrb | Mnemonic::Strb | Mnemonic::Ldrh | Mnemonic::Strh => {
            // Byte/half always use W registers; we just check the size comes from the mnemonic
            false
        }
        _ => reg_is_64(reg, span)?,
    };
    let rt = reg.encoding();
    let params = ldst_params(mnemonic, is_64);

    match &operands[1] {
        Operand::Memory(mem) => encode_ldst_mem(rt, mem, &params, span),
        Operand::Label(name) if params.is_load => {
            // LDR literal: opc 011000 imm19 Rt
            encode_ldr_literal(rt, name, is_64)
        }
        _ => Err(AsmError::InvalidOperand {
            detail: "expected memory operand or label".into(),
            span,
        }),
    }
}

fn encode_ldst_mem(
    rt: u8,
    mem: &MemoryOperand,
    params: &LdstParams,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    match mem {
        MemoryOperand::Base { reg } => {
            // [Xn] — unsigned offset with imm12=0
            encode_ldst_unsigned_offset(rt, reg.encoding(), &Expr::Literal(0), params, span)
        }
        MemoryOperand::BaseOffset { reg, offset } => {
            encode_ldst_unsigned_offset(rt, reg.encoding(), offset, params, span)
        }
        MemoryOperand::PreIndex { reg, offset } => {
            // size 111000 opc 0 imm9 11 Rn Rt
            encode_ldst_pre_post(rt, reg.encoding(), offset, true, params, span)
        }
        MemoryOperand::PostIndex { reg, offset } => {
            // size 111000 opc 0 imm9 01 Rn Rt
            encode_ldst_pre_post(rt, reg.encoding(), offset, false, params, span)
        }
        MemoryOperand::BaseRegister {
            base,
            index,
            extend,
            amount,
        } => encode_ldst_register(
            rt,
            base.encoding(),
            index.encoding(),
            extend,
            amount,
            params,
        ),
        MemoryOperand::Label(name) if params.is_load => {
            let is_64 = params.size >= 0b10;
            encode_ldr_literal(rt, name, is_64)
        }
        _ => Err(AsmError::InvalidOperand {
            detail: "invalid addressing mode for this instruction".into(),
            span,
        }),
    }
}

/// Encode unsigned offset: `size 111 V 01 opc imm12 Rn Rt`
fn encode_ldst_unsigned_offset(
    rt: u8,
    rn: u8,
    offset: &Expr,
    params: &LdstParams,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    // size 111 V 01 opc imm12 Rn Rt — base: 0x39000000
    let mut inst: u32 = 0x3900_0000;
    inst = set_field(inst, 31, 30, params.size);
    inst = set_field(inst, 26, 26, params.v);
    inst = set_field(inst, 23, 22, params.opc);
    inst = set_rt(inst, rt);
    inst = set_rn(inst, rn);

    // Check for relocations
    match offset {
        Expr::Relocated {
            modifier,
            symbol,
            addend,
        } => {
            let kind = match modifier {
                RelocModifier::Lo12 => RelocKind::PageOff12,
                RelocModifier::GotLo12 => RelocKind::GotLoadPageOff12,
                _ => {
                    return Err(AsmError::InvalidOperand {
                        detail: "invalid relocation for load/store".into(),
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
        _ => {
            let value = offset
                .as_literal()
                .ok_or_else(|| AsmError::InvalidOperand {
                    detail: "unsigned offset requires a constant or relocation".into(),
                    span,
                })?;

            let access = access_size_bytes(params.size);
            if value < 0 {
                return Err(AsmError::ImmediateOutOfRange { value, span });
            }
            let uval = value as u64;
            if !uval.is_multiple_of(access as u64) {
                return Err(AsmError::InvalidOperand {
                    detail: format!("offset must be a multiple of {access}"),
                    span,
                });
            }
            let scaled = uval / (access as u64);
            if scaled > 0xFFF {
                return Err(AsmError::ImmediateOutOfRange { value, span });
            }

            inst = set_field(inst, 21, 10, scaled as u32);
            Ok(EncodedInst::new(inst))
        }
    }
}

/// Encode pre-index or post-index: `size 111000 opc 0 imm9 idx Rn Rt`
fn encode_ldst_pre_post(
    rt: u8,
    rn: u8,
    offset: &Expr,
    is_pre: bool,
    params: &LdstParams,
    span: Span,
) -> Result<EncodedInst, AsmError> {
    let value = offset
        .as_literal()
        .ok_or_else(|| AsmError::InvalidOperand {
            detail: "pre/post-index offset must be a literal".into(),
            span,
        })?;

    if !(-256..=255).contains(&value) {
        return Err(AsmError::ImmediateOutOfRange { value, span });
    }

    let idx: u32 = if is_pre { 0b11 } else { 0b01 };

    // size 111 V 00 opc 0 imm9 idx Rn Rt — base: 0x38000000
    let mut inst: u32 = 0x3800_0000;
    inst = set_field(inst, 31, 30, params.size);
    inst = set_field(inst, 26, 26, params.v);
    inst = set_field(inst, 23, 22, params.opc);
    inst = set_field(inst, 20, 12, truncate_imm(value, 9));
    inst = set_field(inst, 11, 10, idx);
    inst = set_rn(inst, rn);
    inst = set_rt(inst, rt);
    Ok(EncodedInst::new(inst))
}

/// Encode register offset: `size 111 V 00 opc 1 Rm option S 10 Rn Rt`
fn encode_ldst_register(
    rt: u8,
    rn: u8,
    rm: u8,
    extend: &Option<ExtendOp>,
    amount: &Option<u8>,
    params: &LdstParams,
) -> Result<EncodedInst, AsmError> {
    let option = extend.unwrap_or(ExtendOp::Uxtx).encoding() as u32;
    let s_bit: u32 = match amount {
        Some(a) if *a > 0 => 1,
        _ => 0,
    };

    // size 111 V 00 opc 1 Rm option S 10 Rn Rt — base: 0x38200800
    let mut inst: u32 = 0x3820_0800;
    inst = set_field(inst, 31, 30, params.size);
    inst = set_field(inst, 26, 26, params.v);
    inst = set_field(inst, 23, 22, params.opc);
    inst = set_rm(inst, rm);
    inst = set_field(inst, 15, 13, option);
    inst = set_field(inst, 12, 12, s_bit);
    inst = set_rn(inst, rn);
    inst = set_rt(inst, rt);
    Ok(EncodedInst::new(inst))
}

/// Encode LDR literal: `opc 011000 imm19 Rt`
fn encode_ldr_literal(rt: u8, label: &str, is_64: bool) -> Result<EncodedInst, AsmError> {
    let opc: u32 = if is_64 { 0b01 } else { 0b00 };
    // opc 011000 imm19 Rt — base: 0x18000000
    let mut inst: u32 = 0x1800_0000;
    inst = set_field(inst, 31, 30, opc);
    inst = set_rt(inst, rt);

    Ok(EncodedInst::with_reloc(
        inst,
        PendingRelocation::simple(RelocKind::Pcrel19, label.to_string()),
    ))
}

/// Encode LDP or STP.
///
/// `opc 101 V 0 type L imm7 Rt2 Rn Rt`
///
/// Addressing modes:
/// - Signed offset: type=10
/// - Pre-index: type=11
/// - Post-index: type=01
pub fn encode_ldp_stp(
    mnemonic: Mnemonic,
    operands: &[Operand],
    span: Span,
) -> Result<EncodedInst, AsmError> {
    if operands.len() != 3 {
        return Err(AsmError::WrongOperandCount {
            expected: 3,
            got: operands.len(),
            span,
        });
    }

    let rt1 = expect_reg(&operands[0], span)?;
    let rt2 = expect_reg(&operands[1], span)?;
    let is_64 = reg_is_64(rt1, span)?;

    let l_bit: u32 = match mnemonic {
        Mnemonic::Ldp => 1,
        Mnemonic::Stp => 0,
        _ => unreachable!(),
    };

    let opc: u32 = if is_64 { 0b10 } else { 0b00 };
    let access = if is_64 { 8u32 } else { 4u32 };

    match &operands[2] {
        Operand::Memory(mem) => {
            let (rn, offset_expr, addr_type) = match mem {
                MemoryOperand::Base { reg } => {
                    (reg.encoding(), &Expr::Literal(0) as &Expr, 0b10u32)
                }
                MemoryOperand::BaseOffset { reg, offset } => {
                    (reg.encoding(), offset as &Expr, 0b10u32)
                }
                MemoryOperand::PreIndex { reg, offset } => {
                    (reg.encoding(), offset as &Expr, 0b11u32)
                }
                MemoryOperand::PostIndex { reg, offset } => {
                    (reg.encoding(), offset as &Expr, 0b01u32)
                }
                _ => {
                    return Err(AsmError::InvalidOperand {
                        detail: "LDP/STP requires base+offset addressing".into(),
                        span,
                    })
                }
            };

            let offset = offset_expr.as_literal().unwrap_or(0);

            if offset % (access as i64) != 0 {
                return Err(AsmError::InvalidOperand {
                    detail: format!("LDP/STP offset must be a multiple of {access}"),
                    span,
                });
            }
            let scaled = offset / (access as i64);
            if !(-64..=63).contains(&scaled) {
                return Err(AsmError::ImmediateOutOfRange {
                    value: offset,
                    span,
                });
            }

            // opc 101 V(0) 0 type L imm7 Rt2 Rn Rt — base: 0x28000000
            let mut inst: u32 = 0x2800_0000;
            inst = set_field(inst, 31, 30, opc);
            inst = set_field(inst, 24, 23, addr_type);
            inst = set_field(inst, 22, 22, l_bit);
            inst = set_field(inst, 21, 15, truncate_imm(scaled, 7));
            inst = set_rt2(inst, rt2.encoding());
            inst = set_rn(inst, rn);
            inst = set_rt(inst, rt1.encoding());
            Ok(EncodedInst::new(inst))
        }
        _ => Err(AsmError::InvalidOperand {
            detail: "expected memory operand".into(),
            span,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ingot_types::register::{GpReg, RegWidth, SpecialReg};
    use ingot_types::Register;

    fn gp_reg(n: u8, w: RegWidth) -> Register {
        Register::Gp(GpReg::new(n, w))
    }

    fn gp(n: u8, w: RegWidth) -> Operand {
        Operand::Register(gp_reg(n, w))
    }

    fn sp() -> Register {
        Register::Special(SpecialReg::Sp)
    }

    fn span() -> Span {
        Span::dummy()
    }

    #[test]
    fn ldr_x0_x1_8() {
        // ldr x0, [x1, #8] → 0xF9400420
        let ops = vec![
            gp(0, RegWidth::X64),
            Operand::Memory(MemoryOperand::BaseOffset {
                reg: gp_reg(1, RegWidth::X64),
                offset: Expr::Literal(8),
            }),
        ];
        let enc = encode_ldst(Mnemonic::Ldr, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xF940_0420);
    }

    #[test]
    fn str_x0_x1_0() {
        // str x0, [x1] → 0xF9000020
        let ops = vec![
            gp(0, RegWidth::X64),
            Operand::Memory(MemoryOperand::Base {
                reg: gp_reg(1, RegWidth::X64),
            }),
        ];
        let enc = encode_ldst(Mnemonic::Str, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xF900_0020);
    }

    #[test]
    fn ldr_w0_x1_4() {
        // ldr w0, [x1, #4] → 0xB9400420
        let ops = vec![
            gp(0, RegWidth::W32),
            Operand::Memory(MemoryOperand::BaseOffset {
                reg: gp_reg(1, RegWidth::X64),
                offset: Expr::Literal(4),
            }),
        ];
        let enc = encode_ldst(Mnemonic::Ldr, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xB940_0420);
    }

    #[test]
    fn strb_w0_x1_0() {
        // strb w0, [x1] → 0x39000020
        let ops = vec![
            gp(0, RegWidth::W32),
            Operand::Memory(MemoryOperand::Base {
                reg: gp_reg(1, RegWidth::X64),
            }),
        ];
        let enc = encode_ldst(Mnemonic::Strb, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0x3900_0020);
    }

    #[test]
    fn ldr_x0_x1_pre_index() {
        // ldr x0, [x1, #16]! → 0xF8410C20
        let ops = vec![
            gp(0, RegWidth::X64),
            Operand::Memory(MemoryOperand::PreIndex {
                reg: gp_reg(1, RegWidth::X64),
                offset: Expr::Literal(16),
            }),
        ];
        let enc = encode_ldst(Mnemonic::Ldr, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xF841_0C20);
    }

    #[test]
    fn str_x0_x1_post_index() {
        // str x0, [x1], #-16 → 0xF81F0420
        let ops = vec![
            gp(0, RegWidth::X64),
            Operand::Memory(MemoryOperand::PostIndex {
                reg: gp_reg(1, RegWidth::X64),
                offset: Expr::Literal(-16),
            }),
        ];
        let enc = encode_ldst(Mnemonic::Str, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xF81F_0420);
    }

    #[test]
    fn stp_x29_x30_sp_pre_neg16() {
        // stp x29, x30, [sp, #-16]! → 0xA9BF7BFD
        let ops = vec![
            gp(29, RegWidth::X64),
            gp(30, RegWidth::X64),
            Operand::Memory(MemoryOperand::PreIndex {
                reg: sp(),
                offset: Expr::Literal(-16),
            }),
        ];
        let enc = encode_ldp_stp(Mnemonic::Stp, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xA9BF_7BFD);
    }

    #[test]
    fn ldp_x29_x30_sp_16() {
        // ldp x29, x30, [sp], #16 → 0xA8C17BFD
        let ops = vec![
            gp(29, RegWidth::X64),
            gp(30, RegWidth::X64),
            Operand::Memory(MemoryOperand::PostIndex {
                reg: sp(),
                offset: Expr::Literal(16),
            }),
        ];
        let enc = encode_ldp_stp(Mnemonic::Ldp, &ops, span()).unwrap();
        assert_eq!(enc.bits, 0xA8C1_7BFD);
    }

    #[test]
    fn ldr_x0_label_reloc() {
        let ops = vec![gp(0, RegWidth::X64), Operand::Label("data".into())];
        let enc = encode_ldst(Mnemonic::Ldr, &ops, span()).unwrap();
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::Pcrel19);
        assert_eq!(reloc.symbol, "data");
    }

    #[test]
    fn ldr_x0_lo12_reloc() {
        let ops = vec![
            gp(0, RegWidth::X64),
            Operand::Memory(MemoryOperand::BaseOffset {
                reg: gp_reg(1, RegWidth::X64),
                offset: Expr::Relocated {
                    modifier: RelocModifier::Lo12,
                    symbol: "sym".into(),
                    addend: 0,
                },
            }),
        ];
        let enc = encode_ldst(Mnemonic::Ldr, &ops, span()).unwrap();
        let reloc = enc.relocation.unwrap();
        assert_eq!(reloc.kind, RelocKind::PageOff12);
    }
}
