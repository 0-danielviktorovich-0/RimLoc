## =============================================================================
## RimLoc English Localization (EN)  -  Reference / Source of Truth
##
## Guidelines:
## 1. English (EN) is the canonical base. All other locales MUST mirror its keys.
## 2. Section order is FIXED:
##    - General messages (app-started, scan, validate, export, import, dry-run, xml, build)
##    - validate-po group
##    - import argument validation
##    - build-mod details
##    - warnings / errors
##    - validation kinds
##    - validation categories
##    - CLI help localization (help-*, grouped by subcommand)
## 3. Adding new keys:
##    - Append new keys at the end of the most relevant section.
##    - If adding a new section, append it to the end of the file with a header.
## 4. Placeholder rules:
##    - Placeholders ($var) must be identical across all locales.
##    - Do not rename or drop placeholders without updating every locale.
## 5. CLI help localization:
##    - Top-level keys: help-about, help-no-color, help-ui-lang.
##    - Per-command keys: help-&lt;cmd&gt;-about and help-&lt;cmd&gt;-&lt;arg&gt;.
##    - Keep naming in kebab-case matching CLI flags/args (e.g., help-importpo-out-xml).
## 6. Tests:
##    - all_locales_have_same_keys ensures all locales match EN.
##    - each_locale_runs_help_successfully uses these help keys to verify output in each locale.
## =============================================================================

app-started = rimloc started - version={ $version } - logdir={ $logdir } - RUST_LOG={ $rustlog }

scan-csv-stdout = Printing CSV to stdout...
scan-csv-saved = CSV saved to { $path }
scan-json-stdout = Printing JSON to stdout...
scan-json-saved = JSON saved to { $path }

validate-clean = All clean, no errors found

export-po-saved = PO saved to { $path }
export-po-tm-coverage = TM prefill: { $filled } / { $total } ({ $pct }%)

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

# === test-only markers (for integration tests) ===
test-app-started = rimloc app_started marker
test-dry-run-marker = DRY-RUN

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
ui-lang-unsupported = UI language code is not supported
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


# === validation details (per-item messages; not yet used by runtime) ===
# Placeholders are standardized across locales:
# - $validator : short validator name, e.g., DuplicateKey, EmptyKey, Placeholder
# - $path      : file path
# - $line      : line number (numeric)
# - $message   : human-readable explanation (already localized within validator)
validate-detail-duplicate = [duplicate] { $validator } ({ $path }:{ $line })  -  { $message }
validate-detail-empty = [empty] { $validator } ({ $path }:{ $line })  -  { $message }
validate-detail-placeholder = [placeholder-check] { $validator } ({ $path }:{ $line })  -  { $message }


# === CLI help localization ===
# Top-level
help-about = RimWorld localization toolkit (Rust)
help-no-color = Disable colored output
help-ui-lang = UI language code (e.g. en, ru, ja; defaults to system locale)
help-quiet = Suppress startup banner and non-essential stdout messages (alias: --no-banner)

# scan
help-scan-about = Scan a mod folder and extract Keyed XML entries
help-scan-root = Path to RimWorld mod root to scan
help-scan-out-csv = Save extracted entries to CSV file
help-scan-out-json = Save extracted entries to JSON file (use with --format json)
help-scan-lang = Language code of the files to scan (e.g., en, ru)
help-scan-source-lang = Source language code for cross-checks
help-scan-source-lang-dir = Path to source language directory for cross-checks
help-scan-format = Output format: "csv" (default) or "json"
help-scan-game-version = Game version folder to use (e.g., 1.6 or v1.6); defaults to latest available under root
help-scan-include-all = Include all version subfolders (disable auto-pick of latest)

# validate
help-validate-about = Validate strings for issues/warnings
help-validate-root = Path to RimWorld mod root to validate
help-validate-source-lang = Source language code to compare against
help-validate-source-lang-dir = Path to source language directory to compare against
help-validate-format = Output format: "text" (default) or "json"
help-validate-game-version = Game version folder to use (e.g., 1.6 or v1.6); defaults to latest under root
help-validate-include-all = Include all version subfolders (disable auto-pick of latest)

# validate-po
help-validatepo-about = Validate .po file placeholder consistency (msgid vs msgstr)
help-validatepo-po = Path to .po file to validate
help-validatepo-strict = Strict mode: return error (exit code 1) if mismatches are found
help-validatepo-format = Output format: "text" (default) or "json"

# export-po
help-exportpo-about = Export extracted strings into a single .po file
help-exportpo-root = Path to RimWorld mod root containing extracted strings
help-exportpo-out-po = Output .po file path
help-exportpo-lang = Target translation language code (e.g., ru, ja, de)
help-exportpo-source-lang = Source language ISO code to export from (e.g., en, ru, ja)
help-exportpo-source-lang-dir = Source language folder name (e.g., English). Overrides --source-lang
help-exportpo-tm-root = Path to translation memory root (e.g., Languages/Russian or a mod root) to prefill msgstr and mark entries as fuzzy
help-exportpo-game-version = Game version folder to scan (e.g., 1.6 or v1.6); defaults to latest under root
help-exportpo-include-all = Include all version subfolders (may create duplicates)

# import-po
help-importpo-about = Import .po  -  either into a single XML, or spread across existing mod structure
help-importpo-po = Path to .po file to import
help-importpo-out-xml = Output XML file path (single-file mode)
help-importpo-mod-root = Mod root to update with imported strings (structure mode)
help-importpo-lang = Target language code for import (e.g., ru)
help-importpo-lang-dir = Target language directory (overrides automatic mapping)
help-importpo-keep-empty = Import empty strings as placeholders
help-importpo-game-version = Game version subfolder to write into (e.g., 1.6 or v1.6); defaults to latest if exists
help-importpo-single-file = Write all imported strings into a single XML file
help-importpo-backup = Create .bak backups when overwriting files
help-importpo-dry-run = Do not write changes; only show what would be done

# build-mod
help-buildmod-about = Build a standalone translation mod from a .po file
help-buildmod-po = Path to .po file to build from
help-buildmod-out-mod = Output mod folder path
help-buildmod-lang = Language code of the translation
help-buildmod-name = Translation mod display name
help-buildmod-package-id = Translation mod PackageId
help-buildmod-rw-version = Target RimWorld version
help-buildmod-lang-dir = Language folder name inside the mod (optional)
help-buildmod-dry-run = Do not write files; only print the build plan
help-buildmod-dedupe = Remove duplicate keys within one XML (last wins)

# diff-xml
help-diffxml-about = Diff source vs translation presence and detect changed source strings using a baseline PO
help-diffxml-root = Path to RimWorld mod root to analyze
help-diffxml-source-lang = Source language ISO code (maps to RimWorld folder)
help-diffxml-source-lang-dir = Source language folder name (e.g., English). Overrides --source-lang
help-diffxml-lang = Target translation language ISO code (maps to RimWorld folder)
help-diffxml-lang-dir = Target translation folder name (e.g., Russian). Overrides --lang
help-diffxml-baseline-po = Baseline PO (previous export) to detect changed source strings
help-diffxml-format = Output format: "text" (default) or "json"
help-diffxml-out-dir = Optional output directory for Text files (ChangedData.txt, TranslationData.txt, ModData.txt)
help-diffxml-game-version = Game version folder to scan (e.g., 1.6 or v1.6); defaults to latest under root

diffxml-saved = Diff results saved to { $path }
diffxml-summary = Diff summary: changed={ $changed }, only-in-translation={ $only_trg }, only-in-mod={ $only_src }

# annotate
help-annotate-about = Add or remove comments with original source text in translation XML files
help-annotate-root = Path to RimWorld mod root
help-annotate-source-lang = Source language ISO code (e.g., en); maps to folder name
help-annotate-source-lang-dir = Source language folder name (e.g., English). Overrides --source-lang
help-annotate-lang = Target translation language ISO code (e.g., ru)
help-annotate-lang-dir = Target translation folder name (e.g., Russian). Overrides --lang
help-annotate-dry-run = Do not write files; only print which files would be updated
help-annotate-backup = Create .bak before overwriting XML files
help-annotate-strip = Strip existing comments instead of adding new ones
help-annotate-game-version = Game version folder under mod root (e.g., 1.6 or v1.6)
annotate-would-write = DRY-RUN: would annotate { $path }
annotate-summary = Annotate done. Processed={ $processed }, commented={ $annotated }

# xml-health
help-xmlhealth-about = Scan XML files for structural/read errors under Languages/
help-xmlhealth-root = Path to RimWorld mod root to scan
help-xmlhealth-format = Output format: "text" (default) or "json"
help-xmlhealth-lang-dir = Restrict scan to a specific language folder name (e.g., Russian)
xmlhealth-summary = XML health: no issues found
xmlhealth-issues = XML health: issues detected (see above)
xmlhealth-issue-line = { $path } — { $error }

# === scan ===
test-csv-header = CSV header must be present

# === startup message checks ===
test-startup-text-must-appear = Startup message must appear for locale { $loc }
