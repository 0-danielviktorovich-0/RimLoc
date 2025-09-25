## Summary
- What’s changed? Why?
- Scope: cli/core/parsers-xml/export-po/export-csv/import-po/validate/docs/ci/release
- Linked issues: Closes #

## Type (choose one)
- [ ] feat — new functionality
- [ ] fix — bug fix
- [ ] refactor — code change without behavior impact
- [ ] docs — documentation updates
- [ ] test — add/update tests
- [ ] chore — infra/build/deps chores
- [ ] ci — workflow/build pipeline changes
- [ ] release — release process/config changes

## Breaking Changes
- None / Describe impact and migration

## How to Test
- Steps/commands to validate locally
- Expected output (paste CLI output or screenshots if behavior changed)

## Changelog
- [ ] CHANGELOG.md updated under Unreleased (Added/Changed/Fixed/Docs/Internal) with PR number
  - Note: add label `internal-only` to skip the changelog CI check for non user-facing changes

## Checklist
- [ ] Build/tests pass: `cargo build --workspace` and `cargo test --workspace`
- [ ] Lints clean: `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] I18n: EN updated, other locales synced (if applicable); `cargo test --package rimloc-cli -- tests_i18n`
- [ ] Docs (EN/RU) updated; `SITE_URL=… mkdocs build` passes (if docs changed)

<!-- For agents: follow AGENTS.md → For agents: Changelog & Versioning.
     - Do NOT bump versions or create tags unless explicitly requested.
     - Limit changes to scope; add `[scope]` in changelog bullets and `(#PR)`.
     - Use `internal-only` label for infra-only PRs. -->
