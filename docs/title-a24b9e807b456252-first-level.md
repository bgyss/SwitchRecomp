# title-a24b9e807b456252 First-Level Scaffolding

This document describes the SPEC-200 scaffolding for a macOS/aarch64 first-level milestone. It keeps all proprietary inputs external and provides placeholders for configuration, patches, and validation.

## Scope
- Provide title-specific config and patch placeholders.
- Capture a stub service list for first-level bring-up.
- Record GPU and validation notes for early testing.
- Document a reproducible macOS/aarch64 build/run flow using existing tooling.

## Scaffolding Files
- `samples/title-a24b9e807b456252/title.toml`
- `samples/title-a24b9e807b456252/provenance.toml`
- `samples/title-a24b9e807b456252/module.json`
- `samples/title-a24b9e807b456252/patches/patches.toml`

## Title Config (samples/title-a24b9e807b456252/title.toml)
- Required config fields are set (`title`, `entry`, `abi_version`).
- `stubs` lists the initial service stub behaviors.
- `runtime.performance_mode` is set to `docked` for first-level capture parity.
- `assets` and `keys` use absolute external paths.
- `inputs`, `patches`, and `validation` provide placeholders for the real pipeline.

## Patch Set Placeholders (samples/title-a24b9e807b456252/patches/patches.toml)
- `skip_intro_cutscene` is a placeholder branch patch entry.
- `force_debug_logging` is a placeholder config override entry.
- Replace offsets and targets once the lifted module and symbol locations are known.

## Service Stub List
The current stub map in `samples/title-a24b9e807b456252/title.toml` is:
- `svc_log`: `log`
- `svc_sleep`: `noop`
- `sm:`: `log`
- `sm:m`: `log`
- `fs`: `log`
- `fs:ldr`: `log`
- `fs:pr`: `log`
- `hid`: `log`
- `irs`: `log`
- `audout`: `log`
- `audren`: `log`
- `appletOE`: `log`
- `acc:u0`: `log`
- `ns:am2`: `log`
- `nifm:u`: `noop`

## GPU Notes
- Target Maxwell-like behavior per `docs/target-platform-baseline.toml` and map to Metal on macOS/aarch64.
- Track first-level draw call ordering, render target formats, and texture swizzles as likely early blockers.
- Capture GPU checksum deltas before enabling aggressive optimizations.

## Validation Notes
- Use the `validation` section in `title.toml` to define the external reference video segment.
- Expect frame pacing variance; allow modest timing drift in the first-level comparison window.
- Continue running the baseline validation suite via `recomp-validation` for regressions unrelated to title-a24b9e807b456252.
- Capture with `scripts/capture-validation.sh` (or `scripts/capture-video-macos.sh`) and store outputs outside the repo.
- Track validation artifacts with `docs/validation-artifacts.md` and an artifact index JSON.
- Capture device settings (resolution, fps, audio rate) alongside each report.
- Run artifact-index validation with `scripts/validate_artifacts.sh`.

## macOS/aarch64 Build and Run
These steps use the existing pipeline tooling and assume you have supplied external assets and updated the placeholders.

1. Enter the dev shell (optional but recommended on macOS):
```
nix develop --impure
```

2. Update external paths and hashes:
- Edit `samples/title-a24b9e807b456252/title.toml` to point to external assets and keys.
- Edit `samples/title-a24b9e807b456252/provenance.toml` with real input hashes and sizes for:
  - XCI (`format = "xci"`).
  - Keyset (`format = "keyset"`).
  - Reference video (`format = "video_mp4"`).
- Replace `samples/title-a24b9e807b456252/module.json` with lifted output when available.

2a. (Optional) Extract ExeFS and RomFS from a real XCI using external tooling:
See [Real XCI intake how-to](xci-intake.md#real-xci-intake-how-to) for the detailed steps and CLI notes.
Use `recomp-cli xci-validate` or `scripts/xci_validate.sh` to confirm the intake manifest.
```
cargo run -p recomp-cli -- xci-intake \
  --xci /Volumes/External/title-a24b9e807b456252/xci/0123456789abcdef.xci \
  --keys /Volumes/External/SwitchKeys/prod.keys \
  --provenance samples/title-a24b9e807b456252/provenance.toml \
  --out-dir out/title-a24b9e807b456252-intake \
  --assets-dir out/title-a24b9e807b456252-assets \
  --xci-tool hactool \
  --xci-tool-path /usr/local/bin/hactool
```

3. Run the pipeline:
```
cargo run -p recomp-cli -- run \
  --module samples/title-a24b9e807b456252/module.json \
  --config samples/title-a24b9e807b456252/title.toml \
  --provenance samples/title-a24b9e807b456252/provenance.toml \
  --out-dir out/title-a24b9e807b456252
```

4. Build the emitted project:
```
cargo build --manifest-path out/title-a24b9e807b456252/Cargo.toml
```
