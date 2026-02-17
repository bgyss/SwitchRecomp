# SPEC-200: title-a24b9e807b456252 First-Level Milestone (macOS/aarch64)

## Status
Draft v0.3

## Rationale
- Added title scaffolding and placeholder workflows for a retail first-level milestone.
- Updated to depend on shared automation and validation contracts from SPEC-210/220/230/240/270.

## Purpose
Define the hashed retail pilot first-level milestone for macOS/aarch64, using the shared automated loop rather than title-specific ad hoc flows.

## Goals
- Produce a macOS/aarch64 static recompilation that reaches the first playable level.
- Validate first-level behavior through shared run-manifest and artifact-index workflows.
- Keep all proprietary assets external and user-supplied.

## Non-Goals
- Full-game completion or compatibility certification.
- Exact performance parity with retail hardware.
- Distribution of keys, assets, or copyrighted captures.

## Background
- The project uses a dual-track policy: homebrew baseline plus hashed retail pilot.
- This spec represents the retail pilot track and must not diverge from shared automation interfaces.

## Requirements
- Intake must extract Program NCA, ExeFS, and NSO segments from user-supplied XCI/keyset inputs.
- Lift/build/run/validate must be executable under the same `recomp automate --config ...` lifecycle used by other tracks.
- Validation artifacts must be external and linked through artifact index plus run-manifest records.
- Retail pilot runs must use hashed title identifiers in local external paths and metadata where feasible.
- Runtime service and graphics deltas discovered by this title must be fed back into shared specs, not title-only docs.

## Operator Inputs
- External reference and capture artifacts listed in `docs/title-a24b9e807b456252-validation-prereqs.md`.
- Confirmed first-level timeline markers and thresholds.

## Interfaces and Data
- `samples/title-a24b9e807b456252/title.toml`
- `samples/title-a24b9e807b456252/provenance.toml`
- `run-manifest.json` for full lifecycle stage accounting.
- `validation-report.json` for first-level pass/fail metrics and triage.
- External `artifacts.json` compatible with `recomp-validation artifacts`.

## Deliverables
- Title configuration + patch scaffolding for first-level bring-up.
- Reproducible automation config and command path for macOS/aarch64.
- Validation report and triage notes for first-level gate.
- Feedback issues/spec deltas for shared service/runtime/gfx constraints.

## Scaffolding
- `samples/title-a24b9e807b456252/title.toml`
- `samples/title-a24b9e807b456252/provenance.toml`
- `samples/title-a24b9e807b456252/patches/patches.toml`
- `samples/title-a24b9e807b456252/module.json`

## Open Questions
- Which additional services are required beyond the current baseline to stabilize first-level gameplay?
- Which per-title threshold overrides are necessary after deterministic capture is enforced?

## Acceptance Criteria
- First-level milestone is runnable through the shared automation loop contracts.
- Validation report includes deterministic metrics and actionable triage output.
- No proprietary assets are stored in repo or generated code output roots.

## Risks
- Title-specific hacks could bypass shared contracts and reduce maintainability.
- Capture variance can still cause false negatives without strict operator setup.

## References
- `docs/title-a24b9e807b456252-first-level.md`
- `docs/title-a24b9e807b456252-runbook.md`
- `docs/automation-loop.md`
- `docs/validation-artifacts.md`
- `docs/validation-video.md`
