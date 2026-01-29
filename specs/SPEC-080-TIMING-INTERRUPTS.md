# SPEC-080: Timing, Scheduling, and Interrupts

## Status
Draft v0.2

## Purpose
Define timing models and interrupt handling needed for correctness.

## Goals
- Provide deterministic scheduling for CPU, GPU, and IO interactions.
- Model interrupts and timing-sensitive behaviors with precision.

## Non-Goals
- Cycle-accurate hardware emulation for all subsystems.

## Timing Model
- A unified scheduler driving CPU, GPU, and IO events.
- Configurable time steps and event priorities.
- Emulation-style coordination for concurrency where static scheduling is insufficient.

## Interrupts
- A defined set of interrupt sources used by titles.
- Deterministic injection points in the scheduler.

## Deliverables
- A timing subsystem in the runtime.
- Instrumentation for tracing and replay.

## Open Questions
- Which subsystems require strict timing for correct gameplay?
- How to validate timing against reference traces?

## Acceptance Criteria
- Deterministic playback of a trace across two runs.
- Stable frame pacing for a minimal test scene.

## References
- https://andrewkelley.me/post/jamulator.html
