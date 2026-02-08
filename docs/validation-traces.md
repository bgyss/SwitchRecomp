# Golden Trace Guidance

This project does not distribute proprietary assets or traces derived from copyrighted content.

## How to Add New Traces
- Collect traces privately from legally obtained inputs.
- Store only non-proprietary summaries (hashes, event counts, timing stats).
- Do not commit raw traces that embed copyrighted data.

## Recommended Metadata
- Title identifier and version.
- Tool versions used for tracing.
- Hashes of input binaries/assets (recorded in provenance).

## Video Validation Manual Review
When using video-based validation, record timing observations separately from the raw
captures.

Manual steps:
- Run the hash-based validation pipeline to produce `validation-report.json`.
- Review the drift summary and triage categories in the report.
- Note any expected mismatches in provenance metadata for follow-up.

## Capture Workflow (macOS)
Use an external capture path and keep outputs outside the repo.

Suggested workflow:
- Use the helper script to capture and hash a fixed-duration run (preferred):
```
scripts/capture-validation.sh --out-dir /Volumes/External/Captures/title-a24b9e807b456252-first-level \
  --duration 360 --fps 60 --video-device 1 --audio-device 0 --resolution 1920x1080
```
- Use `ffmpeg -f avfoundation -list_devices true -i \"\"` to list available device indices.
- Optional: use `scripts/capture_video.sh` if you want a capture-only helper.
- Launch the recompiled runtime and reach the target segment.
- Capture the primary display (or a specific window) with `ffmpeg`:
```
ffmpeg -f avfoundation -framerate 60 -i \"1:0\" -t 360 -pix_fmt yuv420p \
  /Volumes/External/Captures/title-a24b9e807b456252-first-level.mp4
```
- Replace the device index (`1:0`) with the correct screen/audio device for your setup.
- Record the capture path and hashes in provenance metadata.
- Create or update a validation artifact index:
```
scripts/validation_artifacts_init.sh --out /Volumes/External/validation/artifacts.json
```
- Run validation using the artifact index:
```
scripts/validate_artifacts.sh --artifact-index /Volumes/External/validation/artifacts.json
```

For artifact layouts and dependency notes, see `docs/validation-artifacts.md`.
For hash-based comparison steps, see `docs/validation-video.md`.
