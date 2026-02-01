# SPEC-150: Homebrew Asset Packaging

## Status
Draft v0.3

## Purpose
Define how homebrew asset data (icon, NACP, RomFS) is extracted from NROs and packaged with the recompiled output while preserving asset separation.

## Goals
- Extract and preserve NRO asset section contents deterministically.
- Keep code output and asset output strictly separated.
- Emit metadata that allows a runtime to mount RomFS content.

## Non-Goals
- Repacking assets back into NRO format.
- Handling proprietary retail assets.

## Background
NRO files can include an optional asset section that contains icon, NACP metadata, and RomFS content. citeturn1view0
NACP (control.nacp) is a 0x4000-byte metadata file with UTF-8 strings used for title metadata. citeturn3view0

## Requirements
- If an NRO asset section is present, extraction must locate the icon, NACP, and RomFS offsets/sizes and copy them into the output asset directory. citeturn1view0
- Extracted NACP must be stored verbatim as `control.nacp` and validated for size 0x4000. citeturn3view0
- Extracted icon data must be preserved as raw bytes with file metadata describing expected image type when known.
- RomFS content must be extracted into a deterministic directory structure and hashed for provenance.
- The output `manifest.json` must include per-asset hashes and sizes alongside code hashes.

## Interfaces and Data
- Output layout:
  - `out/<title>/assets/control.nacp`
  - `out/<title>/assets/icon.bin`
  - `out/<title>/assets/romfs/<romfs-path>`
  - `out/<title>/manifest.json`
- `manifest.json` fields for asset hashes, sizes, and source offsets.

## Deliverables
- Asset extraction tool.
- Asset manifest schema updates.
- Documentation for runtime RomFS mounting expectations.

## Open Questions
- Should icon data be normalized to a specific image format for downstream tooling?
- How should multi-language NACP strings be surfaced in title metadata?

## Acceptance Criteria
- A homebrew NRO with asset section yields extracted icon, NACP, and RomFS file tree in a deterministic output directory.
- Asset hashes in manifest.json match the extracted bytes.
- Code output remains separate from extracted assets.

## Risks
- Some homebrew titles may omit NACP or RomFS entirely, requiring graceful handling.
- Incorrect RomFS extraction could break resource loading at runtime.

## References
- https://switchbrew.org/wiki/NRO
- https://switchbrew.org/wiki/NACP
