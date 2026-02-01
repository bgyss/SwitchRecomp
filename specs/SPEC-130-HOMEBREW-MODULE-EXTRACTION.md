# SPEC-130: Homebrew Module Extraction

## Status
Draft v0.2

## Purpose
Define how NRO and NSO binaries are parsed and normalized into the internal module representation used by the static recompilation pipeline.

## Goals
- Provide a deterministic, lossless mapping from NRO/NSO to module.json.
- Support compressed NSO segments and record build ids.
- Preserve section boundaries and relocation metadata for later translation.

## Non-Goals
- Full dynamic loader emulation.
- Recovering symbols beyond what the module provides.

## Background
NRO is a Switch executable format used for non-ExeFS binaries and includes header offsets for code, rodata, data, and bss, plus an optional asset section. citeturn1view0
NSO is another Switch executable format that can store segments compressed with LZ4 and includes a ModuleId for build identification. citeturn2view0

## Requirements
- The extractor must parse NRO headers and map text, rodata, data, and bss segments into module.json with file and memory offsets. citeturn1view0
- The extractor must parse NSO headers and segment tables, including any LZ4-compressed segments, and produce decompressed outputs for translation. citeturn2view0
- The extractor must capture the ModuleId/build id from NRO or NSO metadata for reproducible builds. citeturn1view0turn2view0
- If dynamic symbol tables or relocation metadata are present, the extractor must preserve them in module.json for later resolution.
- Extraction must be deterministic: identical inputs produce byte-identical module.json and extracted segment files.

## Interfaces and Data
- Input: `module.nro` and optional `module*.nso`.
- Output:
  - `module.json` with:
    - Segment list (name, file offset, size, vaddr, permissions).
    - BSS size and base address.
    - Build id/module id.
    - Optional relocation and symbol table references.
  - Extracted segment blobs stored under `out/<title>/segments/`.

## Deliverables
- NRO and NSO parsers.
- Module normalization logic.
- Tests covering NRO-only and NRO + NSO ingestion.

## Open Questions
- Do we need to support embedded MOD0 metadata beyond symbol resolution?
- Should compressed NSO segments be cached to avoid repeated LZ4 decoding?

## Acceptance Criteria
- A homebrew NRO produces a module.json with correct segment sizes and a non-empty build id.
- An NSO with compressed segments can be parsed, decompressed, and emitted as deterministic blobs.
- Extraction preserves all section boundaries necessary for later instruction translation.

## Risks
- Incorrect segment alignment or padding could lead to wrong control flow reconstruction.
- Missing relocation metadata may require fallback heuristics.

## References
- https://switchbrew.org/wiki/NRO
- https://switchbrew.org/wiki/NSO
