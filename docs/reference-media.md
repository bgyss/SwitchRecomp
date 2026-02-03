# Reference Media Normalization

Reference videos may come from different sources and formats. Normalization ensures comparisons
are stable and predictable.

## Canonical Profile (Planned)
- Resolution: 1280x720
- Frame rate: 30 fps
- Audio: 48 kHz PCM

## Normalization Steps
1. Trim the source to the first-level timeline.
2. Transcode to the canonical profile.
3. Generate frame and audio hash lists.
4. Record metadata in `reference_video.toml`.

## Storage Policy
Reference media stays outside the repo. Only hashes and metadata are tracked.

## Notes
If the source is variable frame rate, normalize to constant fps before hashing.
