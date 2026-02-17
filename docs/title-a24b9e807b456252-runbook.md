# title-a24b9e807b456252 macOS/aarch64 Runbook (Scaffold)

This runbook documents a reproducible automation-first flow for SPEC-200 on macOS/aarch64.
It uses placeholders and does not bundle any retail assets.

## Prerequisites
- macOS on Apple Silicon (aarch64).
- Rust toolchain installed via `rustup`.
- Optional: `nix develop --impure`.
- External private workspace for XCI/key/reference/capture assets.

## Preferred Path: Automation Command
1. Prepare an automation config based on `samples/automation.toml` with title-specific paths.
2. Point inputs/outputs to private external roots.
3. Run:

```bash
cargo run -p recomp-cli -- automate --config /absolute/path/to/title-a24b9e807b456252-automation.toml
```

Outputs:
- `run-manifest.json`
- `validation-report.json`
- per-stage logs under configured log dir

## Manual Path (Fallback)
Use this only when debugging individual stages.

1. Intake:
```bash
cargo run -p recomp-cli -- xci-intake --xci ... --keys ... --provenance ... --out-dir ... --assets-dir ...
```

2. Lift/pipeline/build/run:
```bash
cargo run -p recomp-cli -- run --module ... --config ... --provenance ... --out-dir ...
cargo build --manifest-path out/title-a24b9e807b456252/Cargo.toml
```

3. Capture and validate:
```bash
scripts/capture-validation.sh --out-dir /absolute/capture/root --duration 360 --fps 60 --video-device 1 --audio-device 0 --resolution 1920x1080
cargo run -p recomp-validation -- artifacts --artifact-index /absolute/path/to/artifacts.json
```

## External Assets
- RomFS and reference/capture media remain external.
- Keep title IDs and paths hashed where possible.
- Do not commit raw captures, keys, or proprietary binaries.

## Related Docs
- `docs/automation-loop.md`
- `docs/validation-artifacts.md`
- `docs/title-a24b9e807b456252-validation-prereqs.md`
