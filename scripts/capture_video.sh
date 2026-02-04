#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/capture_video.sh --out <path> [options]

Options:
  --out <path>           Output MP4 path (required).
  --duration <seconds>   Capture duration in seconds (default: 60).
  --fps <fps>            Frame rate (default: 60).
  --resolution <WxH>     Resolution (default: 1920x1080).
  --video-device <idx>   Video device index for avfoundation (default: 1).
  --audio-device <idx>   Audio device index for avfoundation (default: 0).
  --pix-fmt <fmt>        Pixel format (default: yuv420p).
  --list-devices         List avfoundation devices (macOS only).

Examples:
  scripts/capture_video.sh --list-devices
  scripts/capture_video.sh --out /Volumes/External/Captures/run.mp4 --duration 360 --fps 60 \
    --video-device 1 --audio-device 0 --resolution 1920x1080
USAGE
}

if [[ "${1:-}" == "--list-devices" ]]; then
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Device listing is only supported on macOS." >&2
    exit 2
  fi
  ffmpeg -f avfoundation -list_devices true -i "" 2>&1 | sed -n '1,120p'
  exit 0
fi

OUT=""
DURATION="60"
FPS="60"
RESOLUTION="1920x1080"
VIDEO_DEVICE="1"
AUDIO_DEVICE="0"
PIX_FMT="yuv420p"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT="$2"
      shift 2
      ;;
    --duration)
      DURATION="$2"
      shift 2
      ;;
    --fps)
      FPS="$2"
      shift 2
      ;;
    --resolution)
      RESOLUTION="$2"
      shift 2
      ;;
    --video-device)
      VIDEO_DEVICE="$2"
      shift 2
      ;;
    --audio-device)
      AUDIO_DEVICE="$2"
      shift 2
      ;;
    --pix-fmt)
      PIX_FMT="$2"
      shift 2
      ;;
    --help|-h)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      print_usage
      exit 2
      ;;
  esac
done

if [[ -z "$OUT" ]]; then
  echo "--out is required." >&2
  print_usage
  exit 2
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This helper currently supports macOS (avfoundation) only." >&2
  exit 2
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg not found in PATH." >&2
  exit 2
fi

mkdir -p "$(dirname "$OUT")"

ffmpeg \
  -f avfoundation \
  -framerate "$FPS" \
  -video_size "$RESOLUTION" \
  -i "${VIDEO_DEVICE}:${AUDIO_DEVICE}" \
  -t "$DURATION" \
  -pix_fmt "$PIX_FMT" \
  "$OUT"
