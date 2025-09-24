# Repository Guidelines

## Project Structure & Module Organization
RimLoc is organised as a Cargo workspace under `crates/`. Core translation logic lives in `rimloc-core`, XML ingestion in `rimloc-parsers-xml`, and exporters/importers each have their own crate. The CLI entry point is `crates/rimloc-cli/src/main.rs`, with integration fixtures stored under `test/`. `docs/` contains the MkDocs site sources, while `gui/tauri-app` hosts the experimental desktop shell. Keep generated output in `target/` and commit only curated assets in `docs/`.

## Build, Test, and Development Commands
- `cargo build --workspace` builds every crate and checks cross-crate interfaces.
- `cargo run -p rimloc-cli -- scan --root test/TestMod` exercises the CLI end-to-end during manual checks.
- `cargo test --workspace` runs unit and integration suites; append `-- --nocapture` to inspect stdout.
- `cargo fmt && cargo clippy --workspace --all-targets -- -D warnings` ensures formatting and lint cleanliness before review.
- `mkdocs serve` (from the `.venv`) previews the documentation site locally.

## Coding Style & Naming Conventions
Rust code uses the default 4-space rustfmt profile; rely on `cargo fmt` instead of hand-formatting. Modules, files, and functions stay in `snake_case`; structs/enums use `PascalCase`; constants are `SCREAMING_SNAKE_CASE`. CLI arguments follow long-form kebab-case to match existing subcommands. When editing Fluent localisation files under `crates/rimloc-cli/i18n`, keep keys lowercase with hyphens and update English (`en`) first.

## Testing Guidelines
Prefer unit tests alongside the code they assert. Integration tests for the CLI live in `crates/rimloc-cli/tests`; group scenarios in descriptive modules and reuse helpers from `helpers.rs`. Add sample XML or PO fixtures to `test/` and clean up temporary files via `tempfile`. Run `cargo test --features <feature>` if you introduce gated functionality, and cover new subcommands or exporters.

## Documentation Workflow
- Run `mkdocs serve` from the repo root while editing; it mirrors `docs/en/` and `docs/ru/` with live reload.
- Keep English and Russian pages structurally aligned—add the same sections to both locales in the same commit.
- Build production docs locally with `SITE_URL=https://0-danielviktorovich-0.github.io/RimLoc/ mkdocs build` when you need to verify absolute links.
- Exclude experimental drafts by placing them outside `docs/` or listing them under `exclude_docs` in `mkdocs.yml`.

## Commit & Pull Request Guidelines
Follow the Conventional Commit template captured in `.gitmessage.txt`: `type(scope): summary` within 72 characters, using types such as `feat`, `fix`, `docs`, or `chore`. Commit messages must be written in English; a Russian reference lives at `docs/readme/ru/gitmessage.txt`. Commit bodies should explain motivation and impact. Pull requests need a concise summary, linked issues, and instructions for validation; attach CLI output or screenshots when behaviour changes. Ensure CI passes and that formatting, lint, and test checks are green before requesting review.

### Commit scope policy (mandatory)
- Commit only files that were intentionally edited as part of the change. Do not include unrelated files.
- Avoid drive‑by refactors, renames, and mass formatting across the repository. Keep diffs minimal and focused.
- Run `cargo fmt` but commit only the files you actually touched for the feature/fix. If a repository‑wide reformat is necessary, submit it as a dedicated, separate PR.
- Do not bump versions, shuffle modules, or update generated artifacts unless explicitly part of the task.
- This rule applies to humans and to automated agents working in this repo — agents must obey it as well.

## Localization Workflow Notes
Translations for the CLI ship via `i18n/<lang>/rimloc.ftl` and are embedded at build time. Update the English source, mirror changes to other locales, and run `cargo i18n` (if available) or `cargo test --package rimloc-cli -- tests_i18n` to confirm key integrity.
