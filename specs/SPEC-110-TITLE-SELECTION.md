# SPEC-110: Target Title Selection Criteria

## Status
Draft v0.4

## Purpose
Define a dual-track title policy for static recompilation: a deterministic homebrew baseline and a hashed retail pilot.

## Goals
- Keep a preservation-safe baseline track that can iterate quickly and legally.
- Maintain a realistic retail pilot track that exercises XCI intake and first-level milestone workflows.
- Ensure both tracks use shared automation and validation contracts.

## Non-Goals
- Selecting titles based on commercial popularity.
- Distributing proprietary assets, keys, or copyrighted traces.

## Track Policy
### Track A: Homebrew Baseline
- Must be legally redistributable and compatible with project policy.
- Prioritizes deterministic iteration on intake, lift, runtime surface, and validation.
- Serves as the first regression gate for automation and schema changes.

### Track B: Hashed Retail Pilot
- Uses user-supplied, lawful inputs in private workspaces only.
- Uses hashed title identifiers and external artifact storage.
- Validates retail-specific assumptions (XCI intake, service surface, first-level behavior) without storing proprietary content in-repo.

## Selection Criteria
### Legal and Preservation Fit
- Complies with `docs/LEGAL-POLICY.md`.
- Provenance and acquisition path are documented.
- Asset separation is enforceable by workflow and tooling.

### Technical Feasibility
- Can reach meaningful validation anchors (boot/menu/first playable loop).
- Uses a service surface that can be stubbed or implemented incrementally.
- Has a tractable instruction and graphics feature profile for current roadmap phases.

### Automation and Validation Fit
- Works with deterministic input replay and video/audio comparison workflow.
- Supports run-manifest and artifact-index based traceability.
- Can be represented in the validation matrix and per-title run sheet templates.

## Evaluation Checklist
- [ ] Track assignment recorded (`homebrew_baseline` or `retail_pilot`).
- [ ] Legal and preservation rationale documented.
- [ ] Service dependency map created.
- [ ] ISA and GPU coverage estimate recorded.
- [ ] Validation anchors and thresholds documented.
- [ ] Asset separation and external artifact plan validated.

## Deliverables
- 2-3 candidate shortlist with track assignment and rationale.
- Selection memo identifying active Track A and Track B titles.
- Private trace/reference capture plan with external artifact paths.

## Supporting Docs
- `docs/title-selection/shortlist.md`
- `docs/title-selection/selection-memo.md`
- `docs/title-selection/trace-plan.md`
- `docs/title-run-sheet-template.md`
- `docs/validation-matrix-template.md`

## Open Questions
- What redaction profile defaults should apply by track when cloud automation is enabled?
- Which retail pilot milestones should be required before adding additional retail titles?

## Acceptance Criteria
- Active titles are documented for both tracks.
- Both tracks map to shared automation + validation contracts.
- Selection artifacts are sufficient to run deterministic validation without in-repo proprietary data.
