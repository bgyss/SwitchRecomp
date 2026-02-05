# Validation Baseline Suite

This document defines the baseline validation suite and thresholds for correctness and stability.

## Baseline Cases
- `runtime_config_defaults`: Runtime config defaults to handheld mode.
- `pipeline_minimal_sample`: Minimal sample pipeline emits expected artifacts and detects inputs.

## Video Validation
Video-based validation is invoked separately via `recomp-validation video` and adds a `video`
summary to `validation-report.json`. The video summary includes hash match ratios, drift
statistics, and triage notes.

For runs that stitch together intake outputs, pipeline outputs, and captures, use the
`artifacts` command with an artifact index JSON (see `docs/validation-artifacts.md`).

## Thresholds
- All baseline cases must pass (0 failures).
- Reports must be generated on every run (JSON + text).

## Output
- `validation-report.json`: structured regression summary.
- `validation-report.txt`: human-readable summary.
