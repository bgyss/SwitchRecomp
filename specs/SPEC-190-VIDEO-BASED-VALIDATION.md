# SPEC-190: Video-Based Validation

## Status
Draft v0.2

## Rationale
- Added reference timeline and capture templates in `samples/`.
- Implemented hash-based video/audio comparison with drift reporting.
- Documented capture and manual review workflow with a macOS capture script.

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
- A comparison step must compute frame similarity (perceptual hash or SSIM) and audio similarity.
- The report must highlight drift, dropped frames, or audio desync beyond thresholds.
- Validation artifacts must remain outside the repo and be referenced via provenance metadata.

## Interfaces and Data
- `reference_video.toml` with:
  - input video path
  - timecodes for key events
  - expected resolution and frame rate
- `validation-report.json` with:
  - similarity metrics
  - timecode drift data
  - pass/fail summary

## Deliverables
- A capture script that records the recompiled runtime output.
- A comparison script that generates a validation report from reference and capture.
- Documentation describing the expected workflow and thresholds.

## Open Questions
- What similarity thresholds best indicate a playable first level?
- How should camera motion or cutscenes be treated in the comparison?
- Which input playback method yields the most deterministic run?

## Acceptance Criteria
- A reference timeline for the first level is defined and versioned.
- A single run produces a validation report with explicit pass/fail for the first level.
- Reported metrics are stable within tolerance across two consecutive runs.

## Risks
- Frame pacing differences may cause false mismatches.
- Audio compression or capture devices may skew similarity metrics.

## References
- TBD
