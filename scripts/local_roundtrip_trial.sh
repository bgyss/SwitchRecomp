#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/local_roundtrip_trial.sh setup
  scripts/local_roundtrip_trial.sh run
  scripts/local_roundtrip_trial.sh capture
  scripts/local_roundtrip_trial.sh inspect

Environment overrides:
  LOCAL_TRIAL_CONFIG      Path to automation config (default: samples/automation.local-roundtrip.toml)
  LOCAL_TRIAL_OUT_ROOT    Artifact root (default: out/local-roundtrip-trial)
  LOCAL_TRIAL_CAPTURE_MODE
    - fail_once_then_pass (default): attempt 000 fails hash gate, retry can pass
    - always_fail: every attempt fails hash gate
    - always_pass: first attempt passes; no retry needed
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_PATH="${LOCAL_TRIAL_CONFIG:-$REPO_ROOT/samples/automation.local-roundtrip.toml}"
OUT_ROOT="${LOCAL_TRIAL_OUT_ROOT:-$REPO_ROOT/out/local-roundtrip-trial}"
WORK_ROOT="$OUT_ROOT/work"
ARTIFACT_ROOT="$OUT_ROOT/artifacts"
REF_DIR="$ARTIFACT_ROOT/reference"
CAPTURE_DIR="$ARTIFACT_ROOT/capture"
PROFILE_ROOT="$OUT_ROOT/profiles"
REF_FRAMES_DIR="$REF_DIR/frames"
CAPTURE_FRAMES_DIR="$CAPTURE_DIR/frames"
FAIL_PROFILE_DIR="$PROFILE_ROOT/fail/frames"
PASS_PROFILE_DIR="$PROFILE_ROOT/pass/frames"
REF_HASH_LIST="$REF_DIR/frames.hashes"
CAPTURE_VIDEO_PATH="$CAPTURE_DIR/capture.mp4"
CAPTURE_MODE="${LOCAL_TRIAL_CAPTURE_MODE:-fail_once_then_pass}"

write_frame_set() {
  local target_dir="$1"
  local prefix="$2"
  mkdir -p "$target_dir"
  printf '%s\n' "${prefix}-frame-0001" >"$target_dir/00000001.png"
  printf '%s\n' "${prefix}-frame-0002" >"$target_dir/00000002.png"
  printf '%s\n' "${prefix}-frame-0003" >"$target_dir/00000003.png"
}

setup_fixtures() {
  rm -rf "$OUT_ROOT"
  mkdir -p "$REF_DIR" "$CAPTURE_DIR" "$FAIL_PROFILE_DIR" "$PASS_PROFILE_DIR"

  write_frame_set "$REF_FRAMES_DIR" "reference"
  write_frame_set "$PASS_PROFILE_DIR" "reference"
  write_frame_set "$FAIL_PROFILE_DIR" "mismatch"

  printf 'reference-video-placeholder\n' >"$REF_DIR/reference.mp4"
  printf 'capture-video-placeholder\n' >"$CAPTURE_VIDEO_PATH"

  cargo run -p recomp-validation -- hash-frames \
    --frames-dir "$REF_FRAMES_DIR" \
    --out "$REF_HASH_LIST" >/dev/null

  echo "Prepared local trial fixtures in $OUT_ROOT"
}

select_capture_profile() {
  local attempt_id=0
  if [[ -n "${RECOMP_RUN_MANIFEST:-}" && "$RECOMP_RUN_MANIFEST" =~ /attempts/([0-9]{3})/ ]]; then
    attempt_id=$((10#${BASH_REMATCH[1]}))
  fi
  case "$CAPTURE_MODE" in
    fail_once_then_pass)
      if (( attempt_id > 0 )) || [[ "${RECOMP_INPUT_SCRIPT_TOML:-}" == *"/attempts/"*"/mutations/input_script.toml" ]]; then
        echo "$PASS_PROFILE_DIR"
      else
        echo "$FAIL_PROFILE_DIR"
      fi
      ;;
    always_fail)
      echo "$FAIL_PROFILE_DIR"
      ;;
    always_pass)
      echo "$PASS_PROFILE_DIR"
      ;;
    *)
      echo "Unsupported LOCAL_TRIAL_CAPTURE_MODE: $CAPTURE_MODE" >&2
      exit 2
      ;;
  esac
}

capture_fixture() {
  local profile
  profile="$(select_capture_profile)"

  rm -rf "$CAPTURE_FRAMES_DIR"
  mkdir -p "$CAPTURE_FRAMES_DIR"
  cp "$profile"/* "$CAPTURE_FRAMES_DIR/"

  printf 'mode=%s\ninput_script=%s\n' \
    "$CAPTURE_MODE" \
    "${RECOMP_INPUT_SCRIPT_TOML:-unset}" >"$CAPTURE_VIDEO_PATH"

  echo "Capture fixture wrote $(basename "$(dirname "$profile")") profile to $CAPTURE_FRAMES_DIR"
}

inspect_outputs() {
  local run_summary="$WORK_ROOT/run-summary.json"
  local run_manifest="$WORK_ROOT/run-manifest.json"

  if [[ ! -f "$run_summary" || ! -f "$run_manifest" ]]; then
    echo "Run artifacts not found under $WORK_ROOT. Run setup + automate first." >&2
    exit 1
  fi

  echo "Run summary: $run_summary"
  echo "Run manifest: $run_manifest"

  if command -v jq >/dev/null 2>&1; then
    jq '{status, attempts, winning_attempt, halted_reason}' "$run_summary"
    jq '{final_status, winning_attempt, attempts: [.attempts[] | {attempt, status, strategy}]}' "$run_manifest"
  else
    echo "jq not found; showing key lines"
    rg -n '"status"|"attempts"|"winning_attempt"|"final_status"|"halted_reason"' \
      "$run_summary" "$run_manifest"
  fi

  echo "Attempt artifacts:"
  find "$WORK_ROOT/attempts" -maxdepth 2 -type f \( \
    -name 'attempt-manifest.json' -o -name 'gate-results.json' -o -name 'triage.json' \
  \) | sort
}

run_trial() {
  setup_fixtures
  cargo run -p recomp-cli -- automate --config "$CONFIG_PATH"
  inspect_outputs
}

main() {
  local command="${1:-run}"
  case "$command" in
    setup)
      setup_fixtures
      ;;
    capture)
      capture_fixture
      ;;
    inspect)
      inspect_outputs
      ;;
    run)
      run_trial
      ;;
    -h|--help|help)
      print_usage
      ;;
    *)
      echo "Unknown command: $command" >&2
      print_usage
      exit 2
      ;;
  esac
}

main "$@"
