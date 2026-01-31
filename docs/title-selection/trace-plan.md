# Private Trace Collection Plan

## Goal
Collect baseline traces from a legally obtained title without distributing proprietary assets or raw traces.

## Acquisition and Setup
- Acquire the target title through lawful means.
- Record tool versions used for extraction and tracing in `provenance.toml`.
- Store raw traces locally only; do not commit them.

## Trace Collection
- Capture startup, menu, and initial gameplay loop segments.
- Record service calls, timing events, and graphics command summaries.
- Store only non-proprietary summaries in the repo (hashes, counts, timing stats).

## Validation Outputs
- Maintain a compatibility summary in `docs/validation-baseline.md` when trace summaries are ready.
- Document any gaps in service coverage or instruction support.
