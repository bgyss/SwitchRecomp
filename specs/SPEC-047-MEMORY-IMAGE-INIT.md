# SPEC-047: Memory Image Initialization From Module Segments

## Status
Draft v0.2

## Rationale
- Added segment descriptors in module metadata and build manifests.
- Emit initial segment blobs and zero-fill descriptors into pipeline outputs.
- Runtime initialization loads init segments and zeroes BSS with tests.

## Purpose
Populate runtime memory regions with initial data derived from module segments (code/rodata/data/bss) so lifted output can execute meaningful memory-backed logic.

## Goals
- Define how segment metadata and initial bytes are captured in module inputs.
- Populate runtime memory regions at startup with code/rodata/data images and zero-initialized bss.
- Keep asset separation explicit; no proprietary bytes are embedded in specs or tests.

## Non-Goals
- Full relocation or dynamic loader behavior.
- Complete NSO/NRO loader coverage beyond minimal segment mapping.

## Background
The runtime memory model currently initializes regions as zeroed buffers. To execute non-trivial lifted code, the runtime must load initial segment bytes into memory based on module metadata.

## Requirements
- The pipeline records module segment descriptors with:
  - `name`, `base`, `size`, `permissions`.
  - `init_path` and `init_size` for segments with initial bytes (code/rodata/data).
  - `bss_size` or `zeroed = true` for bss regions.
- Runtime initialization loads initial bytes into their mapped regions and zero-fills bss.
- Initialization validates that init data fits within the target region.
- Initialization errors are surfaced deterministically (bad size, out of bounds, overlap).

## Interfaces and Data
- Module metadata carries segment descriptors (either in `module.json` or a sidecar manifest referenced by it).
- The build manifest records the segment descriptors and initial image paths.
- Runtime exposes an initialization helper that accepts descriptors plus byte slices for each init segment.

## Deliverables
- Segment descriptor data model.
- Pipeline support to emit segment descriptors and copy initial segment bytes into output metadata or assets.
- Runtime memory initialization logic with validation and tests.

## Open Questions
- Should init bytes be embedded as separate files in output or packed into a single image blob?

## Acceptance Criteria
- A sample module with code/data init bytes executes a load/store path against initialized memory.
- BSS regions are zeroed deterministically.
- Invalid init sizes or region mismatches fail with clear errors.

## Risks
- Early segment mapping decisions may need to be revisited when loader/relocation support expands.

## References
- SPEC-045 Runtime Memory Model and Load/Store Lowering
- SPEC-120 Homebrew Intake
- SPEC-130 Homebrew Module Extraction
