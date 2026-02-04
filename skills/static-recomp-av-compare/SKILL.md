---
name: static-recomp-av-compare
description: Compare reference and recompiled audio/video outputs with alignment, metrics, and thresholds. Use when validating visual or audio fidelity, computing similarity metrics, or generating automated A/V comparison reports.
---

# Static Recomp A/V Compare

## Overview
Align and compare reference captures against recompiled outputs using repeatable metrics and clear pass/fail thresholds.

## Workflow
1. Normalize inputs.
   - Match resolution, frame rate, and aspect ratio.
   - Match audio sample rate, channel layout, and length.
2. Align timelines.
   - Use a known sync event (boot logo, sound cue, scene transition).
   - Apply a fixed offset if one stream starts earlier.
   - Verify alignment with a short manual check before running full metrics.
3. Compute video similarity.
   - Use SSIM and PSNR for fast regression checks.
   - Use VMAF when perceptual quality is critical.
4. Compute audio similarity.
   - Compare loudness (EBU R128) and true peak.
   - Inspect for drift or missing segments.
5. Summarize results.
   - Produce per-scene metrics and overall aggregates.
   - Flag outliers for manual review.

## Automation scripts
1. Compare a single scene with `scripts/compare_av.py`.
2. Batch multiple scenes with `scripts/batch_compare_av.py` and a manifest.
3. Convert `summary.json` to pass/fail with `scripts/check_summary.py`.

### Single scene
```bash
python3 "$CODEX_HOME/skills/static-recomp-av-compare/scripts/compare_av.py" \
  --ref ref.mp4 \
  --test recomp.mp4 \
  --out-dir out/scene-01 \
  --label "scene-01" \
  --width 1920 \
  --height 1080 \
  --fps 60 \
  --audio-rate 48000 \
  --offset 0.250 \
  --trim-start 5.0 \
  --duration 30.0
```

### Threshold check
```bash
python3 "$CODEX_HOME/skills/static-recomp-av-compare/scripts/check_summary.py" \
  out/scene-01/summary.json \
  --thresholds thresholds/default.json
```

### Batch run
```bash
python3 "$CODEX_HOME/skills/static-recomp-av-compare/scripts/batch_compare_av.py" \
  manifests/av-batch.json
```

Notes:
- Requires `ffmpeg` on PATH. The scripts will use `libvmaf` if available.
- Use `--no-vmaf` to skip VMAF.
- See `references/av-batch-manifest.md` for manifest schema and example.
- A baseline thresholds file is provided at `references/default-thresholds.json`.

## Outputs
- Per-scene metrics (SSIM, PSNR, VMAF, loudness).
- A summary report with pass/fail thresholds and top mismatches.
- Links to aligned captures used for comparison.

## References
- Batch manifest: `references/av-batch-manifest.md`
- Default thresholds: `references/default-thresholds.json`

## Quality bar
- Alignment must be verified before metric runs.
- Metrics must be repeatable and tied to explicit thresholds.
- Visual or audio mismatches must be paired with evidence artifacts.
