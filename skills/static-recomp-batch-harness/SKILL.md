---
name: static-recomp-batch-harness
description: Design batch execution harnesses for static recompilation across many titles, including run manifests, artifact organization, and automated pass/fail gates. Use when building or improving large-scale static recompilation pipelines or catalog-wide validation.
---

# Static Recomp Batch Harness

## Overview
Scale static recompilation across many titles by standardizing inputs, outputs, and validation gates.

## Workflow
1. Build a run manifest.
   - Track title, version, region, input availability, and status.
   - Record required hardware, emulator versions, and runtime assumptions.
   - Use `references/manifest-schema.md` as the canonical schema.
2. Define the standard pipeline per title.
   - Intake and validate inputs.
   - Build or recompile.
   - Run validation captures.
   - Compare against references.
   - Record summary metrics and artifacts.
3. Set hard gates for automation.
   - Fail fast on missing inputs or build failures.
   - Mark soft failures for visual/audio mismatches that need review.
   - Encode thresholds for performance regressions.
4. Normalize artifact layout.
   - Use stable, predictable paths for each title and run.
   - Keep raw captures, derived metrics, and reports separate.
   - Store metadata next to artifacts for reproducibility.
   - See `references/pipeline-layout.md` for a sample directory layout.
5. Enable safe parallelism.
   - Bound CPU/GPU usage and I/O.
   - Ensure per-title isolation to avoid data collisions.
   - Use deterministic seed values for replayable tests.
6. Provide human review hooks.
   - Generate a compact report with top mismatches and links to artifacts.
   - Allow manual override or acceptance for known deltas.

## Outputs
- A run manifest with status tracking.
- A consistent artifact directory schema.
- A summary report per batch with pass/fail and top regressions.

## References
- Manifest schema: `references/manifest-schema.md`
- Sample pipeline layout: `references/pipeline-layout.md`

## Quality bar
- Batch runs must be reproducible with stable inputs and settings.
- Failures must be classified with enough context to triage quickly.
- The harness should avoid requiring proprietary assets beyond user-provided inputs.
