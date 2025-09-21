## =============================================================================
## RimLoc English Localization (EN) — Reference / Source of Truth
##
## Guidelines:
## 1. English (EN) is the canonical base. All other locales must mirror its keys.
## 2. Section order is FIXED:
##    - General messages (app-started, scan, validate, export, import, dry-run, xml, build)
##    - validate-po group
##    - import argument validation
##    - build-mod details
##    - warnings / errors
##    - validation kinds
##    - validation categories
## 3. Adding new keys:
##    - Append new keys at the end of the most relevant section.
##    - If adding a new section, append it to the end of the file with a header.
## 4. Placeholder rules:
##    - Placeholders ($var) must be identical across all locales.
##    - Do not rename or drop placeholders without updating every locale.
## 5. Tests:
##    - all_locales_have_same_keys ensures all locales match EN.
##    - each_locale_runs_help_successfully ensures help output works in each locale.
## =============================================================================
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

# === build-mod details ===
build-name = Mod name: { $value }
build-package-id = PackageId: { $value }
build-rw-version = RimWorld version: { $value }
build-mod-folder = Mod folder: { $value }
build-language = Language: { $value }
build-divider = -----------------------------------
build-summary = TOTAL: { $n } key(s) will be written

# === warnings / errors ===
ui-lang-unsupported = UI language code is not supported; using system/default locale (requested: { $code }).
warn-unsupported-ui-lang = ⚠ Unsupported UI language: { $lang }. Using system default.
err-placeholder-mismatches = placeholder mismatches detected
validate-po-error = placeholder mismatches detected

# === validation kinds (short labels used in reports) ===
kind-duplicate = duplicate
kind-empty = empty
kind-placeholder-check = placeholder-check

# === validation categories ===
category-duplicate = duplicate
category-empty = empty
category-placeholder-check = placeholder-check