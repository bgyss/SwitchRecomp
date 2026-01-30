# Title Shortlist

This shortlist prioritizes preservation-safe, open-source homebrew titles with redistributable assets and minimal service dependencies. Each candidate requires license verification before selection.

## Candidate A: 2048-style Homebrew (e.g., 2048NX)
Pros:
- Small codebase and minimal assets.
- Limited service usage (input + simple graphics).
- Easy to validate startup flow and rendering.

Cons:
- May not exercise audio or filesystem services.
- Limited instruction coverage.

Service dependencies:
- `hid`, `vi`/`nv`, optional `applet` for lifecycle.

Instruction coverage estimate:
- Integer + branch heavy; minimal SIMD usage.

## Candidate B: Snake-style Homebrew (e.g., SnakeNX)
Pros:
- Simple gameplay loop with consistent input + timing.
- Modest graphics requirements.

Cons:
- Minimal OS service coverage.
- Often limited to 2D rendering path.

Service dependencies:
- `hid`, `vi`/`nv`, optional `applet`.

Instruction coverage estimate:
- Integer + branch; small memory footprint.

## Candidate C: Libnx Graphics Demo (triangle/texture sample)
Pros:
- Explicit GPU usage for early graphics validation.
- Small asset footprint.

Cons:
- Not a full game loop; limited gameplay or input coverage.

Service dependencies:
- `vi`/`nv`, minimal `applet`.

Instruction coverage estimate:
- Graphics setup + minimal logic; low CPU instruction variety.
