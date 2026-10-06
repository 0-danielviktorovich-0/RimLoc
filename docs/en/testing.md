---
title: Testing & Reporting
---

# Testing and reporting

RimLoc has several layers of tests. A green unit suite is not the same as a proven desktop workflow or an in-game result.

## Evidence levels

Use the strongest evidence that actually exists:

1. source/code inspection;
2. unit tests;
3. integration tests;
4. built desktop/CLI E2E;
5. same-corpus differential testing;
6. in-game/runtime proof.

Do not upgrade a claim just because a UI screen exists.

## Rust workspace

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

The GUI crate has platform/frontend prerequisites; CI handles it in its dedicated GUI job.

## React frontend

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

React R1 semantic/WDIO acceptance is maintained separately from simple build/typecheck.

## Svelte fallback

Only when the frozen fallback changes:

~~~bash
cd gui/tauri-app/frontend-v2
npm ci
npm run check
npm test
npm run build
~~~

## CLI smoke

Use the bundled fixture or an isolated copy of a real mod:

~~~bash
rimloc-cli scan --root ./test/TestMod --format json
rimloc-cli validate --root ./test/TestMod
~~~

PO-specific tests are interoperability tests, not the whole product:

~~~bash
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
rimloc-cli validate-po --po ./logs/TestMod.po --strict
~~~

A no-PO build path also exists:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

## Desktop acceptance

For owner/user testing, verify the exact packaged artifact identity:

- source SHA;
- frontend flavor;
- automation flag;
- app/binary SHA where provided.

Do not test a stale or automation build as if it were the production candidate.

Automation should use an isolated data/profile directory so synthetic projects never appear in the owner's Recent Projects.

## Security checks

Security-sensitive work should include the relevant combination of:

- cargo-deny / advisory review;
- JS dependency review;
- CodeQL;
- secret scan;
- Tauri capability/IPC review;
- path traversal and symlink containment tests;
- production automation-bridge exclusion.

See the internal audit material under <code>docs/security/</code> and the public [SECURITY policy](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md).

## Documentation

~~~bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs build --strict
~~~

## Filing a bug

Use the GitHub bug form and include:

- exact build/version/commit;
- OS;
- area (React desktop, CLI, adapter, build/export, etc.);
- reproducible steps;
- expected vs actual;
- sanitized logs/diagnostics;
- a small fixture/project if possible.

Never attach API keys, tokens, private paths, or unreviewed diagnostic bundles publicly.
