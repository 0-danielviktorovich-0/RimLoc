## Summary

- What changed and why?
- User/developer impact:
- Linked issues: Closes #

## Area

- [ ] Rust domain/core/services
- [ ] RimWorld adapter / parsing
- [ ] React desktop UI
- [ ] Svelte fallback (maintenance only)
- [ ] CLI / format interoperability
- [ ] Localization adapter / extensibility
- [ ] Security / dependencies
- [ ] Documentation
- [ ] CI / repository infrastructure
- [ ] Other

## Type

- [ ] feat
- [ ] fix
- [ ] refactor
- [ ] docs
- [ ] test
- [ ] chore
- [ ] ci
- [ ] release

## Public Git history

- [ ] PR title describes the engineering outcome, not private orchestration
- [ ] Branch name and commit subjects use public English engineering language
- [ ] CHANGELOG additions contain no internal workflow/review/status shorthand

The `Public history hygiene` check enforces these rules from the trusted base branch.

## Evidence / testing

List the exact checks you ran and relevant evidence.

- [ ] Rust tests/build/lints relevant to this change
- [ ] React typecheck/build if React changed
- [ ] Svelte checks only if fallback changed
- [ ] MkDocs strict build if docs changed
- [ ] Screenshots / same-state evidence for visible UI changes
- [ ] Security/adversarial tests for filesystem, IPC, network, secrets, or parsing changes

Commands / notes:

~~~text
paste commands/results here
~~~

## Codecov / automated review (for code changes)

- [ ] Inspected all bot comments (especially Codecov), CodeQL and CI at the **latest HEAD**; a green informational Codecov check is not a coverage pass.
- [ ] Checked changed uncovered lines, actual patch/project/flag/component metrics, and report-processing status; documented exceptions with reasons.
- [ ] Verified coverage scope: intended production files included (including untested files), no path-prefix exclusions masking code, no denominator or threshold manipulation.
- [ ] If `codecov.yml` changed, accounted for `strict_yaml_branch: main` and stated whether the new policy is already effective.
- [ ] Reported testing/coverage blind spots and any owner-only UAT or release gates.

## Multi-lane readiness (only for 2+ subsystems, 3+ deliverables, architecture/security/release)

- [ ] `AGENTS.md` mandatory preflight and the existing orchestration skill were loaded before implementation; DAG/worktree owner/acceptance evidence is linked.
- [ ] Workflow run ID + lane receipts / subagent evidence linked, OR exact tool-unavailability blocker recorded. A status description alone is not workflow evidence.
- [ ] Every affected branch's **latest exact SHA** has fully completed CI/CodeQL/Codecov/bot-comment triage; list all failures. No `DONE` claim for a red or unverified gate.
- [ ] Independent implementation reviewer (different worker), actual `local.handoff` receipt OR private reviewer mailbox request linked; outstanding reviews are labeled waiting.
- [ ] User UAT, real RimWorld smoke, paid-account authorization, signing, release and merge remain explicit OWNER_ONLY gates.
- [ ] Stacked/overlapping branches are reconciled and an integrated candidate is tested, not inferred from independently green branch checks.

## Product / architecture checks

- [ ] Domain behavior lives in Rust/shared services, not duplicated in React/CLI
- [ ] PO/CSV/XLIFF/etc. are treated as interchange adapters unless the change is specifically format-related
- [ ] RimWorld-specific semantics stay behind the adapter boundary
- [ ] Source game/mod directories remain read-only
- [ ] Unsupported capabilities are not presented as working UI
- [ ] Test/automation hooks cannot enter production artifacts

## Documentation / changelog

- [ ] User-facing behavior is documented in EN/RU where relevant
- [ ] CHANGELOG.md updated under Unreleased, or PR is truly internal-only
- [ ] No stale Wiki/duplicate docs source was introduced

## Breaking changes

None / describe migration and compatibility impact.

<!--
Agents: follow AGENTS.md.
Do not bump versions, tag, publish, force-push, or create releases unless the owner explicitly authorized that operation.
-->
