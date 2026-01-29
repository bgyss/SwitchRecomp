# SPEC-050: CPU ISA Lifting and Semantics

## Status
Draft v0.4

## Rationale
- Added a minimal ISA execution module for early instruction semantics tests.
- Expanded semantics coverage with arithmetic and NZCV flag updates.

## Purpose
Define instruction coverage and semantics for the Switch CPU ISA lifting layer.

## Goals
- Provide accurate instruction-by-instruction translation.
- Ensure deterministic behavior for control flow, exceptions, and flags.

## Non-Goals
- JIT or dynamic translation.
- Supporting privileged or hypervisor modes unless required by titles.

## Instruction Coverage
- ARMv8-A AArch64 integer ALU operations.
- Branching and exceptions used by user-mode code.
- Floating point operations used by titles.
- Atomics and memory barriers needed for synchronization.

## Semantics Requirements
- Exact flag behavior and side effects.
- Memory alignment and access rules.
- Precise handling of undefined or reserved behavior.

## Test Strategy
- Per-instruction golden tests.
- Block-level equivalence tests.
- Trace-based comparisons against reference execution.

## Deliverables
- A CPU lifting module.
- A test suite that validates instruction semantics.

## Open Questions
- Which optional ISA extensions are required by retail titles?
- How should undefined behavior be modeled?

## Acceptance Criteria
- 90 percent of targeted instruction classes passing tests.
- A documented list of unsupported or stubbed instructions.

## References
- https://en.wikipedia.org/wiki/Nintendo_Switch
- https://images.nvidia.com/content/pdf/tegra/Tegra-X1-whitepaper-v1.0.pdf
