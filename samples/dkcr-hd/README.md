# DKCR HD Sample (Scaffold)

This sample is a non-proprietary scaffold for SPEC-200. It mirrors the intended DKCR HD boot path
without bundling any retail assets or keys. All inputs here are placeholders with minimal magic
bytes so provenance validation can run.

## Files
- `module.json` is a minimal lifted module that invokes boot-related syscalls.
- `title.toml` records runtime, asset path, and stub mapping for the first-level milestone.
- `patches/first-level.toml` lists placeholder patches for a first-level boot path.
- `provenance.toml` tracks placeholder inputs (XCI, keyset, program NCA/ExeFS, NSO, NPDM).

## Asset Policy
- RomFS assets are external and are expected at `game-data/dkcr-hd/romfs`.
- Replace placeholder inputs in `inputs/` with real artifacts in a private workspace.
