# SPEC-100: Validation and Acceptance

## Status
Draft v0.6

## Rationale
- Added a starter test harness for config parsing and pipeline emission.
- Added starter unit tests for ISA, services, graphics, and scheduling scaffolds.
- Added deterministic validation helpers for service dispatch, timing traces, and graphics checksums.

## Purpose
Define validation, testing, and acceptance criteria for the project.

## Goals
- Provide repeatable tests that validate correctness and performance.
- Make regressions easy to detect.

## Non-Goals
- Full game compatibility certification for every title.

## Test Categories
- Instruction semantics tests.
- Runtime ABI and service tests.
- Graphics conformance tests.
- End-to-end gameplay traces.

## Metrics
- Correctness: matching reference traces.
- Stability: consistent output across runs.
- Performance: target frame rate on baseline host.

## Baseline Suite
- Documented in `docs/validation-baseline.md`.
- Regression reports are emitted to JSON and text files.

## Trace Guidance
- `docs/validation-traces.md` documents how to add new golden traces without distributing proprietary data.

## Deliverables
- A test harness and baseline test suite.
- A compatibility matrix for tested titles.
- A regression report (JSON + text summary).

## Open Questions
- How to acquire reference traces without distributing copyrighted content?
- What performance baseline should be considered acceptable?

## Acceptance Criteria
- All required tests pass in CI for a baseline target.
- A regression report is generated for failing tests.
