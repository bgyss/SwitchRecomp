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
- `automation.toml` (planned config schema).
- `reference_video.toml` and `capture_video.toml`.
- `input_script.toml` for deterministic input replay.

## Outputs
- Build artifacts under `out/<title>/`.
- Capture artifacts under `artifacts/<title>/capture/`.
- Validation artifacts under `artifacts/<title>/validation/`.
- `run-manifest.json` for per-step timing, hashes, and provenance.

## Asset Separation
All assets (RomFS, reference video, capture output) remain outside the repo. Only hashes and
metadata should be committed.

## Next Steps
- Implement the automation orchestrator (SPEC-210).
- Add input replay (SPEC-220).
- Normalize reference media (SPEC-230).
