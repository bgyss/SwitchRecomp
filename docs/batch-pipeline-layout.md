# Sample Pipeline Layout

This is a recommended directory layout for batch execution. Adjust names to match
local conventions, but keep the separation of inputs, captures, metrics, and reports.

```
workspace/
  manifests/
    batch-2026-02-03.json
  inputs/
    provenance/
      TID-0001.toml
    traces/
      TRACE-001.json
    references/
      REF-001.mp4
  builds/
    TID-0001/
      recomp/
        Cargo.toml
      logs/
        build.log
  runs/
    TID-0001/
      captures/
        recomp.mp4
      metrics/
        ssim.log
        psnr.log
        vmaf.json
        ref_ebur128.log
        test_ebur128.log
      reports/
        summary.json
        summary.txt
      perf/
        frame_times.csv
        gpu_stats.json
```

## Per-title pipeline stages
1. Intake
   - Validate provenance file.
   - Verify required reference captures and input traces exist.
2. Build
   - Run recompilation and capture build logs.
3. Run + capture
   - Execute with deterministic input trace.
   - Store capture video/audio and raw logs.
4. Compare
   - Run A/V metrics and produce summary.
5. Performance profile
   - Capture frame-time and resource metrics.
6. Report
   - Emit per-title summary and update manifest status.
