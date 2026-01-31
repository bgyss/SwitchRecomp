# SPEC-010: Target Platform Baseline

## Status
Draft v0.3

## Purpose
Define the hardware and platform baseline for Switch static recompilation and runtime support.

## Goals
- Establish a minimum target platform profile for correct execution.
- Identify the hardware features the runtime must model or virtualize.

## Non-Goals
- Supporting every future or hypothetical hardware revision.
- Full hardware emulation accuracy beyond what is needed for correct execution.

## Baseline Requirements
- CPU: ARMv8-A, 64-bit, 4x Cortex-A57 as used by retail Switch titles.
- GPU: NVIDIA Maxwell-based GM20B class GPU.
- Memory: 4 GB LPDDR4/LPDDR4X with Switch-class bandwidth and latency constraints.
- Storage: Game data read via user-supplied assets, not bundled.
- Timing: Handheld vs docked performance modes, with a runtime-configurable policy.

## Compatibility Matrix
- Base Switch (Tegra X1, 20 nm).
- Switch OLED / revised units (Tegra X1+ / Mariko, 16 nm).
- Handheld and docked clocks and thermal behaviors treated as runtime modes, not separate builds.

## Baseline Profile Artifact
- `docs/target-platform-baseline.toml` is the structured baseline profile.
- `docs/target-platform-assumptions.md` lists testable assumptions and dependent specs.

## Platform Assumptions
- A57 core set is sufficient; A53 cores are not used by retail titles.
- GPU feature set is Maxwell-class and should be treated as the minimum renderer target.

## Deliverables
- A written baseline profile and compatibility matrix.
- A list of platform assumptions required by the runtime.
- A runtime timing-mode configuration stub (handheld vs docked).

## Open Questions
- Which hardware behaviors must be reproduced for correctness?
- Are any hardware features optional for initial milestones?

## Acceptance Criteria
- A baseline profile that is stable and usable by other specs.
- A documented list of assumptions that can be tested.

## References
- https://en.wikipedia.org/wiki/Nintendo_Switch
- https://www.nvidia.com/content/tegra/embedded-systems/pdf/tegra-x1-whitepaper.pdf
- https://switchbrew.org/wiki/Hardware
