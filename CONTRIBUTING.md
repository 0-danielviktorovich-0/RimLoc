# Contributing to RimLoc

Thanks for your interest in improving RimLoc. This guide explains how to set up your environment, follow the project conventions, and submit changes that are easy to review and ship.

Русская версия доступна по ссылке: [docs/readme/ru/CONTRIBUTING.md](docs/readme/ru/CONTRIBUTING.md).

## Quick Start Checklist
- Fork the repository and create a topic branch from `main`.
- Install the latest stable Rust toolchain (via rustup) and ensure `cargo` is on your PATH.
- Build and test everything locally with `cargo build --workspace` and `cargo test --workspace`.
- Run `cargo fmt` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` before every commit (this mirrors CI).
- Use Conventional Commits (`type(scope): summary`) following `.gitmessage.txt`.
- Write commit subjects and bodies in English so reviewers share the same context.
- Open a pull request with a clear description, validation steps, and screenshots or CLI output when behaviour changes.

## Development Environment
- **Rust**: RimLoc targets stable Rust (Edition 2021). Install via [rustup](https://rustup.rs/) and keep it up to date (`rustup update`).
- **Optional tooling**: `cargo install cargo-watch` helps with on-save rebuilds, and `just` or `make` are not required.
- **GUI (Tauri + Svelte)**: The desktop client lives under `gui/tauri-app` — Rust backend in `src-tauri/`, front end in `frontend-v2/` (Svelte + TypeScript + Vite; this is the UI that ships). Working on the front end requires a Node.js toolchain. Follow Tauri's platform prerequisites to build the shell locally. The old `frontend/` shell is legacy and kept for reference only — do not extend it.
- **Python docs tooling**: Documentation lives in `docs/` and uses MkDocs. Create a virtualenv (`python -m venv .venv`), activate it, install `requirements-docs.txt`, then run `mkdocs serve` for local previews.

## Repository Layout
- `crates/`: Cargo workspace members.
  - `rimloc-domain`, `rimloc-core`, `rimloc-parsers-xml`: shared domain types, core translation logic, XML ingestion.
  - `rimloc-export-csv|po|xliff`, `rimloc-import-po|xliff`: format adapters for export/import.
  - `rimloc-validate`: validation rules; `rimloc-services`: orchestration shared by CLI and GUI; `rimloc-config`: configuration; `rimloc-llm`: LLM providers; `rimloc-plugin-api` / `rimloc-plugin-jsonftl`: plugin contracts and the built-in plugin.
  - `rimloc-cli`: Command-line interface entry point.
- `test/`: Shared fixtures for integration tests and manual checks.
- `testlab/`: Development lab — synthetic fixtures, off-screen UI automation journeys, adversarial cases and reports (dev-facing; not user documentation).
- `docs/`: MkDocs sources for the documentation site.
- `gui/tauri-app`: Tauri desktop client: `src-tauri/` (Rust backend), `frontend-v2/` (Svelte front end — the one that ships), `frontend/` (legacy v1 shell, reference only).
- `target/`: Build output (keep it out of commits).

## Building & Testing
- Full build: `cargo build --workspace`.
- Full test suite: `cargo test --workspace` (add `-- --nocapture` to view stdout).
- CLI smoke test: `cargo run -p rimloc-cli -- scan --root test/TestMod` for manual verification against bundled fixtures.
- Feature-specific tests: Use `cargo test --package <crate>` or `cargo test --features <feature>` when working behind feature flags.
- When adding new behaviour, prefer unit tests near the code and add or update integration tests under `crates/rimloc-cli/tests` using helpers in `helpers.rs`.
- Use `tempfile` for temporary directories in tests and add long-lived fixtures under `test/`.
- GUI front end: in `gui/tauri-app/frontend-v2` run `npm install`, then `npm run check` (svelte-check), `npm test` (Vitest) and `npm run build` (Vite). Tauri loads `frontend-v2/dist`, so rebuild the front end before `cargo tauri dev` to see your changes.

## Coding Standards
- Formatting: run `cargo fmt` before committing; do not hand-format code.
- Linting: `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass (no warnings allowed; same as CI).
- Naming conventions: modules/files/functions in `snake_case`, structs/enums in `PascalCase`, constants in `SCREAMING_SNAKE_CASE`, CLI flags in kebab-case.
- Logging/tracing: Prefer the existing `tracing` setup; avoid `println!` in library code.
- Error handling: Use `anyhow` in binaries and `thiserror` in libraries for typed errors.

## Localization Workflow
- English Fluent strings live in `crates/rimloc-cli/i18n/en/rimloc.ftl` and act as the source of truth.
- Update the English file first, then mirror changes to `ru` (the only other locale today). New locales go under `crates/rimloc-cli/i18n/<lang>/` and are discovered automatically by `build.rs`.
- Run `cargo test --package rimloc-cli -- tests_i18n` to validate keys.
- Keep keys lowercase with hyphens and document new strings in PR notes for translators.

## Documentation Changes
- Update inline crate documentation (`//!` and doc comments) alongside code changes.
- For site docs under `docs/`, use MkDocs Markdown. Preview locally with `mkdocs serve` (from the activated `.venv`).
- Commit only curated assets; generated content (`site/`, `target/`) should stay untracked.

## Commit Messages
- Follow the `.gitmessage.txt` template in the repository root (English only). Russian guidance is mirrored at `docs/readme/ru/gitmessage.txt`.
- Keep subjects within 72 characters, use lowercase type (`feat`, `fix`, etc.), and pick a scope when it clarifies the impact.
- Use the body to explain **what** changed and **why** the change matters, as bullet points starting with `- `.

Example message:

```
refactor(po): centralize simple PO parsing in rimloc-core

- expose `parse_simple_po` helper that understands msgid/msgstr plus reference lines
- reuse the shared `PoEntry` struct in importer and validator instead of local copies
- switch XML helpers to the core-level parser export
```

Release commits use a detailed body and include the publish order line:

```
chore(release): prep crates for crates.io (0.1.0-dev.0)

- Bump all RimLoc crates to 0.1.0-dev.0
- Add versioned deps for path crates; add metadata (license, repo, docs)
- Exclude logs from CLI package
- Normalize Ko-fi badge to ASCII hyphen to avoid % encoding issues

Run publish in order: core -> parsers -> exporters/importer -> validate -> cli.
```

## Repository Policies

### Commit scope policy (mandatory)
- Commit only files that were intentionally edited as part of the change. Do not include unrelated files.
- Avoid drive-by refactors, renames, and mass formatting across the repository. Keep diffs minimal and focused.
- Run `cargo fmt` but commit only the files you actually touched for the feature/fix. If a repo‑wide reformat is necessary, submit it as a dedicated, separate PR.
- Do not bump versions, shuffle modules, or update generated artifacts unless explicitly part of the task.

### No-revert policy (mandatory)
- Do not revert or discard changes without explicit consent from the maintainer/author.
- Exceptions: only when strictly required to fix broken builds/tests or to complete the current fix/feature. State the rationale clearly in the commit body.
- If you encounter unrelated, uncommitted local changes, ask whether to keep, commit, or drop them. Do not silently undo them.
- When a revert is required, use a dedicated commit referencing the original change (e.g., `revert: <hash> <subject>`). Avoid mixing reverts with functional changes.

## Submitting Changes
1. Keep commits focused and descriptive. Use `.gitmessage.txt` template (`type(scope): summary`).
2. Rebase on top of `main` before opening a pull request to avoid merge conflicts.
3. In the PR description, include:
   - What changed and why.
   - How you validated the change (commands, tests, screenshots).
   - Any follow-up work or known limitations.
4. Ensure CI passes (build, lint, tests, docs if touched).
5. Respond to review feedback promptly; keep discussions respectful and actionable.

## Reporting Issues & Feature Requests
- Use GitHub Issues with clear steps, expected vs actual behaviour, and environment details. See the [Issue Guidelines](docs/en/community/issues.md) for the checklist and examples.
- For translation or localisation issues, mention the locale and provide sample strings.
- Security vulnerabilities are reported privately — see [SECURITY.md](SECURITY.md). Do not open a public issue with exploitation details.

## Need Help?
If you get stuck:
- Search existing issues and discussions.
- Review `AGENTS.md` for automation-specific conventions.
- Open a draft PR early to gather feedback.
- Open a GitHub issue — that is currently the only support channel (GitHub Discussions are not enabled).

We appreciate your contributions—thank you for helping RimLoc grow!
