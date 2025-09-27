#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)

session="${AGENT_SESSION:-}"
subject=""
type=""
scope=""
bullets=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --session) session=${2:-""}; shift 2 ;;
    --subject) subject=${2:-""}; shift 2 ;;
    --type) type=${2:-""}; shift 2 ;;
    --scope) scope=${2:-""}; shift 2 ;;
    -b|--bullet) bullets+=("$2"); shift 2 ;;
    -h|--help)
      cat <<EOF
Initialize an agent session baseline and optional context.

Usage: scripts/agent-begin.sh [--session ID] [--subject TEXT] [--type TYPE] [--scope SCOPE] [-b TEXT ...]

If --session is omitted, a global baseline is recorded.
Context is stored under .git/agent-sessions/<ID>/
EOF
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

if [[ -n "$session" ]]; then
  session_dir="$repo_root/.git/agent-sessions/$session"
  mkdir -p "$session_dir"
  # Save optional context
  [[ -n "$subject" ]] && printf '%s' "$subject" > "$session_dir/subject.txt"
  [[ -n "$type" ]] && printf '%s' "$type" > "$session_dir/type.txt"
  [[ -n "$scope" ]] && printf '%s' "$scope" > "$session_dir/scope.txt"
  if [[ ${#bullets[@]} -gt 0 ]]; then
    printf '%s\n' "${bullets[@]}" > "$session_dir/bullets.txt"
  fi
  echo "Session context initialized at .git/agent-sessions/$session"
  "$repo_root/scripts/agent-commit.sh" --start --session "$session"
else
  "$repo_root/scripts/agent-commit.sh" --start
fi
