# SPEC-260: Agent Pipeline Security and Automation

## Status
Draft v0.2

## Purpose
Define security, governance, and automation requirements for the agent-managed recompilation pipeline using GPT-5.2-Codex.

## Goals
- Establish security controls for model usage and artifact handling.
- Define automation triggers, approvals, and auditability.
- Provide guardrails for deterministic, policy-compliant agent behavior.

## Non-Goals
- Network topology diagrams or detailed infrastructure templates.
- Model evaluation or benchmark methodology.

## Background
- Automated recompilation requires using an LLM to plan and supervise stages.
- The pipeline must keep inputs and outputs cleanly separated while preserving provenance.

## Requirements
- The Model Gateway MUST be the only egress path for model requests.
- The pipeline MUST use the OpenAI Responses API for GPT-5.2-Codex.
- Prompts and responses MUST be logged with run-id correlation.
- Inputs MUST be redacted to remove sensitive content before any model request.
- Model responses MUST be validated against schemas before execution.
- All agent actions MUST be reproducible from stored prompts and artifacts.
- Automation triggers MUST support both manual and scheduled execution.
- High-cost stages MUST support optional human approval gates.
- Secrets MUST be stored in managed secret stores and never in logs.
- Encryption MUST be enforced for all artifact storage and transport.
- Policy-enforced hooks MUST block edits to integrity sentinels and generated files unless an approved generator command is used.
- The pipeline MUST verify required build/test commands ran successfully before allowing commit or artifact publication.
- Agent tasks MUST include explicit file/function scope, and out-of-scope diffs MUST fail closed pending review.
- Retry budgets MUST be enforced per task lane to prevent unbounded unattended loops.
- Model routing MAY use lower-cost models for mechanical cleanup lanes, but routing decisions MUST be policy-driven and auditable.

## Interfaces and Data
- Model request envelope (minimal JSON schema):

```json
{
  "run_id": "uuid",
  "stage": "string",
  "model": "gpt-5.2-codex",
  "reasoning_effort": "low|medium|high|xhigh",
  "input_artifacts": ["artifact://hash"],
  "redaction_profile": "policy-id",
  "response_schema": "schema-id",
  "task_scope": {
    "kind": "function|file|stage",
    "target": "string"
  }
}
```

- Automation policy record (minimal JSON schema):

```json
{
  "policy_id": "string",
  "requires_approval": true,
  "max_cost_usd": 500,
  "allowed_models": ["gpt-5.2-codex", "gpt-5.2"],
  "run_windows": ["weekday:09:00-18:00"],
  "required_checks": ["build-and-verify", "test-suite"],
  "protected_targets": ["manifest-hashes", "generated-files"],
  "max_retries_per_task": 30
}
```

## Deliverables
- Security control checklist for model usage and artifact handling.
- Automation policy definitions for scheduled and manual runs.
- Audit log format covering prompts, responses, and approvals.

## Open Questions
- What redaction profiles are required for homebrew vs research inputs?
- What is the default reasoning_effort for each pipeline stage?
- Which guarded files should be globally protected versus lane-specific exceptions?

## Acceptance Criteria
- Every model call is routed through the Model Gateway with a stored audit record.
- Every automated run can be paused for approval when policy requires.
- A complete run can be replayed with the same prompts and artifacts.
- Out-of-scope edits, skipped checks, and protected-target edits are blocked with auditable policy failures.

## Risks
- Overly strict gating could slow iteration.
- Inconsistent redaction could leak sensitive data.
- Poorly tuned retry budgets may block legitimate tail-end progress.

## References
- SPEC-020-INPUTS-PROVENANCE.md
- SPEC-095-BUILD-MANIFEST-INTEGRITY.md
- SPEC-096-BUNDLE-MANIFEST-INTEGRITY.md
- SPEC-210-AUTOMATED-RECOMP-LOOP.md
