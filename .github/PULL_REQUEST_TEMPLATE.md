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
