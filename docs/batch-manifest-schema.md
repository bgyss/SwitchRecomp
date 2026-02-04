# Batch Manifest Schema (v1)

Machine-validated JSON schema: `docs/batch-manifest-schema.json`.

This schema is intended for catalog-scale batch runs. It records per-title inputs,
validation targets, and status. Store as JSON or TOML; keys below are canonical.

## Top-level
- `schema_version` (string, required): Use `v1`.
- `batch_id` (string, required): Stable identifier for the run.
- `created_at` (string, required): ISO 8601 timestamp.
- `toolchain` (object, required): Versions for the pipeline and tools.
- `global_defaults` (object, optional): Shared defaults for titles.
- `titles` (array, required): Per-title records.

## toolchain
- `pipeline_version` (string)
- `runtime_version` (string)
- `ffmpeg_version` (string)
- `emulator_version` (string, optional)

## global_defaults
- `resolution` (string, example: `1920x1080`)
- `frame_rate` (number, example: `60`)
- `audio_rate` (number, example: `48000`)
- `renderer_settings` (string)
- `metrics_thresholds` (object)

## titles[]
- `title_id` (string, required)
- `title_name` (string, required)
- `version` (string, required)
- `region` (string, required)
- `build_id` (string, optional)
- `inputs` (object, required)
- `validation` (object, required)
- `status` (object, required)
- `artifacts` (object, optional)

## inputs
- `provenance_record` (string, required): Path to provenance file.
- `reference_captures` (array, required): List of reference capture ids.
- `input_traces` (array, required): List of trace ids.

## validation
- `scene_list` (array, required): Scene ids used for comparison.
- `targets` (object, required): Per-title overrides for resolution, fps, audio.
- `metrics_thresholds` (object, optional): Per-title overrides.

## status
- `state` (string, required): `pending`, `running`, `passed`, `failed`, `needs_review`.
- `last_updated` (string, required): ISO 8601 timestamp.
- `notes` (string, optional)

## artifacts
- `report_path` (string, optional)
- `metrics_dir` (string, optional)
- `captures_dir` (string, optional)

## Example (JSON)
```json
{
  "schema_version": "v1",
  "batch_id": "switch-2026-02-03",
  "created_at": "2026-02-03T09:20:00Z",
  "toolchain": {
    "pipeline_version": "0.1.0",
    "runtime_version": "0.1.0",
    "ffmpeg_version": "6.1"
  },
  "global_defaults": {
    "resolution": "1920x1080",
    "frame_rate": 60,
    "audio_rate": 48000,
    "renderer_settings": "default",
    "metrics_thresholds": {
      "ssim_min": 0.95,
      "psnr_min": 35.0,
      "vmaf_min": 90.0,
      "audio_lufs_delta_max": 2.0
    }
  },
  "titles": [
    {
      "title_id": "TID-0001",
      "title_name": "Example Title",
      "version": "1.0.0",
      "region": "US",
      "build_id": "ABCD1234",
      "inputs": {
        "provenance_record": "provenance/TID-0001.toml",
        "reference_captures": ["REF-001"],
        "input_traces": ["TRACE-001"]
      },
      "validation": {
        "scene_list": ["SCN-001", "SCN-002"],
        "targets": {
          "resolution": "1920x1080",
          "frame_rate": 60,
          "audio_rate": 48000
        }
      },
      "status": {
        "state": "pending",
        "last_updated": "2026-02-03T09:20:00Z"
      },
      "artifacts": {
        "report_path": "reports/TID-0001/summary.json",
        "metrics_dir": "reports/TID-0001/metrics",
        "captures_dir": "captures/TID-0001"
      }
    }
  ]
}
```
