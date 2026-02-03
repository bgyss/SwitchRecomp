# Video Validation Workflow

This workflow compares a reference gameplay video against a captured run using deterministic hash lists. The comparison is coarse, intended to detect large visual or audio regressions.

## Inputs
- `reference_video.toml`: reference video metadata, timeline, hash sources, and thresholds.
- `capture_video.toml`: captured video metadata and hash sources.
- Frame hash inputs:
  - A list file (`format = "list"`) with one hash per line, in frame order.
  - A directory (`format = "directory"`) of frame images hashed in filename order.
- Audio hash inputs:
  - A list file (`format = "list"`) with one hash per chunk.
  - A raw file (`format = "file"`) hashed in fixed chunks (4096 bytes).

## Reference Config
Use `samples/reference_video.toml` as a template. Capture configs are similar but only need `[video]` and `[hashes]`.

## Hash Generation
Generate hash lists from deterministic inputs:

```bash
recomp-validation hash-frames --frames-dir artifacts/frames --out artifacts/frames.hashes
recomp-validation hash-audio --audio-file artifacts/audio.wav --out artifacts/audio.hashes
```

If you already have precomputed hashes, point `hashes.frames` or `hashes.audio` at the list files directly.

## Comparison
Run the comparison and emit `validation-report.json`:

```bash
recomp-validation video \
  --reference reference_video.toml \
  --capture capture_video.toml \
  --out-dir artifacts/validation
```

## Report Fields
The JSON report includes:
- `video.status`: overall pass/fail.
- `video.frame_comparison`: matched/compared counts, match ratio, and frame offset.
- `video.audio_comparison`: audio match ratio and chunk drift (if provided).
- `video.drift`: frame and audio drift summary.
- `video.failures`: threshold violations.

## Thresholds
Thresholds are configured in `reference_video.toml`. Defaults are:
- `frame_match_ratio = 0.92`
- `audio_match_ratio = 0.90`
- `max_drift_frames = 3`
- `max_dropped_frames = 0`

Tune thresholds per title and keep the drift window small to avoid false positives.
