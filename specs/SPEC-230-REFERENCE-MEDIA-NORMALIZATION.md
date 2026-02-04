# SPEC-230: Reference Media Normalization

## Status
Draft v0.2

## Rationale
- Documented the canonical profile and normalization workflow.
- Added a normalization script plus sample reference metadata.
- Added tests that validate frame/audio hash stability.

## Purpose
Define how reference videos and audio are normalized into comparable artifacts for validation.

## Goals
- Normalize reference media into a canonical resolution, frame rate, and audio format.
- Record normalization metadata alongside reference timeline data.
- Ensure deterministic hash generation for frame and audio comparisons.

## Non-Goals
- Storing copyrighted reference video files in the repo.
- Pixel-perfect matching against compressed sources.

## Background
Reference videos may come from disparate sources (e.g., YouTube). Normalization ensures that comparisons are stable and that drift detection is meaningful.

## Requirements
- Define a canonical media profile:
  - resolution (e.g., 1280x720)
  - frame rate (e.g., 30 fps)
  - audio sample rate (e.g., 48 kHz, PCM)
- Provide a normalization pipeline that:
  - trims to the first-level timeline
  - exports normalized frames and audio
  - records the normalization command and source metadata
- Store normalization metadata in `reference_video.toml`.
- Keep reference media outside the repo; only hashes and metadata are stored.

## Interfaces and Data
- `reference_video.toml`:
  - source path, normalized path
  - canonical profile (width/height/fps/sample rate)
  - timeline start/end and markers
  - hash sources for frames and audio

## Deliverables
- Normalization script or documented command sequence.
- Reference media metadata schema updates.
- Tests for hash generation stability on normalized assets.

## Open Questions
- Should normalization include color space conversion metadata?
- How to handle variable frame rate sources?

## Acceptance Criteria
- A reference clip can be normalized to the canonical profile.
- Hashes for the normalized clip are stable across two runs.
- Timeline markers align to normalized frames deterministically.

## Risks
- Source compression artifacts may reduce similarity metrics.
- Variable frame rate sources can introduce drift.

## References
- SPEC-190 Video-Based Validation
- SPEC-210 Automated Recompilation Loop
