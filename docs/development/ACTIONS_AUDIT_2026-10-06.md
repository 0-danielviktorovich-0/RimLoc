# GitHub Actions audit — 2026-10-06

Status: **current pre-beta workflow audit**.

This document records the active workflow topology after the React/UI convergence,
documentation refresh, security hardening and Codecov rollout.

## Executive verdict

Active workflows are now separated by concern and generally follow least-privilege,
pinned-action and bounded-runtime rules.

| Workflow | Verdict | Purpose |
| --- | --- | --- |
| `ci.yml` | KEEP | core Rust/GUI/frontend quality gate |
| `dependency-review.yml` | KEEP | block newly introduced High/Critical runtime dependency risk |
| `docs.yml` | KEEP | strict MkDocs validation + controlled Pages deploy |
| `semver.yml` | KEEP, hardened | API compatibility gate + non-blocking API inventory |
| `changelog-check.yml` | KEEP | require changelog for user-facing PRs |
| `workflow-lint.yml` | KEEP | cheap actionlint before expensive jobs |
| `coverage.yml` | KEEP | Codecov Rust + Tauri coverage with OIDC |
| `release-plz.yml` | KEEP, manual | two-step release authority |
| `publish.yml` | KEEP temporarily | manual crates.io recovery path |

Parked `workflows-disabled/release-dev*.yml` remain intentionally inactive and
must be re-audited before reactivation.

## 1. CI

Current protections:

- Rust fmt + clippy;
- Rust tests on Linux/macOS/Windows;
- Tauri Rust build on Linux with required system dependencies;
- frozen Svelte fallback regression checks;
- React production-candidate typecheck/build;
- cargo-deny advisories/licenses/bans;
- generated-schema drift;
- job-level timeouts;
- concurrency cancels stale PR runs;
- docs-only PRs avoid the heavyweight matrix.

Remaining medium-term improvement:

- add React unit/component tests to the existing frontend stack, then attach
  coverage; do not introduce a second test framework only for coverage;
- after final React cutover, decide whether the frozen Svelte regression job can
  be removed.

## 2. Workflow lint

`workflow-lint.yml` is intentionally independent of the full CI matrix.

It runs only when workflow files change and validates every active workflow with
pinned actionlint.

This is the first line of defense for CI YAML mistakes.

## 3. Dependency review

`dependency-review.yml` runs only when dependency manifests/locks change.

Current policy:

- block newly introduced High/Critical runtime vulnerabilities;
- display patched-version and OpenSSF dependency information;
- existing dependency debt remains the responsibility of the security campaign,
  Dependabot and cargo-deny.

License review is currently not enabled in dependency-review. Rust license
policy remains in cargo-deny. If npm/plugin licensing becomes a material release
risk, add one explicit cross-ecosystem license policy instead of silently
enabling a default that may not match GPL-3.0 distribution requirements.

## 4. Documentation / Pages

`docs.yml`:

- runs strict MkDocs build;
- scopes PR runs to documentation/repository-policy paths;
- does not retain a Pages artifact for ordinary PR validation;
- deploys GitHub Pages only from `main` after a docs change or explicit manual
  dispatch;
- keeps PR preview opt-in.

This is the canonical docs path:

`docs/ → MkDocs → GitHub Pages`.

GitHub Wiki should not become a duplicate source of technical truth.

## 5. Codecov / coverage

Coverage is isolated in `coverage.yml`.

Current reports:

- Rust workspace;
- desktop Rust / Tauri.

Security/operability:

- Codecov GitHub App is connected;
- upload uses GitHub OIDC;
- no repository `CODECOV_TOKEN` is required;
- Codecov config is validated before coverage generation;
- all upload actions are pinned to immutable SHAs;
- fork PRs never receive repository secrets;
- timeout and concurrency policies are explicit.

Codecov configuration:

- project/patch coverage starts informational;
- Flags represent independent test families;
- Components represent architecture/product subsystems;
- reports use cargo-llvm-cov's Codecov-native format;
- React coverage is added only when a real frontend test runner emits coverage;
- Test Analytics is future work only when existing runners emit reliable JUnit
  without re-running the suite.

Do not set an arbitrary global percentage gate before the baseline stabilizes.

## 6. API stability

`semver.yml` has two distinct responsibilities:

- `cargo-semver-checks` = **gating**;
- `cargo-public-api` inventory = **informational**.

The informational job now uses `continue-on-error: true`, so a nightly/tool
failure cannot masquerade as a compatibility regression. The semver gate remains
strict.

## 7. Changelog

`changelog-check.yml` remains cheap and focused.

User-facing changes require `CHANGELOG.md`; true internal changes may use the
`internal-only` label.

## 8. Release safety

`release-plz.yml` is manual and uses explicit mutually exclusive modes:

- `release-pr`;
- `release`.

A single dispatch cannot create a Release PR and then immediately publish.

`publish.yml` is manual recovery only:

- dry-run by default;
- real publishing requires literal `PUBLISH` confirmation;
- crates.io token comes only from repository secrets;
- retries and timeout are bounded.

Do not enable automatic release/tagging before the owner-approved beta/RC gate.

## 9. Actions cost / runner policy

RimLoc is public. Standard GitHub-hosted runners for public repositories are
free and unlimited in runner minutes under GitHub's current public-repository
policy.

Still optimize for:

- fast contributor feedback;
- reduced redundant jobs;
- artifact/cache storage;
- fewer flaky external dependencies.

Do not use larger/paid runners without an explicit need.

## 10. Deferred improvements

Do after the relevant product foundation exists:

1. React unit/component coverage + Codecov `react` flag/component.
2. Codecov Test Analytics if existing test runners emit JUnit reliably.
3. Cross-platform packaged React/Tauri smoke (macOS/Windows/Linux) at RC cadence,
   not necessarily on every PR.
4. Stronger Codecov blocking gates after real baseline data.
5. Repository ruleset/branch protection with required checks after check names
   stabilize.
6. Optional cross-ecosystem license policy if plugin/npm distribution scope
   grows.
7. Remove frozen Svelte CI once React cutover is accepted.
8. Delete manual `publish.yml` after release-plz proves reliable across several
   releases.

## 11. Required-check recommendation after stabilization

Recommended minimum branch protection set for `main`:

- CI / rustfmt + clippy
- CI / cargo test (ubuntu-latest)
- CI / cargo-deny
- CI / schema drift
- API stability / cargo-semver-checks
- changelog / verify
- dependency review when dependency files change
- docs / mkdocs build when docs change

Codecov should remain informational initially; promote it to required only after
the baseline and status names are stable.

## 12. Security invariants

All active workflows should continue to satisfy:

- third-party actions pinned to immutable commit SHAs;
- minimum permissions per job;
- no secrets via workflow inputs;
- no `|| true` on required gates;
- irreversible release/deploy jobs are not cancelled mid-operation;
- stale validation jobs may be cancelled;
- explicit timeouts;
- no release/tag/publish triggered by ordinary code pushes.
