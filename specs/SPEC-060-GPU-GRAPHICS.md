# SPEC-060: GPU and Graphics Strategy

## Status
Draft v0.2

## Purpose
Define the graphics strategy and GPU abstractions required for correctness.

## Goals
- Separate CPU recompilation from GPU behavior where possible.
- Define a stable graphics API boundary for the runtime.

## Non-Goals
- A full GPU hardware emulator.
- A cross-platform high-level renderer replacement, unless required later.

## Strategy Options
- GPU command stream interpretation in the runtime.
- Shader translation pipeline.
- Hybrid approach with selective high-level replacements.

## Requirements
- Accurate handling of synchronization between CPU and GPU.
- Feature coverage for a Maxwell-class GPU baseline.
- Deterministic rendering for validation tests.

## Deliverables
- A graphics runtime API surface.
- A documented set of supported GPU features.

## Open Questions
- Which GPU feature set is required by the first target title?
- What level of shader translation is needed?

## Acceptance Criteria
- Render a minimal test scene generated from recompiled code.
- A graphics conformance test set for core features.

## References
- https://en.wikipedia.org/wiki/Nintendo_Switch
- https://www.nvidia.com/content/tegra/embedded-systems/pdf/tegra-x1-whitepaper.pdf
- https://switchbrew.org/wiki/Hardware
