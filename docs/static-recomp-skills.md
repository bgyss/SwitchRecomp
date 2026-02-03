# Static Recompilation Skills

This project uses a set of Codex skills to accelerate static recompilation
validation and batch processing. These skills live in the local Codex skills
folder but are documented here so future projects can reuse the workflow and
project-level configuration.

## Skill Set
- `static-recomp-scope-plan`:
  Define project scope, legal boundaries, validation matrix, and exit criteria.
- `static-recomp-batch-harness`:
  Catalog-scale harness design, manifest schema, artifact layout, and gates.
- `static-recomp-reference-capture`:
  Reference capture and normalization guidance.
- `static-recomp-input-replay`:
  Deterministic input trace capture and replay guidance.
- `static-recomp-av-compare`:
  A/V alignment and metrics, plus batch and threshold automation scripts.
- `static-recomp-perf-profile`:
  Performance profiling and regression reporting guidance.
- `static-recomp-regression-triage`:
  Regression classification and root-cause workflow.

## Project-Level Configuration
Use these repo templates to keep validation and reporting consistent across
future titles and projects.

- Validation matrix template: `docs/validation-matrix-template.md`
- Per-title run sheet template: `docs/title-run-sheet-template.md`
- Default A/V thresholds: `docs/thresholds/default.json`

## Recommended Practice
- Keep all proprietary inputs outside the repo.
- Record provenance in per-title run sheets.
- Use the validation matrix for acceptance criteria and traceability.
