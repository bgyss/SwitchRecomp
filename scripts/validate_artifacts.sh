#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/validate_artifacts.sh --artifact-index <path> [options]

Options:
  --artifact-index <path>  Artifact index JSON (required).
  --out-dir <path>         Override out_dir from the artifact index.
USAGE
}

ARTIFACT_INDEX=""
OUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --artifact-index)
      ARTIFACT_INDEX="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
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

if [[ -z "$ARTIFACT_INDEX" ]]; then
  echo "--artifact-index is required." >&2
  print_usage
  exit 2
fi

ARGS=("cargo" "run" "-p" "recomp-validation" "--" "artifacts" "--artifact-index" "$ARTIFACT_INDEX")
if [[ -n "$OUT_DIR" ]]; then
  ARGS+=("--out-dir" "$OUT_DIR")
fi

"${ARGS[@]}"
