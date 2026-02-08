# title-a24b9e807b456252 macOS/aarch64 Runbook (Scaffold)

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

2. Run the pipeline for the title-a24b9e807b456252 sample.

```
cargo run -p recomp-cli -- run \
  --module samples/title-a24b9e807b456252/module.json \
  --config samples/title-a24b9e807b456252/title.toml \
  --provenance samples/title-a24b9e807b456252/provenance.toml \
  --out-dir out/title-a24b9e807b456252
```

3. Build the emitted project.

```
cargo build --manifest-path out/title-a24b9e807b456252/Cargo.toml
```

4. Run the emitted binary.

```
cargo run --manifest-path out/title-a24b9e807b456252/Cargo.toml
```

5. Capture a validation run and compare against the reference timeline.

```
scripts/capture-video-macos.sh artifacts/title-a24b9e807b456252
ffmpeg -i artifacts/title-a24b9e807b456252/capture.mp4 artifacts/title-a24b9e807b456252/frames/%08d.png
ffmpeg -i artifacts/title-a24b9e807b456252/capture.mp4 -vn -acodec pcm_s16le artifacts/title-a24b9e807b456252/audio.wav

recomp-validation hash-frames --frames-dir artifacts/title-a24b9e807b456252/frames --out artifacts/title-a24b9e807b456252/frames.hashes
recomp-validation hash-audio --audio-file artifacts/title-a24b9e807b456252/audio.wav --out artifacts/title-a24b9e807b456252/audio.hashes

cp samples/capture_video.toml artifacts/title-a24b9e807b456252/capture.toml
# Edit artifacts/title-a24b9e807b456252/capture.toml to point at the capture hashes above.

recomp-validation video \
  --reference samples/reference_video.toml \
  --capture artifacts/title-a24b9e807b456252/capture.toml \
  --out-dir artifacts/title-a24b9e807b456252/validation

scripts/validation_artifacts_init.sh --out artifacts/title-a24b9e807b456252/artifacts.json
# Edit artifacts/title-a24b9e807b456252/artifacts.json to point at intake/pipeline manifests.
scripts/validate_artifacts.sh --artifact-index artifacts/title-a24b9e807b456252/artifacts.json
```

Note: The automated validation loop is paused until SPEC-210/220/230/240 are implemented.

## External Assets
- RomFS assets are expected at `game-data/title-a24b9e807b456252/romfs`.
- Replace placeholder inputs under `samples/title-a24b9e807b456252/inputs/` with real artifacts in a
  private workspace before attempting full title-a24b9e807b456252 validation.
