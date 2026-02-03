# SPEC-240: Validation Orchestration and Triage

## Status
Draft v0.1

## Purpose
Define the orchestration of validation runs, reporting, and triage so regression detection is automated and actionable.

## Goals
- Run validation steps automatically within the recompilation loop.
- Produce structured reports that highlight drift and likely causes.
- Enable iterative tuning of thresholds without losing provenance.

## Non-Goals
- Automatic root-cause analysis for all failures.
- Replacing human judgment for subjective visual quality.

## Background
Validation must be repeatable and consistent across runs. A dedicated orchestration layer can standardize comparison steps and surface failures clearly.

## Requirements
- Accept reference and capture configs plus optional input script metadata.
- Generate a validation report with:
  - frame and audio match ratios
  - drift offsets and dropped frame counts
  - threshold pass/fail results
  - links to artifacts (hash lists, diff frames)
- Emit a triage summary with suggested next steps:
  - re-run capture
  - adjust thresholds
  - check input alignment
- Store validation metadata alongside the run manifest.

## Interfaces and Data
- `validation-config.toml` (optional):
  - threshold overrides
  - drift tolerance windows
  - output artifact paths
- `validation-report.json`:
  - status, metrics, and failure details
  - artifact references (paths and hashes)

## Deliverables
- Validation runner that integrates with recomp-validation.
- Report schema and triage summary generator.
- Documentation for interpreting validation results.

## Open Questions
- Should we emit frame diff image sets on failure by default?
- How should we encode threshold overrides in provenance?

## Acceptance Criteria
- A validation run generates a report and triage summary in one command.
- Reports are deterministic for identical inputs and captures.
- Failures include enough context to reproduce and debug.

## Risks
- Overly strict thresholds can generate false negatives.
- Poor capture quality can mask true regressions.

## References
- SPEC-190 Video-Based Validation
- SPEC-210 Automated Recompilation Loop
- SPEC-230 Reference Media Normalization
