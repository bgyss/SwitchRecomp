# Hypothetical Static Recompilation Flow (macOS)

This document describes a hypothetical end-to-end static recompilation flow for
preservation work. It is not a recipe for distributing proprietary assets, and
assumes lawful access to original data. The steps call out where the current
repo provides scaffolding versus where future tooling is required.

## Scope And Assumptions
- Inputs are legally obtained by the operator and remain separated from outputs.
- The pipeline emits a macOS-hosted, statically recompiled binary.
- Many steps are exploratory and require additional implementation work.

## End-To-End Flow
1. Legal acquisition and provenance capture.
2. Input ingest and validation.
3. Segment extraction and labeling.
4. Lifting to a stable, config-driven IR.
5. Configuration of runtime, memory layout, and stubs.
6. Emission of a recompiled Rust project.
7. macOS build and packaging.
8. Verification pipeline.

## Step Details

1. Legal acquisition and provenance capture.
- Acquire inputs from lawful sources.
- Record provenance metadata: title, version, region, device, collection tool,
  and cryptographic hashes for each input.
- Maintain strict separation between code outputs and proprietary data.

2. Input ingest and validation.
- Use the intake tooling to detect and validate formats (NCA, NSO, NRO, NPDM).
- Store extracted data under a clean, deterministic directory layout.
- Reject inputs that fail hash or format validation.
- For XCI inputs, use external tooling (for example `hactool`) with user-provided keys to
  extract ExeFS/RomFS into separated output roots.

3. Segment extraction and labeling.
- Extract executable segments (text, rodata, data, bss) and record:
  - Base address, size, permissions.
  - Paths to initial segment bytes where applicable.
- Emit a `module.json` (or sidecar manifest) describing segments and metadata.

4. Lifting to a stable IR.
- Lift instructions into a deterministic IR (current repo uses a JSON module
  with ops and blocks as a stand-in).
- Preserve control-flow and data-flow metadata as needed for correctness.
- Record unsupported instructions or gaps for later coverage expansion.

5. Configuration of runtime and memory.
- Write `title.toml` to define:
  - Entry function.
  - Stub behaviors for syscalls.
  - Runtime mode and memory layout.
- Define a memory layout that covers the extracted segments.
- Ensure layout is validated for overlap, size, and overflow errors.

6. Emission of a recompiled Rust project.
- Run the pipeline to generate:
  - `src/main.rs` with runtime initialization.
  - `manifest.json` and build metadata.
  - Segment blob copies used for memory initialization.
- Confirm the emitted manifest captures input hashes and generated file hashes.

7. macOS build and packaging.
- Build the emitted project for the intended macOS target:
  - `cargo build --release --target aarch64-apple-darwin`
  - or `cargo build --release --target x86_64-apple-darwin`
- Package outputs so that proprietary assets remain external.
- Document required runtime assets separately from the binary output.

## Suggested Verification Pipeline

### 1. Data Integrity And Reproducibility
- Verify that `manifest.json` hashes match on-disk outputs.
- Re-run the pipeline and confirm identical manifests for identical inputs.
- Store a signed build manifest for each verification run.

### 2. Functional Correctness Against Original Data
- Capture a reference trace from the original execution environment.
- Normalize the trace into deterministic events:
  - CPU-visible register snapshots at stable sync points.
  - Syscall events and return values.
  - Memory checksums at segment boundaries.
- Run the recompiled output with equivalent inputs and compare traces.
- Flag and investigate divergences with minimal, reproducible test cases.

### 3. Video-Based Validation (Public Gameplay Videos)
This step is a heuristic and should be used only for coarse validation.

- Select a publicly available video with clear, stable footage.
- Capture gameplay from the recompiled output using consistent settings.
- Align the two videos using audio fingerprints or a known sync marker.
- Compute a per-frame perceptual hash or structural similarity score.
- Track divergence windows and correlate with in-game events.
- Treat this as supporting evidence, not proof of correctness.

### 4. Combined Acceptance Checks
- Require all deterministic trace checks to pass for critical paths.
- Use video-based checks to detect gross mismatches or timing drift.
- Record all verification artifacts alongside the build manifest:
  - Trace logs.
  - Frame hash summaries.
  - Verification report and environment metadata.

## Outputs And Artifacts
- `manifest.json` and `bundle-manifest.json` for integrity tracking.
- A deterministic verification report (JSON + human-readable summary).
- A reproducible command log for each verification run.

## Notes
- This flow is intentionally conservative to preserve legal compliance and
  reproducibility.
- Do not embed or distribute proprietary assets as part of the recompiled
  output.
