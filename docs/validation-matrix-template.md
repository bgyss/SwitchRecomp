# Validation Matrix Template

Use this template to define measurable acceptance criteria per scene.

## Instructions
- Keep inputs and outputs separate and do not commit proprietary captures.
- Use user-provided or legally obtained reference captures.
- Record all tool versions and settings.

## Global Targets
- Resolution:
- Frame rate:
- Audio rate:
- Renderer settings:
- Input trace:
- Baseline hardware or emulator:

## Matrix
| Scene ID | Scene Description | Reference Source | Input Trace | Video Metrics (SSIM/PSNR/VMAF) | Audio Metrics (LUFS/Peak/Drift) | Perf Targets (avg/1%/0.1%) | Stability | Pass/Fail | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SCN-001 | Boot to main menu |  |  |  |  |  |  |  |  |
| SCN-002 | First playable loop |  |  |  |  |  |  |  |  |
| SCN-003 | UI overlay stress |  |  |  |  |  |  |  |  |

## Acceptance Criteria Guidance
- Video: specify minimum acceptable SSIM/PSNR/VMAF per scene.
- Audio: specify maximum drift (ms) and acceptable LUFS delta.
- Performance: specify budgets and allowable variance.
- Stability: no crashes or hangs in any scene.

## Evidence Checklist
- Reference capture path and metadata.
- Recompiled capture path and metadata.
- Metric logs and summary JSON.
- Any manual review notes with timestamps.
