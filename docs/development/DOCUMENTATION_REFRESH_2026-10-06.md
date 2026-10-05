# RimLoc documentation refresh — 2026-10-06

Status: active pre-beta documentation convergence.

This file is an internal audit/checkpoint. Public documentation lives in
\`docs/en/\` + \`docs/ru/\` and is rendered by MkDocs. README is a landing
page; GitHub Wiki is not a second technical source of truth.

## Product truth now reflected publicly

- RimLoc is **RimWorld-first**, not architecturally RimWorld-only.
- React 19 R1 is the intended production frontend.
- Svelte v2 is a frozen fallback/reference during convergence.
- Tauri 2 + Rust services remain the shared desktop backend.
- The canonical project model is independent from PO.
- PO is an optional interchange/CAT format.
- \`build-mod --from-root\` documents a no-PO CLI build path.
- New game/application integration belongs behind LocalizationAdapter.
- RimLoc self-localization is a second adapter direction.
- No stable public desktop release is claimed yet.
- Internal Runtime Bridge/automation is not marketed as a production feature.

## Updated public/repository files

### Landing / policy

- \`README.md\`
- \`docs/readme/ru/README.md\`
- \`SECURITY.md\`
- \`SUPPORT.md\`
- \`CONTRIBUTING.md\`
- \`AGENTS.md\`
- \`CHANGELOG.md\`

### MkDocs core EN/RU

- landing pages
- Getting Started
- Install / downloaded CLI
- GUI
- Translator workflow
- FAQ
- Troubleshooting
- Tips
- Testing
- PO guide
- CLI overview
- PO import/export
- Build Mod
- Translate Mod tutorial
- Update Translation tutorial
- RimLoc self-localization guide
- Pull Requests / Issues

### Extensibility

Added:

- \`docs/en/dev/adapters.md\`
- \`docs/ru/dev/adapters.md\`

Legacy scan-plugin documentation now explicitly states that the old native
scan-plugin mechanism is not the future LocalizationAdapter ABI.

### Repository metadata/config

Updated:

- bug/feature issue forms
- PR template
- Dependabot config
- CodeQL config
- CI docs-only path filtering
- docs workflow strict build / artifact policy
- MkDocs navigation/publishing surface

## Single source of truth

Published MkDocs intentionally excludes internal evidence/history trees:

- campaign
- competitive
- design
- development
- integration
- security

These files remain in git for engineering evidence, but are not end-user docs.

The public technical truth should be derived from:

- \`docs/en/\`
- \`docs/ru/\`
- root policy/contributor files

## Wiki

Repository Wiki is enabled in repository metadata but is not initialized with
content.

Recommendation: disable GitHub Wiki in repository settings, or keep it empty.
Do not duplicate MkDocs pages there.

The available GitHub connector used for this refresh does not expose repository
settings mutation, so this setting was not changed automatically.

## GitHub Actions / usage

CI remains PR/manual rather than push/schedule driven.

The heavy Rust/GUI matrix now skips documentation-only PRs. MkDocs uses a
strict build, and PR builds do not retain a Pages artifact unless preview
deployment is explicitly enabled.

Note: RimLoc is a public repository. Standard GitHub-hosted runners for public
repositories are not charged against the normal private-repository minute
quota; artifact/cache storage and non-standard/larger runners are separate
considerations.

## Dependabot

Added npm ecosystems for:

- React production candidate (weekly, grouped);
- frozen Svelte fallback (monthly, grouped).

Rust updates were reduced from daily/high-volume to weekly bounded queues;
documentation and GitHub Actions dependencies use lower cadence/grouping.

Open historical Dependabot PRs still need reconciliation against the current
merged dependency graph; changing this config does not by itself close old PRs.

## CodeQL

The previous config used paths beginning with a non-existent repository prefix
(\`RimLoc/...\`), causing ignore/include rules not to match as intended.

The config now targets actual first-party paths and excludes vendored Tauri
sources, targets, testlab, docs, binary assets and generated dependency trees.

Existing Code Scanning findings will only be reclassified/closed after GitHub
runs the updated scan against the merged/current branch.

## Remaining work before first beta

Documentation is intentionally not claiming completion for product areas still
moving.

Refresh again after:

1. React owner visual acceptance and final screenshot set;
2. TM A+B+C implementation settles;
3. provider/keychain production behavior settles;
4. final existing/update workflow is accepted;
5. Windows/macOS beta artifacts are frozen;
6. security/Dependabot/CodeQL reconciliation reaches beta gate;
7. final version/release process is approved.

At that point:

- replace old screenshots with React owner-approved images;
- remove/archive Svelte-specific public references when cutover is final;
- publish final beta installation/release pages;
- update capability tables against the exact beta artifact.

## Documentation PR

Documentation work is isolated in:

\`docs/pre-beta-documentation-refresh\`

Draft PR:

#61 — docs: refresh pre-beta product and developer truth

Base:

\`feature/ui-r1-convergence\`

This avoids blocking the active GLM implementation lane while keeping the
documentation changes reviewable and mergeable.
