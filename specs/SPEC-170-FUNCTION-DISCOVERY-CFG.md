# SPEC-170: Function Discovery and Control-Flow Graph

## Status
Draft v0.1

## Purpose
Define basic function discovery and control-flow block construction for lifted homebrew code.

## Goals
- Replace single linear decoding with basic blocks and explicit control-flow edges.
- Discover functions starting from entrypoints and direct call targets.
- Produce deterministic, reproducible function layouts in `module.json`.

## Non-Goals
- Full symbol recovery or decompilation quality control flow.
- Indirect branch target resolution beyond explicit metadata.
- Advanced tail-call or exception unwinding analysis.

## Background
Linear decoding fails once branches appear and cannot represent multiple control-flow paths. Basic block construction allows the pipeline to model conditional branches and calls while keeping the IR deterministic.

## Requirements
### Entry Points
- Seed function discovery from the title entrypoint in `title.toml`.
- If a homebrew module provides symbol or relocation metadata, include direct export targets as additional seeds.

### Basic Blocks
- Each function is a list of basic blocks with:
  - A unique label.
  - A starting PC address.
  - A list of IR ops.
  - A terminator (`br`, `br_cond`, `call`, `ret`).
- Decode sequentially until a terminator or a known block boundary is reached.
- Split blocks at branch targets and fallthrough addresses.

### Control-Flow Edges
- `B` creates an unconditional edge to its target.
- `B.<cond>`, `CBZ`/`CBNZ`, `TBZ`/`TBNZ` create two edges: taken and fallthrough.
- `BL` creates a call edge and a fallthrough edge.
- `BR` creates an indirect edge tagged as unresolved.

### Determinism
- Function and block ordering must be stable across runs.
- Use a sorted worklist for addresses to avoid nondeterministic traversal.
- Enforce per-function decode limits with explicit errors.

### Error Handling
- Unsupported opcodes should fail the function decode with PC and opcode.
- Overlapping blocks or invalid alignment should fail with a clear error.

## Interfaces and Data
- Update the lifted module schema to support `blocks` under each function.
- Blocks must preserve their start addresses for traceability.
- The pipeline must accept both linear `ops` (legacy) and block-based functions during the transition.

## Deliverables
- Function discovery engine with a block builder.
- Schema updates and validation for block-based functions.
- Tests for:
  - Simple if/else control flow.
  - Direct calls that seed new functions.
  - Indirect branches recorded as unresolved edges.

## Acceptance Criteria
- A synthetic binary with a conditional branch yields at least two blocks and correct edges.
- Direct call targets are discovered and lifted as separate functions.
- The lifted module is deterministic when run twice on the same input.

## Risks
- Missing branch targets may drop code paths.
- Overly strict decode limits may block real-world binaries.

## References
- https://en.wikipedia.org/wiki/Basic_block
