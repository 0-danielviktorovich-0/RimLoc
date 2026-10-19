# Creator-support + crates.io evidence matrix (Issue #8, 2026-10-19)

Agent: RimLoc autonomous lane (authorized by the owner directive; the
directive itself is the durable reference). Non-financial changes only;
no payout/bank/tax/price/subscription/tier-deletion was touched.

## PLATFORM matrix

| PLATFORM | URL | COPY/SETTINGS_CHANGED | VISUAL_CHECK | REMAINING_GATE |
|---|---|---|---|---|
| Ko-fi | ko-fi.com/danielviktorovich | **YES — About (bio) rewritten creator-wide** via the authenticated session (`/settings` → `#BioText` → form submit; POST `/settings/UpdateProfile` captured). New text: independent OSS developer, RimLoc as the main project with "beta in preparation", honest CTA ("every amount counts; all core functionality stays free and open source"). | Public page re-loaded with cache-buster: new bio live, old project-only bio gone | Membership tier benefit copy ($5/$10/$15) still promises early access / previews / Discord role — the tier editor renders in a lazy iframe unstable through the automation surface. **Zero active members** (supporters page: "No supporters yet"), so no contractual exposure; exact cleanup steps below. |
| Ko-fi memberships | /memberships/settings | NO (read-only audit) | Tiers listed: RimLoc Supporter $5 / Contributor $10 / Patron $15, all "No active members" | Owner (or a follow-up automation pass on a stable window): delete the four benefit chips, set descriptions to the voluntary-appreciation copy below |
| Buy Me a Coffee | buymeacoffee.com/danielviktorovich | NO — no authenticated session available (public page only) | Public page audited: bio is RimLoc-only; tiers carry **Exclusive Discord Role / priority replies / roadmap polls / feature-priorization**; supporters list empty ("Be the first one") → no member obligations | Owner login required for any edit; ready-to-paste copy below |
| GitHub Sponsors | github.com/sponsors/0-danielviktorovich-0 | NO — dashboard requires sudo-mode re-auth (passkey): a security confirmation, **not bypassed** per the directive | Public profile (authenticated preview) audited: bio already creator-wide and honest; goal "10 monthly sponsors" at 0%; no published fixed tiers (custom amount only) | Owner: passkey once → paste the small bio fix below (or accept as-is); crates.io trusted-publisher authorization happens in the same authenticated session |

## Ready-to-paste owner copy

**BMC page description** (creator-wide, replaces the RimLoc-only bio):

> Independent open-source developer building practical tools for game
> localization and beyond. My main project is RimLoc — a local-first
> RimWorld mod translation toolkit with a visual editor and optional
> AI-assisted workflows (beta in preparation). Support funds testing,
> maintenance, documentation and future open-source projects. Like the
> project? A one-time coffee or a monthly contribution helps — every
> amount counts, and all core functionality stays free and open source.

**BMC/Ko-fi tier descriptions** (prices unchanged $5/$10/$15):

- Supporter: "Voluntary appreciation tier. My sincere thanks, optional
  supporter acknowledgement (with your consent), and public development
  updates. Everything stays free and open source regardless."
- Contributor: "Everything in Supporter, plus deeper development
  notes when there's something worth sharing. No guaranteed response
  time, no gated features."
- Patron: "Everything in Contributor, with extra gratitude. Optional
  name-in-README recognition with your explicit consent only."

**Delete from every tier**: Exclusive Discord Role / early access /
previews / priority replies / roadmap polls / feature-priorization
votes / exclusive requests (none of these are implemented; the owner
confirmed the supporter Discord chat is NOT a current commitment).

**GitHub Sponsors bio** (small honesty fix; current text claims
benefit for "thousands of players"):

> …💜 By sponsoring, you directly support independent open-source work
> on RimLoc and future tools. Every amount helps; all core
> functionality stays free and open source, no subscription needed.

## CRATE matrix (registry checked 2026-10-19 via the crates.io API)

| CRATE | REGISTRY | LOCAL | DRY_RUN | PUBLISH_ORDER | WORKFLOW_STATUS |
|---|---|---|---|---|---|
| rimloc-core | 0.1.0 (2025-09-25) | 0.2.0 | **PASS** (package + compile + upload-abort) | 1 | prepared |
| rimloc-parsers-xml | 0.1.0 | 0.2.0 | PASS* | 2 | prepared (assets/ added to include) |
| rimloc-llm | — | 0.2.0 | PASS* | 3 | prepared |
| rimloc-export-csv | 0.1.0 | 0.2.0 | PASS* | 4 | prepared |
| rimloc-import-po | 0.1.0 | 0.2.0 | PASS* | 5 | prepared |
| rimloc-export-po | 0.1.0 | 0.2.0 | PASS | 6 | prepared |
| rimloc-validate | 0.1.0 | 0.2.0 | PASS* | 7 | prepared |
| rimloc-import-xliff | — | 0.2.0 | PASS | 8 | prepared |
| rimloc-export-xliff | — | 0.2.0 | PASS | 9 | prepared |
| rimloc-domain | — | 0.2.0 | PASS* | 10 | prepared |
| rimloc-plugin-api | — | 0.2.0 | PASS | 11 | prepared |
| rimloc-config | — | 0.2.0 | PASS | 12 | prepared |
| rimloc-services | — | 0.2.0 | blocked** | 13 | prepared |
| rimloc-cli | 0.1.0-alpha.1 (2025-09-26) | 0.2.0-alpha.1 | blocked** | 14 | prepared |
| rimloc-plugin-jsonftl | — | 0.2.0 | blocked** | 15 | prepared |

\* dry-run compiles against the REGISTRY versions; dependents resolve
their rimloc deps from crates.io where 0.2.0 does not exist yet —
in-order real publishing satisfies this (each publish updates the
index), which is exactly what the workflow's dependency order does.
\** same reason, plus `cargo publish` refusing a dry-run when a
dependency version is registry-absent; packaging lists verified via
`cargo package --list` for all 15 crates.

**Workflow** `.github/workflows/publish-crates.yml` (commit 4cd637e):
owner-dispatched `workflow_dispatch` with a dry-run default, OIDC
trusted publishing (`id-token: write`, no stored tokens), pinned
actions, `environment: crates-publish` gate, preflight script
`check-publish-deps.py` (fails on any versionless rimloc path dep),
dependency-ordered publish list, run summary with the exact SHA. No
tag/push triggers, no `pull_request_target`/`workflow_run`.

**Owner gates before any real publish**: (1) authorize the GitHub
repository as a trusted publisher for each crate in crates.io settings
(one authenticated session); (2) create the `crates-publish`
environment (optionally with required reviewers = the owner); (3)
dispatch with dry_run=false on the release SHA after the release
approval. Publishing is permanent — no version can be overwritten.

## Badge verification (directive §4)

All three funding badge SVGs return HTTP 200 from shields.io with the
correct Simple Icons slugs (`githubsponsors`, `buymeacoffee`, `kofi`);
jsdelivr serves `kofi.svg` (the old hyphenated slug 404s — confirming
the reviewer's fix). No badge changes needed; FUNDING.yml surfaces
GitHub Sponsors + Ko-fi + BMC custom link correctly.
