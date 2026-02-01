# Exploratory Pipeline Notes

This document captures the initial exploratory pipeline that mirrors proven static recomp workflows while staying aligned with the specs.

## Scope
- Parse a small JSON module format as a stand-in for lifted code.
- Apply a TOML config to select syscall stub behaviors.
- Emit a Rust project that links against a runtime ABI crate.

## Why This Shape
- Mirrors the deterministic, file-driven flow seen in static recomp projects.
- Keeps a strict boundary between inputs and outputs.
- Produces compilable artifacts for validation and iteration.

## Inputs
- `module.json` describes a module, functions, and operations.
- `title.toml` provides the title name, entry function, ABI version, and stub map.
- `provenance.toml` records lawful input provenance and format metadata.
- Homebrew intake can emit a separate `module.json` + `manifest.json` with segment blobs and assets extracted from NRO/NSO inputs.
- Homebrew RomFS assets are emitted as `assets/romfs/romfs.bin`; runtime implementations should mount this image when wiring up RomFS access.

Example stub map:
```
[stubs]
svc_log = "log"
svc_sleep = "noop"
svc_fatal = "panic"
```

Example runtime config:
```
[runtime]
performance_mode = "handheld"
```

## Outputs
- A self-contained Rust crate in the output directory.
- The crate depends on `recomp-runtime` via a relative path.
- The emitted `main.rs` invokes the entry function and records the ABI version.
- A `manifest.json` file with input hashes, provenance hash, and generated file list.

## Next Steps
- Add a real input parser for Switch binaries.
- Replace the JSON module with lifted IR from the pipeline.
- Expand runtime services and ABI validation.
