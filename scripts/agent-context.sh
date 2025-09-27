#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)

session="${AGENT_SESSION:-}"
action=""
subject=""
type=""
scope=""
bullets=()
files=()
clear_files=false

usage() {
  cat <<EOF
Manage per-chat agent session context (subject/scope/bullets/files).

Usage:
  scripts/agent-context.sh --session ID [options]

Options:
  --session ID        Session identifier (e.g., chat/task ID)
  --subject TEXT      Set/replace subject text (without type(scope):)
  --type TYPE         Set commit type (feat|fix|chore|docs|refactor|test|ci|build|perf)
  --scope SCOPE       Set commit scope (cli|core|parsers-xml|...)
  -b, --bullet TEXT   Append a bullet to bullets.txt (repeatable)
  --add-file PATH     Record a file to the allowlist (repeatable)
  --clear-files       Clear the file allowlist for the session

Environment:
  AGENT_SESSION       Default session id
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --session) session=${2:-""}; shift 2 ;;
    --subject) subject=${2:-""}; shift 2 ;;
    --type) type=${2:-""}; shift 2 ;;
    --scope) scope=${2:-""}; shift 2 ;;
    -b|--bullet) bullets+=("$2"); shift 2 ;;
    --add-file) files+=("$2"); shift 2 ;;
    --clear-files) clear_files=true; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

[[ -n "$session" ]] || { echo "--session is required (or set AGENT_SESSION)" >&2; exit 1; }

session_dir="$repo_root/.git/agent-sessions/$session"
mkdir -p "$session_dir"

normpath() {
  local p="$1"
  # Strip repo root prefix if present
  p=${p#"$repo_root/"}
  # Strip leading ./
  p=${p#"./"}
  # Collapse duplicate slashes
  p=$(echo "$p" | sed 's#//\+#/#g')
  echo "$p"
}

[[ -n "$subject" ]] && printf '%s' "$subject" > "$session_dir/subject.txt"
[[ -n "$type" ]] && printf '%s' "$type" > "$session_dir/type.txt"
[[ -n "$scope" ]] && printf '%s' "$scope" > "$session_dir/scope.txt"

if [[ ${#bullets[@]} -gt 0 ]]; then
  touch "$session_dir/bullets.txt"
  for b in "${bullets[@]}"; do echo "$b" >> "$session_dir/bullets.txt"; done
fi

if $clear_files; then
  : > "$session_dir/files.list"
fi

if [[ ${#files[@]} -gt 0 ]]; then
  touch "$session_dir/files.list"
  for f in "${files[@]}"; do
    rp=$(normpath "$f")
    echo "$rp" >> "$session_dir/files.list"
  done
fi

echo "Session context updated: $session"
