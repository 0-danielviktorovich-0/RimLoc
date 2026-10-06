---
title: Coverage & Codecov
---

# Coverage and Codecov

RimLoc uses [Codecov](https://codecov.io/gh/0-danielviktorovich-0/RimLoc) as a coverage dashboard for automated tests.

Coverage is useful, but it is **not a correctness score**. A line being executed does not prove that RimWorld semantics, filesystem safety, UI quality, accessibility, or in-game behavior are correct.

## Current coverage families

The coverage workflow uploads independent reports for:

- **Rust workspace** — generic crates under \`crates/\`;
- **Desktop Rust / Tauri** — the Rust desktop layer, which has extra WebKitGTK/GTK/frontend prerequisites.

Codecov Components then group the combined data by product subsystem, such as Domain & Core, Services, RimWorld parsing/validation, formats, CLI, providers/config, plugins, and desktop Rust.

## Authentication

Uploads use GitHub OIDC. There is no Codecov repository token in the workflow.

For public fork pull requests, the workflow is designed to fall back to Codecov's public/tokenless upload path rather than exposing secrets.

## Policy during pre-beta

Project, patch, and component coverage checks start as **informational** while a stable baseline is collected.

We intentionally do not set an arbitrary global “80%” gate. After several representative PRs, the plan is to:

1. block material regressions in overall project coverage using \`target: auto\`;
2. introduce a sensible patch-coverage expectation for new code;
3. use stricter expectations for safety-critical components;
4. add React component/unit coverage once the React test runner produces LCOV.

Coverage should improve behavior, not become a vanity metric.

## React coverage

React R1 currently has build/typecheck acceptance but does not yet expose a stable unit/component coverage report.

When the React test stack is ready, the expected contract is:

~~~text
npm run test:coverage
→ gui/tauri-app/frontend-react/coverage/lcov.info
→ Codecov flag/component: react
~~~

Use the existing frontend test framework; do not add a second framework solely for Codecov.

## What coverage does not replace

Separate evidence remains required for:

- same-corpus RimWorld differential tests;
- in-game/runtime verification;
- security/adversarial testing;
- CodeQL/Dependabot;
- owner visual review;
- accessibility review;
- macOS/Windows artifact acceptance.

## Local Rust coverage

With \`cargo-llvm-cov\` installed:

~~~bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --exclude rimloc-gui \
  --html
~~~

For the exact CI configuration, see \`.github/workflows/coverage.yml\` and \`codecov.yml\`.
