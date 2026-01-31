# SPEC-095: Build Manifest Integrity

## Status
Draft v0.2

## Purpose
Define requirements for the build `manifest.json` to fully account for its own contents and generated artifacts.

## Goals
- Ensure the build manifest is self-describing and auditable.
- Keep manifest generation deterministic and reproducible.

## Non-Goals
- Defining new input formats or provenance rules.
- Replacing the existing build manifest schema in SPEC-090.

## Background
The pipeline emits `manifest.json` alongside generated source files. Today the manifest does not list itself in the `generated_files` set, which breaks full self-auditing.

## Requirements
- `manifest.json` MUST include an entry for itself in `generated_files`.
- `generated_files` MUST include all emitted files for the project output.
- The manifest MUST be written in a deterministic order.
- Hashes and sizes MUST match the on-disk file contents, except for the self entry which uses the declared hash basis.

## Interfaces and Data
- Extend the build manifest schema to include a `manifest.json` entry.
- Record a `manifest_self_hash_basis` field describing how the self hash is computed.
- Allow a two-pass write or placeholder-hash basis for the self entry if needed.

## Deliverables
- Updated manifest emission logic that includes `manifest.json`.
- Tests that verify manifest integrity and self-inclusion.

## Open Questions
- None.

## Acceptance Criteria
- `manifest.json` lists every generated file including itself.
- Generated file hashes and sizes match the files on disk, except for the self entry which matches the declared hash basis.
- Re-running the pipeline with identical inputs yields the same manifest.

## Risks
- Two-pass generation may require temporary files or careful ordering.

## References
- SPEC-090: Build, Packaging, and Distribution
