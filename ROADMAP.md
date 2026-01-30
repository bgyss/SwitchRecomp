# Roadmap

This roadmap is organized by phases with explicit exit criteria. Dates are intentionally omitted until research milestones are validated.

## Phase 0: Discovery and Architecture
- Lock the project charter and legal policy (`docs/LEGAL-POLICY.md`).
- Identify a minimal test title and input format.
- Choose the internal representation for instruction lifting.
- Define the runtime ABI shape and service boundaries.

Exit criteria:
- SPEC-000, SPEC-020, and SPEC-030 approved.
- A minimal parser can load a test binary and list functions.

## Phase 1: CPU Recompilation MVP
- Implement core instruction lifting for a small instruction set.
- Build the runtime ABI stub library.
- Create a tiny test harness with golden traces.

Exit criteria:
- A test binary recompiles and executes with correct output.
- Instruction tests pass for the supported subset.

## Phase 2: Runtime Services MVP
- Implement core services needed to reach a main loop.
- Add logging and deterministic scheduling.
- Provide a stub and fallback framework for unknown services.

Exit criteria:
- A minimal title reaches a stable loop using stubbed services.
- Deterministic replay works across two runs.

## Phase 3: Graphics Prototype
- Implement a basic GPU command path or a thin translation layer.
- Render a test scene from recompiled code.
- Add graphics conformance tests.

Exit criteria:
- A test scene renders deterministically.
- A documented set of supported GPU features exists.

## Phase 4: First Title Milestone
- Select a preservation-safe title and provide a public build pipeline.
- Expand instruction coverage to what the title needs.
- Document limitations and required assets.

Exit criteria:
- Title boots and reaches gameplay.
- Performance targets met on baseline host.

## Phase 5: Stabilization
- Harden tooling, improve diagnostics, and expand coverage.
- Establish a compatibility list for tested titles.
- Publish a contributor guide and spec updates.

Exit criteria:
- Regression suite is stable.
- Clear compatibility reporting.
