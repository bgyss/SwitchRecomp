# Validation Baseline Suite

This document defines the baseline validation suite and thresholds for correctness and stability.

## Baseline Cases
- `runtime_config_defaults`: Runtime config defaults to handheld mode.
- `pipeline_minimal_sample`: Minimal sample pipeline emits expected artifacts and detects inputs.

## Video Validation
Video-based validation is a separate workflow that compares reference and capture hashes. See `docs/validation-video.md` for configuration, hashing, and report details.

## Thresholds
- All baseline cases must pass (0 failures).
- Reports must be generated on every run (JSON + text).

## Output
- `validation-report.json`: structured regression summary.
- `validation-report.txt`: human-readable summary.
