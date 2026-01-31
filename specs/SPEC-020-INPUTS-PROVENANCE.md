# SPEC-020: Inputs and Provenance

## Status
Draft v0.3

## Purpose
Define input artifacts, provenance requirements, and metadata standards.

## Goals
- Accept legally obtained inputs and require provenance metadata.
- Separate proprietary assets from recompiled outputs.

## Non-Goals
- Acquiring or distributing proprietary assets.
- Automating extraction from hardware in this spec.

## Inputs
- Executable binaries or code segments used by games.
- Metadata that describes version, region, and build identifiers.
- Optional debug symbols if user-supplied.

## Binary Formats (Research Summary)
- NCA is a primary content container; program NCAs typically carry ExeFS and RomFS content.
- ExeFS is a PFS0 container that usually includes `/main` and `/main.npdm`.
- NSO0 is a common executable format for titles and tooling should treat it as a first-class input.
- NRO0 is the executable format for non-ExeFS binaries.
- NRR0 stores a list of NRO hashes used to validate allowed NROs at load time.
- NPDM contains process metadata including ACID/ACI0 service access lists.

## Provenance Requirements
- Input hashes (e.g., SHA-256).
- Source device and tool versions used to produce dumps.
- Game identifiers and version metadata.

## Provenance Schema (v1)
The pipeline consumes a `provenance.toml` file with a strict schema version.

Required fields:
- `schema_version`: currently `"1"`.
- `[title]`: `name`, `title_id`, `version`, `region`.
- `[collection]`: `device`, `collected_at`, `tool.name`, `tool.version`.
- `[[inputs]]`: `path`, `sha256`, `size`, `format`, `role`.

Supported `format` values:
- `nca`, `exefs`, `nso0`, `nro0`, `nrr0`, `npdm`, `lifted_json`.

Example:
```
schema_version = "1"

[title]
name = "Example"
title_id = "0100000000000000"
version = "1.0.0"
region = "US"

[collection]
device = "retail switch"
collected_at = "2026-01-30"

[collection.tool]
name = "nxdumptool"
version = "1.2.3"

[[inputs]]
path = "inputs/main.nso"
format = "nso0"
sha256 = "<sha256>"
size = 123456
role = "main_executable"
```

## Encryption and Access Policy
- The project does not distribute keys or proprietary content; users must supply any required keys or decrypted inputs legally and separately.
- The toolchain should accept both encrypted and pre-decrypted inputs but must record the provenance of decryption tools and versions when used.

## Output Policy
- Recompiled output does not include proprietary assets.
- Assets are provided separately at runtime by the user.

## Format Detection and Extraction Flow
The toolchain must accept multiple input types and record provenance at each step.

```
Input
  |
  +-- NCA (program content)
  |     - record raw hash
  |     - decrypt if required (record tool + version)
  |     - extract ExeFS (PFS0)
  |     - extract /main (NSO0) and /main.npdm
  |
  +-- ExeFS (PFS0)
  |     - record container hash
  |     - extract /main (NSO0) and /main.npdm
  |
  +-- NSO0
  |     - record hash
  |     - validate segment metadata
  |
  +-- NRO0
  |     - record hash
  |     - parse text/rodata/data
  |
  +-- NRR0
  |     - record hash
  |     - validate allowed NRO hashes (optional)
  |
  +-- NPDM
        - record hash
        - parse ACID/ACI0 access lists
```

### Format Detector Pseudocode
```pseudo
function detect_and_extract(path):
    magic = read_u32(path, 0)
    if magic == 'NCA3' or magic == 'NCA2':
        log_hash(path)
        if is_encrypted(path):
            require_user_keys()
            log_decryption_tool()
            path = decrypt_nca(path)
        exefs = extract_exefs(path)
        main = exefs.get('/main')
        npdm = exefs.get('/main.npdm')
        return handle_nso(main), handle_npdm(npdm)
    if magic == 'PFS0':
        log_hash(path)
        main = extract_file(path, '/main')
        npdm = extract_file(path, '/main.npdm')
        return handle_nso(main), handle_npdm(npdm)
    if magic == 'NSO0':
        log_hash(path)
        return handle_nso(path)
    if magic == 'NRO0':
        log_hash(path)
        return handle_nro(path)
    if magic == 'NRR0':
        log_hash(path)
        return handle_nrr(path)
    if is_npdm(path):
        log_hash(path)
        return handle_npdm(path)
    if is_lifted_json(path):
        log_hash(path)
        return handle_lifted_json(path)
    error(\"unsupported input format\")
```

## Deliverables
- A metadata schema definition.
- A validator for provenance metadata.
- A format detector with explicit extraction steps and logs.

## Open Questions
- How should region and version variations be tracked?
- Should the pipeline accept program NCAs directly or require pre-extracted NSO/NPDM?
- How should encrypted input handling be validated without embedding keys?

## Acceptance Criteria
- A metadata schema with validation rules.
- The toolchain refuses to build without provenance metadata.
- A format detector identifies NCA/ExeFS/NSO0/NRO0/NRR0 inputs and logs the chosen path.

## References
- https://switchbrew.org/wiki/NCA
- https://switchbrew.org/wiki/ExeFS
- https://switchbrew.org/wiki/NSO
- https://switchbrew.org/wiki/NRO
- https://switchbrew.org/wiki/NRR
- https://switchbrew.org/wiki/NPDM
- https://github.com/jakcron/nstool
- https://github.com/DinosaurPlanetRecomp/dino-recomp
