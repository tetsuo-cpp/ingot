#![cfg(target_os = "macos")]

mod common;

use common::{assemble_ingot, link_and_run, read_fixture};

#[test]
fn exit_zero() {
    let source = read_fixture("e2e", "exit_zero.s");
    let obj = assemble_ingot(&source);
    assert_eq!(link_and_run(&obj), 0);
}

#[test]
fn exit_42() {
    let source = read_fixture("e2e", "exit_42.s");
    let obj = assemble_ingot(&source);
    assert_eq!(link_and_run(&obj), 42);
}

#[test]
fn branch_and_exit() {
    let source = read_fixture("e2e", "branch_and_exit.s");
    let obj = assemble_ingot(&source);
    assert_eq!(link_and_run(&obj), 0);
}

#[test]
fn conditional() {
    let source = read_fixture("e2e", "conditional.s");
    let obj = assemble_ingot(&source);
    assert_eq!(link_and_run(&obj), 0);
}

#[test]
fn load_store_stack() {
    let source = read_fixture("e2e", "load_store_stack.s");
    let obj = assemble_ingot(&source);
    assert_eq!(link_and_run(&obj), 0);
}
