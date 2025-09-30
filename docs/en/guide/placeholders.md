---
title: Placeholders
---

# Placeholders

!!! info "Terminology"
    New to placeholders? See the Glossary: ../glossary.md#placeholder

Placeholders are special tokens embedded in strings that are replaced at runtime with dynamic values (numbers, names, etc.). RimLoc understands two common families:

- Brace style: `{...}`
  - Examples: `{0}`, `{count}`, `{PAWN_label}`, `{weapon}`
- Percent style: `%...`
  - Examples: `%s`, `%d`, `%i`, `%f`, positional forms like `%1$s`, width like `%02d`

RimLoc treats placeholders as opaque tokens: it checks presence and basic shape, not the semantic type. Your translation must keep the same placeholders as the source.

## Rules and Recommendations

- Do not drop, rename, or alter placeholders between source and translation.
- Keep brace pairs balanced and non‑empty: `{}` is invalid; `{ name }` → `{ name }` is fine (spaces allowed).
- Percent tokens must be valid: `%%` is treated as a literal percent, not a placeholder.
- Order is generally tolerated, but mismatched sets are flagged.

## How RimLoc validates

- `rimloc-cli validate --root <MOD>`
  - Emits a `placeholder-check` category when placeholders exist.
  - Warns on malformed tokens (unbalanced `{}`, empty/invalid names, suspicious `%` tokens).
- `rimloc-cli validate-po --po <FILE>`
  - Compares placeholders in `msgid` vs `msgstr`. Any difference is reported as a mismatch.
  - Use `--strict` to make mismatches fail CI (exit code 1).

See also: ../cli/validate_po.md

## Typical pitfalls

- Dropping a placeholder entirely: the translated string must still contain all tokens.
- Mismatched braces `{name` or `{}`.
- Typos in percent tokens: `%z`, `% 2d` are invalid.
- Mixing literal braces with placeholders: escape appropriately in the source application; in RimWorld XML, literal braces rarely appear.

## Quick checks

```bash
# Validate XML placeholders in a mod (human readable)
rimloc-cli --quiet validate --root ./Mods/MyMod --format text

# Validate PO placeholders strictly (machine readable)
rimloc-cli --quiet validate-po --po ./out/mymod.po --strict --format json | jq .
```
