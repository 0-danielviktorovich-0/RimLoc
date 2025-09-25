#!/usr/bin/env bash
set -euo pipefail

# Detect if this PR contains user-facing changes and ensure CHANGELOG.md is updated.
# Heuristics: any changes under crates/* (code/tests), docs/*, or README/AGENTS touching CLI behavior
# require a changelog update.

BASE_REF="${1:-origin/${GITHUB_BASE_REF:-origin/main}}"

git fetch --no-tags --depth=50 origin "+refs/heads/*:refs/remotes/origin/*" >/dev/null 2>&1 || true

MERGE_BASE=$(git merge-base HEAD "$BASE_REF" || git rev-parse HEAD)
CHANGED=$(git diff --name-only --diff-filter=ACMRTUXB "$MERGE_BASE"...HEAD)

requires_changelog=false
while IFS= read -r f; do
  # ignore housekeeping-only changes
  case "$f" in
    CHANGELOG.md|.github/*|scripts/check-changelog.sh|AGENTS.md)
      continue
      ;;
  esac
  if [[ "$f" == crates/* || "$f" == docs/* || "$f" == README.md ]]; then
    requires_changelog=true
    break
  fi
done <<<"$CHANGED"

if [[ "$requires_changelog" == true ]]; then
  if ! git diff --name-only "$MERGE_BASE"...HEAD | grep -q '^CHANGELOG.md$'; then
    echo "ERROR: User-facing changes detected, but CHANGELOG.md was not updated under Unreleased." >&2
    echo "Changed files:" >&2
    echo "$CHANGED" >&2
    exit 1
  fi
fi

echo "Changelog check passed."

