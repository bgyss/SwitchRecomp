# SPEC-260: Agent Pipeline Security and Automation

## Status
Draft v0.1

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
  "response_schema": "schema-id"
}
```

- Automation policy record (minimal JSON schema):

```json
{
  "policy_id": "string",
  "requires_approval": true,
  "max_cost_usd": 500,
  "allowed_models": ["gpt-5.2-codex", "gpt-5.2"],
  "run_windows": ["weekday:09:00-18:00"]
}
```

## Deliverables
- Security control checklist for model usage and artifact handling.
- Automation policy definitions for scheduled and manual runs.
- Audit log format covering prompts, responses, and approvals.

## Open Questions
- What redaction profiles are required for homebrew vs research inputs?
- What is the default reasoning_effort for each pipeline stage?

## Acceptance Criteria
- Every model call is routed through the Model Gateway with a stored audit record.
- Every automated run can be paused for approval when policy requires.
- A complete run can be replayed with the same prompts and artifacts.

## Risks
- Overly strict gating could slow iteration.
- Inconsistent redaction could leak sensitive data.

## References
- SPEC-020-INPUTS-PROVENANCE.md
- SPEC-095-BUILD-MANIFEST-INTEGRITY.md
- SPEC-096-BUNDLE-MANIFEST-INTEGRITY.md
- SPEC-210-AUTOMATED-RECOMP-LOOP.md
