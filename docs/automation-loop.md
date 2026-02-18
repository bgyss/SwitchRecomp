# Automated Recompilation Loop

This document describes the implemented local-first automation loop for static recompilation.
The loop is exposed through `recomp automate --config <automation.toml>`.

## Loop Overview
1. Validate inputs, config, and provenance.
2. Intake (homebrew or XCI) when required.
3. Analysis (optional) for headless export and evidence artifacts.
4. Lift (homebrew decoder or external lift command for XCI).
5. Run pipeline + build + runtime/capture commands.
6. Hash capture outputs and run validation.
7. Emit and finalize `run-manifest.json` plus `validation-report.json`.

## Core Inputs
- `automation.toml`
- `reference_video.toml`
- `capture_video.toml`
- optional `validation_config.toml`
- optional `input_script.toml`

## Outputs
- stage artifacts under configured work roots
- per-step logs under `logs/`
- `run-manifest.json`
- `validation-report.json`

## Asset Separation
All proprietary assets (RomFS, reference media, capture outputs, keys) must remain outside repo-tracked paths.
Only metadata, hashes, and non-proprietary configs should be committed.

## Automation Config Sections
Start from `samples/automation.toml`.

- `schema_version`
- `[inputs]`: mode (`homebrew`, `xci`, `lifted`) and required input paths.
- `[outputs]`: work root and optional stage output overrides.
- `[reference]`: reference/capture/validation/input replay config paths.
- `[capture]`: capture output paths used for hashing and validation.
- `[commands]`: build/run/capture/extract commands (plus optional `lift` for XCI mode).
- `[analysis]` (optional): analysis command, expected outputs, and optional `name_map_json`/trace manifest.
- `[policy]` (optional): execution mode and governance metadata (`requires_approval`, cost/runtime bounds, redaction profile, allowed models, run windows).
- `[run]` (optional): resume behavior and homebrew lift settings.

## Run Manifest Notes
`run-manifest.json` records:
- run metadata (`run_id`, `execution_mode`, host fingerprint, tool versions)
- deterministic input fingerprint
- per-step status, stage attempt, cache-hit state, and cache key
- artifact hashes and sizes

## Invocation
```bash
cargo run -p recomp-cli -- automate --config samples/automation.toml
```

## title-a24b9e807b456252 Validation Inputs
For retail pilot runs, gather external artifacts listed in
`docs/title-a24b9e807b456252-validation-prereqs.md` and use absolute local paths in automation config.
