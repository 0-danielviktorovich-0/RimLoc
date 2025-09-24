# Contributing to RimLoc

Thanks for your interest in improving RimLoc. This guide explains how to set up your environment, follow the project conventions, and submit changes that are easy to review and ship.

Русская версия доступна по ссылке: [docs/readme/ru/CONTRIBUTING.md](docs/readme/ru/CONTRIBUTING.md).

## Quick Start Checklist
- Fork the repository and create a topic branch from `main`.
- Install the latest stable Rust toolchain (via rustup) and ensure `cargo` is on your PATH.
- Build and test everything locally with `cargo build --workspace` and `cargo test --workspace`.
- Run `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` before every commit.
- Use Conventional Commits (`type(scope): summary`) following `.gitmessage.txt`.
- Write commit subjects and bodies in English so reviewers share the same context.
- Open a pull request with a clear description, validation steps, and screenshots or CLI output when behaviour changes.

## Development Environment
- **Rust**: RimLoc targets stable Rust (Edition 2021). Install via [rustup](https://rustup.rs/) and keep it up to date (`rustup update`).
- **Optional tooling**: `cargo install cargo-watch` helps with on-save rebuilds, and `just` or `make` are not required.
- **GUI experiments**: The Tauri desktop shell lives under `gui/tauri-app`. Follow Tauri's platform prerequisites if you plan to work there.
- **Python docs tooling**: Documentation lives in `docs/` and uses MkDocs. Create a virtualenv (`python -m venv .venv`), activate it, install `requirements-docs.txt`, then run `mkdocs serve` for local previews.

## Repository Layout
- `crates/`: Cargo workspace members.
  - `rimloc-core`: Core translation and validation logic.
  - `rimloc-parsers-xml`: XML ingestion utilities.
  - `rimloc-export-*` / `rimloc-import-*`: Exporters and importers for PO/CSV.
  - `rimloc-cli`: Command-line interface entry point.
  - `rimloc-validate`: Shared validation routines.
- `test/`: Shared fixtures for integration tests and manual checks.
- `docs/`: MkDocs sources for the documentation site.
- `gui/tauri-app`: Experimental desktop client shell.
- `target/`: Build output (keep it out of commits).

## Building & Testing
- Full build: `cargo build --workspace`.
- Full test suite: `cargo test --workspace` (add `-- --nocapture` to view stdout).
- CLI smoke test: `cargo run -p rimloc-cli -- scan --root test/TestMod` for manual verification against bundled fixtures.
- Feature-specific tests: Use `cargo test --package <crate>` or `cargo test --features <feature>` when working behind feature flags.
- When adding new behaviour, prefer unit tests near the code and add or update integration tests under `crates/rimloc-cli/tests` using helpers in `helpers.rs`.
- Use `tempfile` for temporary directories in tests and add long-lived fixtures under `test/`.

## Coding Standards
- Formatting: run `cargo fmt` before committing; do not hand-format code.
- Linting: `cargo clippy --workspace --all-targets -- -D warnings` must pass (no warnings allowed).
- Naming conventions: modules/files/functions in `snake_case`, structs/enums in `PascalCase`, constants in `SCREAMING_SNAKE_CASE`, CLI flags in kebab-case.
- Logging/tracing: Prefer the existing `tracing` setup; avoid `println!` in library code.
- Error handling: Use `anyhow` in binaries and `thiserror` in libraries for typed errors.

## Localization Workflow
- English Fluent strings live in `crates/rimloc-cli/i18n/en/rimloc.ftl` and act as the source of truth.
- Update the English file first, then mirror changes to other locales under the same directory structure.
- Run `cargo test --package rimloc-cli -- tests_i18n` (or `cargo i18n` if available) to validate keys.
- Keep keys lowercase with hyphens and document new strings in PR notes for translators.

## Documentation Changes
- Update inline crate documentation (`//!` and doc comments) alongside code changes.
- For site docs under `docs/`, use MkDocs Markdown. Preview locally with `mkdocs serve` (from the activated `.venv`).
- Commit only curated assets; generated content (`site/`, `target/`) should stay untracked.

## Commit Messages
- Follow the `.gitmessage.txt` template in the repository root (English only). Russian guidance is mirrored at `docs/readme/ru/gitmessage.txt`.
- Keep subjects within 72 characters, use lowercase type (`feat`, `fix`, etc.), and pick a scope when it clarifies the impact.
- Use the body to explain **what** changed and **why** the change matters.

Example message:

```
refactor(po): centralize simple PO parsing in rimloc-core

- expose `parse_simple_po` helper that understands msgid/msgstr plus reference lines
- reuse the shared `PoEntry` struct in importer and validator instead of local copies
- switch XML helpers to the core-level parser export
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
- Security vulnerabilities should be reported privately to the maintainer (see repository contact info).

## Need Help?
If you get stuck:
- Search existing issues and discussions.
- Review `AGENTS.md` for automation-specific conventions.
- Open a draft PR early to gather feedback.
- Reach out on the project's preferred communication channel (GitHub Discussions or linked community spaces).

We appreciate your contributions—thank you for helping RimLoc grow!
