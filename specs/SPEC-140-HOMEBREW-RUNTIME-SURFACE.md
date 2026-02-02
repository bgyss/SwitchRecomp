# SPEC-140: Homebrew Runtime Surface

## Status
Draft v0.2

## Purpose
Define the runtime ABI surface required to boot a recompiled homebrew title and satisfy the Switch homebrew ABI expectations.

## Goals
- Provide a minimal, deterministic runtime surface for homebrew startup.
- Map required loader configuration fields into the runtime environment.
- Establish a clear contract for unsupported services.

## Non-Goals
- Re-implementing the full Horizon OS service set.
- Supporting dynamically loaded NROs at runtime.

## Background
The Switch homebrew ABI defines how NRO entrypoints receive a loader configuration and which config entries must be present at startup, including EndOfList, MainThreadHandle, and AppletType. citeturn0view0
It also defines the register arguments used for NRO entrypoints. citeturn0view0

## Requirements
- The runtime must provide an entrypoint shim that invokes the recompiled NRO entrypoint with:
  - X0 pointing to the loader configuration structure.
  - X1 set to 0xFFFFFFFFFFFFFFFF for NROs. citeturn0view0
- The runtime must populate loader config entries for EndOfList, MainThreadHandle, and AppletType at a minimum. citeturn0view0
- The runtime must surface loader config entries for optional fields (Argv, OverrideHeap, AllocPages, LockRegion) when present, and fail with a clear error if a required field is missing. citeturn0view0
- The runtime must provide a stable, deterministic time source and input event queue to minimize nondeterminism in validation.
- The runtime must document which Horizon OS services are stubbed and which are implemented, with a hard failure for unsupported calls.

## Interfaces and Data
- Runtime ABI struct definitions for loader config entries.
- A generated `runtime_manifest.json` describing:
  - Supported loader config keys.
  - Stubbed services and behavior.
  - Determinism knobs (time, input).

## Deliverables
- Entry shim implementation for the homebrew ABI.
- Loader config builder.
- Runtime service capability documentation.

## Open Questions
- Should we map libnx service calls through a thin compatibility layer or directly implement a minimal subset?
- What is the smallest deterministic input/timing surface that still allows real gameplay?

## Acceptance Criteria
- A recompiled homebrew binary boots and reaches its main loop with the runtime providing required loader config keys.
- Unsupported services fail with an explicit, logged error that references the missing service.
- The runtime manifest enumerates which loader config keys were provided for a run.

## Risks
- Some homebrew titles may assume additional loader config keys not covered by the minimum set.
- Overly strict service stubs may block otherwise runnable titles.

## References
- https://switchbrew.org/wiki/Homebrew_ABI
