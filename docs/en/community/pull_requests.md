---
title: Pull Requests
---

# Pull Requests

This guide explains how to prepare changes for review and what a good PR looks like in RimLoc.

## Workflow

1) Create a topic branch from `main`.
2) Keep commits focused and descriptive (use Conventional Commits in messages).
3) Write tests (unit/integration) alongside code changes.
4) Run local checks before pushing:

```bash
cargo build --workspace
cargo test --workspace
cargo fmt && cargo clippy --workspace --all-targets -- -D warnings
```

5) If CLI flags/behavior changed — update docs under `docs/` and i18n help keys.
6) Open a PR with validation steps and output snippets.

## What to include in the PR description

- Summary: what changed and why.
- Type: fix/feat/docs/refactor/chore.
- Validation: commands you ran and key outputs (use `--quiet` for JSON pipelines).
- Impact: docs updated? i18n keys touched? any migration notes?
- Related issues: `Closes #123`.

## Size and structure

- Prefer several small, logical commits over one large commit.
- Avoid drive-by refactors or repo-wide formatting unless the PR is dedicated to that.
- Keep diffs minimal and focused on the task.

## Testing expectations

- Add or update tests close to the code you change.
- CLI integration tests live in `crates/rimloc-cli/tests/`; reuse helpers in `helpers.rs`.
- For i18n keys, run `cargo test --package rimloc-cli -- tests_i18n`.

## Documentation and i18n

- Help text is localized via Fluent. Update EN keys first, then mirror to other locales.
- For new flags, update:
  - FTL help keys (EN/RU)
  - CLI pages in `docs/en/cli/` and `docs/ru/cli/`
  - Testing docs if logs/flags change

## PR Template

The repository includes `.github/PULL_REQUEST_TEMPLATE.md` with a checklist — use it to keep reviews fast and predictable.

