# Public Git History Policy

RimLoc's public Git history describes the software, not the internal agent
orchestration that produced it.

This policy applies to new public metadata from the point it was introduced.
Existing commit history is **not rewritten**: release evidence, CodeQL results,
pull-request links, artifact identities, and external clones already depend on
those SHAs.

## Public surfaces

The following must use normal engineering language:

- commit subjects;
- pull-request titles;
- public branch names used by pull requests;
- release titles;
- user-facing CHANGELOG additions.

Commit and PR bodies may link to internal development documents when that
context is useful, but the title/subject must still stand on its own as a
description of the product or repository change.

## Internal vocabulary

The following concepts are useful inside ZCode/agent orchestration, local
release state, reviewer messages, and explicitly internal development docs,
but are not suitable as public Git metadata:

- mandate / mandat / мандат;
- reviewer iteration / Reviewer Bridge protocol markers;
- `DO_NOW`, `OWNER_ONLY`, `EVIDENCE_REQUEST`, `IMPLEMENT_NOW`;
- owner-gate;
- workflow ids such as `WF-UI`;
- orchestration waves and lanes;
- campaign phases;
- phase-only markers such as `R4 §2` or `C-gates`.

Product/version names remain valid. For example, **UI R1** is a real product
design identifier and is not rejected merely because it contains `R1`.

## Translate internal work into public engineering outcomes

Before:

~~~text
Internal task: R4 / WF-UI / reviewer DO_NOW / C2 Source Inspector
~~~

Public commit:

~~~text
feat(gui): show live source provenance in the workspace

- render file, line and winner provenance from the canonical snapshot
- add built-app coverage for source location and honest missing data
~~~

Do not publish:

~~~text
docs(review): DO_NOW iteration 1
docs(gate): C-gates closed after reviewer mandate
chore(integration): wave integration 2
docs(process): R4 §1 owner-gate ledger
~~~

Use:

~~~text
docs(release): reconcile rel23 verification evidence
docs(security): document provider credential boundaries
feat(chat): add persistent no-API translation batches
fix(modview): resolve conditional content from active mods
test(gui): cover the chat translation round trip
~~~

## Enforcement

Local commits are checked by `.githooks/commit-msg`, which invokes:

~~~bash
python3 scripts/check-public-git-language.py \
  --surface commit-subject --text "feat(gui): ..." \
  --staged-changelog
~~~

Pull requests repeat the policy in CI and check:

- PR title;
- PR branch name;
- every new commit subject in the PR;
- newly added CHANGELOG lines.

This means `git commit --no-verify` does not silently bypass the policy for
changes that go through a pull request.

The reusable checker also supports release titles:

~~~bash
python3 scripts/check-public-git-language.py \
  --surface release-title --text "RimLoc 0.2.0-beta.1"
~~~

Release automation must call this check before creating a public release.

## Scope and false positives

The guard intentionally checks public metadata, not all source comments or
historical development documents. Internal specs may still describe workflows,
lanes, gates, or reviewer protocols when that terminology is genuinely useful.

If a legitimate engineering term collides with a rule, prefer rewriting the
public title to the concrete effect. If that would make the title less clear,
adjust the checker with a regression test rather than bypassing it.

## Historical debt

Older RimLoc commits and PRs include orchestration-oriented names. They remain
part of immutable project history. Do not rebase, force-push, or rewrite them
for cosmetic cleanup.

The migration boundary is simple: **all new public Git metadata follows this
policy.**
