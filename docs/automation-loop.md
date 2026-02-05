# Automated Recompilation Loop

This document describes the intended automation loop for static recompilation. The goal is a
single command that runs intake, build, capture, and validation without copying proprietary
assets into the repo.

## Loop Overview
1. Validate provenance and input formats.
2. Intake (XCI or homebrew) and lift to `module.json`.
3. Build the emitted Rust project.
4. Run the rebuilt binary with deterministic runtime settings.
5. Capture video/audio output to an external artifact root.
6. Generate hashes and run validation.
7. Emit `run-manifest.json` and `validation-report.json`.

## Core Inputs
- `automation.toml` (config schema implemented in `recomp automate`).
- `reference_video.toml` and `capture_video.toml`.
- `input_script.toml` for deterministic input replay.

## Outputs
- Build artifacts under `out/<title>/`.
- Capture artifacts under `artifacts/<title>/capture/`.
- Validation artifacts under `artifacts/<title>/validation/`.
- `run-manifest.json` for per-step timing, hashes, and provenance.
- `artifacts.json` to link intake manifests, capture configs, and validation reports.

## Asset Separation
All assets (RomFS, reference video, capture output) remain outside the repo. Only hashes and
metadata should be committed.

## Automation Config
`automation.toml` defines inputs, outputs, capture paths, and commands. Start from
`samples/automation.toml` and update the paths for your environment. Key sections:
- `schema_version`
- `[inputs]` mode (`homebrew`, `xci`, `lifted`), provenance, title config, and inputs.
- `[outputs]` work root and optional overrides for intake/lift/build dirs.
- `[reference]` reference/capture video config paths (plus optional validation config).
- `[capture]` capture video path and extracted frames/audio locations.
- `[commands]` build/run/capture/extract commands (plus optional lift command for XCI).
- `[run]` resume and lift settings (optional).

Invoke the loop with:
```bash
recomp automate --config automation.toml
```

## DKCR Validation Inputs
The DKCR validation run requires external reference and capture artifacts. Track the required
paths and timecodes in `docs/dkcr-validation-prereqs.md` before wiring a DKCR-specific
`automation.toml`.

## Next Steps
- Iterate on capture automation and tighten determinism for external tools.
