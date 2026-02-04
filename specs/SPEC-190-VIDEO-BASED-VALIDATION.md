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
- A comparison step must compute frame similarity (perceptual hash or SSIM) and audio similarity.
- The report must highlight drift, dropped frames, or audio desync beyond thresholds.
- Validation artifacts must remain outside the repo and be referenced via provenance metadata.
- Validation must accept precomputed `summary.json` outputs or invoke the comparison scripts directly.

## Operator Inputs
- External reference and capture artifacts are required to run DKCR validation.
- Absolute paths and timeline confirmations are tracked in `docs/dkcr-validation-prereqs.md`.

## Interfaces and Data
- Provenance metadata should record the reference capture as an input:
  - `format = "video_mp4"` for MP4 reference videos.
- `reference_video.toml` with:
  - `schema_version = "v1"`
  - `label`
  - `reference_video` path (absolute or relative)
  - `[expected]` video settings (width, height, fps, audio_rate)
  - `[comparison]` settings (offset_seconds, trim_start_seconds, duration_seconds, no_vmaf)
  - `[thresholds]` overrides (ssim_min, psnr_min, vmaf_min, audio_lufs_delta_max, audio_peak_delta_max, event_drift_max_seconds)
  - `[[events]]` with id, label, and timecode
- `event_observations.json` (optional) with observed timecodes for drift calculations.
- `validation-report.json` with:
  - similarity metric checks and thresholds
  - timecode drift summaries (per-event + aggregates)
  - pass/fail summary

## Deliverables
- A capture script that records the recompiled runtime output.
- A comparison script that generates a validation report from reference and capture.
- A validation CLI flow that runs the comparison scripts or parses a precomputed summary.
- Documentation describing the expected workflow and thresholds.

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
- TBD
