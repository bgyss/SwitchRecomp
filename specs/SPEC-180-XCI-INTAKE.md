# SPEC-180: XCI Title Intake

## Status
Draft v0.2

## Rationale
- Added XCI intake CLI wiring with optional program selection config.
- Enforced provenance inputs for XCI images and keysets.
- Implemented deterministic ExeFS/NSO extraction and RomFS asset separation.
- Added non-proprietary tests using a mock extractor.

## Purpose
Define how the pipeline ingests a user-supplied XCI and extracts code and assets while preserving legal separation and deterministic outputs.

## Goals
- Accept XCI inputs with provenance metadata and deterministic hashing.
- Extract Program NCAs, ExeFS, and NSO segments for recompilation.
- Extract RomFS assets into a separate, user-managed directory.

## Non-Goals
- Distributing keys, firmware, or proprietary assets.
- Circumventing encryption without user-provided keys.
- Packaging assets into the repo or build outputs.

## Background
- Retail titles are distributed as NCAs inside XCI images.
- NCAs are encrypted and require user-supplied keys for extraction.
- NSO executables may contain compressed segments that must be decompressed before lifting.

## Requirements
- Intake must require a valid provenance record before processing an XCI.
- Intake must accept a user-provided keyset path and fail with clear errors when keys are missing.
- Program NCA selection must be explicit and logged (TitleID, content type, version).
- ExeFS and NSO segments must be extracted deterministically and hashed.
- RomFS assets must be emitted to a separate asset output root, never mixed with code outputs.
- The intake manifest must record tool versions, hashes, and extracted file sizes.
- Use external tooling for decryption and container extraction (for example `hactool`), driven by
  a keyset in `key_name = HEX` format and passing the XCI/NCA extraction flags needed to emit
  ExeFS and RomFS outputs. citeturn2view0

## Interfaces and Data
- Inputs
  - `[[inputs]]` entries in provenance metadata with:
    - `format = "xci"` for the XCI file.
    - `format = "keyset"` for the keyset file (`*.keys`).
  - Keyset files are expected to use `name = hex` lines (32 or 64 hex chars).
  - Optional `title.toml` overrides for main program selection.
- Outputs
  - `intake/` directory with NCA metadata, ExeFS, and NSO segment blobs.
  - `assets/` directory for RomFS extraction (external to code output).
  - `module.json` and `manifest.json` with hashes and offsets.

## Deliverables
- CLI intake command that accepts an XCI plus keyset and emits deterministic extraction outputs.
- A validator that enforces asset separation and provenance requirements for XCI inputs.
- Documentation describing the intake flow and supported keyset formats.

## Implementation Notes
- The current intake accepts an unencrypted `XCI0` fixture layout (magic `XCI0`) for
  deterministic tests and uses external tooling (for example `hactool`) for real
  XCI extraction. citeturn2view0

## Open Questions
- How should update and DLC NCAs be layered or merged?
- Should the pipeline support NSP inputs in addition to XCI?
- What is the minimal metadata set needed to select the correct Program NCA?

## Acceptance Criteria
- Given a user-supplied XCI and keyset, the intake emits ExeFS/NSO and RomFS outputs deterministically.
- Missing keys or ambiguous Program NCA selection results in a clear, actionable error.
- Code outputs and asset outputs are separated and hashed in the manifest.

## Risks
- Legal and policy risk if asset separation is violated.
- Toolchain drift if external extraction tools change output formats.
- Titles with multiple Program NCAs may require manual selection rules.

## References
- https://github.com/SciresM/hactool
- https://github.com/Thealexbarney/hactoolnet
- https://github.com/jakcron/nstool
- https://switchbrew.org/wiki/NCA
