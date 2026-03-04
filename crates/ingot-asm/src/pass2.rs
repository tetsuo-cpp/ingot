use ingot_encode::encode;
use ingot_object::ObjectBuilder;
use ingot_types::expr::Expr;
use ingot_types::{AsmError, Directive, PendingRelocation, RelocKind, Span, Statement};

use crate::context::AssemblyContext;

/// NOP encoding (used as placeholder on encoding errors).
const NOP: u32 = 0xD503201F;

/// Pass 2: encode instructions, emit data, resolve branches, emit relocations.
pub fn pass2(
    statements: &[Statement],
    ctx: &AssemblyContext,
    builder: &mut ObjectBuilder,
    errors: &mut Vec<AsmError>,
) {
    let mut current_section = ("__TEXT".to_string(), "__text".to_string());

    for stmt in statements {
        match stmt {
            Statement::Label { name, span: _ } => {
                let section = builder.current_section().unwrap();
                let offset = builder.section_offset(section);
                if let Err(e) = builder.define_symbol(name, offset, section) {
                    errors.push(AsmError::ObjectEmission {
                        detail: e.to_string(),
                        span: Span::dummy(),
                    });
                }
                if ctx.globals.contains(name) {
                    builder.set_global(name);
                }
            }

            Statement::Instruction(inst) => {
                match encode(inst) {
                    Ok(encoded) => {
                        let section = builder.current_section().unwrap();
                        // Get the offset where this instruction will be placed
                        let offset = builder.section_offset(section);

                        // Resolve relocation: either patch bits or prepare linker relocation
                        let (final_bits, linker_reloc) = if let Some(reloc) = &encoded.relocation {
                            resolve_relocation(
                                reloc,
                                encoded.bits,
                                offset,
                                &current_section,
                                ctx,
                                inst.span,
                                errors,
                            )
                        } else {
                            (encoded.bits, None)
                        };

                        let emit_offset =
                            builder.emit_instruction(final_bits).unwrap_or_else(|e| {
                                errors.push(AsmError::ObjectEmission {
                                    detail: e.to_string(),
                                    span: inst.span,
                                });
                                0
                            });

                        if let Some(reloc) = linker_reloc {
                            if let Err(e) = builder.add_relocation(section, emit_offset, &reloc) {
                                errors.push(AsmError::ObjectEmission {
                                    detail: e.to_string(),
                                    span: inst.span,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(e);
                        let _ = builder.emit_instruction(NOP);
                    }
                }
            }

            Statement::Directive { directive, span } => {
                handle_directive(directive, *span, &mut current_section, builder, errors);
            }
        }
    }
}

/// Resolve a relocation: returns (patched_bits, optional linker relocation).
///
/// For same-section Branch26/Pcrel19/Adr21: patches bits directly, returns no linker reloc.
/// For cross-section/external or Page21/PageOff12/Got*: returns original bits + linker reloc.
fn resolve_relocation(
    reloc: &PendingRelocation,
    bits: u32,
    inst_offset: u64,
    current_section: &(String, String),
    ctx: &AssemblyContext,
    span: Span,
    errors: &mut Vec<AsmError>,
) -> (u32, Option<PendingRelocation>) {
    let sym_info = ctx.symbols.get(&reloc.symbol);
    let same_section = sym_info
        .map(|info| info.section == *current_section)
        .unwrap_or(false);

    match reloc.kind {
        RelocKind::Branch26 => {
            if same_section {
                let sym = sym_info.unwrap();
                let pc_offset = (sym.offset as i64) - (inst_offset as i64) + reloc.addend;
                if !(-(1 << 27)..(1 << 27)).contains(&pc_offset) || pc_offset & 3 != 0 {
                    errors.push(AsmError::BranchOutOfRange { span });
                    return (bits, None);
                }
                let imm26 = ((pc_offset >> 2) as u32) & 0x03FF_FFFF;
                let patched = (bits & !0x03FF_FFFF) | imm26;
                (patched, None)
            } else {
                (bits, Some(reloc.clone()))
            }
        }

        RelocKind::Pcrel19 => {
            if same_section {
                let sym = sym_info.unwrap();
                let pc_offset = (sym.offset as i64) - (inst_offset as i64) + reloc.addend;
                if !(-(1 << 20)..(1 << 20)).contains(&pc_offset) || pc_offset & 3 != 0 {
                    errors.push(AsmError::BranchOutOfRange { span });
                    return (bits, None);
                }
                let imm19 = ((pc_offset >> 2) as u32) & 0x7FFFF;
                let patched = (bits & !0x00FF_FFE0) | (imm19 << 5);
                (patched, None)
            } else {
                errors.push(AsmError::ObjectEmission {
                    detail: format!(
                        "PC-relative 19-bit relocation for `{}` cannot cross sections",
                        reloc.symbol
                    ),
                    span,
                });
                (bits, None)
            }
        }

        RelocKind::Adr21 => {
            if same_section {
                let sym = sym_info.unwrap();
                let pc_offset = (sym.offset as i64) - (inst_offset as i64) + reloc.addend;
                if !(-(1 << 20)..(1 << 20)).contains(&pc_offset) {
                    errors.push(AsmError::BranchOutOfRange { span });
                    return (bits, None);
                }
                let imm = pc_offset as u32;
                let immlo = imm & 0x3;
                let immhi = (imm >> 2) & 0x7FFFF;
                let mask = (0x3 << 29) | (0x7FFFF << 5);
                let patched = (bits & !mask) | (immlo << 29) | (immhi << 5);
                (patched, None)
            } else {
                errors.push(AsmError::ObjectEmission {
                    detail: format!(
                        "ADR 21-bit relocation for `{}` cannot cross sections",
                        reloc.symbol
                    ),
                    span,
                });
                (bits, None)
            }
        }

        RelocKind::Page21
        | RelocKind::PageOff12
        | RelocKind::GotLoadPage21
        | RelocKind::GotLoadPageOff12 => (bits, Some(reloc.clone())),
    }
}

fn handle_directive(
    directive: &Directive,
    span: Span,
    current_section: &mut (String, String),
    builder: &mut ObjectBuilder,
    errors: &mut Vec<AsmError>,
) {
    match directive {
        Directive::Text => {
            builder.switch_section("__TEXT", "__text");
            *current_section = ("__TEXT".to_string(), "__text".to_string());
        }
        Directive::Data => {
            builder.switch_section("__DATA", "__data");
            *current_section = ("__DATA".to_string(), "__data".to_string());
        }
        Directive::Section(spec) => {
            builder.switch_section(&spec.segment, &spec.section);
            *current_section = (spec.segment.clone(), spec.section.clone());
        }

        Directive::Global(name) => {
            builder.set_global(name);
        }
        Directive::Local(_) => {}

        Directive::Align(expr) | Directive::P2Align(expr) => {
            if let Some(val) = expr.as_literal() {
                if (0..=30).contains(&val) {
                    let alignment = 1u64 << val;
                    if let Err(e) = builder.emit_align(alignment) {
                        errors.push(AsmError::ObjectEmission {
                            detail: e.to_string(),
                            span,
                        });
                    }
                }
            }
        }

        Directive::Byte(exprs) => emit_data_exprs(exprs, 1, span, builder, errors),
        Directive::Short(exprs) => emit_data_exprs(exprs, 2, span, builder, errors),
        Directive::Word(exprs) => emit_data_exprs(exprs, 4, span, builder, errors),
        Directive::Quad(exprs) => emit_data_exprs(exprs, 8, span, builder, errors),

        Directive::Ascii(s) => {
            if let Err(e) = builder.emit_data(s.as_bytes(), 1) {
                errors.push(AsmError::ObjectEmission {
                    detail: e.to_string(),
                    span,
                });
            }
        }
        Directive::Asciz(s) => {
            let mut bytes = s.as_bytes().to_vec();
            bytes.push(0);
            if let Err(e) = builder.emit_data(&bytes, 1) {
                errors.push(AsmError::ObjectEmission {
                    detail: e.to_string(),
                    span,
                });
            }
        }

        Directive::Space { size, fill } => {
            if let Some(sz) = size.as_literal() {
                if sz > 0 {
                    let fill_byte = fill.as_ref().and_then(|f| f.as_literal()).unwrap_or(0) as u8;
                    let data = vec![fill_byte; sz as usize];
                    if let Err(e) = builder.emit_data(&data, 1) {
                        errors.push(AsmError::ObjectEmission {
                            detail: e.to_string(),
                            span,
                        });
                    }
                }
            }
        }

        Directive::SubsectionsViaSymbols => {
            builder.set_subsections_via_symbols();
        }
        Directive::BuildVersion(bv) => {
            builder.set_build_version(bv);
        }
    }
}

fn emit_data_exprs(
    exprs: &[Expr],
    size: usize,
    span: Span,
    builder: &mut ObjectBuilder,
    errors: &mut Vec<AsmError>,
) {
    let mut buf = [0u8; 8];
    for expr in exprs {
        if let Some(val) = expr.as_literal() {
            match size {
                1 => buf[0] = val as u8,
                2 => buf[..2].copy_from_slice(&(val as u16).to_le_bytes()),
                4 => buf[..4].copy_from_slice(&(val as u32).to_le_bytes()),
                8 => buf[..8].copy_from_slice(&(val as u64).to_le_bytes()),
                _ => unreachable!(),
            };
            if let Err(e) = builder.emit_data(&buf[..size], 1) {
                errors.push(AsmError::ObjectEmission {
                    detail: e.to_string(),
                    span,
                });
            }
        } else {
            errors.push(AsmError::UnresolvedExpression { span });
            buf[..size].fill(0);
            let _ = builder.emit_data(&buf[..size], 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::AssemblyContext;
    use crate::pass1::pass1;

    fn run_passes(stmts: &[Statement]) -> (ObjectBuilder, Vec<AsmError>) {
        let mut ctx = AssemblyContext::new();
        pass1(stmts, &mut ctx);
        let mut builder = ObjectBuilder::new();
        builder.switch_section("__TEXT", "__text");
        let mut errors = Vec::new();
        pass2(stmts, &ctx, &mut builder, &mut errors);
        errors.extend(ctx.errors);
        (builder, errors)
    }

    fn parse_obj(bytes: &[u8]) -> object::File<'_> {
        object::File::parse(bytes).unwrap()
    }

    #[test]
    fn nop_emission() {
        use ingot_types::instruction::{Instruction, Mnemonic};
        use object::Object as _;
        use object::ObjectSection as _;

        let stmts = vec![Statement::Instruction(Instruction {
            mnemonic: Mnemonic::Nop,
            operands: vec![],
            span: Span::new(0, 3),
        })];
        let (builder, errors) = run_passes(&stmts);
        assert!(errors.is_empty());
        let bytes = builder.finish().unwrap();
        let file = parse_obj(&bytes);
        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        assert_eq!(text.data().unwrap(), &0xD503201Fu32.to_le_bytes());
    }

    #[test]
    fn label_defined_as_global() {
        use object::Object as _;
        use object::ObjectSymbol as _;

        let stmts = vec![
            Statement::Directive {
                directive: Directive::Global("_main".to_string()),
                span: Span::new(0, 13),
            },
            Statement::Label {
                name: "_main".to_string(),
                span: Span::new(14, 5),
            },
            Statement::Instruction(ingot_types::Instruction {
                mnemonic: ingot_types::Mnemonic::Nop,
                operands: vec![],
                span: Span::new(20, 3),
            }),
        ];
        let (builder, errors) = run_passes(&stmts);
        assert!(errors.is_empty());
        let bytes = builder.finish().unwrap();
        let file = parse_obj(&bytes);
        let main_sym = file.symbols().find(|s| s.name() == Ok("_main")).unwrap();
        assert!(main_sym.is_global());
    }

    #[test]
    fn data_section_asciz() {
        use object::Object as _;
        use object::ObjectSection as _;

        let stmts = vec![
            Statement::Directive {
                directive: Directive::Data,
                span: Span::new(0, 5),
            },
            Statement::Directive {
                directive: Directive::Asciz("hello".to_string()),
                span: Span::new(6, 13),
            },
        ];
        let (builder, errors) = run_passes(&stmts);
        assert!(errors.is_empty());
        let bytes = builder.finish().unwrap();
        let file = parse_obj(&bytes);
        let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();
        assert_eq!(data.data().unwrap(), b"hello\0");
    }

    #[test]
    fn encoding_error_emits_nop_placeholder() {
        use ingot_types::instruction::{Instruction, Mnemonic};
        use object::Object as _;
        use object::ObjectSection as _;

        let stmts = vec![
            Statement::Instruction(Instruction {
                mnemonic: Mnemonic::Add,
                operands: vec![],
                span: Span::new(0, 3),
            }),
            Statement::Instruction(Instruction {
                mnemonic: Mnemonic::Nop,
                operands: vec![],
                span: Span::new(4, 3),
            }),
        ];
        let (builder, errors) = run_passes(&stmts);
        assert!(!errors.is_empty());
        let bytes = builder.finish().unwrap();
        let file = parse_obj(&bytes);
        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        assert_eq!(text.data().unwrap().len(), 8);
    }

    #[test]
    fn external_branch_emits_relocation() {
        use ingot_types::instruction::{Instruction, Mnemonic};
        use ingot_types::operand::Operand;
        use object::Object as _;
        use object::ObjectSection as _;

        let stmts = vec![Statement::Instruction(Instruction {
            mnemonic: Mnemonic::Bl,
            operands: vec![Operand::Label("_extern".to_string())],
            span: Span::new(0, 10),
        })];
        let (builder, errors) = run_passes(&stmts);
        assert!(errors.is_empty(), "errors: {:?}", errors);
        let bytes = builder.finish().unwrap();
        let file = parse_obj(&bytes);
        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        let relocs: Vec<_> = text.relocations().collect();
        assert_eq!(relocs.len(), 1);
    }
}
