#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/capture-validation.sh --out-dir <path> [options]

Options:
  --out-dir <path>        Output directory for capture + hashes (required).
  --source <path>         Use an existing MP4 instead of capturing.
  --duration <seconds>    Capture duration in seconds (default: 60).
  --fps <fps>             Frame rate (default: 60).
  --resolution <WxH>      Resolution (default: 1920x1080).
  --video-device <idx>    Video device index for avfoundation (default: 1).
  --audio-device <idx>    Audio device index for avfoundation (default: 0).
  --pix-fmt <fmt>         Pixel format (default: yuv420p).
  --list-devices          List capture devices (macOS only).

Environment:
  AUDIO_RATE              Audio sample rate for extraction (default: 48000).
  AUDIO_CHANNELS          Audio channel count (default: 2).

Examples:
  scripts/capture-validation.sh --list-devices
  scripts/capture-validation.sh --out-dir /Volumes/External/Captures/run --duration 360 --fps 60 \
    --video-device 1 --audio-device 0 --resolution 1920x1080
  scripts/capture-validation.sh --out-dir /Volumes/External/Captures/run --source run.mp4
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

OUT_DIR=""
SOURCE=""
DURATION="60"
FPS="60"
RESOLUTION="1920x1080"
VIDEO_DEVICE="1"
AUDIO_DEVICE="0"
PIX_FMT="yuv420p"

if [[ "${1:-}" == "--list-devices" ]]; then
  "${SCRIPT_DIR}/capture_video.sh" --list-devices
  exit 0
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --source)
      SOURCE="$2"
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

if [[ -z "$OUT_DIR" ]]; then
  echo "--out-dir is required." >&2
  print_usage
  exit 2
fi

if ! command -v ffmpeg >/dev/null 2>&1; then
  echo "ffmpeg not found in PATH." >&2
  exit 2
fi

mkdir -p "$OUT_DIR"

VIDEO_OUT="$SOURCE"
if [[ -z "$VIDEO_OUT" ]]; then
  VIDEO_OUT="$OUT_DIR/capture.mp4"
  "${SCRIPT_DIR}/capture_video.sh" \
    --out "$VIDEO_OUT" \
    --duration "$DURATION" \
    --fps "$FPS" \
    --resolution "$RESOLUTION" \
    --video-device "$VIDEO_DEVICE" \
    --audio-device "$AUDIO_DEVICE" \
    --pix-fmt "$PIX_FMT"
fi

FRAMES_DIR="$OUT_DIR/frames"
AUDIO_WAV="$OUT_DIR/audio.wav"
FRAMES_HASHES="$OUT_DIR/frames.hashes"
AUDIO_HASHES="$OUT_DIR/audio.hashes"

mkdir -p "$FRAMES_DIR"

AUDIO_RATE="${AUDIO_RATE:-48000}"
AUDIO_CHANNELS="${AUDIO_CHANNELS:-2}"

ffmpeg -y -i "$VIDEO_OUT" "$FRAMES_DIR/%08d.png"
ffmpeg -y -i "$VIDEO_OUT" -vn -acodec pcm_s16le -ar "$AUDIO_RATE" -ac "$AUDIO_CHANNELS" "$AUDIO_WAV"

VALIDATOR=()
if command -v recomp-validation >/dev/null 2>&1; then
  VALIDATOR=(recomp-validation)
elif command -v cargo >/dev/null 2>&1; then
  VALIDATOR=(cargo run -p recomp-validation --)
else
  echo "recomp-validation not found and cargo unavailable" >&2
  exit 1
fi

"${VALIDATOR[@]}" hash-frames --frames-dir "$FRAMES_DIR" --out "$FRAMES_HASHES"
"${VALIDATOR[@]}" hash-audio --audio-file "$AUDIO_WAV" --out "$AUDIO_HASHES"

echo "capture video: $VIDEO_OUT"
echo "frames: $FRAMES_DIR"
echo "audio: $AUDIO_WAV"
echo "hashes: $FRAMES_HASHES, $AUDIO_HASHES"
