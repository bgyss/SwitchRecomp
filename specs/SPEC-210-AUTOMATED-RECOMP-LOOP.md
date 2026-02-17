# SPEC-210: Automated Recompilation Loop

## Status
Draft v0.3

## Rationale
- Added an automation.toml schema and validator for end-to-end runs.
- Added a CLI orchestrator that drives intake, lift, build, capture, hash, and validation steps.
- Added deterministic run-manifest emission with artifact hashes and step summaries.
- Added long-tail orchestration requirements: similarity-guided candidate ordering, specialist lanes, and stricter unattended guardrails.

## Purpose
Define an automated loop that drives intake, recompilation, execution, capture, and validation in a repeatable pipeline.

## Goals
- Provide a single entry point that runs the full static recompilation loop.
- Generate deterministic artifacts and a run manifest for every attempt.
- Support incremental iteration while keeping proprietary assets external.

## Non-Goals
- Fully automated legal intake of retail assets.
- Replacing human review of subjective rendering issues.

## Background
Validation depends on comparing a captured run against a reference video with user inputs. The project needs a stable automation loop so iteration is fast and reproducible while asset separation stays intact.

## Requirements
- The loop must accept a config that points to:
  - input artifacts (XCI, keyset, module.json, etc.)
  - output roots (build, capture, validation)
  - reference timeline and input script paths
  - toolchain paths (hactool/hactoolnet, ffmpeg)
- The loop must:
  - validate provenance and input formats before running
  - run intake/lift/build steps and capture stdout/stderr per step
  - execute the rebuilt binary with a deterministic runtime config
  - capture video/audio output into an external artifact root
  - generate frame/audio hashes and run validation
  - emit a run manifest with step timings and artifact paths
- The loop must support deterministic long-tail candidate ordering for unresolved functions:
  - Compute and store similarity references to matched functions (for example opcode-sequence distance and/or embedding neighbors).
  - Record the selected candidate inputs in `run-manifest.json` so retries are reproducible.
- The loop must support specialist task lanes (`general`, `gfx`, `math`, `cleanup`) with explicit lane selection recorded per attempt.
- The loop must enforce hard unattended guardrails:
  - stop on first failed build/test/hook step
  - block skipped-test commits and integrity-sentinel edits
  - block edits to generated files outside approved generation commands
- The loop must emit long-tail metrics for triage (`attempt_count`, percentile summaries, and `stall_reason` tags).
- The loop must allow resuming from intermediate stages when inputs are unchanged.
- The loop must never copy proprietary assets into the repo or build outputs.

## Interfaces and Data
- `automation.toml` (example fields):
  - `[inputs]` paths for XCI, keyset, module.json, provenance.
  - `[outputs]` build_root, capture_root, validation_root.
  - `[tools]` hactool_path, ffmpeg_path.
  - `[reference]` reference_video_toml, input_script_toml.
  - `[run]` command overrides for build/run/capture.
- Output:
  - `run-manifest.json` (step results, hashes, timings)
  - `run-manifest.json.long_tail`:
    - candidate selection trace
    - task lane
    - attempt count
    - stall reason (if any)
  - `validation-report.json` from the validation step

## Deliverables
- Automation config schema and validator.
- Orchestrator CLI command (or script) that runs the full loop.
- Run manifest format with deterministic ordering.

## Open Questions
- How should caching be keyed (full input hash, partial stage hash)?
- How should partial failures be recorded for rerun?
- What blend of similarity signals should be default for candidate ranking?

## Acceptance Criteria
- A single command runs intake, build, capture, and validation in sequence.
- The run manifest lists all artifacts with hashes and sizes.
- Re-running with identical inputs yields identical artifacts and validation results.
- Long-tail metadata is emitted deterministically for the same inputs and candidate pool.

## Risks
- External tool versions can break determinism.
- Capture timing jitter can cause false validation failures.
- Poor similarity anchors can amplify brittle code patterns if cleanup lanes lag behind.

## References
- SPEC-180 XCI Intake
- SPEC-190 Video-Based Validation
- SPEC-220 Input Replay
