#![allow(dead_code)]

use std::path::PathBuf;

/// Path to the workspace-level `tests/fixtures/` directory.
pub fn fixtures_dir() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("../../tests/fixtures")
        .canonicalize()
        .expect("fixtures dir should exist")
}

/// Read a fixture `.s` file by category and name.
pub fn read_fixture(category: &str, name: &str) -> String {
    let path = fixtures_dir().join(category).join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Assemble source with ingot, panic on error.
pub fn assemble_ingot(source: &str) -> Vec<u8> {
    ingot_asm::assemble(source).expect("ingot assembly should succeed")
}

/// Assemble source with clang, return `.o` bytes.
#[cfg(target_os = "macos")]
pub fn assemble_clang(source: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.s");
    let output = dir.path().join("output.o");
    std::fs::write(&input, source).unwrap();
    let result = std::process::Command::new("clang")
        .args(["-c", "-target", "arm64-apple-macos", "-o"])
        .arg(&output)
        .arg(&input)
        .output()
        .expect("clang should be available");
    assert!(
        result.status.success(),
        "clang assembly failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::read(&output).unwrap()
}

/// Assemble with ingot, link with ld, run, return exit code.
#[cfg(target_os = "macos")]
pub fn link_and_run(obj_bytes: &[u8]) -> i32 {
    let dir = tempfile::tempdir().unwrap();
    let obj_path = dir.path().join("test.o");
    let bin_path = dir.path().join("test");
    std::fs::write(&obj_path, obj_bytes).unwrap();

    let sdk_output = std::process::Command::new("xcrun")
        .args(["--show-sdk-path"])
        .output()
        .expect("xcrun should be available");
    let sdk_path = String::from_utf8(sdk_output.stdout).unwrap();
    let sdk_path = sdk_path.trim();

    let result = std::process::Command::new("ld")
        .args([
            "-lSystem",
            "-syslibroot",
            sdk_path,
            "-e",
            "_main",
            "-arch",
            "arm64",
            "-platform_version",
            "macos",
            "14.0.0",
            "14.0.0",
            "-o",
        ])
        .arg(&bin_path)
        .arg(&obj_path)
        .output()
        .expect("ld should be available");
    assert!(
        result.status.success(),
        "linking failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let output = std::process::Command::new(&bin_path)
        .output()
        .expect("binary should run");
    output.status.code().unwrap_or(-1)
}

/// Compare section bytes between two object files.
#[cfg(target_os = "macos")]
pub fn compare_section_bytes(ingot: &[u8], clang: &[u8], section_name: &str) {
    use object::{Object as _, ObjectSection as _};

    let ingot_file = object::File::parse(ingot).unwrap();
    let clang_file = object::File::parse(clang).unwrap();

    let ingot_section = ingot_file.sections().find(|s| s.name() == Ok(section_name));
    let clang_section = clang_file.sections().find(|s| s.name() == Ok(section_name));

    match (ingot_section, clang_section) {
        (Some(i), Some(c)) => {
            pretty_assertions::assert_eq!(
                i.data().unwrap(),
                c.data().unwrap(),
                "section {section_name} bytes differ"
            );
        }
        (None, None) => {}
        (Some(_), None) => panic!("section {section_name} present in ingot but not clang"),
        (None, Some(_)) => panic!("section {section_name} present in clang but not ingot"),
    }
}

/// Compare symbol names and bindings between two object files (filtering ltmp symbols).
#[cfg(target_os = "macos")]
pub fn compare_symbols(ingot: &[u8], clang: &[u8]) {
    use object::{Object as _, ObjectSymbol as _};

    let ingot_file = object::File::parse(ingot).unwrap();
    let clang_file = object::File::parse(clang).unwrap();

    let ingot_syms: std::collections::BTreeSet<_> = ingot_file
        .symbols()
        .filter_map(|s| {
            let name = s.name().ok()?;
            if name.starts_with("ltmp") || name.is_empty() {
                return None;
            }
            Some((name.to_string(), s.is_global()))
        })
        .collect();

    let clang_syms: std::collections::BTreeSet<_> = clang_file
        .symbols()
        .filter_map(|s| {
            let name = s.name().ok()?;
            if name.starts_with("ltmp") || name.is_empty() {
                return None;
            }
            Some((name.to_string(), s.is_global()))
        })
        .collect();

    pretty_assertions::assert_eq!(ingot_syms, clang_syms, "symbols differ");
}

/// Compare relocation entries between two object files for a given section.
#[cfg(target_os = "macos")]
pub fn compare_relocations(ingot: &[u8], clang: &[u8], section_name: &str) {
    use object::{Object as _, ObjectSection as _};

    let ingot_file = object::File::parse(ingot).unwrap();
    let clang_file = object::File::parse(clang).unwrap();

    let ingot_section = ingot_file
        .sections()
        .find(|s| s.name() == Ok(section_name))
        .unwrap();
    let clang_section = clang_file
        .sections()
        .find(|s| s.name() == Ok(section_name))
        .unwrap();

    let ingot_relocs: Vec<_> = ingot_section
        .relocations()
        .map(|(offset, reloc)| (offset, format!("{:?}", reloc.kind())))
        .collect();

    let clang_relocs: Vec<_> = clang_section
        .relocations()
        .map(|(offset, reloc)| (offset, format!("{:?}", reloc.kind())))
        .collect();

    pretty_assertions::assert_eq!(
        ingot_relocs,
        clang_relocs,
        "relocations differ in {section_name}"
    );
}
