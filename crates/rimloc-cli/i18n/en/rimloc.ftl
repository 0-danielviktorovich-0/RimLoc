app-started = rimloc started • version={ $version } • logdir={ $logdir } • RUST_LOG={ $rustlog }

scan-csv-stdout = Printing CSV to stdout…
scan-csv-saved = CSV saved to { $path }

validate-clean = All clean, no errors found

export-po-saved = PO saved to { $path }

import-dry-run-header = DRY-RUN plan:
import-total-keys = TOTAL: { $n } key(s)
import-only-empty = PO contains only empty strings. Add --keep-empty if you want to import placeholders.
import-nothing-to-do = Nothing to import (all strings are empty; add --keep-empty if placeholders are needed).
import-done = Import completed to { $root }

dry-run-would-write = DRY-RUN: would write { $count } key(s) to { $path }

xml-saved = XML saved to { $path }

build-dry-run-header = === DRY RUN: building translation mod ===
build-built-at = Translation mod built at { $path }
build-done = Translation mod built at { $out }


# === validate-po ===
validate-po-ok = ✔ Placeholders OK ({ $count } lines)
validate-po-mismatch = ✖ Placeholder mismatch { $ctxt } { $reference }
validate-po-msgid = msgid: { $value }
validate-po-msgstr = msgstr: { $value }
validate-po-expected = expected: { $ph }
validate-po-got = got: { $ph }
validate-po-total-mismatches = Total mismatches: { $count }
validate-po-report-line = { $ctxt } → { $reference }
validate-po-summary = Total mismatches: { $count }

# import argument validation
import-need-target = Error: either --out-xml or --mod-root must be specified
import-dry-run-line = { $path }  ({ $n } key(s))