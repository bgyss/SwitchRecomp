# Legal Use and Asset Separation Policy

This project is a preservation-focused research effort. It does not distribute proprietary assets, keys, or copyrighted binaries.

## Legal Acquisition
- Users must obtain any inputs (binaries, assets, metadata) through lawful means.
- The project does not provide instructions for bypassing protections or extracting data from hardware.
- Inputs must be accompanied by provenance metadata that records how they were obtained.

## Asset Separation
- Recompiled output must not include proprietary assets.
- User-supplied assets remain outside the packaged output and are loaded at runtime.
- Build and packaging artifacts must clearly separate code from assets.

## Prohibited Content
- Encryption keys, DRM circumvention tools, or instructions to bypass protections.
- Proprietary binaries, game assets, or copyrighted media.
- Links to hosted proprietary assets or piracy resources.

## Provenance Guardrails
- The toolchain requires a validated `provenance.toml` file before building.
- Inputs are hashed and checked against the declared metadata.
- Missing or mismatched provenance data causes the build to fail.

## Contributor Responsibilities
- Do not add or reference proprietary content in issues, code, or documentation.
- Keep inputs and outputs strictly separated in specs and tooling.
- Prefer primary, public sources when documenting technical research.
