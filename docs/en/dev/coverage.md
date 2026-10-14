---
title: Coverage & Codecov
---

# Coverage and Codecov

RimLoc uses [Codecov](https://codecov.io/gh/0-danielviktorovich-0/RimLoc) to track automated-test coverage across the repository.

Coverage is a **signal, not a correctness score**. Executing a line does not prove that RimWorld semantics, filesystem safety, UI quality, accessibility, or in-game behavior are correct.

## Current coverage families

The workflow uploads three independently generated reports:

- **rust** — Rust workspace under `crates/`, excluding desktop Tauri;
- **gui-rust** — the Tauri Rust crate with its own GTK/WebKit/frontend prerequisites;
- **frontend-react** — full production React source (`src/**/*.{ts,tsx}`), with untested screens included as 0%, not hidden.

Codecov **Components** slice those reports into product areas without re-running tests:

- Domain & Core; Services; RimWorld parsers & validation; import/export formats;
- CLI; providers/config; plugin compatibility; desktop Rust/Tauri; React frontend.

**Flags** identify uploaded test families; **Components** describe architectural areas.

## Report format

Rust CI uses `cargo-llvm-cov` and emits Codecov's native coverage JSON rather than plain LCOV. React uses Vitest V8 and uploads Istanbul JSON (`coverage/coverage-final.json`), with text and LCOV generated for local inspection.

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

## Configuration trust

Codecov reads repository policy only from the default branch: `codecov.strict_yaml_branch: main` is enabled after the first successful `main` Rust and desktop coverage uploads. A pull request can change coverage-producing code, but it cannot weaken the Codecov policy used to judge that same pull request.

## Pre-beta coverage policy

Coverage statuses are initially **informational**.

The repository deliberately does not start with an arbitrary global "80%" rule. First collect a real baseline from representative PRs, then tighten policy using evidence:

1. project coverage: `target: auto` + small regression tolerance;
2. patch coverage: reasonable expectation for newly changed code;
3. stricter component expectations for safety-critical code once its baseline is stable;
4. explicit exemptions only for generated/platform glue where justified.

The goal is to prevent regression and expose untested code, not optimize a vanity percentage.

## React coverage

The React job in `.github/workflows/coverage.yml` runs `npm test` in
`gui/tauri-app/frontend-react` (Vitest V8 with `test.coverage`) and uploads
`coverage/coverage-final.json` under Codecov flag `frontend-react`.
A job-level guard checks the JSON and the presence of `App.tsx`,
`Workspace.tsx`, `project.ts`, and `entryUid.ts` in the report.

The report deliberately includes **all production `src/**/*.{ts,tsx}` files**
rather than only audited state modules. Untested UI components therefore
appear as 0%; this is an honest baseline, not a release-blocking score.
Always examine `project.ts` and `entryUid.ts` separately when reviewing
Data Safety changes. Passing Rust/Tauri tests does not establish frontend
coverage or native packaged-app behavior.

## Pull request review: do not stop at green checks

- Review the Codecov **bot comment**, patch/project/flag/component statuses,
  actual missing changed lines, and all other bot/reviewer threads for the
  **exact PR HEAD**. `informational: true` means a green Codecov status
  can be below target; upload success only means it was queued.
- **`strict_yaml_branch: main` is deliberate**: during a PR, Codecov uses
  the `main` policy, not unmerged PR edits to `codecov.yml`. Confirm
  the effective `main` `ignore`/component/flag configuration before
  claiming a newly introduced source area is actually measured.
- Be careful with path-prefix exclusions: `ignore: "gui/tauri-app/frontend"`
  also matches `frontend-react`. The legacy `frontend/` exclusion must
  end in a slash, and the new flag must be observable after Codecov finishes
  processing; a flag marked "Coverage not affected" requires investigation.
- Inspect the runner's *per-file* table and the generated file inventory.
  `coverage.include` belongs under `test.coverage`; an option at the
  Vite top level is ignored. A passing job with an incorrect denominator is
  not evidence of the claimed coverage.
- If the denominator, exclusion policy, or baseline changes, explicitly
  label percentage comparisons as **non-comparable**. Never hide code,
  remove an uncovered source file, or relax policy to improve a number.
  Add behavioral/negative tests for real risk and document justified gaps.
- Supplement coverage with Tauri IPC integration tests, native WDIO,
  RimWorld export checks, CodeQL/dependency scanning, and owner UAT.

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
