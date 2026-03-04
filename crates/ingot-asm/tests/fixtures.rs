mod common;

use common::{assemble_ingot, read_fixture, read_inst};
use object::{Object as _, ObjectSection as _, ObjectSymbol as _};

// ── Basic instruction fixtures ──

#[test]
fn basic_add_sub() {
    let source = read_fixture("basic", "add_sub.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // 8 instructions × 4 bytes = 32
    assert_eq!(text.data().unwrap().len(), 32);
}

#[test]
fn basic_mov() {
    let source = read_fixture("basic", "mov.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // 8 instructions × 4 bytes = 32
    assert_eq!(text.data().unwrap().len(), 32);
}

#[test]
fn basic_branch() {
    let source = read_fixture("basic", "branch.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    let data = text.data().unwrap();
    // b, nop, bl, nop, ret = 5 instructions
    assert_eq!(data.len(), 20);

    // Same-section branches: no relocations
    assert_eq!(text.relocations().count(), 0);

    // Verify forward branch encoding (b _target skips 1 nop = offset +8, imm26=2)
    let imm26 = read_inst(data, 0) & 0x03FF_FFFF;
    assert_eq!(imm26, 2); // +8 bytes / 4 = 2
}

#[test]
fn basic_cond_branch() {
    let source = read_fixture("basic", "cond_branch.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // cmp + 6 branches + cbz + cbnz + nop = 10 instructions
    assert_eq!(text.data().unwrap().len(), 40);

    // All branches are same-section, so no relocations
    assert_eq!(text.relocations().count(), 0);
}

#[test]
fn basic_load_store() {
    let source = read_fixture("basic", "load_store.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // 10 instructions × 4 bytes = 40
    assert_eq!(text.data().unwrap().len(), 40);
}

#[test]
fn basic_logical() {
    let source = read_fixture("basic", "logical.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // 9 instructions × 4 bytes = 36
    assert_eq!(text.data().unwrap().len(), 36);
}

#[test]
fn basic_system() {
    let source = read_fixture("basic", "system.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    let data = text.data().unwrap();
    // 3 instructions × 4 bytes = 12
    assert_eq!(data.len(), 12);

    // Verify NOP encoding
    assert_eq!(read_inst(data, 0), 0xD503201F);
}

// ── Directive fixtures ──

#[test]
fn directives_sections() {
    let source = read_fixture("directives", "sections.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    // Should have both __text and __data sections
    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();

    // 2 nops in __text = 8 bytes
    assert_eq!(text.data().unwrap().len(), 8);
    // "hello\0" in __data = 6 bytes
    assert_eq!(data.data().unwrap(), b"hello\0");
}

#[test]
fn directives_data() {
    let source = read_fixture("directives", "data.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();
    let raw = data.data().unwrap();

    // .byte 0x41, 0x42, 0x43 = 3 bytes
    assert_eq!(&raw[0..3], &[0x41, 0x42, 0x43]);
    // .short 0x1234 = 2 bytes (little-endian)
    assert_eq!(&raw[3..5], &[0x34, 0x12]);
    // .word 0x12345678 = 4 bytes
    assert_eq!(&raw[5..9], &[0x78, 0x56, 0x34, 0x12]);
    // .quad = 8 bytes
    assert_eq!(
        &raw[9..17],
        &[0xF0, 0xDE, 0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]
    );
    // .ascii "hello" = 5 bytes (no null)
    assert_eq!(&raw[17..22], b"hello");
    // .asciz "world" = 6 bytes (with null)
    assert_eq!(&raw[22..28], b"world\0");
}

#[test]
fn directives_alignment() {
    let source = read_fixture("directives", "alignment.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    let data = text.data().unwrap();

    // nop (4 bytes) + padding to 16-byte boundary (12 bytes) + nop (4 bytes) = 20 bytes
    assert_eq!(data.len(), 20);
    // First instruction is NOP
    assert_eq!(read_inst(data, 0), 0xD503201F);
    // Last instruction is NOP at offset 16
    assert_eq!(read_inst(data, 16), 0xD503201F);
}

#[test]
fn directives_symbols() {
    let source = read_fixture("directives", "symbols.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let exported = file
        .symbols()
        .find(|s| s.name() == Ok("_exported"))
        .expect("_exported symbol should exist");
    assert!(exported.is_global());

    let internal = file.symbols().find(|s| s.name() == Ok("_internal"));
    // _internal should be local (or not have the global flag)
    if let Some(sym) = internal {
        assert!(!sym.is_global());
    }
}

#[test]
fn directives_space() {
    let source = read_fixture("directives", "space.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let data = file.sections().find(|s| s.name() == Ok("__data")).unwrap();
    let raw = data.data().unwrap();

    // .space 8 = 8 zero bytes + .space 4, 0xFF = 4 bytes of 0xFF
    assert_eq!(raw.len(), 12);
    assert_eq!(&raw[0..8], &[0u8; 8]);
    assert_eq!(&raw[8..12], &[0xFF; 4]);
}

#[test]
fn directives_metadata() {
    let source = read_fixture("directives", "metadata.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    // Check MH_SUBSECTIONS_VIA_SYMBOLS flag
    if let object::FileFlags::MachO { flags } = file.flags() {
        assert_ne!(
            flags & object::macho::MH_SUBSECTIONS_VIA_SYMBOLS,
            0,
            "MH_SUBSECTIONS_VIA_SYMBOLS should be set"
        );
    } else {
        panic!("expected Mach-O flags");
    }
}

// ── Relocation fixtures ──

#[test]
fn relocation_adrp_add() {
    let source = read_fixture("relocation", "adrp_add.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // 2 instructions
    assert_eq!(text.data().unwrap().len(), 8);
    // 2 relocations (PAGE21 + PAGEOFF12)
    assert_eq!(text.relocations().count(), 2);
}

#[test]
fn relocation_branch_extern() {
    let source = read_fixture("relocation", "branch_extern.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    // 1 instruction
    assert_eq!(text.data().unwrap().len(), 4);
    // 1 relocation (BRANCH26)
    assert_eq!(text.relocations().count(), 1);

    // _extern_func should be an external symbol
    let ext = file
        .symbols()
        .find(|s| s.name() == Ok("_extern_func"))
        .expect("_extern_func should exist");
    assert!(ext.is_undefined());
}

#[test]
fn relocation_same_section_branch() {
    let source = read_fixture("relocation", "same_section_branch.s");
    let bytes = assemble_ingot(&source);
    let file = object::File::parse(&*bytes).unwrap();

    let text = file.sections().find(|s| s.name() == Ok("__text")).unwrap();
    let data = text.data().unwrap();
    // b + nop + nop + b = 4 instructions
    assert_eq!(data.len(), 16);
    // Same-section branches are resolved, no relocations
    assert_eq!(text.relocations().count(), 0);

    // Verify forward branch: b _forward skips 2 nops = offset +12, imm26=3
    let imm26_fwd = read_inst(data, 0) & 0x03FF_FFFF;
    assert_eq!(imm26_fwd, 3);

    // Verify backward branch: b _start from offset 12 to 0 = -12, imm26 sign-extended
    let imm26_bwd = read_inst(data, 12) & 0x03FF_FFFF;
    // -12/4 = -3, as 26-bit two's complement: 0x03FF_FFFD
    assert_eq!(imm26_bwd, 0x03FF_FFFD);
}
