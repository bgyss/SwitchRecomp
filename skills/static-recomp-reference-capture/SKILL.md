---
name: static-recomp-reference-capture
description: Collect and normalize reference video/audio and metadata for validating static recompilation outputs. Use when creating capture pipelines, curating reference footage, or standardizing A/V inputs for comparison.
---

# Static Recomp Reference Capture

## Overview
Gather clean, legally obtained reference captures and normalize them into a consistent format for automated comparison.

## Workflow
1. Confirm legal capture sources.
   - Prefer user-provided hardware captures or authorized recordings.
   - If using emulator footage, record emulator version and settings.
   - Do not ingest proprietary binaries or keys.
2. Standardize capture settings.
   - Lock resolution, aspect ratio, and frame rate.
   - Disable dynamic resolution and variable frame pacing when possible.
   - Set audio sample rate (example: 48 kHz) and channel layout.
3. Capture anchor scenes.
   - Identify scenes that stress rendering, audio, UI, and gameplay.
   - Capture from boot to first interactive state.
   - Include a repeatable gameplay loop segment.
4. Normalize formats.
   - Re-encode to a common container and codec.
   - Trim to exact segments with timestamps.
   - Preserve original captures as raw archives.
5. Produce metadata.
   - Capture start time, duration, settings, and source details.
   - Record any known differences (patches, mods, settings changes).
6. Validate capture quality.
   - Check for dropped frames and audio desync.
   - Ensure overlays, watermarks, or UI from capture tools are absent.

## Outputs
- Normalized reference captures (video and audio).
- A metadata file per capture with settings and source details.
- A scene list with timestamps for automated comparison.

## Quality bar
- Reference captures must be stable, reproducible, and legally obtained.
- Metadata must be sufficient to reproduce the capture.
- Segment selection must cover core rendering, audio, and gameplay behaviors.
