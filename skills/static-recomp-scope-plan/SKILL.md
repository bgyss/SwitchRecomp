---
name: static-recomp-scope-plan
description: Plan and scope a static recompilation effort with clear legal input boundaries, behavioral targets, and validation acceptance criteria. Use when starting or rebooting a static recompilation project, or when a user asks for a validation plan, scope definition, or success criteria.
---

# Static Recomp Scope Plan

## Overview
Define what "correct" means for a static recompilation and produce a concrete validation plan that can scale across many titles.

## Workflow
1. Confirm legal and preservation boundaries.
   - Require user-provided, legally obtained inputs.
   - Do not request or store proprietary binaries, keys, or assets.
   - Keep outputs and metadata separated from inputs.
2. Identify target build and execution envelope.
   - Capture title, version, region, build ID, and platform assumptions.
   - State target runtime environment (OS, GPU, audio stack, input devices).
   - Set frame rate and resolution targets up front.
3. Define the behavioral surface area.
   - Boot and menu flow.
   - Core gameplay loops and scene transitions.
   - Input handling and response timing.
   - Audio, rendering, and loading behavior.
4. Build a validation matrix.
   - Rows: features or scenes.
   - Columns: video, audio, input, performance, stability.
   - Mark each cell with acceptance criteria and a verification method.
5. Select reference sources.
   - Prefer legal captures from hardware or user-supplied recordings.
   - If using emulator footage, record emulator version and settings.
   - Pick multiple anchor scenes that cover rendering, UI, audio, and gameplay.
6. Decide instrumentation and artifacts.
   - Capture raw video/audio, logs, and performance counters.
   - Ensure time sources are explicit and stable.
   - Plan per-scene capture durations and naming conventions.
7. Define exit criteria.
   - Set numeric thresholds (example: average VMAF >= 90, max audio drift < 20 ms).
   - Require zero crashes and stable progression through anchor scenes.

## Outputs
- A validation plan with acceptance criteria and a scene checklist.
- A reference capture plan describing sources, settings, and formats.
- A per-title run sheet describing artifacts to collect.

## Quality bar
- Criteria must be measurable and repeatable.
- The plan must be runnable without proprietary assets beyond user-provided inputs.
- The plan should be optimized for batch execution across many titles.
