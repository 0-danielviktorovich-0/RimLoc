---
title: Coverage & Codecov
---

# Coverage and Codecov

RimLoc uses [Codecov](https://codecov.io/gh/0-danielviktorovich-0/RimLoc) to track automated-test coverage across the repository.

Coverage is a **signal, not a correctness score**. Executing a line does not prove that RimWorld semantics, filesystem safety, UI quality, accessibility, or in-game behavior are correct.

## Current coverage families

The workflow uploads two independent Rust reports:

- **rust** — the Rust workspace under `crates/`, excluding the desktop Tauri crate;
- **gui-rust** — the desktop Rust/Tauri crate, which needs its own GTK/WebKit/frontend prerequisites.

Codecov **Components** then slice those reports into product areas without re-running tests per crate:

- Domain & Core;
- Services;
- RimWorld parsers & validation;
- import/export formats;
- CLI;
- providers/config;
- plugin compatibility layer;
- desktop Rust/Tauri.

This gives both a test-family view (**Flags**) and an architecture view (**Components**).

## Report format

Rust CI uses `cargo-llvm-cov` and emits Codecov's native coverage JSON rather than plain LCOV.

Why:

- it preserves richer LLVM region information;
- it avoids intentionally enabling cargo-llvm-cov's still-unstable branch-coverage mode;
- Codecov receives the format directly instead of reducing everything to line-only LCOV first.

Local HTML remains the easiest human report:

~~~bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --exclude rimloc-gui \
  --html
~~~

To mirror CI's upload format:

~~~bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --exclude rimloc-gui \
  --codecov \
  --output-path codecov.json
~~~

## Authentication

Uploads use **GitHub OIDC**. RimLoc does not need a repository `CODECOV_TOKEN`.

For same-repository PRs and pushes, Codecov receives the GitHub OIDC identity. Fork PRs never receive repository secrets; the workflow allows Codecov's public/tokenless path without making an external contributor's PR fail solely because upload authentication is unavailable.

## Pre-beta coverage policy

Coverage statuses are initially **informational**.

The repository deliberately does not start with an arbitrary global "80%" rule. First collect a real baseline from representative PRs, then tighten policy using evidence:

1. project coverage: `target: auto` + small regression tolerance;
2. patch coverage: reasonable expectation for newly changed code;
3. stricter component expectations for safety-critical code once its baseline is stable;
4. explicit exemptions only for generated/platform glue where justified.

The goal is to prevent regression and expose untested code, not optimize a vanity percentage.

## React coverage

React R1 currently has build/typecheck acceptance but no stable unit/component coverage report in its package scripts.

When the frontend test stack is ready, the intended contract is:

~~~text
npm run test:coverage
→ frontend-react/coverage/lcov.info (or another Codecov-supported report)
→ Codecov flag: react
→ Codecov component: frontend-react
~~~

Use the existing frontend test framework; do not introduce a second framework solely to satisfy Codecov.

## Test Analytics — future

Codecov can also ingest JUnit-style **test results** for Test Analytics (duration, failure rate, flaky-test visibility). RimLoc should add that only when the Rust/React test runners already produce a reliable JUnit report.

Do not run the full suite a second time only to create Test Analytics data.

## What coverage does not replace

Separate evidence remains required for:

- same-corpus RimWorld differential tests;
- in-game/runtime verification;
- path/symlink/adversarial filesystem tests;
- CodeQL, dependency review, Dependabot and cargo-deny;
- owner visual review;
- accessibility review;
- macOS/Windows artifact acceptance.

## CI files

- `.github/workflows/coverage.yml` — report generation/upload;
- `codecov.yml` — Codecov policy, components, flags and comments.

The workflow validates `codecov.yml` before generating reports.
