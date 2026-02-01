# SPEC-120: Homebrew Candidate Intake

## Status
Draft v0.2

## Purpose
Define the intake requirements and metadata capture needed to select a Switch homebrew title and feed it into the static recompilation pipeline.

## Goals
- Accept a legally distributable homebrew candidate with clear provenance.
- Normalize input artifacts into a deterministic intake manifest.
- Preserve asset separation while extracting optional metadata and RomFS content.

## Non-Goals
- Supporting retail titles or proprietary content.
- Recompiling dynamically generated code.
- Handling titles that require runtime emulation rather than static recompilation.

## Background
Homebrew on Switch is commonly distributed as NRO modules, which are executable formats for non-ExeFS binaries and may include an optional asset section for icon, NACP metadata, and RomFS content. citeturn1view0

## Requirements
- Intake must accept an NRO as the primary module, with optional auxiliary NSO modules when supplied.
- Intake must reject inputs that contain proprietary retail assets or encrypted formats.
- The NRO header and module identifier must be parsed to capture segment sizes, memory offsets, and build id metadata. citeturn1view0
- If a homebrew asset section is present, intake must detect and extract the icon, NACP, and RomFS offsets and sizes. citeturn1view0
- If NACP is present, intake must capture it as a 0x4000-byte control.nacp blob with UTF-8 strings. citeturn3view0
- Inputs must be hashed (SHA-256) and stored alongside provenance metadata.
- Intake must emit a deterministic manifest describing:
  - Input file paths, sizes, hashes.
  - Parsed module id/build id.
  - Asset section presence and sizes.
  - Tool versions used for parsing.

## Interfaces and Data
- Inputs are stored under a per-title directory containing:
  - `module.nro`
  - Optional `module*.nso`
  - Optional `assets/` extracted from the NRO asset section
  - `provenance.toml` and `title.toml` metadata files
- Intake produces a `module.json` and `manifest.json` compatible with the pipeline CLI.

## Deliverables
- NRO and optional NSO intake parser.
- Deterministic intake manifest schema.
- Documentation for required input layout and provenance fields.

## Open Questions
- Should the intake step allow raw ELF inputs for developer-built homebrew, or require NRO only?
- How should optional NSO modules be mapped into a single module.json layout?

## Acceptance Criteria
- A sample homebrew NRO can be ingested with a generated manifest that records hashes, build id, and asset offsets.
- If the NRO contains NACP and RomFS, intake extracts and records them without mixing assets into code output.
- Intake fails fast with a clear error when a required field is missing or unsupported.

## Risks
- Homebrew titles that embed unexpected custom data in the asset section may require extra parsing rules.
- Incorrect module id parsing could break reproducibility or provenance tracking.

## References
- https://switchbrew.org/wiki/NRO
- https://switchbrew.org/wiki/NACP
