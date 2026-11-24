# Ad-hoc code signing for the public beta; Developer ID deferred

Every staged macOS bundle reached users with a bare linker-signed main
binary and no bundle-level seal, which triggers the Gatekeeper "damaged"
dialog on quarantined downloads. We decided the public beta ships with an
ad-hoc bundle signature (`codesign --force --sign -`, verified to pass
strict validation) plus documented install instructions, and we will NOT
purchase a Developer ID ($99/yr) for now: the decision is revisited only if
community donations create a funding stream (the Ko-fi destination is
already wired but hidden until payment setup is ready).

## Consequences

- The release guard fails any macOS bundle without a strict-valid signature
  (testlab/release-guard.sh check 1.5), so unsigned RCs cannot ship again.
- Beta users on macOS see a one-step "open anyway" path documented in the
  README instead of a clean double-click install.
