# SPEC-260: Agent Pipeline Security and Automation

## Status
Draft v0.2

## Purpose
Define security and governance controls for model-assisted automation, with local dry-run metadata support now and cloud enforcement later.

## Goals
- Keep model usage policy-compliant and auditable.
- Define redaction and approval controls before cloud rollout.
- Ensure model-facing interfaces are schema-validated and reproducible.

## Non-Goals
- Full network topology or IAM implementation details.
- Immediate mandatory model execution in local pipelines.

## Background
Local-first automation does not require live model calls, but must still produce metadata and schema contracts that prevent policy drift when cloud/agent execution is enabled.

## Requirements
- Model gateway contract:
  - Model egress must route through a gateway in cloud/hybrid modes.
  - Local mode records policy metadata without requiring gateway infrastructure.
- Audit requirements:
  - Prompt/response records correlate to `run_id` and `stage`.
  - Approval decisions and policy settings are captured in run metadata.
- Redaction requirements:
  - Redaction profile ID must be recorded per model request envelope.
  - Homebrew and hashed retail tracks can use different profile defaults.
- Validation requirements:
  - model outputs must be schema-validated before action execution.
  - action replay must be reproducible from stored prompts/artifacts.
- Secret handling requirements:
  - no key material in repo-tracked files
  - no secret values in logs

## Interfaces and Data
- Model request envelope schema (v1):

```json
{
  "schema_version": "1",
  "run_id": "uuid-or-run-id",
  "stage": "string",
  "model": "gpt-5.2-codex",
  "reasoning_effort": "low|medium|high|xhigh",
  "input_artifacts": ["artifact://hash"],
  "redaction_profile": "policy-id",
  "response_schema": "schema-id"
}
```

- Automation policy schema (v1):

```json
{
  "schema_version": "1",
  "policy_id": "string",
  "execution_mode": "local|cloud|hybrid",
  "requires_approval": true,
  "max_cost_usd": 500,
  "max_runtime_minutes": 240,
  "allowed_models": ["gpt-5.2-codex", "gpt-5.2"],
  "run_windows": ["weekday:09:00-18:00"]
}
```

## Deliverables
- Security control checklist for local, hybrid, and cloud modes.
- Versioned JSON schemas for model request envelopes and automation policies.
- Audit-log format requirements covering prompts, responses, policy, and approvals.

## Open Questions
- What baseline redaction profiles should be shipped for homebrew and hashed retail tracks?
- Which stages should be approval-gated by default in `hybrid` execution mode?

## Acceptance Criteria
- Local runs can record policy metadata without live model dependency.
- Cloud/hybrid interfaces are schema-complete for gated enforcement.
- Security artifacts are sufficient to replay and audit model-assisted actions.

## Risks
- Weak redaction defaults could leak sensitive context in future cloud mode.
- Overly strict approval defaults could degrade iteration speed without clear risk reduction.

## References
- SPEC-020-INPUTS-PROVENANCE.md
- SPEC-210-AUTOMATED-RECOMP-LOOP.md
- SPEC-250-AUTOMATION-SERVICES.md
- SPEC-270-COMPREHENSIVE-AUTOMATED-SOLUTION.md
