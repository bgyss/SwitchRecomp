# DKCR Validation Prerequisites

This document captures the external inputs required to run DKCR HD video validation. These
artifacts are not stored in the repo and must be supplied locally for each run.

## Required Inputs
- Absolute path to the reference video file, or absolute paths to precomputed reference frame
  and audio hash lists.
- Absolute path to the capture video file, or absolute paths to precomputed capture frame
  and audio hash lists.
- Confirmed first-level start and end timecodes for the reference timeline.

## Hash List Paths (If Precomputed)
- Reference frames hash list path (absolute).
- Reference audio hash list path (absolute).
- Capture frames hash list path (absolute).
- Capture audio hash list path (absolute).

## Timeline Confirmation
Provide the exact first-level start and end timecodes in HH:MM:SS.mmm or seconds format. If the
existing timeline in `samples/reference_video.toml` is correct, explicitly confirm it.

## Optional Inputs
- Input replay script path (absolute) if you want deterministic input playback.
- Capture device settings (resolution, fps) used during recording.

## Once Provided
- Update `samples/reference_video.toml` with the absolute reference video path and timeline.
- Update `samples/capture_video.toml` with the absolute capture video path.
- Run the validation command described in `docs/validation-video.md`.
- Review `validation-report.json` and capture any triage notes.
