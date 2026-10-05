---
title: Tips & Tricks
---

# Tips and tricks

- Prefer the **desktop project workflow** for normal translation; use CLI for automation and format-specific work.
- Keep original game/Workshop/mod sources read-only.
- Use separate project/output locations and <code>--dry-run</code> for CLI writes.
- PO is optional. Use it when Poedit/CAT handoff helps; otherwise translate directly in RimLoc.
- Run validation after imports, bulk edits, TM reuse or AI-assisted changes.
- Treat AI/provider output as draft until reviewed.
- For source updates, use the existing/update workflow instead of starting the translation again.
- Keep target locales isolated; never reuse a result from one target as an automatic overwrite of another.
- Test the final result in RimWorld — context and layout cannot be proven from XML alone.
- When reporting a bug, include the exact artifact/commit identity and sanitized diagnostics.

For developers: adapter-specific semantics belong behind <code>LocalizationAdapter</code>, not in the generic core.
