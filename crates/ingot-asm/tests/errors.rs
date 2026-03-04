mod common;

use common::read_fixture;

#[test]
fn unknown_mnemonic() {
    let source = read_fixture("errors", "unknown_mnemonic.s");
    let result = ingot_asm::assemble(&source);
    assert!(result.is_err(), "should fail on unknown mnemonic");
    let errors = result.unwrap_err();
    assert!(!errors.errors.is_empty());
    let has_error = errors.errors.iter().any(|e| {
        matches!(e, ingot_asm::AsmError::UnknownMnemonic { .. })
    });
    assert!(has_error, "expected unknown mnemonic error");
}

#[test]
fn bad_immediate() {
    let source = read_fixture("errors", "bad_immediate.s");
    let result = ingot_asm::assemble(&source);
    assert!(result.is_err(), "should fail on out-of-range immediate");
    let errors = result.unwrap_err();
    let has_error = errors.errors.iter().any(|e| {
        matches!(e, ingot_asm::AsmError::ImmediateOutOfRange { .. })
    });
    assert!(has_error, "expected immediate out of range error");
}

#[test]
fn multiple_errors() {
    let source = read_fixture("errors", "multiple_errors.s");
    let result = ingot_asm::assemble(&source);
    assert!(result.is_err(), "should fail with multiple errors");
    let errors = result.unwrap_err();
    assert!(
        errors.errors.len() >= 2,
        "expected at least 2 errors, got {}",
        errors.errors.len()
    );
}
