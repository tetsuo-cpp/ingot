use ingot_object::ObjectBuilder;
use ingot_types::{AsmError, AsmErrors, Span};

use crate::context::AssemblyContext;
use crate::pass1::pass1;
use crate::pass2::pass2;

/// Assemble ARM64 source text into Mach-O object file bytes.
pub fn assemble(source: &str) -> Result<Vec<u8>, AsmErrors> {
    // Parse
    let (statements, parse_errors) = ingot_parser::parse(source);

    // Pass 1: symbol collection + sizing
    let mut ctx = AssemblyContext::new();
    pass1(&statements, &mut ctx);

    // Pass 2: encoding + emission
    let mut builder = ObjectBuilder::new();
    builder.switch_section("__TEXT", "__text");
    let mut pass2_errors = Vec::new();
    pass2(&statements, &ctx, &mut builder, &mut pass2_errors);

    // Declare undefined globals as external symbols
    for name in &ctx.globals {
        if !ctx.symbols.contains_key(name) {
            builder.declare_external(name);
        }
    }

    // Merge all errors
    let mut all_errors = parse_errors;
    all_errors.extend(ctx.errors);
    all_errors.extend(pass2_errors);

    if !all_errors.is_empty() {
        return Err(AsmErrors::new(all_errors));
    }

    // Emit object file
    builder.finish().map_err(|e| {
        AsmErrors::new(vec![AsmError::ObjectEmission {
            detail: e.to_string(),
            span: Span::dummy(),
        }])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_exit_program() {
        let source = "\
.global _main
_main:
    mov x0, #0
    mov x16, #1
    svc #0x80
";
        let bytes = assemble(source).expect("assembly should succeed");

        use object::Object as _;
        use object::ObjectSection as _;
        use object::ObjectSymbol as _;

        let file = object::File::parse(&*bytes).unwrap();
        let main_sym = file.symbols().find(|s| s.name() == Ok("_main")).unwrap();
        assert!(main_sym.is_global());

        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        assert_eq!(text.data().unwrap().len(), 12); // 3 instructions × 4 bytes
    }

    #[test]
    fn data_section_contents() {
        let source = "\
.data
.asciz \"hello\"
";
        let bytes = assemble(source).expect("assembly should succeed");

        use object::Object as _;
        use object::ObjectSection as _;

        let file = object::File::parse(&*bytes).unwrap();
        let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();
        assert_eq!(data.data().unwrap(), b"hello\0");
    }

    #[test]
    fn external_symbol_relocation() {
        let source = "\
.global _main
_main:
    bl _extern
";
        let bytes = assemble(source).expect("assembly should succeed");

        use object::Object as _;
        use object::ObjectSection as _;

        let file = object::File::parse(&*bytes).unwrap();

        // There should be a relocation referencing _extern
        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        let relocs: Vec<_> = text.relocations().collect();
        assert_eq!(relocs.len(), 1);
    }

    #[test]
    fn adrp_add_pattern() {
        let source = "\
.global _main
_main:
    adrp x0, :pg_hi21:_foo
    add x0, x0, :lo12:_foo
";
        let bytes = assemble(source).expect("assembly should succeed");

        use object::Object as _;
        use object::ObjectSection as _;

        let file = object::File::parse(&*bytes).unwrap();
        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        let relocs: Vec<_> = text.relocations().collect();
        assert_eq!(relocs.len(), 2);
    }

    #[test]
    fn forward_branch_same_section() {
        let source = "\
.global _main
_main:
    b _target
    nop
_target:
    nop
";
        let bytes = assemble(source).expect("assembly should succeed");

        use object::Object as _;
        use object::ObjectSection as _;

        let file = object::File::parse(&*bytes).unwrap();
        let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
        let data = text.data().unwrap();
        // b _target: offset is +8 (skip nop), imm26 = 8/4 = 2
        let b_inst = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let imm26 = b_inst & 0x03FF_FFFF;
        assert_eq!(imm26, 2); // +8 bytes / 4 = 2
                              // No relocations needed for same-section branch
        let relocs: Vec<_> = text.relocations().collect();
        assert_eq!(relocs.len(), 0);
    }

    #[test]
    fn error_collection() {
        let source = "\
    invalid_instruction_here
";
        let result = assemble(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn subsections_via_symbols() {
        let source = "\
.subsections_via_symbols
.global _main
_main:
    nop
";
        let bytes = assemble(source).expect("assembly should succeed");
        use object::Object as _;
        let file = object::File::parse(&*bytes).unwrap();
        // Check MH_SUBSECTIONS_VIA_SYMBOLS flag
        if let object::FileFlags::MachO { flags } = file.flags() {
            assert_ne!(flags & 0x2000, 0); // MH_SUBSECTIONS_VIA_SYMBOLS = 0x2000
        } else {
            panic!("expected Mach-O flags");
        }
    }

    #[test]
    fn byte_data_emission() {
        let source = "\
.data
.byte 0x41, 0x42, 0x43
";
        let bytes = assemble(source).expect("assembly should succeed");
        use object::Object as _;
        use object::ObjectSection as _;
        let file = object::File::parse(&*bytes).unwrap();
        let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();
        assert_eq!(data.data().unwrap(), &[0x41, 0x42, 0x43]);
    }

    #[test]
    fn space_directive() {
        let source = "\
.data
.space 8, 0xFF
";
        let bytes = assemble(source).expect("assembly should succeed");
        use object::Object as _;
        use object::ObjectSection as _;
        let file = object::File::parse(&*bytes).unwrap();
        let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();
        assert_eq!(data.data().unwrap(), &[0xFF; 8]);
    }
}
