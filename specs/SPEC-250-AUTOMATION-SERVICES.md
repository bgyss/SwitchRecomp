# SPEC-250: Automation Services and Data Flow

## Status
Draft v0.2

## Purpose
Define the service architecture and data flow for fully automated static recompilation across local and AWS environments.

## Goals
- Describe the core services and their responsibilities.
- Define the run lifecycle and required data flow events.
- Provide minimal interface schemas for run submission and status.

## Non-Goals
- Detailed runtime ABI or module formats (covered elsewhere).
- UI or operator console requirements.

## Background
- The pipeline must be fully automated while preserving strict input and output separation.
- Hybrid deployment is required to support local testing and cloud scale.

## Requirements
- The architecture MUST support both local-only and AWS-backed execution.
- Each run MUST be traceable from intake to output with immutable provenance records.
- Artifact storage MUST be content-addressed and immutable once written.
- The orchestration layer MUST support retries and resumable stages.
- Workers MUST be stateless and operate on explicit inputs and outputs.
- The model interface MUST be isolated behind a Model Gateway service.
- The orchestration layer MUST support a candidate-selection service that ranks unresolved work using deterministic similarity scoring and returns reproducible candidate lists.
- Every task request MUST declare a task lane (`general`, `gfx`, `math`, `cleanup`) and a scope (`function`, `file`, or `stage`) for auditing and policy checks.
- The control plane MUST expose long-tail triage metadata (`attempt_count`, `stall_reason`, `similarity_refs`) in run status APIs.
- Failed retries above policy thresholds MUST raise an escalation event instead of looping indefinitely.

## Interfaces and Data
- Run submission request (minimal JSON schema):

```json
{
  "run_id": "uuid",
  "module_manifest": "artifact://hash",
  "config_manifest": "artifact://hash",
  "provenance_manifest": "artifact://hash",
  "requested_by": "principal_id",
  "priority": "standard",
  "execution_mode": "local|cloud|hybrid",
  "task_lane": "general|gfx|math|cleanup",
  "candidate_selector": "opcode_distance|embedding|hybrid"
}
```

- Run status record (minimal JSON schema):

```json
{
  "run_id": "uuid",
  "state": "queued|running|blocked|failed|succeeded",
  "current_stage": "string",
  "task_lane": "general|gfx|math|cleanup",
  "attempt_count": 12,
  "stall_reason": "none|large_function|graphics_macro|math_transform|other",
  "artifacts": ["artifact://hash"],
  "started_at": "rfc3339",
  "updated_at": "rfc3339"
}
```

Required events:
- `recomp.run.requested`
- `recomp.run.planned`
- `recomp.run.candidate.selected`
- `recomp.run.stage.completed`
- `recomp.run.long_tail.flagged`
- `recomp.run.validation.completed`
- `recomp.run.completed`

## Deliverables
- Service inventory with ownership and run-time responsibilities.
- Run lifecycle state machine definition.
- Documented data flow with required events and artifacts.
- Candidate-selection and task-lane policy definitions with deterministic replay semantics.

## Open Questions
- Should run state be sourced from a single metadata store or event log only?
- What is the minimum artifact retention policy for failed runs?
- Should candidate scoring be computed inline in orchestration or by a dedicated retrieval service?

## Acceptance Criteria
- A run can be submitted using the minimal schema and observed end-to-end.
- Every stage emits an event with deterministic artifacts and logs.
- The architecture supports running the same input locally or in AWS without changing manifests.

## Risks
- Overly granular services could increase operational complexity.
- Divergent local and cloud behavior could reduce determinism.
- Over-specialized task lanes could starve shared backlog work without balancing policies.

## References
- SPEC-030-RECOMP-PIPELINE.md
- SPEC-210-AUTOMATED-RECOMP-LOOP.md
- SPEC-240-VALIDATION-ORCHESTRATION.md
