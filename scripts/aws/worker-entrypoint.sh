#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <automation-config-path>" >&2
  exit 2
fi

AUTOMATION_CONFIG="$1"
OUTPUT_DIR="${RECOMP_WORKER_OUTPUT_DIR:-out/worker}"
mkdir -p "${OUTPUT_DIR}"

recomp automate --config "${AUTOMATION_CONFIG}"

MANIFEST_PATH="${RECOMP_RUN_MANIFEST:-}"
if [[ -n "${MANIFEST_PATH}" && -f "${MANIFEST_PATH}" ]]; then
  cp "${MANIFEST_PATH}" "${OUTPUT_DIR}/run-manifest.json"
fi

if [[ -n "${RECOMP_RUN_SUMMARY:-}" && -f "${RECOMP_RUN_SUMMARY}" ]]; then
  cp "${RECOMP_RUN_SUMMARY}" "${OUTPUT_DIR}/run-summary.json"
fi
