mod common;

use common::read_fixture;

#[test]
fn unknown_mnemonic() {
    let source = read_fixture("errors", "unknown_mnemonic.s");
    let result = ingot_asm::assemble(&source);
    assert!(result.is_err(), "should fail on unknown mnemonic");
    let errors = result.unwrap_err();
    assert!(!errors.errors.is_empty());
    // Should contain a parse error for the unknown mnemonic
    let has_parse_error = errors.errors.iter().any(|e| {
        matches!(e, ingot_asm::AsmError::ParseError { .. })
            || matches!(e, ingot_asm::AsmError::UnknownMnemonic { .. })
    });
    assert!(has_parse_error, "expected parse or unknown mnemonic error");
}

#[test]
fn bad_immediate() {
    let source = read_fixture("errors", "bad_immediate.s");
    let result = ingot_asm::assemble(&source);
    assert!(result.is_err(), "should fail on out-of-range immediate");
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
