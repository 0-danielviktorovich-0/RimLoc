# Contributing to RimLoc

Thanks for helping RimLoc. The project is RimWorld-first today, but the architecture is deliberately adapter-oriented so new games and applications can be added without rewriting the localization core.

Русская пользовательская документация: [docs/ru/](docs/ru/).

## Start here

1. Fork or clone the repository and create a focused branch.
2. Read [AGENTS.md](AGENTS.md) if you are using a coding agent.
3. Build the relevant part of the project.
4. Add tests/evidence for the behavior you changed.
5. Keep the diff scoped and use Conventional Commits.
6. Open a PR with what changed, why, and how you verified it.

## Repository architecture

- <code>crates/rimloc-domain</code> — canonical types/project contracts; no UI logic.
- <code>crates/rimloc-core</code> — generic localization logic.
- <code>crates/rimloc-services</code> — orchestration and filesystem-aware services.
- <code>crates/rimloc-*</code> — parsers/import/export/validation/provider components.
- <code>crates/rimloc-cli</code> — thin CLI surface over shared services.
- <code>gui/tauri-app/frontend-react</code> — intended production React 19 frontend.
- <code>gui/tauri-app/frontend-v2</code> — frozen Svelte fallback during convergence.
- <code>gui/tauri-app/src-tauri</code> — Tauri 2 / Rust desktop bridge.
- <code>docs/</code> — canonical documentation sources (MkDocs).
- <code>test/</code> / <code>testlab/</code> — fixtures and controlled acceptance infrastructure.

Business/domain behavior should live in Rust services/core, not be reimplemented in React.

## PO and other interchange formats

PO is an interchange format, not the canonical RimLoc project model.

Format-specific commands such as <code>export-po</code> and <code>import-po</code> remain useful for external CAT workflows, but new product behavior should prefer canonical project state plus import/export adapters.

Do not make a new feature depend on PO unless the feature is specifically about PO interoperability.

## Adding another game or application

RimWorld-specific knowledge should stay behind the localization-adapter boundary.

Read:

- [Localization adapter architecture](docs/architecture/LOCALIZATION_ADAPTERS.md)
- [Adapter authoring guide](docs/en/dev/adapters.md)

A useful built-in adapter normally needs to:

1. declare identity and capabilities;
2. detect/discover its source;
3. map source data into canonical <code>SourceEntry</code> inventory;
4. import existing translations where applicable;
5. expose source context/provenance;
6. validate target-specific rules;
7. build/export safely;
8. add fixtures and conformance tests.

Do not put Minecraft, RimWorld, or other game concepts into the generic core simply because one adapter needs them.

There is **not yet a stable third-party binary plugin ABI** for arbitrary localization adapters. Keep new first-party adapters in-tree until a versioned external protocol is designed.

## Build and test

Rust:

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

React frontend:

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

Svelte fallback should only be changed when a task explicitly targets it.

Docs:

~~~bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs build --strict
~~~

## Documentation

<code>docs/</code> is the canonical documentation source. README is the landing page. Do not create a second technical source of truth in GitHub Wiki.

Keep EN/RU user-facing pages structurally aligned when practical. If a translation lags, mark it honestly rather than silently documenting different behavior.

## Safety

- Treat RimWorld/Workshop/source mod folders as read-only inputs.
- Use isolated temporary/output directories in tests.
- Do not commit credentials, tokens or real provider keys.
- Production artifacts must not include automation/test bridges.
- Do not add broad Tauri/filesystem/shell permissions without a documented need.
- Prefer dry-run and containment tests for write paths.

## Commits and PRs

Use Conventional Commits, for example:

~~~text
feat(adapter): add source discovery capability

- map source records into canonical entries
- add update and containment fixtures
~~~

Keep commits focused. Do not force-push shared history or create releases/tags unless explicitly authorized.

Public Git metadata should describe the engineering outcome, not the development orchestration.
Do not put internal agent terms such as mandates, waves, lanes, reviewer iterations, workflow ids,
or reviewer-protocol directives in commit subjects, PR/release titles, or user-facing changelog text.
Run `python3 scripts/check-public-git-language.py --help` for the repository guard and see
[Public Git History Policy](docs/development/PUBLIC_GIT_HISTORY_POLICY.md) for examples.

A PR should include:

- summary and motivation;
- user/developer impact;
- tests/commands run;
- screenshots for visible UI changes;
- security implications if filesystem/network/IPC/secrets changed;
- docs updates for user-facing behavior.

## Changelog and versions

RimLoc uses SemVer and a curated <code>CHANGELOG.md</code>.

During pre-beta development, do not bump versions or publish just because a feature landed. Release preparation is a separate controlled step.

## Need help?

Use [GitHub Issues](https://github.com/0-danielviktorovich-0/RimLoc/issues) for bugs/features and [SUPPORT.md](SUPPORT.md) for support guidance. Security issues belong in the private process described in [SECURITY.md](SECURITY.md).
