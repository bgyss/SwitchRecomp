# SPEC-030: Static Recompilation Pipeline

## Status
Draft v0.5

## Rationale
- Added an exploratory pipeline scaffold and config schema to validate the end-to-end shape.
- Added deterministic build manifest emission with input hashes.
- Added a placeholder homebrew lifter command to bridge intake into lifted JSON.

## Purpose
Define the end-to-end pipeline for static recompilation from input binaries to native output.

## Goals
- Provide a deterministic, repeatable pipeline.
- Translate instructions into a portable intermediate form or C/C++.
- Allow per-title configuration and patching.

## Non-Goals
- A full compiler toolchain for arbitrary languages.
- Dynamic binary translation at runtime.

## Pipeline Stages
1. Input parsing and format validation.
2. Symbol and relocation extraction.
3. Function discovery and splitting.
4. Instruction translation into IR or C/C++.
5. Linking against a runtime ABI layer.
6. Build artifact generation.

## Configuration
- A per-title config file (TOML) for stubs, skips, and patches.
- Deterministic ordering for reproducible builds.
- Overlay and relocation support with runtime indirection.

## Deliverables
- A CLI tool that drives the pipeline.
- A config schema and validator.

## Open Questions
- Which intermediate representation best fits the project goals?
- How should overlays or dynamic segments be handled?

## Acceptance Criteria
- A minimal end-to-end build that compiles output for a test binary.
- A config file that can stub at least one system call.

## References
- https://github.com/N64Recomp/N64Recomp
