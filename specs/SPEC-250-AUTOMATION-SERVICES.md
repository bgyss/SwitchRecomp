# SPEC-250: Automation Services and Data Flow

## Status
Draft v0.2

## Purpose
Define local-first service boundaries and data flow contracts for automated static recompilation, with cloud interfaces staged behind explicit readiness gates.

## Goals
- Standardize the run lifecycle event model and service responsibilities.
- Keep local and cloud execution contract-compatible.
- Ensure immutable artifact and provenance traceability across all stages.

## Non-Goals
- Full infrastructure templates for AWS deployment.
- Detailed UI/console design.

## Background
The implementation priority is deterministic local execution. Cloud orchestration should reuse the same manifests and stage contracts rather than introducing divergent behavior.

## Requirements
- Execution modes:
  - `local` is required and first-class.
  - `cloud` and `hybrid` are schema-defined and gated.
- Run state requirements:
  - every run has immutable IDs and stage transitions
  - every stage emits deterministic logs and artifact references
- Artifact requirements:
  - content-addressed and immutable once written
  - external asset separation preserved
- Worker requirements:
  - stateless stage execution with explicit input/output contracts
  - deterministic retries and resumable stages
- Event model requirements:
  - `recomp.run.requested`
  - `recomp.run.started`
  - `recomp.run.stage.completed`
  - `recomp.run.validation.completed`
  - `recomp.run.completed`

## Interfaces and Data
- Run submission schema (v1):

```json
{
  "run_id": "uuid-or-run-id",
  "execution_mode": "local|cloud|hybrid",
  "module_manifest": "artifact://hash",
  "config_manifest": "artifact://hash",
  "provenance_manifest": "artifact://hash",
  "requested_by": "principal_id",
  "priority": "standard"
}
```

- Run status schema (v1):

```json
{
  "run_id": "uuid-or-run-id",
  "state": "queued|running|blocked|failed|succeeded",
  "current_stage": "string",
  "stage_attempt": 1,
  "artifacts": ["artifact://hash"],
  "started_at": "rfc3339",
  "updated_at": "rfc3339"
}
```

## Deliverables
- Service inventory mapped to local and cloud execution responsibilities.
- Run lifecycle state machine with transition rules.
- Event and schema docs for submission, state, and artifact indexing.

## Open Questions
- Should cloud run state be source-of-truth in a single metadata store or reconstructed from event logs?
- What minimum retention policy is required for failed run artifacts and logs?

## Acceptance Criteria
- Same manifest contracts support local runs and cloud-ready state records.
- Stage events and artifact references are deterministic and auditable.
- Local-first implementation does not require cloud infrastructure to run end to end.

## Risks
- Service granularity that is too fine can complicate operations.
- Drift between local and cloud orchestration can break reproducibility.

## References
- SPEC-210-AUTOMATED-RECOMP-LOOP.md
- SPEC-240-VALIDATION-ORCHESTRATION.md
- SPEC-260-AGENT-PIPELINE-SECURITY.md
- SPEC-270-COMPREHENSIVE-AUTOMATED-SOLUTION.md
