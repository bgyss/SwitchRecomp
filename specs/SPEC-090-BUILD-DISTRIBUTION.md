# SPEC-090: Build, Packaging, and Distribution

## Status
Draft v0.3

## Rationale
- Added a Nix + devenv development scaffold and initial build outputs for exploratory work.

## Purpose
Define build outputs, packaging, and distribution policy.

## Goals
- Produce reproducible builds with clear versioning.
- Distribute only non-proprietary artifacts.

## Non-Goals
- Shipping proprietary assets or binaries.
- Providing a commercial installer.

## Build Outputs
- Recompiled native binary.
- Runtime library and support files.
- Metadata files for provenance and configuration.

## Packaging
- A bundle layout that separates code from user-supplied assets.
- A deterministic build manifest and checksums.

## Distribution Policy
- Open source code and tooling only.
- Clear instructions for user-supplied assets.

## Deliverables
- A packaging spec and reference implementation.
- A release checklist that includes legal compliance checks.

## Open Questions
- What is the initial host platform support matrix?
- How should platform-specific dependencies be distributed?

## Acceptance Criteria
- A build that can be reproduced from the same inputs.
- A packaged output that runs when assets are supplied externally.
