#!/usr/bin/env bash
set -euo pipefail

print_usage() {
  cat <<'USAGE'
Usage:
  scripts/rewrite_title_refs_in_history.sh --needle <text> [--needle <text> ...] [options]

Purpose:
  Rewrite commit messages to replace clear-text title mentions with a hash-based id.

Options:
  --needle <value>       Source text to replace in commit messages (repeatable, required).
  --replacement <value>  Replacement text (default: title-a24b9e807b456252).
  --refs <revset>        Refs/revset to rewrite (default: --all).
  --help, -h             Show this help text.

Notes:
  - Requires a clean worktree.
  - Rewrites history and requires force-push for rewritten refs.
USAGE
}

REPLACEMENT="title-a24b9e807b456252"
REFS="--all"
declare -a NEEDLES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --needle)
      NEEDLES+=("$2")
      shift 2
      ;;
    --replacement)
      REPLACEMENT="$2"
      shift 2
      ;;
    --refs)
      REFS="$2"
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

if [[ ${#NEEDLES[@]} -eq 0 ]]; then
  echo "At least one --needle is required." >&2
  print_usage
  exit 2
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Worktree is not clean. Commit or stash changes before rewriting history." >&2
  exit 1
fi

if [[ "$REFS" == "--all" ]]; then
  FILTER_ARGS=(-- --all)
else
  FILTER_ARGS=(-- "$REFS")
fi

export REPLACEMENT
NEEDLE_LIST=""
for needle in "${NEEDLES[@]}"; do
  NEEDLE_LIST+="${needle}"$'\n'
done
export NEEDLE_LIST

git filter-branch -f --msg-filter '
  perl -Mstrict -Mwarnings -e "
    my \$replacement = \$ENV{REPLACEMENT};
    my \@needles = grep { length \$_ > 0 } split(/\\n/, \$ENV{NEEDLE_LIST});
    while (<STDIN>) {
      for my \$needle (\@needles) {
        my \$quoted = quotemeta(\$needle);
        s/\$quoted/\$replacement/g;
      }
      print;
    }
  "
' "${FILTER_ARGS[@]}"

echo "History rewrite complete."
echo "If rewritten refs are published, force-push with:"
echo "  git push --force-with-lease --all"
echo "  git push --force-with-lease --tags"
