# SPEC-000: Project Charter and Ethics

## Status
Draft v0.2

## Purpose
Define the mission, scope, ethics, and constraints for the Switch static recompilation preservation project.

## Goals
- Enable preservation-oriented static recompilation of Switch game binaries to native code.
- Keep a strict separation between recompiled code and proprietary assets.
- Provide a lawful and ethical framework that avoids distributing protected content.

## Non-Goals
- Circumventing DRM or device security.
- Distributing proprietary assets, keys, or copyrighted binaries.
- Providing instructions for bypassing protections.

## Scope
- Recompile code and provide a runtime that can execute recompiled output.
- Require users to supply their own legally obtained binaries and assets.
- Prioritize preservation and research over gameplay convenience.

## Ethical Constraints
- Document an explicit policy for legal acquisition and user responsibility.
- Avoid hosting or linking to proprietary content.
- Require provenance metadata for input artifacts.

## Deliverables
- A recompiler toolchain.
- A runtime library and host platform integration.
- A documentation set describing requirements and limitations.

## Open Questions
- What is the approved legal acquisition path for binaries and assets?
- Which jurisdictions must be considered for preservation rules?

## Acceptance Criteria
- A published policy on legal use and asset separation.
- A tooling architecture that does not embed or require proprietary assets.

## Risks
- Legal and policy ambiguity.
- Community confusion about allowed inputs or outputs.
