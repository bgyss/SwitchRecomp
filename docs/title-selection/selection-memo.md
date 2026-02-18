# Title Selection Memo

## Active Tracks
- Homebrew baseline track: open-source 2048-style homebrew (provisional).
- Hashed retail pilot track: `title-a24b9e807b456252` first-level milestone.

## Rationale
- Homebrew baseline keeps deterministic iteration fast and legally straightforward.
- Retail pilot exercises XCI intake, service surface growth, and first-level validation constraints.
- Both tracks share the same automation and validation lifecycle contracts.

## Checklist Coverage
- Legal/preservation rationale:
  - homebrew track uses redistributable inputs.
  - retail pilot uses user-supplied lawful assets in private external workspaces.
- Service dependency map:
  - homebrew: hid + basic gfx/runtime services.
  - retail pilot: expanded service surface tracked in SPEC-070/SPEC-200.
- ISA/GPU profile:
  - homebrew validates core decode and deterministic flow.
  - retail pilot validates first-level shader/render/service deltas.
- Asset separation:
  - both tracks keep assets external and commit hashes/metadata only.

## Follow-ups
- Finalize homebrew candidate license/asset checks.
- Continue retail pilot service and shader gap tracking.
- Keep per-track validation matrix and run-sheet artifacts updated.
