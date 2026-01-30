# Plans

This file tracks implementation work derived from specs that do not yet have a concrete implementation in the repo. Each section links to a spec and lists work items needed to reach that spec's acceptance criteria.

## Scope
- SPEC-000 Project Charter and Ethics
- SPEC-010 Target Platform Baseline
- SPEC-020 Inputs and Provenance
- SPEC-090 Build, Packaging, and Distribution
- SPEC-100 Validation and Acceptance
- SPEC-110 Target Title Selection Criteria

## SPEC-000: Project Charter and Ethics
Outcome
- Publish a clear legal-use and asset-separation policy that users and contributors must follow.

Work items
- [x] Add a standalone policy document that covers legal acquisition, asset separation, and prohibited content.
- [x] Add a short policy summary to `README.md` and link the policy doc from `RESEARCH.md` and `ROADMAP.md`.
- [x] Add a tooling guardrail note describing how provenance requirements are enforced (ties into SPEC-020).

Exit criteria (from SPEC-000)
- A published policy on legal use and asset separation.
- A tooling architecture that does not embed or require proprietary assets.

## SPEC-010: Target Platform Baseline
Outcome
- Define a stable baseline profile that other specs and runtime decisions can depend on.

Work items
- [x] Formalize the baseline profile as a structured document or config (CPU, GPU, memory, timing modes).
- [x] Add a runtime configuration stub that can switch between handheld and docked timing modes.
- [x] Record platform assumptions and trace which specs depend on them.

Exit criteria (from SPEC-010)
- A baseline profile that is stable and usable by other specs.
- A documented list of assumptions that can be tested.

## SPEC-020: Inputs and Provenance
Outcome
- The pipeline accepts inputs only with provenance metadata and can detect core formats.

Work items
- [x] Define a provenance metadata schema (TOML or JSON) and add a validator.
- [x] Add CLI support that refuses to build without valid provenance metadata.
- [x] Implement format detection for NCA, ExeFS (PFS0), NSO0, NRO0, NRR0, and NPDM inputs.
- [x] Add non-proprietary test fixtures and tests that prove format detection and provenance logging.

Exit criteria (from SPEC-020)
- A metadata schema with validation rules.
- The toolchain refuses to build without provenance metadata.
- A format detector identifies NCA/ExeFS/NSO0/NRO0/NRR0 inputs and logs the chosen path.

## SPEC-090: Build, Packaging, and Distribution
Outcome
- Produce a reproducible, policy-compliant bundle layout with a release checklist.

Work items
- [x] Define a packaging layout spec (code vs assets separation) and include it in docs.
- [x] Add a reference packaging command to the CLI or a build script.
- [x] Create a release checklist template that includes legal compliance checks.
- [x] Add tests that verify build manifest checksums match the emitted bundle contents.

Exit criteria (from SPEC-090)
- A build that can be reproduced from the same inputs.
- A packaged output that runs when assets are supplied externally.

## SPEC-100: Validation and Acceptance
Outcome
- Expand the test harness into a baseline suite with clear regression reporting.

Work items
- [x] Define the baseline test suite and target thresholds for correctness and stability.
- [x] Add a regression report generator (for example, JSON summary + human-readable output).
- [x] Add CI wiring that runs the baseline suite and fails on regressions.
- [x] Document how to add new golden traces without distributing proprietary assets.

Exit criteria (from SPEC-100)
- All required tests pass in CI for a baseline target.
- A regression report is generated for failing tests.

## SPEC-110: Target Title Selection Criteria
Outcome
- Select a preservation-safe title and document the rationale and validation plan.

Work items
- [x] Create a shortlist of 2 to 3 candidate titles and document pros/cons.
- [x] Produce a service dependency map and estimated instruction coverage for each candidate.
- [x] Write a final selection memo and a private trace-collection plan.

Exit criteria (from SPEC-110)
- A documented selection that satisfies all checklist items.
- A published plan for obtaining inputs legally and privately.
