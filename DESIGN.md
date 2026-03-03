# Ingot: ARM64 Assembler for macOS — High-Level Design

## Context

Build an ARM64 (AArch64) assembler in Rust that reads `.s` assembly source files and emits Mach-O relocatable object files (`.o`) for macOS on Apple Silicon. The assembler handles lexing, parsing, two-pass assembly, instruction encoding, and object file emission. Linking and code signing are left to the system linker (`ld`).

---

## Workspace Layout

Cargo workspace with 7 crates. Clean separation enables independent testing, incremental compilation, and clear dependency boundaries. The encoding tables are the largest body of code and change on a different cadence than the parser or emitter.

```
ingot/
  Cargo.toml                    # [workspace] virtual manifest
  crates/
    ingot-types/                # Shared types, zero external deps
    ingot-lexer/                # Tokenizer (logos)
    ingot-parser/               # Recursive descent parser
    ingot-encode/               # ARM64 instruction encoder
    ingot-asm/                  # Two-pass assembler pipeline
    ingot-object/               # Mach-O .o emission (wraps `object` crate)
    ingot-cli/                  # CLI binary (clap)
  tests/
    fixtures/                   # .s test files organized by category
```

**Dependency graph:**
```
ingot-cli -> ingot-asm -> ingot-parser -> ingot-lexer -> ingot-types
                       -> ingot-encode -> ingot-types
                       -> ingot-object -> ingot-types
```

---

## Crate Details

### `ingot-types` — Shared Type Definitions
Zero external dependencies. Every other crate depends on this.

| Module | Contents |
|--------|----------|
| `register.rs` | `GpReg` (X0-X30, W0-W30), `SpecialReg` (SP, XZR, WZR), `FpReg`, `VecReg`, unified `Register` enum |
| `operand.rs` | `Operand` enum: Register, Immediate, Label, Memory, Shift, Extend, Condition |
| `instruction.rs` | `Mnemonic` enum (~100 variants for MVP, grows over time), `Instruction` struct (mnemonic + operands + span) |
| `directive.rs` | `Directive` enum for assembler directives |
| `expr.rs` | `Expr` type for expressions in immediates (symbol refs, arithmetic, reloc modifiers like `:lo12:`, `:pg_hi21:`) |
| `span.rs` | `Span` (byte offset + length), compatible with `miette::SourceSpan` |
| `error.rs` | Shared error types via `thiserror` + `miette` with source location annotations |

### `ingot-lexer` — Tokenizer
Uses the `logos` crate for zero-cost lexing.

- Registers recognized at token level (not as generic identifiers) for parsing efficiency
- `#` prefix on immediates is optional
- Newlines are significant (statement separators)
- Comments: `//`, `;`, and `/* */`
- Outputs `Token` stream with `Span` positions

### `ingot-parser` — Recursive Descent Parser
Hand-written, no parser framework. The ARM64 assembly grammar is line-oriented and simple enough that a framework adds overhead without benefit.

- Parses line-by-line: optional label, then instruction or directive
- Mnemonic lookup maps lowercase string -> `Mnemonic` enum
- Validates operand structure (correct number/types) at parse time
- Produces `Vec<Statement>` where `Statement` is `Label | Instruction | Directive | Empty`

### `ingot-encode` — ARM64 Instruction Encoder
The largest crate by code volume. Maps `Instruction` -> 32-bit machine code.

| Module | Instruction Group |
|--------|-------------------|
| `fields.rs` | Bit-field insertion/extraction helpers (`set_field`, `get_field`) |
| `dp_imm.rs` | ADD/SUB immediate, MOV/MOVZ/MOVN/MOVK, ADR/ADRP, logical immediate |
| `dp_reg.rs` | ADD/SUB shifted/extended register, logical shifted, MUL/DIV, shifts |
| `branch.rs` | B, BL, BR, BLR, RET, B.cond, CBZ/CBNZ, TBZ/TBNZ, SVC/BRK |
| `ldst.rs` | LDR/STR (immediate, register, literal), LDP/STP, LDRB/LDRH/STRB/STRH |
| `system.rs` | NOP, WFE, WFI, SEV, ISB, DMB, DSB, MRS, MSR |
| `simd_fp.rs` | FMOV, FADD, FSUB, FMUL, FDIV, FCMP (Phase 3) |
| `encode.rs` | Top-level dispatch: matches mnemonic -> calls appropriate module |

All ARM64 instructions are fixed 32-bit width. Encoding is hierarchical (bits 25-28 select the group). The encoder returns `EncodedInst { bits: u32, relocation: Option<PendingRelocation> }`.

**Encoding approach:** Hand-written per instruction group. Each group is 50-150 lines of clear, debuggable Rust matching the ARM spec hierarchy. Code generation from ARM's XML spec is a future option for exhaustive SIMD/SVE coverage.

### `ingot-asm` — Two-Pass Assembler Pipeline

| Module | Purpose |
|--------|---------|
| `assembler.rs` | Orchestrates the full pipeline: lex -> parse -> pass1 -> pass2 -> emit |
| `pass1.rs` | Symbol collection, section sizing (labels -> symbol table, instructions -> +4 bytes, directives -> process) |
| `pass2.rs` | Instruction encoding, branch resolution, relocation emission |
| `context.rs` | `AssemblyContext`: symbol table, section state, current address, pending relocations |
| `diagnostics.rs` | Multi-error collection and reporting via `miette` |

**Pass 1:** Iterates statements, records labels with section+offset, processes directives (section switches, alignment, data emission sizes), advances offset by 4 for each instruction.

**Pass 2:** Encodes each instruction via `ingot-encode`. For PC-relative references: resolves same-section targets directly when possible, otherwise emits relocations. Writes encoded bytes (little-endian) into section data buffers.

### `ingot-object` — Mach-O Object File Emission
Wraps the `object` crate (`BinaryFormat::MachO`, `Architecture::Aarch64`).

- Creates sections (`__TEXT,__text`, `__DATA,__data`, etc.) with proper alignment
- Adds symbols with correct binding/scope
- Maps abstract `RelocKind` to Mach-O ARM64 relocation types: `BRANCH26`, `PAGE21`, `PAGEOFF12`, `UNSIGNED`, `GOT_LOAD_PAGE21`, `GOT_LOAD_PAGEOFF12`
- Mach-O uses REL format (implicit addends)
- 16KB page alignment for segments

**Why `object` crate over custom writer:** Saves ~2000 lines of format code. Handles header layout, load command ordering, symbol table sorting, string table construction. Wrapped in its own crate so it can be swapped later if needed.

### `ingot-cli` — Command-Line Interface
Thin wrapper using `clap`.

```
ingot input.s -o output.o
```

---

## Assembler Directives (MVP)

`.text`, `.data`, `.section`, `.global`, `.local`, `.align`, `.p2align`, `.byte`, `.short`/`.hword`, `.word`/`.long`, `.quad`, `.ascii`, `.asciz`/`.string`, `.space`/`.skip`, `.subsections_via_symbols`, `.build_version`

**Deferred:** `.macro`/`.endm`, `.include`, `.if`/`.else`/`.endif`, `.set`/`.equ`, `.comm`, `.zerofill`, `.fill`

---

## Error Handling

- Errors defined with `thiserror`, rendered with `miette` (fancy source-annotated diagnostics)
- Assembler collects all errors rather than failing on the first one
- Every error carries a `Span` for precise source location
- Error categories: unknown mnemonic, invalid operand, immediate out of range, undefined symbol, duplicate symbol, branch out of range

---

## External Dependencies

| Crate | Purpose |
|-------|---------|
| `logos` 0.14 | Lexer derivation |
| `thiserror` 2 | Error type derivation |
| `miette` 7 (fancy) | Source-annotated error reporting |
| `object` 0.36 (write) | Mach-O object file writing |
| `clap` 4 (derive) | CLI argument parsing |
| `pretty_assertions` 1 | Better test diffs (dev) |

---

## Testing Strategy

### 1. Unit Tests — Instruction Encoding (`ingot-encode`)
Verify individual instruction encodings against known-good values (obtained via `clang -c` + `objdump`).

```rust
#[test]
fn test_add_x0_x1_42() {
    // ADD X0, X1, #42 -> expected 0x9100A820
    let result = encode(&inst, &ctx).unwrap();
    assert_eq!(result.bits, 0x9100A820);
}
```

### 2. Comparison Tests — vs System Assembler
Assemble `.s` fixtures with both `ingot` and `clang -c`, then compare the resulting `.o` files:
- Section contents (byte-for-byte)
- Symbol tables (names, bindings)
- Relocation entries

### 3. Fixture-Based Tests
```
tests/fixtures/
  basic/          # add_sub.s, mov.s, branch.s, load_store.s
  directives/     # sections.s, data.s, alignment.s, symbols.s
  relocation/     # adrp_add.s, branch_extern.s, got.s
  errors/         # bad_immediate.s, unknown_mnemonic.s, bad_register.s
```

### 4. End-to-End Tests
Assemble -> link with `ld` -> run -> verify exit code:

```asm
.global _main
_main:
    mov x0, #0       // exit code
    mov x16, #1      // syscall: exit
    svc #0x80
```

### 5. Fuzz Testing (later phase)
`cargo-fuzz` on lexer, parser, and encoder to ensure no panics on arbitrary input.

---

## Implementation Phases

### Phase 1: MVP
Assemble a minimal program that links and runs on macOS ARM64.

**Instructions:** ADD/SUB (imm + reg), MOV/MOVZ/MOVN/MOVK, CMP/CMN/TST, B/BL/BR/BLR/RET, B.cond, CBZ/CBNZ, LDR/STR/LDP/STP/LDRB/LDRH/STRB/STRH, NOP/SVC/BRK, ADRP/ADR

**Scope:** ~3000-5000 lines. Validation target: assemble + link + run a hello-world.

### Phase 2: Practical Completeness
Shifts, extends, bitfield ops, conditional select (CSEL/CSINC/etc.), all load/store variants, logical immediates (bitmask encoding), expression evaluation, `.set`/`.equ`/`.comm`, multi-error reporting.

### Phase 3: SIMD/FP
Scalar FP, Advanced SIMD, vector register arrangements. ~500+ instructions — may benefit from code generation at this point.

### Phase 4: Polish
Macro support, include files, conditional assembly, fuzz testing, performance, LSP integration (stretch).

---

## Verification

After implementation, validate by:
1. `cargo test` — all unit and integration tests pass
2. `cargo build` — workspace compiles without warnings
3. Assemble `tests/fixtures/e2e/hello.s` -> `hello.o`
4. Link: `ld -o hello hello.o -lSystem -syslibroot $(xcrun --show-sdk-path) -e _main -arch arm64`
5. Run `./hello` and verify correct exit code
6. Compare `hello.o` against `clang -c hello.s -o hello_clang.o` using `objdump` — section bytes and relocations should match
