# Validation Baseline Suite

This document defines the baseline validation suite and thresholds for correctness and stability.

## Baseline Cases
- `runtime_config_defaults`: Runtime config defaults to handheld mode.
- `pipeline_minimal_sample`: Minimal sample pipeline emits expected artifacts and detects inputs.

## Thresholds
- All baseline cases must pass (0 failures).
- Reports must be generated on every run (JSON + text).

## Output
- `validation-report.json`: structured regression summary.
- `validation-report.txt`: human-readable summary.
