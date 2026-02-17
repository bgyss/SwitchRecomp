# SPEC-270: Comprehensive Automated Static Recompilation Solution

## Status
Draft v0.1

## Purpose
Define the integrated, local-first architecture for automated static recompilation across homebrew and hashed retail pilot tracks.

## Goals
- Align existing specs into one decision-complete automation architecture.
- Standardize a deterministic run lifecycle with immutable artifacts and explicit provenance.
- Keep local execution as the first-class implementation target while preserving cloud and agent extension points.
- Integrate new research workflows (analysis regen loops, evidence-backed rename maps, runtime instrumentation) as optional but formal stages.

## Non-Goals
- Immediate deployment of cloud control-plane services.
- Replacement of existing per-domain specs (ISA, runtime ABI, validation, intake).
- Inclusion of proprietary assets, keys, or copyrighted captures in the repo.

## Background
- Existing implementation already includes a local automation engine, but the CLI entrypoint and some planning artifacts have drifted from current code.
- Specs through SPEC-200 are largely represented in implementation planning; SPEC-210 through SPEC-260 require active, explicit work tracking.
- Research additions need a stable stage contract so experiments remain reproducible and provenance-safe.

## Requirements
- The project must support a dual-track target policy:
  - homebrew baseline track for deterministic iteration.
  - hashed retail pilot track for first-level milestone realism.
- The local lifecycle must expose these stages:
  1. intake
  2. analysis (optional)
  3. lift
  4. build
  5. run/capture
  6. validate
  7. archive
- Every stage must produce deterministic logs and artifact references in `run-manifest.json`.
- Stage cache keys must include:
  - non-command input fingerprint
  - stage command or stage configuration signature
  - tool version tuple
- A command change must invalidate only that stage and downstream stages.
- Analysis stage contracts must support:
  - headless export command hooks
  - expected outputs
  - optional `name_map.json` and runtime trace manifest references
- Policy hooks must exist in config for:
  - execution mode (`local`, `cloud`, `hybrid`)
  - approval and cost/runtime bounds
  - redaction profile metadata
  - allowed model metadata
- Local mode treats policy gates as metadata/no-op controls; cloud enforcement is deferred.

## Interfaces and Data
- CLI:
  - `recomp automate --config <path>`
- `automation.toml`:
  - add optional `[analysis]`
  - add optional `[policy]`
- `run-manifest.json`:
  - add `run_id`, `execution_mode`, `tool_versions`, `host_fingerprint`
  - add per-step `stage_attempt`, `cache_hit`, `cache_key`
- Artifact index:
  - maintain compatibility with `recomp-validation artifacts`
  - formalize schema in docs and align with `run-manifest.json` artifact paths
- Security envelope schemas:
  - model request envelope
  - automation policy record
  - versioned JSON schemas for cloud/agent readiness

## Deliverables
- Spec updates that reconcile dual-track policy and local-first rollout.
- CLI and automation implementation parity for single-command loop execution.
- Deterministic run-manifest schema docs and tests.
- Artifact index schema docs and examples.
- Security envelope schema docs for future cloud enforcement.

## Open Questions
- Which redaction profiles should be default for homebrew vs hashed retail pilot runs?
- Which stages should require approval when execution mode is `cloud` or `hybrid`?

## Acceptance Criteria
- A local run can execute intake through validation with `recomp automate --config ...`.
- A command change in `automation.toml` reuses unaffected upstream stage outputs.
- Specs/docs/plans/roadmap are internally consistent for dual-track and local-first policy.
- Cloud and agent interfaces are schema-defined without blocking local deterministic progress.

## Risks
- Stage contract creep could reduce determinism if optional analysis outputs are weakly defined.
- Excessive policy surface in local mode could create false confidence before cloud enforcement exists.

## References
- SPEC-210-AUTOMATED-RECOMP-LOOP.md
- SPEC-220-INPUT-REPLAY.md
- SPEC-240-VALIDATION-ORCHESTRATION.md
- SPEC-250-AUTOMATION-SERVICES.md
- SPEC-260-AGENT-PIPELINE-SECURITY.md
- docs/automated-recomp-architecture-plan.md
