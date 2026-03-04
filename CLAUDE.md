# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Ingot?

Ingot is an ARM64 (AArch64) assembler for macOS written in Rust. It reads `.s` assembly source files and emits Mach-O relocatable object files (`.o`) for Apple Silicon. The project is in early development (Phase 1 MVP) — `ingot-types` has implementation, other crates are stubs.

## Build & Test Commands

```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Run all tests
cargo test -p ingot-types        # Test a single crate
cargo test -p ingot-types -- register  # Run tests matching "register" in ingot-types
cargo fmt --all -- --check       # Check formatting
cargo clippy --workspace --all-targets -- -D warnings  # Lint (CI treats warnings as errors)

# Integration tests (in ingot-asm)
cargo test -p ingot-asm --test fixtures    # Fixture-based structural validation (cross-platform)
cargo test -p ingot-asm --test errors      # Error path validation (cross-platform)
cargo test -p ingot-asm --test comparison  # Byte-for-byte comparison vs clang (macOS-only)
cargo test -p ingot-asm --test e2e         # Assemble → link → run (macOS-only)
```

## Architecture

Cargo workspace with 7 crates. Dependency flow is strictly unidirectional:

```
ingot-cli → ingot-asm → ingot-parser → ingot-lexer → ingot-types
                       → ingot-encode → ingot-types
                       → ingot-object → ingot-types
```

- **ingot-types** — Shared types (registers, operands, instructions, directives, expressions, errors, spans). Zero external deps beyond thiserror/miette. Every other crate depends on this.
- **ingot-lexer** — Tokenizer using `logos`. Registers recognized at token level.
- **ingot-parser** — Hand-written recursive descent parser. Line-oriented: optional label, then instruction or directive.
- **ingot-encode** — ARM64 instruction encoder. Maps `Instruction` → 32-bit machine code. Organized by instruction group (dp_imm, dp_reg, branch, ldst, system, simd_fp). Returns `EncodedInst { bits: u32, relocation: Option<PendingRelocation> }`.
- **ingot-asm** — Two-pass assembler pipeline (pass1: symbol collection/sizing, pass2: encoding/relocation). Orchestrates lex → parse → pass1 → pass2 → emit.
- **ingot-object** — Mach-O `.o` emission wrapping the `object` crate. Handles sections, symbols, and ARM64 relocations (BRANCH26, PAGE21, PAGEOFF12, etc.).
- **ingot-cli** — Thin CLI via `clap`: `ingot input.s -o output.o`.

## Key Conventions

- All ARM64 instructions are fixed 32-bit width. Encoding is hierarchical (bits 25-28 select group).
- Errors use `thiserror` for derivation + `miette` for source-annotated diagnostics. Every error carries a `Span`.
- The assembler collects all errors rather than failing on first one (`AsmErrors` wraps `Vec<AsmError>`).
- `Span` is 8 bytes (u32 offset + u32 len), converts to `miette::SourceSpan`.
- `Expr` represents assembly immediates (literals, symbols, relocations, arithmetic) with constant folding via `as_literal()`.
- `b.cond` is parsed by upgrading `Mnemonic::B` to `Mnemonic::BCond` when a condition suffix is present.
- Condition codes `cs`/`cc` are aliases for `hs`/`lo`.
- Verification target: assemble `.s` → link with `ld` → run on macOS ARM64. Compare output against `clang -c`.

## CI

GitHub Actions on push to `canon` branch and on PRs: fmt check, clippy (deny warnings), tests on ubuntu + macos.
