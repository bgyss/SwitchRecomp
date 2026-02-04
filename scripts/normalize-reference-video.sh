#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <source_video> <out_dir>" >&2
  exit 2
fi

SOURCE_VIDEO="$1"
OUT_DIR="$2"

WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-720}"
FPS="${FPS:-30}"
AUDIO_RATE="${AUDIO_RATE:-48000}"
AUDIO_CHANNELS="${AUDIO_CHANNELS:-2}"

NORMALIZED_VIDEO="${OUT_DIR}/reference-normalized.mp4"
FRAMES_DIR="${OUT_DIR}/frames"
AUDIO_WAV="${OUT_DIR}/audio.wav"
FRAMES_HASHES="${OUT_DIR}/frames.hashes"
AUDIO_HASHES="${OUT_DIR}/audio.hashes"

mkdir -p "${OUT_DIR}" "${FRAMES_DIR}"

ffmpeg -y -i "${SOURCE_VIDEO}" \
  -vf "scale=${WIDTH}:${HEIGHT},fps=${FPS}" \
  -r "${FPS}" -fps_mode cfr \
  -c:v libx264 -preset slow -crf 18 -pix_fmt yuv420p \
  -c:a pcm_s16le -ar "${AUDIO_RATE}" -ac "${AUDIO_CHANNELS}" \
  "${NORMALIZED_VIDEO}"

ffmpeg -y -i "${NORMALIZED_VIDEO}" "${FRAMES_DIR}/%08d.png"
ffmpeg -y -i "${NORMALIZED_VIDEO}" -vn -acodec pcm_s16le -ar "${AUDIO_RATE}" -ac "${AUDIO_CHANNELS}" "${AUDIO_WAV}"

VALIDATOR=()
if command -v recomp-validation >/dev/null 2>&1; then
  VALIDATOR=(recomp-validation)
elif command -v cargo >/dev/null 2>&1; then
  VALIDATOR=(cargo run -p recomp-validation --)
else
  echo "recomp-validation not found and cargo unavailable" >&2
  exit 1
fi

"${VALIDATOR[@]}" hash-frames --frames-dir "${FRAMES_DIR}" --out "${FRAMES_HASHES}"
"${VALIDATOR[@]}" hash-audio --audio-file "${AUDIO_WAV}" --out "${AUDIO_HASHES}"

echo "normalized reference written to ${NORMALIZED_VIDEO}"
echo "hashes written to ${FRAMES_HASHES} and ${AUDIO_HASHES}"
