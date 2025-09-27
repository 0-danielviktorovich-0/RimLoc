#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

session="${AGENT_SESSION:-}"
cmd=""
path=""

usage() {
  cat <<EOF
Mark per-chat file edits to capture exact hunks for staging later.

Usage:
  scripts/agent-mark-change.sh --session ID begin --file PATH
  scripts/agent-mark-change.sh --session ID end --file PATH

Environment:
  AGENT_SESSION   Default session id

Description:
  begin: snapshot current working copy of PATH (or absence) to session 'edit-before/'.
  end:   diff 'edit-before/' vs current PATH; append a minimized patch to 'patches/'.
         Also refresh session 'snapshots/' for future diffs and clear 'edit-before/'.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --session) session=${2:-""}; shift 2 ;;
    begin|end) cmd="$1"; shift ;;
    --file) path=${2:-""}; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$session" ]] || { echo "--session is required (or set AGENT_SESSION)" >&2; exit 1; }
[[ -n "$cmd" ]] || { echo "command 'begin' or 'end' is required" >&2; exit 1; }
[[ -n "$path" ]] || { echo "--file PATH is required" >&2; exit 1; }

# Normalize path to repo-relative
path=${path#"$repo_root/"}
path=${path#"./"}

session_dir="$repo_root/.git/agent-sessions/$session"
before_dir="$session_dir/edit-before"
patches_dir="$session_dir/patches"
snaps_dir="$session_dir/snapshots"
miss_dir="$session_dir/snapshots-missing"

mkdir -p "$session_dir" "$before_dir" "$patches_dir" "$snaps_dir" "$miss_dir"

case "$cmd" in
  begin)
    # Save 'before' snapshot for this edit
    if [[ -e "$path" ]]; then
      mkdir -p "$(dirname "$before_dir/$path")"
      cp "$path" "$before_dir/$path"
    else
      mkdir -p "$(dirname "$before_dir/$path")"
      : > "$before_dir/$path.missing"
    fi
    echo "Captured 'begin' snapshot for $path"
    ;;
  end)
    # Build a unidiff-0 patch from 'before' to current
    before="$before_dir/$path"
    if [[ -f "$before" ]]; then
      old="$before"
    elif [[ -f "$before_dir/$path.missing" ]]; then
      old="/dev/null"
    else
      echo "No 'begin' snapshot for $path; capturing from session baseline instead." >&2
      # Fallback to session baseline
      if [[ -f "$snaps_dir/$path" ]]; then old="$snaps_dir/$path"; else old="/dev/null"; fi
    fi

    new="$path"
    raw=$(mktemp)
    out=$(mktemp)
    if ! git diff --no-index --unified=0 -- "$old" "$new" > "$raw"; then true; fi

    # Normalize headers to a/path b/path for later git apply
    awk -v p="$path" '
      BEGIN { shown=0 }
      /^diff --git / { if (!shown) { print "diff --git a/" p " b/" p; shown=1 } next }
      /^--- / { print "--- a/" p; next }
      /^\+\+\+ / { print "+++ b/" p; next }
      { print }
    ' "$raw" > "$out"

    # Append to per-file patch log
    mkdir -p "$(dirname "$patches_dir/$path.patch")"
    cat "$out" >> "$patches_dir/$path.patch"

    # Refresh session snapshot to current
    if [[ -e "$path" ]]; then
      mkdir -p "$(dirname "$snaps_dir/$path")"
      cp "$path" "$snaps_dir/$path"
      rm -f "$miss_dir/$path" 2>/dev/null || true
    else
      mkdir -p "$(dirname "$miss_dir/$path")"
      : > "$miss_dir/$path"
    fi

    # Cleanup 'before'
    rm -f "$before" "$before_dir/$path.missing" 2>/dev/null || true

    rm -f "$raw" "$out"
    echo "Recorded patch for $path"
    ;;
esac

