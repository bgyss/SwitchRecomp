# SPEC-210: Automated Recompilation Loop

## Status
Draft v0.3

## Rationale
- Automation engine internals exist in `recomp-cli` with deterministic stage execution, resume, and run-manifest output.
- This revision closes the former CLI surface gap by standardizing `recomp automate --config ...`.
- Adds optional analysis and policy contracts for local-first rollout.

## Purpose
Define a single-command, deterministic automation loop that drives intake through validation and archive metadata generation.

## Goals
- Provide one CLI entrypoint for intake, lift, build, run/capture, and validation.
- Emit deterministic lifecycle metadata with stage-level cache semantics.
- Support homebrew and retail pilot inputs while preserving strict asset separation.

## Non-Goals
- Fully automatic legal acquisition of retail assets.
- Immediate cloud orchestration enforcement.

## Background
Automation must stay deterministic and resumable while allowing iterative command and config changes without re-running unaffected stages.

## Requirements
- CLI surface:
  - `recomp automate --config <automation.toml>`
- Stage lifecycle:
  1. intake
  2. analysis (optional contract stage)
  3. lift
  4. pipeline
  5. build
  6. run
  7. capture
  8. extract/hash
  9. validate
  10. archive (manifest finalization)
- `automation.toml` supports:
  - existing sections (`inputs`, `outputs`, `reference`, `capture`, `commands`, `tools`, `run`)
  - optional `[analysis]` for headless analysis command + expected outputs
  - optional `[policy]` for execution mode and governance metadata
- Stage caching:
  - cache keys include input fingerprint + stage command/config signature + tool version tuple
  - command change invalidates only affected stage and downstream stages
- `run-manifest.json` includes:
  - run-level metadata (`run_id`, `execution_mode`, `tool_versions`, `host_fingerprint`)
  - per-step metadata (`stage_attempt`, `cache_hit`, `cache_key`)
- Pipeline never copies proprietary assets into repo-tracked roots.

## Interfaces and Data
- Input: `automation.toml`
- Output:
  - `run-manifest.json`
  - `validation-report.json`
  - per-step stdout/stderr logs
- Env contracts for stage commands:
  - `RECOMP_*` variables for work roots, config paths, and policy metadata

## Deliverables
- Exposed CLI command for automation loop.
- Updated automation config and run-manifest schemas/docs.
- Tests for deterministic resume, cache invalidation behavior, and failure recording.

## Open Questions
- Which stage-level policies should become hard gates when execution mode is `cloud`?
- Should archive stage also emit a batch-manifest status update by default?

## Acceptance Criteria
- `recomp --help` lists `automate`.
- Valid lifted-mode config runs end to end with deterministic outputs.
- Invalid config fails with explicit actionable diagnostics.
- Stage-level cache behavior preserves upstream work when downstream commands change.

## Risks
- External tools can still introduce non-determinism if versions drift.
- Overly broad cache signatures can mask intended stage reuse.

## References
- SPEC-220-INPUT-REPLAY.md
- SPEC-230-REFERENCE-MEDIA-NORMALIZATION.md
- SPEC-240-VALIDATION-ORCHESTRATION.md
- SPEC-270-COMPREHENSIVE-AUTOMATED-SOLUTION.md
