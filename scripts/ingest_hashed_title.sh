#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/ingest_hashed_title.sh --title <clear-title-slug> [options]

Purpose:
  Assign a hash-based title id, move/copy title data into a hashed directory, rename XCI and
  gameplay-video files to hashed names, and update a local decoder-ring registry.

Options:
  --title <value>           Clear title slug (required, e.g. sample-title).
  --game-data-root <path>   Game data root (default: game-data).
  --source-dir <path>       Source title directory (default: <game-data-root>/<title>).
  --title-id <value>        Override hash-based id (default: title-<sha256(title)[:16]>).
  --xci <path>              XCI path to hash/rename (repeatable). If omitted, scans <title>/xci.
  --video <path>            Video path to hash/rename (repeatable). If omitted, scans <title>/gameplay-video.
  --registry <path>         Decoder ring file (default: config/title-hash-registry.toml).
  --tracking <path>         Tracking log file (default: config/title-hash-tracking.log).
  --copy                    Copy files/directories instead of moving.
  --move                    Move files/directories (default).
  --help, -h                Show this help text.
USAGE
}

hash_text() {
  printf "%s" "$1" | shasum -a 256 | awk '{print $1}'
}

hash_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

ensure_parent_dir() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
}

transfer_file() {
  local src="$1"
  local dst="$2"
  if [[ "$MODE" == "copy" ]]; then
    cp "$src" "$dst"
  else
    mv "$src" "$dst"
  fi
}

transfer_dir() {
  local src="$1"
  local dst="$2"
  if [[ "$MODE" == "copy" ]]; then
    cp -R "$src" "$dst"
  else
    mv "$src" "$dst"
  fi
}

append_tracking() {
  local kind="$1"
  local detail="$2"
  printf "%s\t%s\t%s\t%s\n" "$TIMESTAMP_UTC" "$TITLE_ID" "$kind" "$detail" >>"$TRACKING_PATH"
}

TITLE=""
GAME_DATA_ROOT="game-data"
SOURCE_DIR=""
TITLE_ID_OVERRIDE=""
REGISTRY_PATH="config/title-hash-registry.toml"
TRACKING_PATH="config/title-hash-tracking.log"
MODE="move"
declare -a XCI_PATHS=()
declare -a VIDEO_PATHS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --title)
      TITLE="$2"
      shift 2
      ;;
    --game-data-root)
      GAME_DATA_ROOT="$2"
      shift 2
      ;;
    --source-dir)
      SOURCE_DIR="$2"
      shift 2
      ;;
    --title-id)
      TITLE_ID_OVERRIDE="$2"
      shift 2
      ;;
    --xci)
      XCI_PATHS+=("$2")
      shift 2
      ;;
    --video)
      VIDEO_PATHS+=("$2")
      shift 2
      ;;
    --registry)
      REGISTRY_PATH="$2"
      shift 2
      ;;
    --tracking)
      TRACKING_PATH="$2"
      shift 2
      ;;
    --copy)
      MODE="copy"
      shift
      ;;
    --move)
      MODE="move"
      shift
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

if [[ -z "$TITLE" ]]; then
  echo "--title is required." >&2
  print_usage
  exit 2
fi

GAME_DATA_ROOT="${GAME_DATA_ROOT%/}"
if [[ -z "$SOURCE_DIR" ]]; then
  SOURCE_DIR="${GAME_DATA_ROOT}/${TITLE}"
fi

TITLE_HASH_FULL="$(hash_text "$TITLE")"
TITLE_ID="${TITLE_ID_OVERRIDE:-title-${TITLE_HASH_FULL:0:16}}"
TARGET_DIR="${GAME_DATA_ROOT}/${TITLE_ID}"
TIMESTAMP_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

ensure_parent_dir "$REGISTRY_PATH"
ensure_parent_dir "$TRACKING_PATH"

if [[ "$SOURCE_DIR" != "$TARGET_DIR" && -d "$SOURCE_DIR" ]]; then
  if [[ -e "$TARGET_DIR" ]]; then
    echo "Refusing to rename: target already exists: $TARGET_DIR" >&2
    exit 1
  fi
  transfer_dir "$SOURCE_DIR" "$TARGET_DIR"
  append_tracking "title_dir" "$SOURCE_DIR -> $TARGET_DIR"
fi

mkdir -p "$TARGET_DIR"

if [[ ${#XCI_PATHS[@]} -eq 0 && -d "$TARGET_DIR/xci" ]]; then
  while IFS= read -r -d '' path; do
    XCI_PATHS+=("$path")
  done < <(find "$TARGET_DIR/xci" -maxdepth 1 -type f -print0)
fi

if [[ ${#VIDEO_PATHS[@]} -eq 0 && -d "$TARGET_DIR/gameplay-video" ]]; then
  while IFS= read -r -d '' path; do
    VIDEO_PATHS+=("$path")
  done < <(find "$TARGET_DIR/gameplay-video" -maxdepth 1 -type f -print0)
fi

XCI_RENAMED=0
for src in "${XCI_PATHS[@]}"; do
  if [[ ! -f "$src" ]]; then
    echo "Skipping missing XCI path: $src" >&2
    continue
  fi

  xci_hash="$(hash_file "$src")"
  ext="${src##*.}"
  if [[ "$ext" == "$src" ]]; then
    ext="xci"
  fi
  dst_dir="$TARGET_DIR/xci"
  mkdir -p "$dst_dir"
  dst="$dst_dir/${xci_hash:0:16}.${ext}"

  if [[ "$src" == "$dst" ]]; then
    continue
  fi

  if [[ -e "$dst" ]]; then
    existing_hash="$(hash_file "$dst")"
    if [[ "$existing_hash" == "$xci_hash" ]]; then
      if [[ "$MODE" == "move" ]]; then
        rm -f "$src"
      fi
      continue
    fi
    echo "Refusing to overwrite non-matching file: $dst" >&2
    exit 1
  fi

  transfer_file "$src" "$dst"
  append_tracking "xci" "$src -> $dst"
  XCI_RENAMED=$((XCI_RENAMED + 1))
done

VIDEO_RENAMED=0
for src in "${VIDEO_PATHS[@]}"; do
  if [[ ! -f "$src" ]]; then
    echo "Skipping missing video path: $src" >&2
    continue
  fi

  video_hash="$(hash_file "$src")"
  hash_short="${video_hash:0:16}"
  ext="${src##*.}"
  if [[ "$ext" == "$src" ]]; then
    ext="bin"
  fi
  dst_dir="$TARGET_DIR/gameplay-video"
  mkdir -p "$dst_dir"

  base="$(basename "$src")"
  if [[ "$(dirname "$src")" == "$dst_dir" && "$base" =~ ^${hash_short}-[0-9]{3}\.${ext}$ ]]; then
    continue
  fi

  seq=1
  while :; do
    dst="$(printf "%s/%s-%03d.%s" "$dst_dir" "$hash_short" "$seq" "$ext")"
    if [[ ! -e "$dst" ]]; then
      break
    fi
    seq=$((seq + 1))
  done

  transfer_file "$src" "$dst"
  append_tracking "video" "$src -> $dst"
  VIDEO_RENAMED=$((VIDEO_RENAMED + 1))
done

if [[ ! -f "$REGISTRY_PATH" ]]; then
  cat <<'REGISTRY_HEADER' >"$REGISTRY_PATH"
schema_version = "1"

# Local-only secret decoder ring. Keep this file untracked.

REGISTRY_HEADER
fi

if ! grep -Fq "id = \"$TITLE_ID\"" "$REGISTRY_PATH"; then
  cat <<REGISTRY_TITLE >>"$REGISTRY_PATH"
[[titles]]
name = "$TITLE"
id = "$TITLE_ID"
name_sha256 = "$TITLE_HASH_FULL"
game_data_dir = "$TARGET_DIR"
created_at = "$TIMESTAMP_UTC"

REGISTRY_TITLE
fi

cat <<REGISTRY_RUN >>"$REGISTRY_PATH"
[[ingest_runs]]
timestamp = "$TIMESTAMP_UTC"
title = "$TITLE"
id = "$TITLE_ID"
game_data_dir = "$TARGET_DIR"
mode = "$MODE"
xci_renamed = $XCI_RENAMED
video_renamed = $VIDEO_RENAMED

REGISTRY_RUN

printf "title=%s\n" "$TITLE"
printf "title_id=%s\n" "$TITLE_ID"
printf "target_dir=%s\n" "$TARGET_DIR"
printf "xci_renamed=%s\n" "$XCI_RENAMED"
printf "video_renamed=%s\n" "$VIDEO_RENAMED"
printf "registry=%s\n" "$REGISTRY_PATH"
printf "tracking=%s\n" "$TRACKING_PATH"
