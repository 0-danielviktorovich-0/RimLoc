# GUI Preview & Inline Editing

RimLoc GUI provides a live Preview panel to compare English (source) and the target language and to quickly edit missing or outdated entries.

Features:

- Filter keys by substring, show only Missing keys, and show only items with Placeholder or List (li) issues.
- Click a key to see its English text and edit the Target value inline; press Apply to save.
- After Apply, RimLoc writes to `Languages/<target>/Keyed/_Edited.xml` (created if missing) and updates the in‑memory view.
- The editor shows live warnings when placeholders differ from English and when the number of list items (lines) mismatches.

Notes:

- Placeholder detection mirrors CLI (percent specifiers like `%s` or `{NAME}` braces).
- Use the Build panel to assemble a complete translation mod from PO or from an existing tree.

