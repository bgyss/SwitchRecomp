# Reference Media Normalization

Reference videos may come from different sources and formats. Normalization ensures comparisons
are stable and predictable.

## Canonical Profile
- Resolution: 1280x720
- Frame rate: 30 fps, constant (CFR)
- Audio: 48 kHz PCM, 2 channels

## Normalization Workflow
1. Transcode the source to the canonical profile.
2. Extract frames and audio from the normalized output.
3. Generate frame and audio hash lists.
4. Record normalization metadata and hashes in `reference_video.toml`.

## Scripted Pipeline
`scripts/normalize-reference-video.sh` runs the full workflow.

```bash
scripts/normalize-reference-video.sh /path/to/source.mov artifacts/reference
```

Outputs:
- `artifacts/reference/reference-normalized.mp4`
- `artifacts/reference/frames/` (PNG frames)
- `artifacts/reference/audio.wav`
- `artifacts/reference/frames.hashes`
- `artifacts/reference/audio.hashes`

## Storage Policy
Reference media stays outside the repo. Only hashes and metadata are tracked.

## Notes
If the source is variable frame rate, normalize to constant fps before hashing.
Record the normalization profile and source path in `[normalization]` within
`reference_video.toml`.
