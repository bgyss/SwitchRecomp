# SPEC-190: Video-Based Validation

## Status
Draft v0.2

## Purpose
Define a validation workflow that compares recompiled output against a reference gameplay video when no instrumented emulator is available.

## Goals
- Provide a repeatable comparison method using video and audio captures.
- Establish a first-level milestone gate based on reference footage.
- Produce a machine-readable report for regressions.

## Non-Goals
- Pixel-perfect or audio-sample-perfect matching.
- Replacing manual playtesting for qualitative issues.
- Distributing copyrighted reference footage.

## Background
- The project does not currently have access to instrumented Switch emulation traces.
- A long-form gameplay video is available as a behavioral reference.
- Video comparison can detect major regressions in rendering, timing, and audio.

## Requirements
- A reference timeline must define the first-level start and completion timecodes.
- Validation must capture native macOS output at a stable resolution and frame rate.
- A comparison step must compute frame and audio similarity using deterministic hash lists.
- The report must highlight drift, dropped frames, or audio desync beyond thresholds.
- Validation artifacts must remain outside the repo and be referenced via provenance metadata.
- Validation runs should be tracked with an external artifact index that records intake manifests,
  capture paths, and report outputs.

## Operator Inputs
- External reference and capture artifacts are required to run DKCR validation.
- Absolute paths and timeline confirmations are tracked in `docs/dkcr-validation-prereqs.md`.

## Interfaces and Data
- Provenance metadata should record the reference capture as an input:
  - `format = "video_mp4"` for MP4 reference videos.
- `reference_video.toml` with:
  - `schema_version` (optional)
  - `[video]` metadata (width, height, fps)
  - `[timeline]` start/end timecodes
  - `[hashes]` frame/audio hash sources
  - optional `[validation]` thresholds and requirements
- `capture_video.toml` with:
  - `[video]` metadata
  - `[hashes]` sources for the captured run
- `validation_config.toml` (optional) for per-run overrides.
- `validation-report.json` with:
  - match ratios, drift summaries, triage categories, and pass/fail summary

## Deliverables
- A capture script that records the recompiled runtime output.
- A comparison flow that generates a validation report from reference and capture hash lists.
- A validation CLI flow that compares hash lists and emits a report.
- Documentation describing the expected workflow and thresholds.
- An artifact index template and helper scripts for external validation runs.

## Open Questions
- What similarity thresholds best indicate a playable first level?
- How should camera motion or cutscenes be treated in the comparison?
- Which input playback method yields the most deterministic run?

## Acceptance Criteria
- A reference timeline for the first level is defined and versioned.
- A single run produces a validation report with explicit pass/fail for the first level.
- Reported metrics are stable within tolerance across two consecutive runs.
- Drift summaries are recorded for each reference event when observed timecodes are supplied.

## Risks
- Frame pacing differences may cause false mismatches.
- Audio compression or capture devices may skew similarity metrics.

## References
- `docs/validation-artifacts.md`
- `docs/validation-traces.md`
- `docs/validation-video.md`
