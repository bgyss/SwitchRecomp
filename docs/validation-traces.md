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
- Run the capture and comparison pipeline to produce `summary.json`.
- Review the aligned captures and note the observed timecodes for each reference event
  in `reference_video.toml`.
- Save observed timecodes in an `event_observations.json` file (outside the repo) and
  re-run validation to compute drift metrics.
- Flag any event drift or audio/video mismatches that exceed thresholds for follow-up.
