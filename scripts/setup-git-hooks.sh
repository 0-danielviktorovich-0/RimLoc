#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_root"

git config core.hooksPath .githooks
echo "Configured Git hooks path to: .githooks"
echo "commit-msg hook will now enforce Conventional Commits + bullet body."

