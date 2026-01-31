# Title Selection Memo

## Selected Title (Provisional)
- Open-source 2048-style homebrew (e.g., 2048NX).

## Rationale
- Preservation-safe profile with open-source code and minimal assets.
- Exercises core input + graphics without large service surface.
- Small binary size enables fast iteration and validation.

## Checklist Coverage
- Legal/preservation rationale: open-source codebase; requires license verification and asset redistribution confirmation.
- Minimal service dependency map: input (hid), graphics (vi/nv), optional filesystem for settings.
- Instruction coverage estimate: primarily integer + branch operations, modest SIMD usage.
- GPU feature usage: basic 2D/texture rendering, minimal shader complexity.
- Asset separation plan: assets are either embedded as open-source data or supplied separately by the user.

## Follow-ups
- Verify candidate license and asset redistribution terms.
- Collect private traces on a legally obtained binary.
- Expand service map once the title is confirmed.
