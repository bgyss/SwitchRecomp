# SPEC-045: Runtime Memory Model and Load/Store Lowering

## Status
Draft v0.1

## Purpose
Define the minimal runtime memory model and the lowering rules needed so block-based output can execute load/store instructions beyond stub paths.

## Goals
- Establish a byte-addressable, little-endian memory model shared by the runtime and lifted code.
- Specify the runtime ABI surface for minimal load/store operations.
- Define lowering rules for load/store instructions into runtime memory operations.
- Enable block-based output to execute simple memory-backed logic in tests and samples.

## Non-Goals
- Full MMU emulation or virtual memory paging.
- Cache coherency modeling or performance tuning.
- Memory-mapped IO or GPU memory behavior.

## Background
The current ISA execution and block-based output rely on stubbed or in-memory test scaffolds. A minimal runtime memory model is needed to execute lifted blocks with load/store instructions consistently across pipeline and runtime layers.

## Requirements
- The runtime address space is 64-bit, byte-addressable, and little-endian.
- A minimal set of memory regions is defined: code, rodata, data, heap, and stack.
- Each region has a base address, size, and permissions (R/W/X) provided by config or manifest metadata.
- Runtime memory access enforces:
  - Alignment rules per access size (1, 2, 4, 8 bytes).
  - Bounds checks against the owning region.
  - Permission checks for read, write, and execute as applicable.
- Load/store semantics are deterministic:
  - Loads return the zero-extended value of the addressed bytes.
  - Stores write the least-significant bytes of the value.
- Errors are surfaced with explicit error codes that can be reported by the runtime and by test harnesses.

## Interfaces and Data
- Runtime ABI exposes memory access helpers with C ABI stability:
  - `recomp_mem_load_u8`, `recomp_mem_load_u16`, `recomp_mem_load_u32`, `recomp_mem_load_u64`.
  - `recomp_mem_store_u8`, `recomp_mem_store_u16`, `recomp_mem_store_u32`, `recomp_mem_store_u64`.
  - Each function returns a status code and writes the value via out-parameter (for loads).
- A memory layout descriptor is emitted in output metadata, listing each region's base, size, and permissions.
- Lowering rules map ISA load/store operations to the corresponding runtime helpers using computed effective addresses.

## Deliverables
- A runtime memory module implementing region tracking and load/store helpers.
- Lowering logic that rewrites load/store instructions into runtime calls.
- Metadata schema updates for memory layout descriptors.
- Tests that execute blocks containing load/store instructions and validate memory effects and error paths.

## Open Questions
- Which error codes should be standardized across runtime and pipeline layers?
- How should initial memory images be populated for code and data regions?
- When should the runtime trap versus return an error code to the caller?

## Acceptance Criteria
- Block-based output executes a test block with loads and stores using the runtime helpers.
- Unaligned or out-of-bounds accesses return deterministic error codes.
- A sample pipeline output includes a memory layout descriptor that matches the runtime regions.

## Risks
- Mismatched assumptions between ISA semantics and runtime helpers could cause subtle correctness bugs.
- Early memory model decisions may constrain future MMU or IO modeling.

## References
- https://developer.arm.com/documentation/den0024/a
