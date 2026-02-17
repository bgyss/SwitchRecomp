# Artifact Index Schema (v1)

Machine-readable JSON schema: `docs/artifact-index-schema.json`.

This schema documents the artifact index consumed by:

```bash
cargo run -p recomp-validation -- artifacts --artifact-index <path>
```

## Core Fields
- `label`: run label.
- `xci_intake_manifest`: optional intake manifest path.
- `pipeline_manifest`: optional pipeline manifest path.
- `run_manifest`: optional `recomp automate` run-manifest path.
- `reference_config`: optional `reference_video.toml`.
- `capture_config`: optional `capture_video.toml`.
- `validation_config`: optional override config path.
- `out_dir`: optional report output directory.

## Compatibility Notes
- Current parser accepts the existing fields used in `crates/recomp-validation/src/artifacts.rs`.
- Additional fields are allowed to support evolving metadata without breaking old indexes.

## Related Docs
- `docs/validation-artifacts.md`
- `docs/run-manifest-schema.md`
