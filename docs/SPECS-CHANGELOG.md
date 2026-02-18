# SPECS Change Log

Canonical detailed history of specification changes under `specs/`.

## Why This Exists
`README.md` contains a short summary log. This file is the detailed, commit-level changelog for all `specs/` adds/modifies/renames.

## Maintenance Rules
- Aggressively update this file whenever a spec file is added, modified, renamed, or removed.
- Keep entries in chronological order and include the commit hash, date, summary, and touched spec files.
- Keep `README.md` `Specs Update Log` synchronized with high-level milestones from this file.

## Reconstructed History (from git, no merges)
- 2026-01-29 `7b64c4f` Add exploratory pipeline scaffold
  - Spec files: A `specs/README.md`; A `specs/SPEC-000-CHARTER.md`; A `specs/SPEC-010-TARGET-PLATFORM.md`; A `specs/SPEC-020-INPUTS-PROVENANCE.md`; A `specs/SPEC-030-RECOMP-PIPELINE.md`; A `specs/SPEC-040-RUNTIME-ABI.md`; A `specs/SPEC-050-CPU-ISA.md`; A `specs/SPEC-060-GPU-GRAPHICS.md`; A `specs/SPEC-070-OS-SERVICES.md`; A `specs/SPEC-080-TIMING-INTERRUPTS.md`; A `specs/SPEC-090-BUILD-DISTRIBUTION.md`; A `specs/SPEC-100-VALIDATION.md`; A `specs/SPEC-110-TITLE-SELECTION.md`; A `specs/SPEC-TEMPLATE.md`

- 2026-01-29 `b3d04ef` Implement spec scaffolds for ISA, services, graphics, timing
  - Spec files: M `specs/SPEC-030-RECOMP-PIPELINE.md`; M `specs/SPEC-040-RUNTIME-ABI.md`; M `specs/SPEC-050-CPU-ISA.md`; M `specs/SPEC-060-GPU-GRAPHICS.md`; M `specs/SPEC-070-OS-SERVICES.md`; M `specs/SPEC-080-TIMING-INTERRUPTS.md`; M `specs/SPEC-090-BUILD-DISTRIBUTION.md`; M `specs/SPEC-100-VALIDATION.md`

- 2026-01-29 `7483693` Expand ISA semantics with flags
  - Spec files: M `specs/SPEC-050-CPU-ISA.md`

- 2026-01-29 `1cb1355` Add ISA shifts and rotates
  - Spec files: M `specs/SPEC-050-CPU-ISA.md`

- 2026-01-29 `29d0c12` Add ISA load/store stubs
  - Spec files: M `specs/SPEC-050-CPU-ISA.md`

- 2026-01-29 `e2f307f` Expand services, timing, and graphics scaffolds
  - Spec files: M `specs/SPEC-060-GPU-GRAPHICS.md`; M `specs/SPEC-070-OS-SERVICES.md`; M `specs/SPEC-080-TIMING-INTERRUPTS.md`; M `specs/SPEC-100-VALIDATION.md`

- 2026-01-30 `d8940e9` Add legal policy and documentation links
  - Spec files: M `specs/SPEC-000-CHARTER.md`

- 2026-01-30 `8765e91` Run cargo fmt
  - Spec files: M `specs/SPEC-010-TARGET-PLATFORM.md`; M `specs/SPEC-020-INPUTS-PROVENANCE.md`; M `specs/SPEC-090-BUILD-DISTRIBUTION.md`

- 2026-01-30 `e5dd4ea` Add validation baseline tool and CI wiring
  - Spec files: M `specs/SPEC-100-VALIDATION.md`

- 2026-01-30 `06d31ae` Add title selection docs and update plans
  - Spec files: M `specs/SPEC-110-TITLE-SELECTION.md`

- 2026-01-30 `cc26533` Add specs for manifest self-inclusion
  - Spec files: M `specs/README.md`; A `specs/SPEC-095-BUILD-MANIFEST-INTEGRITY.md`; A `specs/SPEC-096-BUNDLE-MANIFEST-INTEGRITY.md`

- 2026-01-30 `828daa4` Implement bundle manifest self inclusion
  - Spec files: M `specs/SPEC-096-BUNDLE-MANIFEST-INTEGRITY.md`

- 2026-01-30 `1543fb8` Implement manifest self-inclusion handling
  - Spec files: M `specs/SPEC-095-BUILD-MANIFEST-INTEGRITY.md`; M `specs/SPEC-096-BUNDLE-MANIFEST-INTEGRITY.md`

- 2026-01-31 `ada785e` Add homebrew end-to-end recomp specs
  - Spec files: M `specs/README.md`; A `specs/SPEC-120-HOMEBREW-INTAKE.md`; A `specs/SPEC-130-HOMEBREW-MODULE-EXTRACTION.md`; A `specs/SPEC-140-HOMEBREW-RUNTIME-SURFACE.md`; A `specs/SPEC-150-HOMEBREW-ASSET-PACKAGING.md`

- 2026-02-01 `28252d1` Implement homebrew intake and extraction
  - Spec files: M `specs/SPEC-120-HOMEBREW-INTAKE.md`; M `specs/SPEC-130-HOMEBREW-MODULE-EXTRACTION.md`; M `specs/SPEC-140-HOMEBREW-RUNTIME-SURFACE.md`; M `specs/SPEC-150-HOMEBREW-ASSET-PACKAGING.md`

- 2026-02-01 `ee06c01` Extract RomFS file tree during intake
  - Spec files: M `specs/SPEC-150-HOMEBREW-ASSET-PACKAGING.md`

- 2026-02-02 `2da3ca5` Reject homebrew module.json in pipeline
  - Spec files: M `specs/SPEC-120-HOMEBREW-INTAKE.md`

- 2026-02-02 `f97d4d7` Add homebrew lifter stub
  - Spec files: M `specs/SPEC-030-RECOMP-PIPELINE.md`

- 2026-02-02 `89d7831` Add decode-mode homebrew lifter
  - Spec files: M `specs/SPEC-030-RECOMP-PIPELINE.md`; M `specs/SPEC-050-CPU-ISA.md`

- 2026-02-02 `8be2193` Add specs for decode and CFG work
  - Spec files: M `specs/README.md`; A `specs/SPEC-160-AARCH64-DECODE-COVERAGE.md`; A `specs/SPEC-170-FUNCTION-DISCOVERY-CFG.md`

- 2026-02-02 `2375fb9` Add runtime memory model spec
  - Spec files: M `specs/README.md`; A `specs/SPEC-045-RUNTIME-MEMORY.md`

- 2026-02-02 `7a2095d` Implement runtime memory model and load/store lowering
  - Spec files: M `specs/SPEC-045-RUNTIME-MEMORY.md`

- 2026-02-02 `1e60503` Add configurable memory layout and init images
  - Spec files: M `specs/README.md`; A `specs/SPEC-046-RUNTIME-MEMORY-CONFIG.md`; A `specs/SPEC-047-MEMORY-IMAGE-INIT.md`

- 2026-02-02 `2fff0b9` Add XCI intake and DKCR HD milestone specs
  - Spec files: M `specs/README.md`; A `specs/SPEC-180-XCI-INTAKE.md`; A `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; A `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`

- 2026-02-03 `2f311c2` Update plan tracking and spec statuses
  - Spec files: M `specs/SPEC-046-RUNTIME-MEMORY-CONFIG.md`; M `specs/SPEC-047-MEMORY-IMAGE-INIT.md`; M `specs/SPEC-180-XCI-INTAKE.md`; M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; M `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`

- 2026-02-03 `aeee176` Add automation loop planning specs
  - Spec files: M `specs/README.md`; M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; M `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`; A `specs/SPEC-210-AUTOMATED-RECOMP-LOOP.md`; A `specs/SPEC-220-INPUT-REPLAY.md`; A `specs/SPEC-230-REFERENCE-MEDIA-NORMALIZATION.md`; A `specs/SPEC-240-VALIDATION-ORCHESTRATION.md`

- 2026-02-03 `3d1241b` Add automated recompilation architecture plan and specs
  - Spec files: M `specs/README.md`; A `specs/SPEC-250-AUTOMATION-SERVICES.md`; A `specs/SPEC-260-AGENT-PIPELINE-SECURITY.md`

- 2026-02-03 `fc90fc7` Fix SPEC-250 event list formatting
  - Spec files: M `specs/SPEC-250-AUTOMATION-SERVICES.md`

- 2026-02-03 `9125651` Add input replay and validation normalization
  - Spec files: M `specs/SPEC-220-INPUT-REPLAY.md`; M `specs/SPEC-230-REFERENCE-MEDIA-NORMALIZATION.md`; M `specs/SPEC-240-VALIDATION-ORCHESTRATION.md`

- 2026-02-03 `a797a32` Add automation loop orchestrator
  - Spec files: M `specs/SPEC-210-AUTOMATED-RECOMP-LOOP.md`

- 2026-02-03 `800dc62` Document DKCR validation prerequisites
  - Spec files: M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; M `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`

- 2026-02-04 `a837f5a` Implement runtime memory layout validation
  - Spec files: M `specs/SPEC-046-RUNTIME-MEMORY-CONFIG.md`; M `specs/SPEC-047-MEMORY-IMAGE-INIT.md`

- 2026-02-04 `34705b0` Add video-based validation workflow
  - Spec files: M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`

- 2026-02-04 `f18d29a` Add XCI intake scaffolding
  - Spec files: M `specs/SPEC-180-XCI-INTAKE.md`

- 2026-02-04 `2a76495` Add DKCR HD first-level scaffolding
  - Spec files: M `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`

- 2026-02-04 `47eb05a` Integrate external XCI extraction tooling
  - Spec files: M `specs/SPEC-180-XCI-INTAKE.md`

- 2026-02-04 `37f33f5` Expand validation artifacts and XCI plumbing
  - Spec files: M `specs/SPEC-100-VALIDATION.md`; M `specs/SPEC-180-XCI-INTAKE.md`; M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; M `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`

- 2026-02-04 `ba11589` Add capture validation helper and docs links
  - Spec files: M `specs/SPEC-180-XCI-INTAKE.md`; M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; M `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md`

- 2026-02-07 `ccedf85` Hash dkcr-hd assets
  - Spec files: M `specs/README.md`; M `specs/SPEC-190-VIDEO-BASED-VALIDATION.md`; R `specs/SPEC-200-DKCR-HD-FIRST-LEVEL.md` -> `specs/SPEC-200-TITLE-A24B9E807B456252-FIRST-LEVEL.md`

- 2026-02-16 `04516f8` Ingest long-tail LLM decomp findings into plans and specs
  - Spec files: M `specs/SPEC-210-AUTOMATED-RECOMP-LOOP.md`; M `specs/SPEC-250-AUTOMATION-SERVICES.md`; M `specs/SPEC-260-AGENT-PIPELINE-SECURITY.md`
