#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/xci_validate.sh --manifest <path>

Options:
  --manifest <path>  Path to XCI intake manifest.json (required).
USAGE
}

MANIFEST=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      MANIFEST="$2"
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

if [[ -z "$MANIFEST" ]]; then
  echo "--manifest is required." >&2
  print_usage
  exit 2
fi

cargo run -p recomp-cli -- xci-validate --manifest "$MANIFEST"
