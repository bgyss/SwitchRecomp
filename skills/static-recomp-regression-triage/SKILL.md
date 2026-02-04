---
name: static-recomp-regression-triage
description: Triage visual, audio, input, and performance regressions in static recompilation results by mapping symptoms to pipeline components and proposing next debugging steps. Use when investigation or root-cause analysis is needed.
---

# Static Recomp Regression Triage

## Overview
Classify regressions and map them to likely pipeline components so fixes can be prioritized quickly.

## Workflow
1. Classify the failure.
   - Crash, hang, or boot failure.
   - Visual mismatch or missing draw calls.
   - Audio artifacts, drift, or missing channels.
   - Input latency or incorrect mapping.
   - Performance regression or stutter.
2. Narrow to the smallest reproducible scene.
   - Use the existing input trace and scene list.
   - Trim captures to the shortest failing segment.
3. Map to pipeline components.
   - Loader and relocation issues.
   - CPU instruction semantics or ABI mismatches.
   - OS or service stubs.
   - GPU command translation and shader issues.
   - Audio mixing and timing.
4. Collect targeted evidence.
   - Logs, trace snippets, and minimal repro data.
   - Side-by-side captures and metric deltas.
5. Propose next steps.
   - Candidate instrumentation to add.
   - Specific unit tests or microbenchmarks to create.
   - Suggested code areas to inspect.

## Outputs
- A triage report with a root-cause hypothesis.
- A prioritized fix list with evidence links.
- A minimal reproduction recipe.

## Quality bar
- Each hypothesis must be backed by concrete evidence.
- Proposed fixes must be testable with the same validation harness.
- The report must enable another engineer to reproduce the issue quickly.
