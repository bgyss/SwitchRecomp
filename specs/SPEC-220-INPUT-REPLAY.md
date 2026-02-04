# SPEC-220: Input Replay and Interaction Scripts

## Status
Draft v0.2

## Rationale
- Added an input script parser/validator and deterministic playback queue in the runtime.
- Added sample input script data plus docs to align with reference timelines.
- Added unit tests for ordering and marker alignment.

## Purpose
Define a deterministic input replay format and runtime integration so validation runs can mirror reference video interactions.

## Goals
- Record or author input scripts that can be replayed deterministically.
- Support time-based and frame-based event scheduling.
- Keep input data separate from proprietary assets.

## Non-Goals
- Full fidelity controller emulation for all hardware variants.
- Automated extraction of inputs from videos.

## Background
Reference videos include user interactions. To compare recompiled output to the reference, we need repeatable input playback that can be aligned to the reference timeline.

## Requirements
- Define a versioned input script format with:
  - metadata (title, controller profile, timing mode)
  - ordered input events with timestamps or frame indices
  - optional markers for timeline alignment
- Support common input types:
  - button press/release
  - analog axis values
  - system/menu button events (optional)
- Provide deterministic playback in the runtime:
  - stable ordering for simultaneous events
  - configurable timing base (ms or frame index)
  - ability to pause, fast-forward, or rewind for debugging
- Emit a replay log for validation and debugging.

## Interfaces and Data
- `input_script.toml`:
  - `schema_version`
  - `[metadata]` title, controller, timing_mode
  - `[[events]]` time or frame, control, value
  - `[[markers]]` name, time/frame
- Runtime integration:
  - input script loader
  - playback queue feeding the runtime input backend

## Deliverables
- Input script parser and validator.
- Runtime playback module that feeds input events deterministically.
- Tests that confirm repeatable playback and alignment.

## Open Questions
- Should input scripts support multiple controller sources?
- How to express analog deadzones and smoothing?

## Acceptance Criteria
- A sample input script replays deterministically across two runs.
- Playback order is stable for simultaneous events.
- Markers can be aligned to reference video timecodes.

## Risks
- Input timing drift can skew validation results.
- Games with dynamic input latency may require per-title tuning.

## References
- SPEC-190 Video-Based Validation
- SPEC-210 Automated Recompilation Loop
