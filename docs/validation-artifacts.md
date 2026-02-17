# Validation Artifacts and Workflow

This document defines the external artifact layout and workflow for validation runs.
All real captures, keys, and proprietary inputs stay outside the repo.

## Artifact Index (JSON)
The artifact index ties together intake outputs, recompiled outputs, and validation captures.
It is consumed by `recomp-validation artifacts`.
Schema reference: `docs/artifact-index-schema.json`.

Required or common fields:
- `label`: short name for the run (for example, `title-a24b9e807b456252-first-level`).
- `xci_intake_manifest`: absolute path to `manifest.json` from XCI intake.
- `pipeline_manifest`: absolute path to `manifest.json` from the pipeline output.
- `run_manifest`: optional absolute path to `run-manifest.json` from `recomp automate`.
- `reference_config`: absolute path to `reference_video.toml`.
- `capture_config`: absolute path to `capture_video.toml`.
- `validation_config`: optional path to `validation_config.toml`.
- `out_dir`: output directory for validation reports.

Example:
```
cat samples/validation/artifacts.example.json
```

Helper scripts:
- `scripts/validation_artifacts_init.sh` writes a template artifact index.
- `scripts/validate_artifacts.sh` runs `recomp-validation artifacts` with the index.
- `scripts/xci_validate.sh` checks the XCI intake manifest and referenced files.
- `scripts/ingest_hashed_title.sh` hashes title directory/XCI/video names and updates the local decoder ring.
- `scripts/capture-validation.sh` captures and hashes video/audio for validation.
- `scripts/capture-video-macos.sh` records macOS captures for validation.
- `scripts/capture_video.sh` provides a more configurable capture helper.

CLI helpers:
- `recomp-cli xci-validate --manifest <path>` validates the XCI intake manifest directly.

## Suggested External Layout
Use a stable, date-stamped structure so automation can find artifacts later:

- `/Volumes/External/inputs/` for intake inputs and key material.
- `/Volumes/External/outputs/` for pipeline outputs and emitted projects.
- `/Volumes/External/validation/reference/` for reference captures + configs.
- `/Volumes/External/validation/captures/` for recompiled captures.
- `/Volumes/External/validation/reports/YYYY-MM-DD/` for validation reports.

## Workflow Summary
1. Run XCI intake (external tooling) and record `manifest.json`.
2. Run the pipeline to produce a recompiled project and record `manifest.json`.
3. Normalize reference footage and capture recompiled footage with matching settings.
4. Generate hash lists, update `reference_video.toml` and `capture_video.toml`.
5. Build an artifact index JSON and run `recomp-validation artifacts`.
6. Store the reports and drift notes alongside the run date.

Hash generation helpers:
```
recomp-validation hash-frames --frames-dir /path/to/frames --out /path/to/frames.hashes
recomp-validation hash-audio --audio-file /path/to/audio.wav --out /path/to/audio.hashes
```

## Dependencies
- Rust toolchain (`cargo`).
- `hactool` or `hactoolnet` for XCI extraction.
- `ffmpeg` for capture and extraction.
- `python3` if you use external comparison scripts.

## Related Docs
- `docs/static-recompilation-flow.md` (Real XCI intake section).
- `docs/validation-traces.md` (capture workflow details).
- `docs/validation-video.md` (hash-based comparison workflow).
- `docs/title-a24b9e807b456252-first-level.md` (title-a24b9e807b456252-specific validation notes).
- `docs/run-manifest-schema.md` (automation run metadata schema).
