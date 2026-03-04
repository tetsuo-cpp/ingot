#![cfg(target_os = "macos")]

mod common;

use common::{
    assemble_clang, assemble_ingot, compare_section_bytes, compare_symbols, read_fixture,
};

// ── Basic instruction comparison ──

#[test]
fn compare_add_sub() {
    let source = read_fixture("basic", "add_sub.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
    compare_symbols(&ingot, &clang);
}

#[test]
fn compare_mov() {
    let source = read_fixture("basic", "mov.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
    compare_symbols(&ingot, &clang);
}

#[test]
fn compare_branch() {
    let source = read_fixture("basic", "branch.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
    compare_symbols(&ingot, &clang);
}

#[test]
fn compare_cond_branch() {
    let source = read_fixture("basic", "cond_branch.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
    compare_symbols(&ingot, &clang);
}

#[test]
fn compare_load_store() {
    let source = read_fixture("basic", "load_store.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
}

#[test]
fn compare_logical() {
    let source = read_fixture("basic", "logical.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
}

#[test]
fn compare_system() {
    let source = read_fixture("basic", "system.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__text");
    compare_symbols(&ingot, &clang);
}

// ── Directive comparison ──

#[test]
fn compare_data_directives() {
    let source = read_fixture("directives", "data.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__data");
}

#[test]
fn compare_space_directives() {
    let source = read_fixture("directives", "space.s");
    let ingot = assemble_ingot(&source);
    let clang = assemble_clang(&source);
    compare_section_bytes(&ingot, &clang, "__data");
}
