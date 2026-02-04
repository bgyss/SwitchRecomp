# Research Directions and Required Research

This document lists research topics needed to complete the specs and deliver a working static recompilation pipeline.

## Core Research Directions

### 1) Binary Formats and Loading
- Identify Switch executable formats used by retail titles.
- Map sections, relocations, and symbol handling.
- Determine how overlays or dynamic code segments are represented.

Needed research:
- Public documentation or reverse engineering notes on Switch executable formats.
- Tools and libraries that can parse these formats reliably.

### 2) CPU ISA and Semantics
- Confirm the exact ARMv8-A subset used by Switch titles.
- Enumerate required SIMD and floating point instructions.
- Define precise flag and exception semantics for those instructions.

Needed research:
- ARMv8-A reference manuals.
- Instruction usage analysis on sample titles.

### 3) OS, Services, and IPC
- Identify essential system services required for boot and main loop.
- Define a minimal service surface for early milestones.
- Determine which services can be stubbed without breaking games.

Needed research:
- Public technical references on Switch OS services.
- Trace-based profiling of service calls for selected titles.

### 4) GPU and Graphics
- Confirm GPU architecture and the command stream interface used by games.
- Assess feasibility of command stream interpretation vs shader translation.
- Identify core texture formats and shader features.

Needed research:
- GPU architecture references.
- Shader translation tooling that can be adapted.

### 5) Timing and Determinism
- Determine which timing-sensitive behaviors must be modeled.
- Identify stable methods for trace collection and replay.

Needed research:
- Prior work on deterministic replay in emulation or recompilation.
- Instrumentation approaches for low overhead tracing.

### 6) Audio, Input, and I/O
- Identify the minimal audio path needed for gameplay.
- Map controller input handling to the runtime.
- Define file system access boundaries and safety rules.

Needed research:
- Audio subsystem assumptions in Switch titles.
- Input service behavior and common usage patterns.

### 7) Legal and Preservation Policy
- Define acceptable inputs, distributions, and legal compliance expectations.
- Ensure documentation is consistent with preservation goals.
- Follow and update `docs/LEGAL-POLICY.md` as policy decisions are finalized.

Needed research:
- Jurisdiction-specific rules affecting preservation.
- Best practices for open source preservation tooling.

## Seed Resources (Reviewed)
- Jamulator write-up on static recompilation pitfalls and concurrency: https://andrewkelley.me/post/jamulator.html
- N64Recomp repository for pipeline patterns: https://github.com/N64Recomp/N64Recomp
- Dinosaur Planet recomp for asset separation precedent: https://github.com/DinosaurPlanetRecomp/dino-recomp
- Nintendo Switch platform baseline: https://en.wikipedia.org/wiki/Nintendo_Switch
- Tegra X1 whitepaper: https://www.nvidia.com/content/tegra/embedded-systems/pdf/tegra-x1-whitepaper.pdf
- Switch hardware overview: https://switchbrew.org/wiki/Hardware
- Switch homebrew NRO format: https://switchbrew.org/wiki/NRO
- Switch NSO format and compression: https://switchbrew.org/wiki/NSO
- Homebrew ABI entrypoint and loader config: https://switchbrew.org/wiki/Homebrew_ABI
- NACP title metadata format: https://switchbrew.org/wiki/NACP
- Switch NCA container format: https://switchbrew.org/wiki/NCA
- hactool (XCI/NCA extraction and keyset handling): https://github.com/SciresM/hactool
- hactoolnet (XCI/NCA extraction with user keys): https://github.com/Thealexbarney/hactoolnet
- nstool (XCI/NCA/NSO extraction): https://github.com/jakcron/nstool
- Ghidra SLEIGH language reference (p-code semantics): https://github.com/NationalSecurityAgency/ghidra/blob/master/GhidraDocs/languages/html/sleigh.html
- sleigh library (p-code lifting implementation): https://github.com/lifting-bits/sleigh
- Resurrecting Crimsonland (banteg, 2026-02-01): headless Ghidra pipeline with evidence-backed rename map and regen loop, plus runtime instrumentation (WinDbg/cdb, Frida) for behavior capture; useful automation and validation ideas. https://banteg.xyz/posts/crimsonland/

## Research Deliverables
- A research summary for each category with sources.
- A list of confirmed requirements to update the specs.
- A compatibility matrix based on real title traces.

## Open Research Questions
- What is the minimal instruction coverage needed for a first title?
- Which OS services are required to reach a game loop without patches?
- What is the simplest graphics path that still produces correct output?
- How can we generate reference traces without distributing proprietary content?
