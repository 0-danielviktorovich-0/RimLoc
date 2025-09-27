#!/usr/bin/env bash
set -euo pipefail

# Commit only files changed during the current agent session.
#
# Usage:
#   scripts/agent-commit.sh --start [--session ID]
#   scripts/agent-commit.sh -m "type(scope): subject" -b "bullet 1" [-b "bullet 2" ...]
#   scripts/agent-commit.sh -F /path/to/message.txt
#
# Options:
#   --start                 Record a baseline of current local changes and exit.
#   --session ID            Use a named session to isolate baseline/context.
#   -m, --message SUBJECT   Commit subject (Conventional Commit subject line).
#   -b, --bullet TEXT       Add a bullet line to the body (can be repeated).
#   -F, --file FILE         Use a full commit message from file (overrides -m/-b).
#   --include-preexisting   Include files that already had changes at baseline.
#   --dry-run               Show what would be committed and the message, then exit.
#   --no-verify             Pass --no-verify to git commit (not recommended).
#
# Baseline file lives under .git/agent-baseline.txt

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

baseline_file=".git/agent-baseline.txt"
session="${AGENT_SESSION:-}"
session_dir=""

die() { echo "Error: $*" >&2; exit 1; }

have_cmd() { command -v "$1" >/dev/null 2>&1; }

collect_changes() {
  # Print list of changed/untracked files, one per line, relative to repo root
  {
    git diff --name-only --cached
    git diff --name-only
    git ls-files --others --exclude-standard
  } | grep -v '^$' | sort -u
}

read_list_into_array() {
  # $1: filepath, fills global array files_to_commit
  files_to_commit=()
  while IFS= read -r __line || [ -n "${__line:-}" ]; do
    [ -n "$__line" ] && files_to_commit+=("$__line")
  done < "$1"
}

write_baseline() {
  mkdir -p .git
  collect_changes > "$baseline_file"
  echo "Baseline recorded to $baseline_file"
}

start_only=false
include_preexisting=false
dry_run=false
no_verify=false
msg_subject=""
msg_file=""
bullets=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --start) start_only=true; shift ;;
    --session) session=${2:-""}; shift 2 ;;
    -m|--message) msg_subject=${2:-""}; [[ -n "$msg_subject" ]] || die "--message requires a value"; shift 2 ;;
    -b|--bullet) bullets+=("$2"); shift 2 ;;
    -F|--file) msg_file=${2:-""}; [[ -n "$msg_file" ]] || die "--file requires a path"; shift 2 ;;
    --include-preexisting) include_preexisting=true; shift ;;
    --dry-run) dry_run=true; shift ;;
    --no-verify) no_verify=true; shift ;;
    -h|--help)
      sed -n '1,40p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) die "Unknown option: $1" ;;
  esac
  # If patch is empty (no hunks), signal caller to fallback to full add
  if [[ ! -s "$tmp_patch" ]]; then
    rm -f "$raw_patch" "$tmp_patch"
    return 1
  fi

  # If patch is empty (no hunks), signal caller to fallback to full add
  if [[ ! -s "$tmp_patch" ]]; then
    rm -f "$raw_patch" "$tmp_patch"
    return 1
  fi

  # If patch is empty (no hunks), signal caller to fallback to full add
  if [[ ! -s "$tmp_patch" ]]; then
    rm -f "$raw_patch" "$tmp_patch"
    return 1
  fi

done

have_cmd git || die "git is required"

if [[ -n "$session" ]]; then
  session_dir=".git/agent-sessions/$session"
  mkdir -p "$session_dir"
  baseline_file="$session_dir/baseline.txt"
fi

if $start_only; then
  write_baseline
  exit 0
fi

# Compute current and baseline sets
current_changes=$(mktemp)
collect_changes > "$current_changes"

if [[ -f "$baseline_file" ]]; then
  baseline_changes="$baseline_file"
else
  baseline_changes=$(mktemp)
  : > "$baseline_changes"
fi

new_changes=$(mktemp)
if $include_preexisting || [[ ! -s "$baseline_changes" ]]; then
  # Include all current changes
  cat "$current_changes" > "$new_changes"
else
  # Set difference: current - baseline
  comm -13 <(sort -u "$baseline_changes") <(sort -u "$current_changes") > "$new_changes"
fi

read_list_into_array "$new_changes"

# Intersect with session allowlist if provided
if [[ -n "$session_dir" && -f "$session_dir/files.list" ]]; then
  allow=$(mktemp)
  sort -u "$session_dir/files.list" > "$allow"
  intersect=$(mktemp)
  comm -12 <(sort -u "$new_changes") "$allow" > "$intersect"
  read_list_into_array "$intersect"
  rm -f "$allow" "$intersect"
fi

if [[ ${#files_to_commit[@]} -eq 0 ]]; then
  echo "No new changes to commit (working set matches baseline)."
  rm -f "$current_changes" "$new_changes"
  exit 0
fi

echo "Files to commit (${#files_to_commit[@]}):"
printf ' - %s\n' "${files_to_commit[@]}"

if $dry_run; then
  if [[ -n "$msg_file" ]]; then
    echo "--- commit message (file) ---"
    cat "$msg_file" || true
  else
    # Compose preview message similar to real commit
    if [[ -z "$msg_subject" ]]; then
      ctx_type=""; ctx_scope=""; ctx_subject="";
      if [[ -n "$session_dir" ]]; then
        [[ -f "$session_dir/type.txt" ]] && ctx_type=$(<"$session_dir/type.txt")
        [[ -f "$session_dir/scope.txt" ]] && ctx_scope=$(<"$session_dir/scope.txt")
        [[ -f "$session_dir/subject.txt" ]] && ctx_subject=$(<"$session_dir/subject.txt")
      fi
      detect_scope() {
        for f in "${files_to_commit[@]}"; do
          case "$f" in
            crates/rimloc-core/*) echo core; return;;
            crates/rimloc-parsers-xml/*) echo parsers-xml; return;;
            crates/rimloc-export-po/*) echo export-po; return;;
            crates/rimloc-export-csv/*) echo export-csv; return;;
            crates/rimloc-import-po/*) echo import-po; return;;
            crates/rimloc-validate/*) echo validate; return;;
            crates/rimloc-cli/*) echo cli; return;;
            docs/*|mkdocs.yml|site/*) echo docs; return;;
            .github/*) echo ci; return;;
            test/*|crates/*/tests/*) echo tests; return;;
          esac
        done
        echo repo
      }
      [[ -z "$ctx_scope" ]] && ctx_scope=$(detect_scope)
      [[ -z "$ctx_type" ]] && ctx_type="chore"
      if [[ -n "$ctx_subject" ]]; then
        msg_subject="${ctx_type}(${ctx_scope}): ${ctx_subject}"
      else
        msg_subject="${ctx_type}(${ctx_scope}): apply session changes"
      fi
    fi
    if [[ -n "$session_dir" && -f "$session_dir/bullets.txt" ]]; then
      while IFS= read -r line; do [[ -n "$line" ]] && bullets+=("$line"); done < "$session_dir/bullets.txt"
    fi
    if [[ ${#bullets[@]} -eq 0 ]]; then
      [[ -n "$session" ]] && bullets+=("Commit only files touched in session '$session'")
      count=${#files_to_commit[@]}
      preview=$(printf '%s, ' "${files_to_commit[@]:0:3}" | sed 's/, $//')
      bullets+=("Update ${count} file(s): ${preview}")
    fi
    echo "--- commit message (composed) ---"
    echo "$msg_subject"
    echo
    for b in "${bullets[@]}"; do echo "- $b"; done
  fi
  rm -f "$current_changes" "$new_changes"
  exit 0
fi

# Stage only the new changes
for f in "${files_to_commit[@]}"; do
  git add -A -- "$f"
done

# Compose/validate commit message
commit_args=()
if [[ -n "$msg_file" ]]; then
  [[ -f "$msg_file" ]] || die "message file not found: $msg_file"
  commit_args+=( -F "$msg_file" )
else
  # Construct message from provided args or session context
  if [[ -z "$msg_subject" ]]; then
    ctx_type=""; ctx_scope=""; ctx_subject="";
    if [[ -n "$session_dir" ]]; then
      [[ -f "$session_dir/type.txt" ]] && ctx_type=$(<"$session_dir/type.txt")
      [[ -f "$session_dir/scope.txt" ]] && ctx_scope=$(<"$session_dir/scope.txt")
      [[ -f "$session_dir/subject.txt" ]] && ctx_subject=$(<"$session_dir/subject.txt")
    fi
    # Detect scope from paths if missing
    detect_scope() {
      for f in "${files_to_commit[@]}"; do
        case "$f" in
          crates/rimloc-core/*) echo core; return;;
          crates/rimloc-parsers-xml/*) echo parsers-xml; return;;
          crates/rimloc-export-po/*) echo export-po; return;;
          crates/rimloc-export-csv/*) echo export-csv; return;;
          crates/rimloc-import-po/*) echo import-po; return;;
          crates/rimloc-validate/*) echo validate; return;;
          crates/rimloc-cli/*) echo cli; return;;
          docs/*|mkdocs.yml|site/*) echo docs; return;;
          .github/*) echo ci; return;;
          test/*|crates/*/tests/*) echo tests; return;;
        esac
      done
      echo repo
    }
    [[ -z "$ctx_scope" ]] && ctx_scope=$(detect_scope)
    [[ -z "$ctx_type" ]] && ctx_type="chore"
    if [[ -n "$ctx_subject" ]]; then
      msg_subject="${ctx_type}(${ctx_scope}): ${ctx_subject}"
    else
      msg_subject="${ctx_type}(${ctx_scope}): apply session changes"
    fi
  fi

  # Merge bullets with session bullets
  if [[ -n "$session_dir" && -f "$session_dir/bullets.txt" ]]; then
    while IFS= read -r line; do [[ -n "$line" ]] && bullets+=("$line"); done < "$session_dir/bullets.txt"
  fi
  # Auto bullets if none provided
  if [[ ${#bullets[@]} -eq 0 ]]; then
    [[ -n "$session" ]] && bullets+=("Commit only files touched in session '$session'")
    count=${#files_to_commit[@]}
    preview=$(printf '%s, ' "${files_to_commit[@]:0:3}" | sed 's/, $//')
    bullets+=("Update ${count} file(s): ${preview}")
  fi

  tmp_msg=$(mktemp)
  {
    echo "$msg_subject"
    echo
    for b in "${bullets[@]}"; do echo "- $b"; done
  } > "$tmp_msg"
  commit_args+=( -F "$tmp_msg" )
fi

$no_verify && commit_args+=( --no-verify )

git commit "${commit_args[@]}"

# Cleanup baseline (next session should call --start anew)
rm -f "$current_changes" "$new_changes"
if [[ -w "$baseline_file" && -z "$session_dir" ]]; then
  rm -f "$baseline_file"
fi

echo "Commit created."
