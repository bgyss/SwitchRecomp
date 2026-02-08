# Automated Recompilation Loop

This document describes the implemented automation loop for static recompilation. The goal is a
single command that runs intake, build, capture, and validation with bounded retry support and
without copying proprietary assets into the repo.

## Loop Overview
1. Validate provenance and input formats.
2. Intake (XCI or homebrew) and lift to `module.json`.
3. Build the emitted Rust project.
4. Run the rebuilt binary with deterministic runtime settings.
5. Capture video/audio output to an external artifact root.
6. Generate hashes and run validation.
7. Evaluate hash and perceptual gates, emit triage, and apply bounded strategy retries.
8. Emit `run-manifest.json`, `run-summary.json`, and per-attempt manifests.

## Core Inputs
- `automation.toml` (config schema implemented in `recomp automate`).
- `reference_video.toml` and `capture_video.toml`.
- `input_script.toml` for deterministic input replay.
- Optional `strategy-catalog.toml` for strategy enable/disable policy.

## Outputs
- Build artifacts under `out/<title>/`.
- Capture artifacts under `artifacts/<title>/capture/`.
- Validation artifacts under `artifacts/<title>/validation/`.
- `run-manifest.json` for per-step timing, hashes, attempts, and final status.
- `run-summary.json` for final run outcome and timing.
- `attempts/<NNN>/attempt-manifest.json` per-attempt step and gate details.
- `attempts/<NNN>/gate-results.json` hash/perceptual gate results.
- `attempts/<NNN>/triage.json` retry classification and suggested next strategy.
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
- `[loop]` retry budget, runtime budget, strategy order, and catalog path.
- `[gates.hash]` optional hard-gate overrides.
- `[gates.perceptual]` perceptual thresholds (SSIM/PSNR/VMAF/LUFS/peak).
- `[agent]` model policy metadata (for governance and future gateway integration).
- `[cloud]` local vs aws_hybrid mode metadata.
- `[[scenes]]` weighted scene windows for perceptual validation.

Invoke the loop with:
```bash
recomp automate --config automation.toml
```

Dev invocation:
```bash
cargo run -p recomp-cli -- automate --config samples/automation.toml
```

## title-a24b9e807b456252 Validation Inputs
The title-a24b9e807b456252 validation run requires external reference and capture artifacts. Track the required
paths and timecodes in `docs/title-a24b9e807b456252-validation-prereqs.md` before wiring a title-a24b9e807b456252-specific
`automation.toml`.

## Notes
- Ghidra headless evidence export is optional via `[tools.ghidra]`.
- Perceptual validation requires `python3` and `ffmpeg` (see skill scripts under
  `skills/static-recomp-av-compare/scripts/`).
