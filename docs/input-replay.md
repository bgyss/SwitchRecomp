# Input Replay Notes

Input replay is required to align validation runs with a reference video that includes
player interactions. This document summarizes the expected workflow and artifacts.

## Workflow
1. Author or record an `input_script.toml`.
2. Run the rebuilt binary with the input replay enabled.
3. Capture video/audio and validate against the reference timeline.

## Input Script Schema
`input_script.toml` is a versioned, deterministic script describing input events and alignment markers.
All timestamps are relative to replay start (time zero).

Top-level fields:
- `schema_version` (string, currently `"1"`).
- `[metadata]` (required).
- `[[events]]` (required, ordered list; order is preserved for same timestamp).
- `[[markers]]` (optional, ordered list).

`[metadata]` fields:
- `title` (string, required).
- `controller` (string, required; descriptive profile name).
- `timing_mode` (string, required; `"ms"` or `"frames"`).
- `recorded_at` (string, optional; ISO 8601).
- `notes` (string, optional).

`[[events]]` fields:
- `time_ms` (u64, required when `timing_mode = "ms"`).
- `frame` (u64, required when `timing_mode = "frames"`).
- `control` (u32, required; runtime input code).
- `value` (i32, required; button/axis value).
- `note` (string, optional).

`[[markers]]` fields:
- `name` (string, required; unique).
- `time_ms` (u64, required when `timing_mode = "ms"`).
- `frame` (u64, required when `timing_mode = "frames"`).
- `note` (string, optional).

Example:
```toml
schema_version = "1"

[metadata]
title = "Sample Replay"
controller = "pro_controller"
timing_mode = "ms"

[[events]]
time_ms = 0
control = 100
value = 1

[[markers]]
name = "boot"
time_ms = 0
```

Parser rules:
- `schema_version` must match the runtime's supported version.
- `metadata` must include `title`, `controller`, and `timing_mode`.
- `events` must be non-empty and use the time field for the selected `timing_mode`.
- `markers` must have unique names and use the same timing base as events.

## Playback Integration
- Load and validate the script before boot.
- Build a deterministic playback queue and feed the runtime input backend as time advances.
- For identical timestamps, playback preserves script order.
- Marker ordering is stable for identical timestamps.

## Alignment Tips
- Keep a deterministic start point (boot marker).
- Align the first interaction with a visible cue in the reference video.
- Use markers to resync at key events.

## Notes
- Inputs remain external; only hashes and metadata are stored in the repo.
- Deterministic replay is required for stable validation.
