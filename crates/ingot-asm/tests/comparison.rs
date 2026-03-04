#![cfg(target_os = "macos")]

mod common;

use common::compare_fixture;

// ── Basic instruction comparison ──

#[test]
fn compare_add_sub() {
    compare_fixture("basic", "add_sub.s", "__text");
}

#[test]
fn compare_mov() {
    compare_fixture("basic", "mov.s", "__text");
}

#[test]
fn compare_branch() {
    compare_fixture("basic", "branch.s", "__text");
}

#[test]
fn compare_cond_branch() {
    compare_fixture("basic", "cond_branch.s", "__text");
}

#[test]
fn compare_load_store() {
    compare_fixture("basic", "load_store.s", "__text");
}

#[test]
fn compare_logical() {
    compare_fixture("basic", "logical.s", "__text");
}

#[test]
fn compare_system() {
    compare_fixture("basic", "system.s", "__text");
}

// ── Directive comparison ──

#[test]
fn compare_data_directives() {
    compare_fixture("directives", "data.s", "__data");
}

#[test]
fn compare_space_directives() {
    compare_fixture("directives", "space.s", "__data");
}
