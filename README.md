# SwitchRecomp

This repository contains a draft specification set for a Nintendo Switch static recompilation project focused on preservation. The documents define scope, requirements, and research needs without distributing any proprietary assets or binaries. An exploratory Rust workspace now mirrors the intended pipeline shape.

## Contents
- `specs/` contains the numbered specification series.
- `crates/` holds the exploratory pipeline/runtime scaffolding.
- `skills/` provides the Codex skill set used for validation workflows.
- `ROADMAP.md` provides phased milestones and exit criteria.
- `RESEARCH.md` lists research directions and required sources.
- `docs/` contains development notes.
 - `docs/LEGAL-POLICY.md` defines legal use and asset separation rules.
 - `docs/static-recomp-skills.md` documents the Codex skill set and project-level validation templates.

## How to Use the Specs
- Read `specs/README.md` for ordering.
- Each spec starts as Draft and should be updated as research is completed.
- Open questions are explicitly listed at the end of each spec.

## Contribution Notes
- Do not add or link to proprietary assets, keys, or copyrighted binaries.
- Keep inputs and outputs strictly separated.
- When updating a spec, add a short rationale and keep acceptance criteria testable.

Legal and provenance policy:
- Follow `docs/LEGAL-POLICY.md` for legal acquisition, asset separation, and prohibited content.
- All builds require a validated `provenance.toml` describing lawful inputs.

## Development
- The dev environment is managed with Nix + devenv.
- See `docs/DEVELOPMENT.md` for commands and sample usage.

## Samples and Flow Docs
- `samples/memory-image/` shows the memory image initialization flow (segment blob + lifted module).
- `docs/static-recompilation-flow.md` outlines a hypothetical macOS static recompilation flow and verification pipeline (see the Real XCI intake section for external-tool usage).

## Back Pressure Hooks
These hooks add fast, consistent feedback to keep the repo autonomous and reduce review churn. Hooks are defined in `.pre-commit-config.yaml` and can be run with `prek` (preferred) or `pre-commit`.

- Install hooks: `prek install` or `pre-commit install`.
- Run on demand: `prek run --all-files` or `pre-commit run --all-files`.
- macOS note: the Nix dev shell ships `prek` only (to avoid Swift/.NET builds); install `pre-commit` separately if you need it.

Configured hooks:
- Pre-commit: `trailing-whitespace`, `end-of-file-fixer`, `check-merge-conflict`, `check-yaml`, `check-toml`, `check-json`, `check-added-large-files`, `detect-private-key`, `check-executables-have-shebangs`, `check-symlinks`, `check-case-conflict`, `cargo fmt --check`.
- Pre-push: `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace`.

## Specs Update Log
- 2026-01-29: v0.2 pass started; added hardware baseline details, input/binary format research, OS/services surface notes, and timing/interrupts refinements across specs.
- 2026-01-29: Added `SPEC-TEMPLATE.md` and expanded `RESEARCH.md` with seed sources.
- 2026-01-29: Added exploratory pipeline/runtime crates, a minimal sample, and dev environment scaffolding.
- 2026-01-29: Added ISA, services, graphics, and timing scaffolds plus build manifest output.

## Status
Early draft. Expect frequent revisions as research progresses.
