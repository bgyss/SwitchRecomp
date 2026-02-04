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
- `module.json` describes a lifted module, functions, and operations.
- `title.toml` provides the title name, entry function, ABI version, and stub map.
- `provenance.toml` records lawful input provenance and format metadata.
- Homebrew intake emits a separate `module.json` + `manifest.json` with segment blobs and assets extracted from NRO/NSO inputs; this homebrew module.json is not consumed by the translator until a lifter produces a lifted module.json.
- The `homebrew-lift` command defaults to decoding a small AArch64 subset (mov wide, add, ret). Use `--mode stub` to emit a placeholder lifted module when decoding is not possible.
- Homebrew RomFS assets are emitted as a file tree under `assets/romfs/`; runtime implementations should mount or map this directory when wiring up RomFS access.

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

## Process Automation Ideas (Crimsonland Write-up)
- Build a deterministic analysis regen loop: drive a headless decompiler to export XML and decompile artifacts; treat exports as generated outputs and keep only inputs plus a rename/type map under version control.
- Maintain a structured `name_map.json` (or equivalent) where each rename/type entry includes address and evidence; reapply it to regenerate names and types consistently.
- Detect bundled third-party libraries via version strings and inject known headers/typedefs before decompilation to improve type recovery.
- Create a long-running runtime analysis session with log tailing so behavioral observations can be captured while keeping the debugger attached.
- Use runtime hooks to capture validation fixtures (framebuffer dumps, deterministic samples) and store them alongside provenance for later regression checks.
- Consider agent-assisted rename and pattern discovery backed by a curated knowledge base; only promote renames with evidence.

## Next Steps
- Add a real input parser for Switch binaries.
- Expand the lifter to cover more AArch64 instructions and control flow.
- Expand runtime services and ABI validation.

## Further Reading
- `docs/static-recompilation-flow.md` for a hypothetical end-to-end flow and verification plan.
