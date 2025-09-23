# -----------------------------------------------------------------------------
# RimLoc — test-only strings for CLI integration tests
# File: crates/rimloc-cli/i18n/en/rimloc-tests.ftl
# NOTE:
# • These messages are used by tests (not shipped to end users).
# • Keep keys stable; update values freely.
# • Prefer descriptive, grouped keys: test-validate-*, test-build-*, test-import-*, etc.
# -----------------------------------------------------------------------------

test-binary-built = binary rimloc-cli must be built by cargo
test-tempdir = tempdir
test-outpo-exist = out.po must exist
test-outpo-not-empty = out.po must not be empty

# === validate (categories and items) ===
test-validate-dup-category = expected [duplicate] category in output
test-validate-empty-category = expected [empty] category in output
test-validate-ph-category = expected [placeholder-check] category in output
test-validate-dup-items = expected DuplicateKey items listed
test-validate-empty-items = expected EmptyKey items listed
test-validate-ph-items = expected Placeholder items listed

# === validate (counters) ===
test-validate-atleast-duplicates = expected at least { $min } duplicate(s), found { $count }
test-validate-atleast-empty = expected at least { $min } empty, found { $count }
test-validate-atleast-placeholder = expected at least { $min } placeholder issue(s), found { $count }

# === import-po (single file dry run) ===
test-importpo-expected-path-not-found =
    Expected path not found.
    stdout=
    ```
    { $out }
    ```
    stderr=
    ```
    { $err }
    ```

# === build-mod (structure & content) ===
test-build-path-must-exist = { $path } must exist in built mod
test-build-folder-must-exist = { $path } folder must exist
test-build-xml-under-path = at least one XML file must be generated under { $path }
test-build-about-readable = About/About.xml must be readable
test-build-contain-tag = { $path } must contain correct { $tag }

# === ftl loading (helpers used inside tests) ===
test-ftl-failed-read = failed to read FTL file at { $path }

# === locale ordering/errors in tests ===
test-locale-order-mismatch = Locale { $loc } has keys in different order than en. Please reorder to match en.

# === repository-wide nonlocalized scan (tests) ===

test-nonlocalized-found =
    Found non-localized user-facing strings in repository (print/terminate macros).
    Rule applies to the whole project, including tests.
    Please replace with tr!(...) where appropriate.
    { $offenders }

# === warnings (robust check token used by tests) ===
test-warn-unsupported-lang = UI language code is not supported
# === harness requirements (keys referenced by tests code) ===
test-help-about-key-required = FTL must contain `help-about`

# === meta tests / CLI UX checks ===
test-en-locale-required = English locale (en) must exist as the reference
test-app-started-key-required = FTL must contain `app-started` (at least in en)
test-help-about-must-be-localized = help-about must be localized for { $lang }
test-help-must-list-snip = --help must list '{ $snip }' in { $lang }
test-no-ansi-help = --no-color --help should be plain (no ANSI)
test-fallback-locale-expected =
    Unknown locale should fall back to an existing locale (EN or RU).
    stdout:
    ```
    { $stdout }
    ```
# === startup ===
test-startup-text-must-appear = startup text must appear for { $loc }