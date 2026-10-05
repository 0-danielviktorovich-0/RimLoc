---
title: Pull Requests
---

# Pull requests

RimLoc uses focused branches and PRs so code, documentation, security and UI evidence can be reviewed independently.

## Before opening a PR

1. Branch from the current integration/main line used by the task.
2. Keep the change scoped.
3. Read [CONTRIBUTING](../../../CONTRIBUTING.md) and [AGENTS](../../../AGENTS.md) when using coding agents.
4. Run the checks relevant to the files you changed.
5. Update user-facing EN/RU docs when behavior changed.

Do not mix an unrelated refactor, dependency sweep and product feature into one PR.

## Evidence by area

### Rust/domain/services

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

### React UI

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

Visible changes should include screenshots/same-state evidence where useful.

### Documentation

~~~bash
mkdocs build --strict
~~~

### Security-sensitive changes

Explain the threat/safety impact and include relevant containment, IPC, dependency, secret-handling or adversarial tests.

## Architecture checklist

A good PR should preserve these boundaries:

- domain behavior lives in Rust/shared services;
- React/CLI are product surfaces, not separate domain implementations;
- PO/CSV/XLIFF are interchange adapters unless the task is specifically format-related;
- RimWorld-specific semantics stay behind the adapter boundary;
- source game/mod folders remain read-only;
- automation/test hooks do not ship in production artifacts.

## PR description

Include:

- problem and outcome;
- important implementation choices;
- tests/evidence;
- screenshots for UI;
- migration/breaking impact;
- security impact where relevant;
- linked issues.

The repository PR template mirrors this checklist.

## Large migrations

Occasionally a migration checkpoint is necessarily large. In that case, isolate it from unrelated work, keep a durable migration report, and make follow-up PRs small again. Do not normalize permanent “hundreds of files per PR” as the everyday workflow.
