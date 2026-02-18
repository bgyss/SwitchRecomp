# Run Manifest Schema (v1)

Machine-readable JSON schema: `docs/run-manifest-schema.json`.

`run-manifest.json` is emitted by `recomp automate` and captures deterministic run metadata,
stage outcomes, logs, and artifact hashes.

## Required Top-Level Fields
- `schema_version`: currently `"1"`.
- `run_id`: stable run identifier.
- `execution_mode`: `local`, `cloud`, or `hybrid`.
- `host_fingerprint`: host hash used for reproducibility context.
- `tool_versions`: tool/version tuple used in cache signatures.
- `input_fingerprint`: deterministic fingerprint for non-command inputs.
- `inputs[]`: hashed run inputs.
- `steps[]`: stage-level execution records.
- `artifacts[]`: emitted artifacts and hashes.

## Step Fields
Each `steps[]` entry includes:
- `name`
- `status` (`succeeded` or `failed`)
- `duration_ms`
- `stage_attempt`
- `cache_hit`
- `cache_key`
- optional `command`
- optional `stdout_path` / `stderr_path`
- optional `outputs[]`
- optional `notes`

## Compatibility Notes
- New fields are additive for existing tooling that only needs paths/hashes.
- `recomp-validation artifacts` remains compatible; it can consume manifests by path without needing every field.

## Related Docs
- `docs/automation-loop.md`
- `docs/validation-artifacts.md`
- `docs/artifact-index-schema.json`
