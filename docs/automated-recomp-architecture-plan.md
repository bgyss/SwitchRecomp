# Automated Recompilation Architecture Plan

## Status
Draft v0.1

## Goals
- Provide a concrete, end-to-end architecture for fully automated static recompilation.
- Define a hybrid local plus AWS deployment that keeps inputs and outputs cleanly separated.
- Specify an agent-managed pipeline using the GPT-5.2-Codex API via the OpenAI Responses API.
- Make security and provenance a first-class concern across the pipeline.

## Scope
- Config-driven recompilation of non-proprietary inputs as defined by existing specs.
- Local developer runs and AWS-backed scale-out runs.
- Automation, observability, and auditability for the full pipeline lifecycle.

## Non-Goals
- Running or storing proprietary game assets.
- Replacing existing spec-level definitions for formats or runtime ABI.
- Defining UI experiences beyond minimal operator dashboards.

## Architecture Overview

### Local Stack
- Recomp Orchestrator (local): CLI and daemon that accepts run requests and manages the pipeline.
- Local Artifact Store: content-addressed cache for inputs, intermediate artifacts, and outputs.
- Local Execution Pool: sandboxed workers for parsing, analysis, and codegen steps.
- Local Validation Harness: deterministic replays and output validation on local hardware.

### AWS Stack
- Run Control Plane: API layer for submission, status, and metadata.
- Orchestration Service: AWS Step Functions for stateful pipelines and retries.
- Job Queue: SQS for work item fanout to workers.
- Compute Pool: ECS or Batch for stateless workers (CPU/GPU tiers).
- Artifact Store: S3 with immutable object versioning and lifecycle policies.
- Metadata Store: DynamoDB or Postgres for run state, provenance, and indexing.
- Model Gateway: service that brokers access to GPT-5.2-Codex via the Responses API.
- Validation Farm: managed runners that execute deterministic replays and compare outputs.

## Core Services and Responsibilities
- Run Control Plane: authenticate requests, enforce policy, and emit run events.
- Orchestrator: define stages, retries, and dependency ordering for each run.
- Artifact Store: store all immutable inputs and outputs with content hashes.
- Metadata Store: track run status, provenance, and artifact lineage.
- Execution Workers: perform deterministic transforms using the pipeline specs.
- Model Gateway: normalize prompts, enforce redaction, and apply model routing rules.
- Validation Harness: execute deterministic checks and write validation reports.

## Data Flow (Hybrid)
1. Intake: local or cloud intake validates inputs and creates a run request.
2. Normalize: inputs are normalized, hashed, and written to the Artifact Store.
3. Plan: the agent planner generates a run plan using GPT-5.2-Codex.
4. Execute: workers process plan stages and emit artifacts and logs.
5. Validate: validation runners compare outputs to reference baselines.
6. Package: build outputs are packaged with manifests and integrity reports.
7. Publish: outputs are stored in the Artifact Store and indexed in Metadata.

## Security and Compliance
- Classify inputs and outputs by provenance and sensitivity.
- Enforce least privilege IAM roles for each service and worker tier.
- Store secrets in AWS Secrets Manager and local equivalents.
- Encrypt data at rest and in transit with KMS-managed keys.
- Maintain immutable audit logs for all run requests and model prompts.
- Enforce redaction rules before any model request leaves the environment.

## Agent-Managed Pipeline Using GPT-5.2-Codex
- Use the OpenAI Responses API as the sole model interface for GPT-5.2-Codex.
- Use structured responses with explicit schemas for plans, diffs, and decisions.
- Apply model routing rules that can fall back to GPT-5.2 if GPT-5.2-Codex is unavailable.
- Capture prompts, responses, and model metadata in the audit log.
- Provide tool access only through the Model Gateway to enforce policy.

## Automation and Operations
- Local runs: CLI triggers a local orchestrator workflow with deterministic stages.
- Cloud runs: EventBridge schedules and manual triggers submit runs to the Control Plane.
- Retry policy: bounded retries with exponential backoff and circuit breakers.
- Approval gates: optional human approval for high-cost or high-risk stages.
- Rollbacks: failed stages retain artifacts and logs for replay.

## Observability
- Structured logs for each stage with run-id correlation.
- Metrics for queue depth, worker utilization, and validation pass rates.
- Traces for end-to-end run latency across services.

## Rollout Phases
- Phase 1: local-only orchestration and agent planning.
- Phase 2: hybrid runs with shared Artifact Store and cloud validation.
- Phase 3: full AWS orchestration with auto-scaling execution pools.

## Open Questions
- Do we need a dedicated schema registry for agent outputs?
- Which stages should be allowed to run without human approval?
- What is the minimum local hardware profile for deterministic validation?
