#!/usr/bin/env bash
set -euo pipefail

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg is required for capture. Install it with 'brew install ffmpeg'." >&2
  exit 1
fi

OUT_DIR=${1:-artifacts/capture}
DURATION_SECONDS=${DURATION_SECONDS:-30}
FPS=${FPS:-30}
VIDEO_SIZE=${VIDEO_SIZE:-1280x720}
VIDEO_DEVICE=${VIDEO_DEVICE:-1}
AUDIO_DEVICE=${AUDIO_DEVICE:-0}

mkdir -p "$OUT_DIR"

ffmpeg \
  -f avfoundation \
  -framerate "$FPS" \
  -video_size "$VIDEO_SIZE" \
  -i "${VIDEO_DEVICE}:${AUDIO_DEVICE}" \
  -t "$DURATION_SECONDS" \
  "$OUT_DIR/capture.mp4"

echo "Capture complete: $OUT_DIR/capture.mp4"
