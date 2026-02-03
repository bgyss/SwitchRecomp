# A/V Batch Manifest (v1)

This manifest drives `scripts/batch_compare_av.py`.

## Schema
Top-level object:
- `schema_version` (string, required): use `v1`.
- `scenes` (array, required).

Scene entry keys:
- `id` (string, required)
- `label` (string, optional)
- `ref` (string, required): path to reference video
- `test` (string, required): path to recompiled video
- `out_dir` (string, required): output directory
- `width` (number, optional)
- `height` (number, optional)
- `fps` (number, optional)
- `audio_rate` (number, optional)
- `offset` (number, optional)
- `trim_start` (number, optional)
- `duration` (number, optional)
- `no_vmaf` (boolean, optional)
- `thresholds` (string, optional): path to thresholds JSON

## Thresholds JSON
Keys:
- `ssim_min` (number)
- `psnr_min` (number)
- `vmaf_min` (number)
- `audio_lufs_delta_max` (number)
- `audio_peak_delta_max` (number)

## Example
```json
{
  "schema_version": "v1",
  "scenes": [
    {
      "id": "SCN-001",
      "label": "boot-to-menu",
      "ref": "captures/ref/boot.mp4",
      "test": "captures/recomp/boot.mp4",
      "out_dir": "reports/boot",
      "width": 1920,
      "height": 1080,
      "fps": 60,
      "audio_rate": 48000,
      "offset": 0.25,
      "trim_start": 5.0,
      "duration": 30.0,
      "thresholds": "thresholds/default.json"
    }
  ]
}
```
