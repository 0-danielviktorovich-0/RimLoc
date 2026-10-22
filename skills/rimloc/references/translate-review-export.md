# Review/export semantics (professional)

- Existing translation import (folder, PO, XLIFF) runs a dry-run
  analysis first: reusable / conflicts / obsolete / ambiguous /
  invalid. Conflicts NEVER overwrite existing human work.
- AI previews: per-row accept/reject. Unselected translated rows stay
  reviewable (`translated`), explicitly rejected rows are ledgered
  (`rejected`), applied rows are marked (`applied`); a partial apply
  keeps the job in preview with an honest "N rows remain" note.
- Previews over 400 rows: a full apply requires the explicit
  full-scope acknowledgement; apply-selected has no such requirement.
- Revision discipline: every apply pins `expected_revision` + session
  epoch from a snapshot taken AFTER the last user-visible change;
  stale revision / epoch is a typed refusal, not a retry loop.
- Source drift: the project records a source fingerprint; a changed
  source is a typed refusal that asks for a rescan, never a silent
  merge into stale translations.
- Export: guarded build to a user-chosen directory; reparse check
  guarantees the written files re-read exactly as reported.
