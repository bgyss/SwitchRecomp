# Input Replay Notes

Input replay is required to align validation runs with a reference video that includes
player interactions. This document summarizes the expected workflow and artifacts.

## Workflow
1. Author or record an `input_script.toml`.
2. Run the rebuilt binary with the input replay enabled.
3. Capture video/audio and validate against the reference timeline.

## Input Script (Planned)
- `schema_version`
- `metadata` (controller profile, timing mode)
- `events` with timestamps or frame indices
- `markers` for alignment

## Alignment Tips
- Keep a deterministic start point (boot marker).
- Align the first interaction with a visible cue in the reference video.
- Use markers to resync at key events.

## Notes
- Inputs remain external; only hashes and metadata are stored in the repo.
- Deterministic replay is required for stable validation.
