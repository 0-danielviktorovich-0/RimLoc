# -----------------------------------------------------------------------------
# RimLoc — test-only strings for CLI integration tests
# File: crates/rimloc-cli/i18n/en/rimloc-tests.ftl
# NOTE:
# • These messages are used by tests (not shipped to end users).
# • Keep keys stable; update values freely.
# • Prefer descriptive, grouped keys: test-validate-*, test-build-*, test-import-*, etc.
# -----------------------------------------------------------------------------

test-binary-built = binary rimloc-cli should be built by cargo
test-tempdir = tempdir
test-outpo-exist = out.po should exist
test-outpo-not-empty = out.po should not be empty

# validate (categories and items)
test-validate-dup-category = expected [duplicate] category in output
test-validate-empty-category = expected [empty] category in output
test-validate-ph-category = expected [placeholder-check] category in output
test-validate-dup-items = expected DuplicateKey items listed
test-validate-empty-items = expected EmptyKey items listed
test-validate-ph-items = expected Placeholder items listed

# validate (counters)
test-validate-atleast-duplicates = expected at least { $min } duplicate(s), found { $count }
test-validate-atleast-empty = expected at least { $min } empty, found { $count }
test-validate-atleast-placeholder = expected at least { $min } placeholder issue(s), found { $count }

# import-po (single file dry run)
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

# build-mod (structure & content)
test-build-path-must-exist = { $path } must exist in built mod
test-build-folder-must-exist = { $path } folder must exist
test-build-xml-under-path = at least one XML file must be generated under { $path }
test-build-about-readable = About/About.xml should be readable
test-build-should-contain-tag = { $path } should contain correct { $tag }

# ftl loading (helpers used inside tests)
test-ftl-failed-read = failed to read FTL file at { $path }

# locale ordering/errors in tests
test-locale-order-mismatch = Locale { $loc } has keys in different order than en. Please reorder to match en.

# repository-wide nonlocalized scan (tests)

test-nonlocalized-found =
    Found non-localized user-facing strings in repository (print/terminate macros).
    Rule applies to the whole project, including tests.
    Please replace with tr!(...) where appropriate.
    { $offenders }

# warnings (robust check token used by tests)
test-warn-unsupported-lang = UI language code is not supported