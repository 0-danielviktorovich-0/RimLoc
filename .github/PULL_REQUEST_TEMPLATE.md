# Pull Request

Thank you for contributing to RimLoc!

## Summary

- What does this change do? (1–3 sentences)
- Why is it needed?

## Type

- [ ] fix (bug fix)
- [ ] feat (new feature)
- [ ] docs (documentation only)
- [ ] refactor (no functional change)
- [ ] chore (build/infra/tests)

## Validation

- [ ] `cargo build --workspace`
- [ ] `cargo test --workspace`
- [ ] `cargo fmt && cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Docs updated (if CLI or behavior changed)
- [ ] i18n keys updated (EN first; other locales mirrored)

### Commands / Output (copy relevant snippets)

```bash
# Example
rimloc-cli --quiet validate --root ./test/TestMod --format json | jq . | head
```

## Related Issues

Closes #<issue>, related to #<issue>

## Notes for Reviewers

- Breaking changes? Migration notes?
- Follow-ups planned?

