# SwitchRecomp

This repository contains a draft specification set for a Nintendo Switch static recompilation project focused on preservation. The documents define scope, requirements, and research needs without distributing any proprietary assets or binaries. An exploratory Rust workspace now mirrors the intended pipeline shape.

## Contents
- `specs/` contains the numbered specification series.
- `crates/` holds the exploratory pipeline/runtime scaffolding.
- `ROADMAP.md` provides phased milestones and exit criteria.
- `RESEARCH.md` lists research directions and required sources.
- `docs/` contains development notes.

## How to Use the Specs
- Read `specs/README.md` for ordering.
- Each spec starts as Draft and should be updated as research is completed.
- Open questions are explicitly listed at the end of each spec.

## Contribution Notes
- Do not add or link to proprietary assets, keys, or copyrighted binaries.
- Keep inputs and outputs strictly separated.
- When updating a spec, add a short rationale and keep acceptance criteria testable.

## Development
- The dev environment is managed with Nix + devenv.
- See `docs/DEVELOPMENT.md` for commands and sample usage.

## Specs Update Log
- 2026-01-29: v0.2 pass started; added hardware baseline details, input/binary format research, OS/services surface notes, and timing/interrupts refinements across specs.
- 2026-01-29: Added `SPEC-TEMPLATE.md` and expanded `RESEARCH.md` with seed sources.
- 2026-01-29: Added exploratory pipeline/runtime crates, a minimal sample, and dev environment scaffolding.

## Status
Early draft. Expect frequent revisions as research progresses.
