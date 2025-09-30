#!/usr/bin/env bash
set -euo pipefail

# Guard to ensure agents finish with a commit.
#
# Usage:
#   scripts/agent-ensure-commit.sh [--session ID] [--auto]
#
# Options:
#   --session ID   Optional session id to pass to agent-commit.sh when using --auto
#   --auto         If there are pending changes, run agent-commit.sh automatically
#
# Behavior:
#   - Exits 0 when the working tree is clean (no staged/unstaged/untracked changes)
#   - Exits non-zero when there are pending changes and --auto is not provided,
#     printing next-step instructions for the agent.

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

session="${AGENT_SESSION:-}"
auto=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --session) session=${2:-""}; shift 2 ;;
    --auto) auto=true; shift ;;
    -h|--help)
      sed -n '1,40p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

have_pending() {
  # Any unstaged, staged, or untracked changes?
  if ! git diff --quiet --ignore-submodules --; then return 0; fi
  if ! git diff --quiet --cached --ignore-submodules --; then return 0; fi
  if [[ -n "$(git ls-files --others --exclude-standard)" ]]; then return 0; fi
  return 1
}

if have_pending; then
  echo "✖ Pending changes detected: commit is required at the end of the task." >&2
  if $auto; then
    cmd=("$repo_root/scripts/agent-commit.sh")
    [[ -n "$session" ]] && cmd+=("--session" "$session")
    "${cmd[@]}"
    exit 0
  else
    echo "Next steps:" >&2
    if [[ -n "$session" ]]; then
      echo "  - Run: scripts/agent-commit.sh --session $session" >&2
    else
      echo "  - Run: scripts/agent-commit.sh" >&2
    fi
    exit 2
  fi
fi

echo "✔ Working tree clean."
exit 0

