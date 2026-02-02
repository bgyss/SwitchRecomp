# SPEC-160: AArch64 Decode Coverage

## Status
Draft v0.1

## Purpose
Define the minimum AArch64 instruction decode coverage needed to lift real homebrew code into the exploratory IR.

## Goals
- Expand decode coverage beyond the current MOV/ADD/RET subset.
- Map decoded instructions into deterministic, testable IR operations.
- Capture enough semantics to support basic control flow and memory access.

## Non-Goals
- Full AArch64 ISA coverage.
- SIMD, floating point, or system instruction support.
- Accurate exception modeling beyond explicit traps for unsupported opcodes.

## Background
The current homebrew lifter decodes only a tiny subset of instructions and produces a single linear block. Real homebrew titles quickly encounter control flow, memory access, and wider arithmetic operations that must be lifted to progress.

## Requirements
### Decode Coverage (Phase 1)
- Move wide: `MOVZ`, `MOVN`, `MOVK` (64-bit).
- Register move: `ORR` with `XZR` (64-bit) as a move alias.
- Integer arithmetic: `ADD`/`SUB` (immediate and register, 64-bit).
- Compare/test: `CMP`/`CMN`/`TST` via `SUBS`/`ADDS`/`ANDS` (64-bit), emitting flag updates.
- PC-relative: `ADR`, `ADRP` (64-bit).
- Loads/stores: `LDR`/`STR` unsigned immediate for byte/half/word/dword sizes (64-bit base).
- Branching: `B`, `BL`, `BR`, `RET`, `B.<cond>`, `CBZ`/`CBNZ`, `TBZ`/`TBNZ`.
- NOP: `NOP`.

### IR Extensions
- Add IR ops for:
  - Arithmetic: `sub_i64`, `and_i64`, `or_i64`, `xor_i64`.
  - Comparisons: `cmp_i64` that updates flags.
  - Shifts: `lsl_i64`, `lsr_i64`, `asr_i64`.
  - Memory: `load_i{8,16,32,64}`, `store_i{8,16,32,64}`.
  - Control flow: `br`, `br_cond`, `call`, `ret`.
  - PC-relative: `pc_rel` or explicit `const_i64` of resolved addresses.
- When decoding 32-bit variants (W registers), zero-extend to 64-bit in the IR.

### Decode Rules
- Decode little-endian 32-bit words with 4-byte alignment.
- Reject unsupported instructions with the opcode and PC offset.
- Enforce deterministic decode limits per function to avoid runaway scans.

## Interfaces and Data
- The lifted `module.json` must include the instruction-derived ops and any new op fields.
- Flag updates must be explicit in the IR so later stages do not infer hidden side effects.

## Deliverables
- A decoder module covering Phase 1 instructions.
- IR extensions with serialization support.
- Tests that validate opcode decoding and IR emission for each instruction class.

## Acceptance Criteria
- A synthetic instruction stream containing Phase 1 opcodes lifts without errors.
- Unsupported opcodes report the PC and opcode value.
- Tests confirm 32-bit variants are zero-extended.
- Loads/stores emit correctly typed IR ops with aligned access checks.

## Risks
- Partial decode coverage may still be insufficient for real titles.
- Incorrect flag modeling can break control flow and comparisons.

## References
- https://developer.arm.com/documentation/ddi0596/latest
