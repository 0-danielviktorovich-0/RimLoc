# Codecov / coverage strategy

Status: pre-beta baseline, October 2026.

This file documents how RimLoc uses coverage. It is an engineering signal, not a substitute for integration, same-corpus, visual, accessibility, security, or in-game evidence.

## Current integration

- Codecov GitHub App: installed and responding to PRs.
- Upload authentication: GitHub OIDC; no repository Codecov token is required for normal same-repository runs.
- Rust coverage: cargo-llvm-cov → LCOV → Codecov.
- Coverage uploads are split by test family:
  - rust — generic workspace crates;
  - gui-rust — Tauri/Rust desktop layer.
- Codecov Components divide the combined reports into product subsystems without rerunning the whole test suite per crate.
- Project and patch statuses are intentionally informational while a stable baseline is collected. Components are shown in the PR comment/dashboard without spawning a separate status check for every subsystem.
- GitHub line annotations are disabled because Codecov is deprecating them and they conflict with Components/Flags. The PR comment and Codecov file view are the canonical coverage review surfaces.

## Components

- Domain & Core
- Services
- RimWorld parsers & validation
- Import / Export formats
- CLI
- Providers & Config
- Plugin compatibility layer
- Desktop Rust / Tauri

A future React component should only be activated after the React frontend has a real unit/component coverage runner. Do not report a fake 0% frontend simply because no coverage-producing test runner exists yet.

## What coverage is allowed to prove

Coverage can answer:

- which executable lines were exercised by the instrumented test suites;
- how coverage changed in a patch;
- which subsystem has obvious untested regions;
- whether new code is arriving without corresponding tests.

Coverage cannot prove:

- that extracted RimWorld strings are semantically correct;
- that a generated translation works in game;
- visual/UX quality;
- accessibility quality;
- absence of security vulnerabilities;
- absence of race conditions or filesystem escape bugs;
- that mocks match real integrations.

Those require their own evidence.

## Rollout

### Phase 0 — baseline (current)

- project/patch statuses informational; component views informational without per-component check spam;
- collect several representative PRs;
- verify path mapping and component filters;
- find generated/vendor/test files that should not count;
- do not set an arbitrary “80% overall” gate.

### Phase 1 — regression protection

After the baseline is stable:

- make project coverage blocking with target=auto;
- allow a small threshold (around 0.5–1 percentage point) to avoid noise;
- keep patch coverage visible;
- add component-specific blocking statuses only for subsystems where a separate gate is genuinely useful.

The goal is “do not silently make the tested surface materially worse”, not “chase a vanity number”.

### Phase 2 — new-code quality

Once coverage-producing tests exist for the main product surfaces:

- introduce a patch-coverage expectation for new Rust code;
- use stricter component expectations for safety-critical code (persistence, filesystem containment, parsing/validation);
- add React unit/component coverage as a separate flag/report;
- keep WDIO/E2E, owner visual review and game runtime evidence separate from line coverage.

### Phase 3 — beta/release policy

Before a public beta/release, coverage policy should be one input into the release gate, alongside:

- Rust/React tests;
- security audits;
- CodeQL/Dependabot;
- same-corpus competitor/runtime tests;
- artifact identity;
- macOS/Windows acceptance;
- owner visual/a11y acceptance.

A release must never be approved only because Codecov is green.

## React coverage contract (future)

When the React lane has stable unit/component tests, add a deterministic script that produces:

gui/tauri-app/frontend-react/coverage/lcov.info

Suggested script contract:

npm run test:coverage

Then upload it with a dedicated Codecov flag such as react and add a React component. Prefer Vitest + v8 coverage if that matches the chosen frontend test stack. Do not introduce a second frontend test framework merely for Codecov.

## Tauri / desktop coverage

The Tauri Rust layer has its own coverage job because it needs WebKitGTK/GTK and a built frontendDist on Linux. Vendor code is excluded from Codecov. The validator, generic Rust upload, and desktop Rust upload have all completed successfully on the integration PR.

Automation-only features are not a reason to inflate production coverage. Coverage should reflect the production/default desktop contract unless a dedicated automation test report is explicitly useful.

## Fork pull requests

Same-repository runs use OIDC. Public fork PRs should be able to fall back to Codecov’s public/tokenless upload behavior rather than exposing repository secrets. Do not add CODECOV_TOKEN to fork-readable workflow paths.

## CI cost

RimLoc is public, so standard GitHub-hosted runner minutes are not charged against the normal private-repository included-minute allowance. We still optimize for feedback speed and avoid redundant jobs/artifacts.

Current principles:

- Linux-only instrumentation unless OS-specific coverage is genuinely needed;
- docs-only PRs skip coverage;
- use Components instead of rerunning tests per crate;
- no stored LCOV artifact unless troubleshooting requires it;
- cancel stale coverage runs for the same PR/ref.

## Maintenance

When changing codecov.yml:

1. validate it with Codecov’s validator;
2. keep GitHub Actions pinned to exact commit SHAs;
3. verify at least one real upload;
4. check the PR comment/components;
5. update this document if the policy changes.

When the policy becomes blocking, consider locking Codecov YAML to the protected/default branch so an ordinary PR cannot weaken the coverage gate inside the same change.
