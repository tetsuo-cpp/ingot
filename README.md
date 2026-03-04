# Ingot

An ARM64 (AArch64) assembler for macOS, written in Rust.

[![CI](https://github.com/tetsuo-cpp/ingot/actions/workflows/ci.yml/badge.svg?branch=canon)](https://github.com/tetsuo-cpp/ingot/actions/workflows/ci.yml)

Ingot reads `.s` assembly source files and emits Mach-O relocatable object files (`.o`) for Apple Silicon. The project is in early development (Phase 1 MVP).

## Quick Start

```bash
# Build
cargo build --workspace

# Assemble a source file
cargo run -- input.s -o output.o

# Link and run (macOS ARM64)
ld -o program output.o -lSystem -syslibroot $(xcrun --show-sdk-path) -e _main -arch arm64
./program
```

## Usage

```
ingot <input> [-o <output>]
```

- `input` — Assembly source file (`.s`)
- `-o, --output` — Output object file (defaults to input with `.o` extension)

## Example

```asm
// exit_42.s — exits with code 42
.global _main
_main:
    mov x0, #42       // exit code
    mov x16, #1       // syscall: exit
    svc #0x80
```

```bash
ingot exit_42.s -o exit_42.o
ld -o exit_42 exit_42.o -lSystem -syslibroot $(xcrun --show-sdk-path) -e _main -arch arm64
./exit_42
echo $?  # 42
```

## Architecture

Cargo workspace with 7 crates. Dependency flow is strictly unidirectional:

```
ingot-cli → ingot-asm → ingot-parser → ingot-lexer → ingot-types
                       → ingot-encode → ingot-types
                       → ingot-object → ingot-types
```

| Crate | Description |
|-------|-------------|
| **ingot-types** | Shared types (registers, operands, instructions, directives, expressions, errors, spans) |
| **ingot-lexer** | Tokenizer using `logos`. Registers recognized at token level |
| **ingot-parser** | Hand-written recursive descent parser. Line-oriented: optional label, then instruction or directive |
| **ingot-encode** | ARM64 instruction encoder. Maps `Instruction` → 32-bit machine code, organized by instruction group |
| **ingot-asm** | Two-pass assembler pipeline (pass1: symbol collection/sizing, pass2: encoding/relocation) |
| **ingot-object** | Mach-O `.o` emission wrapping the `object` crate. Handles sections, symbols, and ARM64 relocations |
| **ingot-cli** | Thin CLI via `clap` |

## Supported Instructions

| Category | Instructions |
|----------|-------------|
| Arithmetic | `add`, `adds`, `sub`, `subs`, `cmp`, `cmn` |
| Move | `mov`, `movz`, `movn`, `movk` |
| Logical | `and`, `ands`, `orr`, `eor`, `tst` |
| Branch | `b`, `bl`, `br`, `blr`, `ret`, `b.cond`, `cbz`, `cbnz` |
| Load/Store | `ldr`, `str`, `ldp`, `stp`, `ldrb`, `ldrh`, `strb`, `strh` |
| PC-relative | `adr`, `adrp` |
| System | `nop`, `svc`, `brk` |

## Assembler Directives

`.text`, `.data`, `.section`, `.global`, `.local`, `.align`, `.p2align`, `.byte`, `.short`/`.hword`, `.word`/`.long`, `.quad`, `.ascii`, `.asciz`/`.string`, `.space`/`.skip`, `.subsections_via_symbols`, `.build_version`

## Testing

```bash
cargo test --workspace                         # Run all tests
cargo test -p ingot-asm --test fixtures        # Fixture-based structural validation (cross-platform)
cargo test -p ingot-asm --test errors          # Error path validation (cross-platform)
cargo test -p ingot-asm --test comparison      # Byte-for-byte comparison vs clang (macOS-only)
cargo test -p ingot-asm --test e2e             # Assemble → link → run (macOS-only)
```

**Test categories:**

- **Unit tests** — Instruction encoding verified against known-good values from `clang -c` + `objdump`
- **Fixture tests** — Structural validation of assembled output from `.s` fixtures organized by category (basic, directives, relocation)
- **Comparison tests** — Byte-for-byte comparison of ingot output against `clang -c` (macOS-only)
- **End-to-end tests** — Assemble → link with `ld` → run → verify exit code (macOS-only)
- **Error tests** — Validate diagnostics for invalid input (bad immediates, unknown mnemonics, multiple errors)

## Development

```bash
cargo build --workspace                                      # Build all crates
cargo fmt --all -- --check                                   # Check formatting
cargo clippy --workspace --all-targets -- -D warnings        # Lint (warnings are errors)
```

CI runs on push to `canon` and on pull requests: format check, clippy, and tests on both Ubuntu and macOS.

## Roadmap

| Phase | Focus | Status |
|-------|-------|--------|
| **1** | MVP — minimal programs that link and run on macOS ARM64 | In progress |
| **2** | Practical completeness — shifts, extends, bitfield ops, conditional select, logical immediates, expression evaluation | Planned |
| **3** | SIMD/FP — scalar FP, Advanced SIMD, vector register arrangements | Planned |
| **4** | Polish — macros, includes, conditional assembly, fuzz testing, LSP integration | Planned |
