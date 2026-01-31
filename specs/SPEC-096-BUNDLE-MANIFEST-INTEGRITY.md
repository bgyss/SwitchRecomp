# SPEC-096: Bundle Manifest Integrity

## Status
Draft v0.1

## Purpose
Define requirements for `bundle-manifest.json` to fully enumerate bundle contents, including itself.

## Goals
- Ensure bundle manifests are self-describing and audit-ready.
- Keep bundle manifests deterministic and reproducible.

## Non-Goals
- Changing the bundle layout defined in SPEC-090.
- Defining new provenance fields.

## Background
The bundle manifest is generated after enumerating bundle files. As a result, it does not list itself, which prevents full integrity validation.

## Requirements
- `bundle-manifest.json` MUST include an entry for itself in the bundle file list.
- The manifest MUST include every file in the bundle, including metadata files.
- Hashes and sizes MUST match the on-disk file contents.

## Interfaces and Data
- Extend the bundle manifest schema or generation to record `bundle-manifest.json`.
- Allow a two-pass write or a `manifest_self_sha256` field if needed.

## Deliverables
- Updated bundle packaging logic to include the manifest entry.
- Tests that verify bundle manifest self-inclusion.

## Open Questions
- Should the bundle manifest include itself in the `files` list or a dedicated field?

## Acceptance Criteria
- `bundle-manifest.json` lists every bundle file including itself.
- Checksums and sizes match the bundle contents.

## Risks
- Two-pass generation can complicate reproducibility if not deterministic.

## References
- SPEC-090: Build, Packaging, and Distribution
