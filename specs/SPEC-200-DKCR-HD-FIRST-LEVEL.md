# SPEC-200: DKCR HD First-Level Milestone (macOS/aarch64)

## Status
Draft v0.2

## Rationale
- Added non-proprietary scaffolding for title config, patch placeholders, and run instructions.

## Purpose
Define the first title milestone for the DKCR HD XCI on macOS/aarch64, using video-based validation to confirm the first level is playable.

## Goals
- Produce a macOS/aarch64 static recompilation that boots and reaches the first level.
- Validate the run against the reference gameplay video for timing, visuals, and audio.
- Keep all proprietary assets external and user-supplied.

## Non-Goals
- Completing or validating the full game.
- Achieving exact performance parity with Switch hardware.
- Distributing assets, keys, or copyrighted footage.

## Background
- A retail XCI and a long-form gameplay reference video are available locally.
- The project already defines pipeline, runtime, and validation scaffolding for homebrew.
- This milestone focuses on proving the retail-title path works end to end.

## Requirements
- Intake must extract Program NCA, ExeFS, and NSO segments from the XCI using user keys.
- Recompiled output must build on macOS/aarch64 and link against the runtime ABI.
- Runtime must implement enough OS services, GPU translation, audio, and input to reach the first level.
- RomFS assets must be loaded from an external, user-managed path.
- Validation must compare the first level segment against the reference video and record results.

## Interfaces and Data
- `title.toml` for DKCR HD configuration (stubbed services, patches, asset paths).
- `provenance.toml` for XCI and reference video inputs.
- `validation-report.json` for the first-level comparison results.
- `docs/dkcr-hd-first-level.md` for the scaffolding walkthrough.

## Deliverables
- Title-specific configuration and patch set placeholders to reach the first playable level.
- A reproducible build and run command for macOS/aarch64.
- A validation report demonstrating the first-level gate.

## Scaffolding
- `samples/dkcr-hd/title.toml` with external asset and key paths.
- `samples/dkcr-hd/provenance.toml` with placeholder inputs and metadata.
- `samples/dkcr-hd/patches/patches.toml` with patch placeholders.
- `samples/dkcr-hd/module.json` placeholder for lifted output wiring.

## Open Questions
- Which services are the minimum set required to reach the first level?
- What frame rate and resolution should be treated as the baseline for comparison?
- Which shader or GPU features are required for the first level?

## Acceptance Criteria
- The macOS/aarch64 build boots and reaches the first playable level.
- The first-level run is playable and passes the video-based validation threshold.
- No proprietary assets are committed to the repo or build outputs.

## Risks
- GPU translation gaps may block rendering.
- Missing OS services may cause boot failure.
- Validation thresholds may need tuning for frame pacing variance.

## References
- TBD
