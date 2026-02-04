# SPEC-046: Runtime Memory Layout Configuration

## Status
Implemented v0.1

## Purpose
Allow runtime memory layout to be configured via `title.toml`, with safe defaults when omitted.

## Goals
- Extend `title.toml` to define memory regions and permissions.
- Keep existing default layout when no config is provided.
- Validate region definitions deterministically.
- Emit configured layout in build metadata.

## Non-Goals
- Per-title MMU or virtual memory modeling.
- Dynamic resizing or runtime reconfiguration.

## Background
The current runtime memory layout is hardcoded in the pipeline. A config-driven layout enables per-title tuning while preserving deterministic output.

## Requirements
- `title.toml` supports a `runtime.memory_layout` section with explicit regions.
- Each region defines: `name`, `base`, `size`, and `permissions` (read/write/execute).
- Regions must be non-overlapping, non-zero size, and not overflow `u64`.
- If no memory layout is provided, the pipeline uses the existing minimal default.
- The build manifest records the configured memory layout.

## Interfaces and Data
- TOML schema:
  - `[runtime.memory_layout]` contains an array of `[[runtime.memory_layout.regions]]`.
  - Each region uses `{ name, base, size, permissions = { read, write, execute } }`.
- Validation errors are surfaced during pipeline parsing.

## Deliverables
- Config parser extensions for memory layout.
- Deterministic validation and defaulting logic.
- Updated manifest emission and generated runtime initialization.
- Tests covering default layout and custom layout parsing.

## Open Questions
- Should region permissions support compact string notation (e.g., "rwx") in addition to booleans?

## Acceptance Criteria
- Custom memory layout in `title.toml` is parsed and emitted in `manifest.json`.
- Invalid layouts (overlaps, zero size) fail the pipeline with clear errors.
- Default behavior is unchanged when no layout is specified.

## Risks
- User-provided layouts may diverge from actual module segment expectations if not validated together.

## References
- SPEC-045 Runtime Memory Model and Load/Store Lowering
