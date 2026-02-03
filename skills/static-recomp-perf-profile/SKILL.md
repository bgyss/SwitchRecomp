---
name: static-recomp-perf-profile
description: Profile and compare performance of statically recompiled builds against reference runs, including frame time, CPU/GPU utilization, memory, and load times. Use when measuring performance regressions or tuning runtime behavior.
---

# Static Recomp Performance Profile

## Overview
Measure performance with repeatable inputs and produce regression reports that map to specific subsystems.

## Workflow
1. Define performance targets.
   - Frame time budget and variance.
   - Load times for boot and scene transitions.
   - CPU/GPU utilization ranges.
2. Use deterministic inputs.
   - Reuse input traces for comparable runs.
   - Ensure identical settings and hardware.
3. Capture metrics.
   - Collect frame times and present 1% low, 0.1% low, and average.
   - Record CPU, GPU, and memory usage.
   - Track shader compilation stutter or asset streaming spikes.
4. Compare against baseline.
   - Use the same scene list as A/V validation.
   - Flag regressions above threshold.
5. Attribute causes.
   - Map spikes to pipeline stages (CPU translation, GPU command handling, audio mixing).
   - Collect logs or flamegraphs if needed.

## Outputs
- A performance report with per-scene metrics.
- A regression summary with thresholds and suspected causes.
- A list of profiles or traces attached to the report.

## Quality bar
- Results must be gathered under consistent settings and hardware.
- Each regression must include scene context and a suspected subsystem.
- Performance changes must be reproducible with the same input trace.
