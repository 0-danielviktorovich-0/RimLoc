# Repository Guidelines

## Project Structure & Module Organization
RimLoc is organised as a Cargo workspace under `crates/`. Core translation logic lives in `rimloc-core`, XML ingestion in `rimloc-parsers-xml`, and exporters/importers each have their own crate. The CLI entry point is `crates/rimloc-cli/src/main.rs`, with integration fixtures stored under `test/`. `docs/` contains the MkDocs site sources, while `gui/tauri-app` hosts the experimental desktop shell. Keep generated output in `target/` and commit only curated assets in `docs/`.

### Architecture invariants (mandatory)
- Preserve crate boundaries and responsibilities:
  - `rimloc-domain` — shared types/JSON schemas; no IO.
  - `rimloc-core` — core logic; no UI/CLI specifics.
  - `rimloc-parsers-xml` — XML reading/parsing only.
  - `rimloc-export-*` / `rimloc-import-*` — format adapters and IO for export/import.
  - `rimloc-validate` — validation rules and checks.
  - `rimloc-services` — orchestration/helpers reusable by CLI/GUI; file IO allowed.
  - `rimloc-cli` — thin command layer only; no business logic.
- New features land in the appropriate crate (prefer `rimloc-services` for orchestration) and are exposed via the CLI; do not place core logic in the CLI.
- Keep outputs and contracts stable (CSV/JSON/PO). For breaking JSON changes, bump `OUTPUT_SCHEMA_VERSION`, regenerate schemas via `rimloc-cli schema`, and update docs.
- Remain platform‑neutral in shared crates; guard OS‑specific code behind features and keep it out of core logic.
- Avoid adding heavy dependencies or cross‑cutting frameworks without prior discussion; prefer small, focused crates.

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

### Testing policy (mandatory)
- After any change (code or docs), run local checks before committing:
  - `cargo build --workspace`
  - `cargo test --workspace` (append `-- --nocapture` when investigating)
  - `cargo fmt && cargo clippy --workspace --all-targets -- -D warnings`
  - Commit your changes using the auto-commit workflow below; do not end a task with uncommitted edits.
- If you touch docs under `docs/`, preview or build the site:
  - `mkdocs serve` locally from a virtualenv, or
  - `SITE_URL=https://0-danielviktorovich-0.github.io/RimLoc/ mkdocs build` to validate links.
- If you modify i18n keys, run `cargo test --package rimloc-cli -- tests_i18n` to verify key integrity across locales.
- If you change CLI flags or behavior, update integration tests in `crates/rimloc-cli/tests` and rerun the whole test suite.
- Automated agents must also execute these checks and report a short summary of results back to the user.

## Documentation Workflow
- Run `mkdocs serve` from the repo root while editing; it mirrors `docs/en/` and `docs/ru/` with live reload.
- Keep English and Russian pages structurally aligned—add the same sections to both locales in the same commit.
- Build production docs locally with `SITE_URL=https://0-danielviktorovich-0.github.io/RimLoc/ mkdocs build` when you need to verify absolute links.
- Exclude experimental drafts by placing them outside `docs/` or listing them under `exclude_docs` in `mkdocs.yml`.

## Release Workflow
- Do not bump versions by hand; use configured tooling (`release-plz`/`cargo-release`) and GitHub Actions (see `release.toml`, `release-plz.toml`).
- Before tagging: update `CHANGELOG.md` under `Unreleased`, run build/tests/lints, and ensure docs are in sync (EN/RU).
- Tag and push via the release workflow; artifacts/signatures and SBOMs are handled by CI (see docs in `docs/en/dev/index.md`).

## Automation Rules (agents)
- Keep diffs minimal and scoped to the task; no drive‑by refactors or large renames.
- Never rewrite history or revert without approval; follow the no‑revert policy below.
- Always run and report: `cargo build/test`, `fmt`, and `clippy` after changes.
- Language preference: if the user writes in Russian, respond and continue the conversation in Russian.
- Ask before destructive actions (deletes/moves/format‑sweeps). Scope of this file is repository‑wide.
- Mandatory: always finish by committing via `scripts/agent-commit.sh`. If committing is not possible in the harness, include the exact commit message and file list in the final reply and ask the user to run the command.

### Secrets & External Services (GH_TOKEN)
- Allowed: when the user explicitly provides `GH_TOKEN`/`GITHUB_TOKEN` and asks to use it, agents may perform the requested GitHub operations (e.g., API calls, cloning private repos, fetching releases).
- How to pass: use an environment variable only (example: `export GH_TOKEN=…`); never hardcode tokens or write them to files under version control.
- Safety: do not print tokens in logs or command output; avoid echoing env vars. If output may include headers, redact them.
- Scope: use the token strictly for the requested operation. Do not publish, tag, or modify GitHub state (releases, labels, settings) unless the user explicitly asks.
- Cleanup: avoid persisting tokens in scripts/commits; unset after use if appropriate (`unset GH_TOKEN`).

### GUI Dependencies (exception for gui/)
- To deliver a high‑quality long‑term Tauri GUI, dependencies in `gui/` may be added as needed (frontend libs, Tauri plugins, ZIP/HTTP, etc.).
- This exception does not apply to core crates under `crates/` — keep them lean and focused.
- Prefer using `rimloc-services` for business logic to avoid duplication.

### Auto-commit workflow (mandatory for agents)
- Start a session tied to the current chat/task: `scripts/agent-begin.sh --session <chat-id> [--type chore --scope cli --subject "short summary" -b "bullet"]`.
- While working, record context as you go:
  - Add files you intentionally touched: `scripts/agent-context.sh --session <chat-id> --add-file <path>` (repeatable).
  - Refine message: `scripts/agent-context.sh --session <chat-id> --subject "…" -b "…"`.
- For precise hunks in shared files, wrap edits:
  - Before editing a file: `scripts/agent-mark-change.sh --session <chat-id> begin --file <path>`
  - After saving your change: `scripts/agent-mark-change.sh --session <chat-id> end --file <path>`
  This records an exact per-chat patch. On commit, we apply only those hunks — чужие правки в том же файле не попадут.
- Finish and commit (mandatory step): `scripts/agent-commit.sh --session <chat-id>` — stages only files changed since baseline and, if a file allowlist exists, intersects with it to avoid accidental pickups.
- Hunk-aware staging: if a session snapshot exists for a file (created automatically on `--add-file`), only the hunks changed in this chat are staged. Independent edits from other chats in the same file stay unstaged. On overlapping edits, the script stops with a clear message.
- Without `--session`, the scripts fall back to a single global baseline (`.git/agent-baseline.txt`). Prefer sessions to avoid confusion between chats.
- Use `--dry-run` to preview the file set and the composed message. The script will auto-detect scope from paths and generate safe bullets if none are provided.
- Ensure hooks are active: run `scripts/setup-git-hooks.sh` once per clone.

- Final guard: run `scripts/agent-ensure-commit.sh` to verify the working tree is clean. Use `--session <chat-id> --auto` to auto-commit pending changes via the session if needed.

Tip: export a default session once per chat

```
export AGENT_SESSION=<chat-id>
scripts/agent-begin.sh --subject "…" --type fix --scope core
# …work…
scripts/agent-commit.sh  # Mandatory finish step
```

## For agents: Changelog & Versioning
- Changelog: keep a single curated `CHANGELOG.md` (Keep a Changelog + SemVer). Update `Unreleased` for every user‑facing change; use sections `Added/Changed/Fixed/Docs/Internal`.
- Entry format: `- [scope] short description (#PR)`, no trailing period. Scopes: `cli`, `core`, `parsers-xml`, `export-po`, `export-csv`, `import-po`, `validate`, `docs`, `ci`, `release`, `tests`.
- Internal‑only changes: add PR label `internal-only` to skip the changelog CI check.
- Do not rewrite past entries. On release: move `Unreleased` into a new version `## [X.Y.Z] - YYYY-MM-DD` and update compare links at the bottom.
- Versioning: SemVer. Libraries follow strict SemVer; CLI may use pre‑releases (`-alpha.N`, `-beta.N`).
- Workspace versions are independent; bump only crates with user‑visible changes (see `release.toml`).
- Agents must not bump versions, create tags, or publish unless explicitly asked. Default: only update changelog.
- When assigned a release task: perform the `Unreleased → [X.Y.Z]` move, update links, then request running the release workflow; tags use `vX.Y.Z`.

### MSRV and SemVer checks
- MSRV: Rust `1.70` across the workspace (`rust-version` pinned in each crate). Increase MSRV only in a major release.
- Libraries: CI runs `cargo-semver-checks` for published crates; breaking API changes require a `major` bump.
- CLI: treat output (JSON/PO/CSV) as a contract. Adding fields is minor; removing/renaming is major. JSON outputs include `schema_version` per item; PO headers include `X-RimLoc-Schema`.

## CLI Conventions
- Subcommands/flags use kebab‑case; help texts live in FTL and must be localized.
- No user strings inline: use `tr!`/FTL; logs via `tracing` only.
- JSON output must remain stable; update integration tests when schemas or flags change.

## Commit & Pull Request Guidelines
Follow the Conventional Commit template captured in `.gitmessage.txt`: `type(scope): summary` within 72 characters, using types such as `feat`, `fix`, `docs`, or `chore`. Commit messages must be written in English; a Russian reference lives at `docs/readme/ru/gitmessage.txt`.

- Always include a body for non-trivial changes and format it as bullet points starting with `- `. Explain what changed, why, and any user/dev impact.
- Avoid bare, context-free subjects such as `tests: update snapshot`. Instead, use a scoped subject and bullets, for example: `tests(cli): update scan snapshot for DefInjected` plus bullets describing exactly what changed in the snapshot and why.
- Keep the subject ≤ 72 chars. Use present tense and be specific.
- Release commits use a detailed body and must include the publish order line below.

Release commit example:

```
chore(release): prep crates for crates.io (0.1.0-dev.0)

- Bump all RimLoc crates to 0.1.0-dev.0
- Add versioned deps for path crates; add metadata (license, repo, docs)
- Exclude logs from CLI package
- Normalize Ko-fi badge to ASCII hyphen to avoid % encoding issues

Run publish in order: core -> parsers -> exporters/importer -> validate -> cli.
```

Pull requests need a concise summary, linked issues, and instructions for validation; attach CLI output or screenshots when behaviour changes. Ensure CI passes and that formatting, lint, and test checks are green before requesting review.

### Git hooks
- Enable local commit checks: run `scripts/setup-git-hooks.sh` once per clone (sets `core.hooksPath` to `.githooks`).
- The `commit-msg` hook enforces the subject pattern, a blank line, and at least one `- ` bullet in the body. Release commits must include the publish order line.

### Changelog policy (mandatory)
- Keep `CHANGELOG.md` up to date for every user‑facing change.
- Use Keep a Changelog format: add entries under `Unreleased` with `Added/Changed/Fixed/Docs` as appropriate and reference PR/issue IDs.
- On release, move `Unreleased` entries under the new version with a date; never rewrite past entries.
- CI enforces this for PRs that touch `crates/*`, `docs/*` or `README.md` (see `.github/workflows/changelog-check.yml`).

### Commit scope policy (mandatory)
- Commit only files that were intentionally edited as part of the change. Do not include unrelated files.
- Avoid drive‑by refactors, renames, and mass formatting across the repository. Keep diffs minimal and focused.
- Run `cargo fmt` but commit only the files you actually touched for the feature/fix. If a repository‑wide reformat is necessary, submit it as a dedicated, separate PR.
- Do not bump versions, shuffle modules, or update generated artifacts unless explicitly part of the task.
- This rule applies to humans and to automated agents working in this repo — agents must obey it as well.
- Recommended scopes: `repo`, `cli`, `core`, `parsers-xml`, `export-csv`, `export-po`, `import-po`, `validate`, `docs`, `ci`, `release`, `tests`.

### No‑revert policy (mandatory)
- Do not revert or discard changes without explicit consent from the maintainer/author.
- Exceptions: only when strictly required to fix broken builds/tests, or when the revert is necessary to complete the current fix/feature. State the rationale clearly in the commit body.
- If you encounter unrelated, uncommitted local changes, ask whether to keep, commit, or drop them. Do not silently undo them.
- When a revert is required, use a dedicated commit that references the original commit/PR (e.g., `revert: <hash> <subject>`). Avoid mixing reverts with functional changes.

## Localization Workflow Notes
Translations for the CLI ship via `i18n/<lang>/rimloc.ftl` and are embedded at build time. Update the English source, mirror changes to other locales, and run `cargo i18n` (if available) or `cargo test --package rimloc-cli -- tests_i18n` to confirm key integrity.
 - EN is the source of truth; other locales mirror keys and structure.
 - Adding a new locale: create `crates/rimloc-cli/i18n/<lang>/` with FTL files — `build.rs` auto‑discovers locales.
 - No hardcoded user‑facing strings in code; integration tests enforce this.

## PR Checklist
- Build/tests pass: `cargo build --workspace` and `cargo test --workspace`.
- Lints clean: `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings`.
- `CHANGELOG.md` updated under `Unreleased` for user‑facing changes.
- I18n: EN updated, other locales synced; `tests_i18n` green.
- Docs: EN/RU updated and `SITE_URL=… mkdocs build` succeeds for changed pages.
## GUI/CLI Parity and i18n

When adding or changing features:

- Keep CLI and GUI in lockstep: every new CLI command or flag must be exposed in the GUI with the same semantics. Avoid introducing functionality in one surface only.
- No hardcoded UI strings. Prefer i18n keys (see `frontend/index.js` I18N map). If you must add a new label, introduce a key in both `en` and `ru` and use `data-i18n` in HTML or `tr(key)` in JS.
- For dynamic messages, prefer composing from i18n tokens or add a dedicated key; do not inline English text.
- If a new backend command is added, register it in Tauri `invoke_handler`, permissions (`src-tauri/permissions/allow-commands.json`) and wire a GUI panel/control for it.
- Aim to keep APIs ergonomic for UI: when a CLI adds a structured option group (e.g., Defs dict/schema), expose a single request struct in Tauri mirroring CLI fields so the GUI can pass-through without transforms.

Review checklist for contributors:

- [ ] CLI: command + args implemented and documented
- [ ] Backend: Tauri command mirrors CLI types and fields
- [ ] Permissions updated, capability model unchanged unless necessary
- [ ] GUI: controls added with `data-i18n`/`tr()` and `localStorage` persistence
- [ ] Logs/progress events wired to the progress panel
- [ ] Build and run `cargo tauri dev` clean
