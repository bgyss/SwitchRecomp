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
- Use `docs/SPECS-CHANGELOG.md` for the detailed, commit-level history of spec changes.

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
- `samples/validation/` contains non-proprietary validation artifact index templates.
- `docs/static-recompilation-flow.md` outlines a hypothetical macOS static recompilation flow and verification pipeline (see the Real XCI intake section for external-tool usage).
- `docs/xci-intake.md` documents the XCI intake workflow and mock extractor format.
- `docs/validation-artifacts.md` defines validation artifact indexing, workflows, and dependencies.
- `docs/run-manifest-schema.md` and `docs/artifact-index-schema.md` document automation/validation metadata schemas.
- `docs/validation-video.md` describes the hash-based video validation workflow.
- `docs/title-hash-ingest.md` documents hash-based title/data ingest and local decoder-ring files.
- `scripts/capture-validation.sh`, `scripts/capture-video-macos.sh`, `scripts/capture_video.sh`, and `scripts/validate_artifacts.sh` provide capture and validation helpers.
- `cargo run -p recomp-cli -- automate --config <automation.toml>` runs the local end-to-end automation loop.
- `scripts/ingest_hashed_title.sh` hashes title/data paths for private local workspaces.
- `scripts/rewrite_title_refs_in_history.sh` rewrites commit-message title references to hash ids.

## Back Pressure Hooks
These hooks add fast, consistent feedback to keep the repo autonomous and reduce review churn. Hooks are defined in `.pre-commit-config.yaml` and can be run with `prek` (preferred) or `pre-commit`.

- Install hooks: `prek install` or `pre-commit install`.
- Run on demand: `prek run --all-files` or `pre-commit run --all-files`.
- macOS note: the Nix dev shell ships `prek` only (to avoid Swift/.NET builds); install `pre-commit` separately if you need it.

Configured hooks:
- Pre-commit: `trailing-whitespace`, `end-of-file-fixer`, `check-merge-conflict`, `check-yaml`, `check-toml`, `check-json`, `check-added-large-files`, `detect-private-key`, `check-executables-have-shebangs`, `check-symlinks`, `check-case-conflict`, `cargo fmt --check`.
- Pre-push: `cargo clippy --workspace --all-targets --all-features -D warnings`, `cargo test --workspace`.

## Specs Update Log
- Detailed commit-by-commit spec history: `docs/SPECS-CHANGELOG.md`.
- 2026-02-16: Ingested long-tail LLM decomp findings into `SPEC-210`, `SPEC-250`, and `SPEC-260`.
- 2026-02-07: Migrated title references to hash IDs, updated validation details, and renamed `SPEC-200` to `SPEC-200-TITLE-A24B9E807B456252-FIRST-LEVEL.md`.
- 2026-02-04: Expanded XCI intake and validation artifact plumbing across `SPEC-100`, `SPEC-180`, `SPEC-190`, and `SPEC-200`.
- 2026-02-03: Added automation loop and orchestration/security spec set (`SPEC-210` through `SPEC-260`) and related plan/status updates.
- 2026-02-02: Added decode/CFG and runtime-memory specs (`SPEC-045`, `SPEC-046`, `SPEC-047`, `SPEC-160`, `SPEC-170`) plus XCI/video milestone specs (`SPEC-180`, `SPEC-190`, `SPEC-200`).
- 2026-01-31 to 2026-02-01: Added and refined homebrew end-to-end intake/extraction/runtime/asset specs (`SPEC-120` through `SPEC-150`).
- 2026-01-30: Added manifest integrity specs (`SPEC-095`, `SPEC-096`) and updated charter/validation/title-selection details.
- 2026-01-29: Bootstrapped the initial spec series (`SPEC-000` through `SPEC-110`) and iterative ISA/services/graphics/timing scaffold updates.

## Status
Early draft. Expect frequent revisions as research progresses.
