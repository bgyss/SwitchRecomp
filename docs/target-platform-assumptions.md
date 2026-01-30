# Target Platform Assumptions

These assumptions capture the baseline expectations used by the runtime and pipeline. Each item should be testable or revisited when new data arrives.

## Assumptions
- CPU: Retail titles execute on the 4x Cortex-A57 cluster (A53 cores are unused).
- GPU: Maxwell-class GM20B feature set is the minimum renderer target.
- Memory: 4 GB LPDDR4/LPDDR4X with Switch-class latency constraints.
- Timing: Handheld vs docked performance are runtime modes, not separate builds.

## Spec Dependencies
- SPEC-010 (Target Platform Baseline): baseline profile and compatibility matrix.
- SPEC-060 (GPU/Graphics): minimum renderer target and GPU feature coverage.
- SPEC-080 (Timing/Interrupts): timing policy derived from handheld/docked modes.
