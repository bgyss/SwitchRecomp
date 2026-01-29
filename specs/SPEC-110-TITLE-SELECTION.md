# SPEC-110: Target Title Selection Criteria

## Status
Draft v0.2

## Purpose
Define criteria for choosing an initial preservation‑safe target title for static recompilation.

## Goals
- Select a title that is legally and ethically suitable for preservation research.
- Minimize technical complexity for the first end‑to‑end milestone.
- Ensure the selection yields high learning value for the pipeline and runtime.

## Non-Goals
- Selecting a title based on commercial popularity.
- Providing or distributing proprietary assets or binaries.

## Selection Criteria
### Legal and Preservation Fit
- Title is no longer commercially available or has clear preservation value.
- Community consensus supports preservation without enabling piracy.
- The project can document lawful acquisition of required inputs.

### Technical Feasibility
- Minimal use of online services and networking features.
- Limited reliance on specialized peripherals.
- Small to moderate code and asset footprint.
- Uses standard service calls (hid, audout, fs, applet) with minimal extras.

### Pipeline Learning Value
- Exercises core instruction coverage without heavy use of obscure ISA extensions.
- Has predictable startup flow to validate service stubs and timing.
- Contains a renderable scene for early GPU validation.

### Reproducibility
- Stable version identifiers and clean region variants.
- Availability of reference traces that can be collected privately.

## Evaluation Checklist
- [ ] Legal and preservation rationale documented.
- [ ] Minimal service dependency map created.
- [ ] Required instruction coverage estimated.
- [ ] GPU feature usage profiled.
- [ ] Asset separation plan validated.

## Deliverables
- A short-list of 2–3 candidate titles with pros/cons.
- A final selection memo and rationale.
- A baseline trace plan for validation.

## Open Questions
- What constitutes “preservation‑safe” for this project’s policy?
- How to verify service usage without distributing traces?

## Acceptance Criteria
- A documented selection that satisfies all checklist items.
- A published plan for obtaining inputs legally and privately.
