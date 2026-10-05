---
title: Issue Guidelines
---

# Issue guidelines

Use GitHub Issues for reproducible bugs, product requests and documentation problems.

## Bug reports

Use the repository bug form. The most useful reports include:

- exact version / commit / artifact identity;
- area: React desktop, fallback, CLI, adapter, validation, build/export, TM/glossary, docs;
- OS;
- exact UI steps or CLI command;
- expected vs actual behavior;
- small sanitized reproduction;
- screenshots where relevant;
- sanitized diagnostics/logs.

For desktop issues, saying only “latest app” is not enough during pre-beta — include the build identity.

## Security

Do **not** put exploit details, secrets or sensitive local data into a public issue. Follow [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md).

## Feature requests

Describe the workflow/problem first.

If requesting a new game/application adapter, include:

- game/application;
- source formats;
- existing translation format;
- version/dependency semantics;
- build/export target;
- a small real example if possible.

This makes it much easier to decide whether the request belongs in a simple adapter, built-in complex adapter, or future plugin protocol.

## Documentation reports

Include the exact page/link and what is stale or misleading.

The canonical public documentation source is <code>docs/</code> → MkDocs. GitHub Wiki is not maintained as a second technical truth.

## Before filing

Search open/closed issues. If an existing report matches, add new reproduction/evidence there rather than creating a duplicate.
