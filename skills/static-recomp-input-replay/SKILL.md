---
name: static-recomp-input-replay
description: Design deterministic input capture and replay harnesses for static recompilation validation. Use when recording controller input traces, building replay systems, or aligning emulator and recompiled runs.
---

# Static Recomp Input Replay

## Overview
Record and replay deterministic input traces so emulator and recompiled builds can be compared using the same stimuli.

## Workflow
1. Define the input trace format.
   - Use time-ordered events with timestamps or frame indices.
   - Record device type, buttons, axes, and analog ranges.
2. Select a stable time base.
   - Prefer frame-indexed ticks if frame rate is locked.
   - Otherwise record high-resolution timestamps with explicit units.
3. Capture input traces.
   - Record from a controlled session with known settings.
   - Save seed values for RNG and time sources when possible.
4. Normalize and validate traces.
   - Ensure monotonic time and no dropped events.
   - Quantize to fixed tick rate if needed.
5. Build a replay harness.
   - Inject inputs into emulator and recompiled runtime.
   - Log applied inputs and any rejected events.
6. Validate determinism.
   - Replay the same trace twice and compare hashes or state summaries.
   - Flag nondeterministic outcomes as blocking issues.

## Outputs
- Input trace files with stable time bases.
- A replay harness config describing injection method and target build.
- Determinism logs and hashes per run.

## Quality bar
- Input traces must be replayable without manual intervention.
- Determinism must be tested and documented for each target build.
- Time bases and units must be explicit and consistent.
