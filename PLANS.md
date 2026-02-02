# Plans

This file tracks implementation work derived from specs that do not yet have a concrete implementation in the repo. Each section links to a spec and lists work items needed to reach that spec's acceptance criteria.

## Scope
- SPEC-000 Project Charter and Ethics
- SPEC-010 Target Platform Baseline
- SPEC-020 Inputs and Provenance
- SPEC-045 Runtime Memory Model and Load/Store Lowering
- SPEC-090 Build, Packaging, and Distribution
- SPEC-095 Build Manifest Integrity
- SPEC-096 Bundle Manifest Integrity
- SPEC-100 Validation and Acceptance
- SPEC-110 Target Title Selection Criteria
- SPEC-120 Homebrew Candidate Intake
- SPEC-130 Homebrew Module Extraction
- SPEC-140 Homebrew Runtime Surface
- SPEC-150 Homebrew Asset Packaging
- SPEC-160 AArch64 Decode Coverage
- SPEC-170 Function Discovery and Control-Flow Graph

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

## SPEC-045: Runtime Memory Model and Load/Store Lowering
Outcome
- Block-based output can execute basic load/store instructions against a minimal runtime memory model.

Work items
- [x] Define a memory layout descriptor schema and emit it with outputs.
- [x] Implement runtime memory regions with alignment, bounds, and permission checks.
- [x] Lower ISA load/store ops to runtime memory helper calls.
- [x] Add tests and sample blocks that validate load/store behavior and error handling.

Exit criteria (from SPEC-045)
- Block-based output executes a test block with loads and stores using runtime helpers.
- Unaligned or out-of-bounds accesses return deterministic error codes.
- A sample pipeline output includes a memory layout descriptor that matches runtime regions.

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

## SPEC-095: Build Manifest Integrity
Outcome
- Ensure `manifest.json` accounts for every emitted file, including itself.

Work items
- [x] Add a manifest self-entry in `generated_files`.
- [x] Add a deterministic two-pass or explicit self-hash field.
- [x] Add a test that validates manifest self-inclusion and checksum correctness.

Exit criteria (from SPEC-095)
- `manifest.json` lists every generated file including itself.
- Generated file hashes and sizes match the files on disk.
- Re-running the pipeline with identical inputs yields the same manifest.

## SPEC-096: Bundle Manifest Integrity
Outcome
- Ensure `bundle-manifest.json` accounts for every bundle file, including itself.

Work items
- [x] Add a bundle manifest self-entry in the bundle file list.
- [x] Implement deterministic ordering for the bundle manifest entries.
- [x] Add a test that validates bundle manifest self-inclusion and checksum correctness.

Exit criteria (from SPEC-096)
- `bundle-manifest.json` lists every bundle file including itself.
- Checksums and sizes match the bundle contents.

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

## SPEC-120: Homebrew Candidate Intake
Outcome
- Accept a legally distributable homebrew candidate and emit a deterministic intake manifest.

Work items
- [x] Define a module intake manifest schema for NRO + optional NSO inputs.
- [x] Implement NRO intake parsing for header fields and asset section offsets.
- [x] Add provenance validation checks for homebrew inputs (reject proprietary or encrypted formats).
- [x] Emit deterministic `module.json` and `manifest.json` with hashes, sizes, and tool versions.
- [x] Add sample intake tests using non-proprietary NRO fixtures.

Exit criteria (from SPEC-120)
- A homebrew NRO can be ingested with hashes, build id, and asset offsets recorded.
- Asset extraction is recorded without mixing assets into code output.
- Intake errors are explicit when required fields are missing or unsupported.

## SPEC-130: Homebrew Module Extraction
Outcome
- Normalize NRO/NSO binaries into module.json and extracted segment blobs.

Work items
- [x] Implement NSO parsing including LZ4 segment decompression.
- [x] Capture build id/module id and preserve section boundaries in module.json.
- [x] Preserve relocation and symbol metadata when present.
- [x] Ensure extraction is deterministic across runs.
- [x] Add tests for NRO-only and NRO + NSO ingestion paths.

Exit criteria (from SPEC-130)
- NRO and NSO inputs yield module.json with correct segment sizes and build id.
- Compressed NSO segments are decompressed and emitted deterministically.
- Section boundaries are preserved for later translation.

## SPEC-140: Homebrew Runtime Surface
Outcome
- Provide a minimal runtime ABI surface that can boot a recompiled homebrew title.

Work items
- [x] Implement homebrew entrypoint shim with loader config setup.
- [x] Define loader config keys and defaults (EndOfList, MainThreadHandle, AppletType).
- [x] Add runtime manifest that enumerates provided config keys and stubbed services.
- [x] Implement deterministic time and input stubs for validation runs.
- [x] Add logging for unsupported service calls with explicit failure behavior.

Exit criteria (from SPEC-140)
- Recompiled binaries boot with required loader config keys present.
- Unsupported services fail with explicit, logged errors.
- Runtime manifest records provided loader config keys.

## SPEC-150: Homebrew Asset Packaging
Outcome
- Extract NRO asset section contents and package them alongside recompiled output.

Work items
- [x] Implement asset section extraction (icon, NACP, RomFS).
- [x] Validate and store NACP as `control.nacp` with expected size.
- [x] Emit deterministic asset output directory and hashes in manifest.json.
- [x] Document runtime RomFS mount expectations.
- [x] Add tests for asset extraction and manifest hashes.

Exit criteria (from SPEC-150)
- Icon, NACP, and RomFS assets are extracted deterministically when present.
- Asset hashes in manifest.json match extracted bytes.
- Code output remains separate from extracted assets.

## SPEC-160: AArch64 Decode Coverage
Outcome
- Expand decode coverage and IR support to lift real homebrew code paths.

Work items
- [x] Extend the lifted IR schema with arithmetic, logical, shift, memory, and branch ops.
- [x] Add decoder support for MOV (ORR alias), SUB, AND/OR/XOR, ADR/ADRP, LDR/STR, and branch opcodes listed in SPEC-160.
- [x] Map 32-bit W-register operations to zero-extended 64-bit IR semantics.
- [x] Add per-op unit tests that validate opcode decoding and emitted IR structure.
- [x] Add decode-limit enforcement tests for oversized text segments.

Exit criteria (from SPEC-160)
- A synthetic instruction stream containing Phase 1 opcodes lifts without errors.
- Unsupported opcodes report the PC and opcode value.
- Tests confirm 32-bit variants are zero-extended.
- Loads/stores emit correctly typed IR ops with aligned access checks.

## SPEC-170: Function Discovery and Control-Flow Graph
Outcome
- Replace linear decoding with basic blocks and deterministic control-flow graphs.

Work items
- [x] Extend the lifted module schema to allow block-based functions alongside legacy linear ops.
- [x] Implement a sorted worklist decoder that builds blocks and edges deterministically.
- [x] Add control-flow terminators for unconditional, conditional, call, and indirect branches.
- [x] Seed function discovery from entrypoint and direct call targets.
- [x] Add tests for if/else blocks, direct call discovery, and unresolved indirect branches.

Exit criteria (from SPEC-170)
- A synthetic binary with a conditional branch yields at least two blocks and correct edges.
- Direct call targets are discovered and lifted as separate functions.
- The lifted module is deterministic when run twice on the same input.
