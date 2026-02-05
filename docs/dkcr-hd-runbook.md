# DKCR HD macOS/aarch64 Runbook (Scaffold)

This runbook documents a reproducible build and run loop for the SPEC-200 scaffold on
macOS/aarch64. It uses placeholder inputs and does not bundle any retail assets.

## Prerequisites
- macOS on Apple Silicon (aarch64).
- Rust toolchain installed via `rustup`.
- Optional: Nix + devenv for the repo dev shell.

## Build and Run
1. Enter the dev shell (optional).

```
nix develop --impure
```

2. Run the pipeline for the DKCR HD sample.

```
cargo run -p recomp-cli -- run \
  --module samples/dkcr-hd/module.json \
  --config samples/dkcr-hd/title.toml \
  --provenance samples/dkcr-hd/provenance.toml \
  --out-dir out/dkcr-hd
```

3. Build the emitted project.

```
cargo build --manifest-path out/dkcr-hd/Cargo.toml
```

4. Run the emitted binary.

```
cargo run --manifest-path out/dkcr-hd/Cargo.toml
```

5. Capture a validation run and compare against the reference timeline.

```
scripts/capture-video-macos.sh artifacts/dkcr-hd
ffmpeg -i artifacts/dkcr-hd/capture.mp4 artifacts/dkcr-hd/frames/%08d.png
ffmpeg -i artifacts/dkcr-hd/capture.mp4 -vn -acodec pcm_s16le artifacts/dkcr-hd/audio.wav

recomp-validation hash-frames --frames-dir artifacts/dkcr-hd/frames --out artifacts/dkcr-hd/frames.hashes
recomp-validation hash-audio --audio-file artifacts/dkcr-hd/audio.wav --out artifacts/dkcr-hd/audio.hashes

cp samples/capture_video.toml artifacts/dkcr-hd/capture.toml
# Edit artifacts/dkcr-hd/capture.toml to point at the capture hashes above.

recomp-validation video \
  --reference samples/reference_video.toml \
  --capture artifacts/dkcr-hd/capture.toml \
  --out-dir artifacts/dkcr-hd/validation

scripts/validation_artifacts_init.sh --out artifacts/dkcr-hd/artifacts.json
# Edit artifacts/dkcr-hd/artifacts.json to point at intake/pipeline manifests.
scripts/validate_artifacts.sh --artifact-index artifacts/dkcr-hd/artifacts.json
```

Note: The automated validation loop is paused until SPEC-210/220/230/240 are implemented.

## External Assets
- RomFS assets are expected at `game-data/dkcr-hd/romfs`.
- Replace placeholder inputs under `samples/dkcr-hd/inputs/` with real artifacts in a
  private workspace before attempting full DKCR HD validation.
