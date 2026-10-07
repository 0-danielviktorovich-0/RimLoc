# Public Git History Policy

RimLoc's public Git history describes the software and repository, not the
private orchestration used to produce a change.

## Public surfaces

The following are public engineering surfaces and must use concise English
product/repository language:

- commit subjects;
- branch names;
- pull request titles;
- release titles;
- newly added CHANGELOG entries.

Internal planning vocabulary belongs in local state, evidence bundles, or
engineering notes. It must not leak into the public surfaces above.

Examples of internal-only vocabulary include workflow/lane/wave identifiers,
private review status tokens, prompt section numbers, owner-gate shorthand,
agent runtime names, and requirement-document shorthand.

## Translate internal work into public outcomes

| Internal description | Public Git wording |
| --- | --- |
| review status update for release evidence | `docs(release): reconcile release evidence` |
| source inspector implementation task | `feat(gui): show live source provenance in workspace` |
| provider build integration task | `fix(llm): compile keychain-only provider builds` |
| packaging decision after filesystem testing | `build(macos): package app as zip on unsupported DMG volumes` |
| process policy update | `docs(process): document release verification policy` |

The body of a commit may explain technical rationale, but it should still read
as engineering documentation rather than as a transcript of an automation
session.

## Enforcement

Local commits are checked by `.githooks/commit-msg`, which delegates the
public-language check to:

~~~bash
python3 scripts/check-public-git-language.py --surface commit --text "feat(gui): ..."
~~~

Pull requests are checked by the `Public history hygiene` workflow. It runs
from the trusted base branch via `pull_request_target` and validates the PR
title, branch name, all new commit subjects, and newly added CHANGELOG lines.

The checker is also available for release tooling:

~~~bash
python3 scripts/check-public-git-language.py --surface release --text "RimLoc v0.2.0"
~~~

## Existing history

Existing commit and PR titles are historical. Do not rewrite published history
for naming cleanup: RimLoc release evidence, security scans, links and artifacts
already refer to those SHAs.

The policy is forward-only. Old orchestration-heavy names remain historical
debt; new public Git metadata is blocked automatically.

## Internal documents

Development notes may still use detailed process terminology when it is useful
for resuming work. A commit that changes those notes must nevertheless use a
public engineering subject describing the repository change.
