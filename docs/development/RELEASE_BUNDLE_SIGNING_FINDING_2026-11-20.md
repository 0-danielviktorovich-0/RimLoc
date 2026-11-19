# Finding — macOS RC bundles ship without a bundle-level code signature

Date: 2026-11-20 · Method: REA `audit_release_exposure` +
`inspect_binary_hardening` (evidence ids `ev_90301b74…`, `ev_01726b79…`) +
`codesign -v --strict`.

## What was found

Every staged owner-UAT macOS bundle fails strict bundle validation:

```
$ codesign -v --strict "RimLoc GUI.app"
RimLoc GUI.app: code has no resources but signature indicates they must be present
```

Root cause: `Contents/_CodeSignature/` is absent entirely. The main binary
(`rimloc-gui`, Mach-O arm64, sha256 `8f5712b3…` in rc-019755ba) carries an
embedded linker ad-hoc signature (`LC_CODE_SIGNATURE` present,
`CodeDirectory flags=0x20002(adhoc,linker-signed)`), but nobody ever ran a
bundle-level `codesign`, so there is no `CodeResources` seal. Verified on
rc-8610e313, rc-fe1af7b3, rc-1194f7e6, rc-019755ba and the deployed
`owner-uat/RimLoc.app` — long-standing, not a regression of one bundle.

Why it matters: for a quarantined download (browser/DMG), Gatekeeper treats
a bundle without a resource seal far more aggressively ("damaged" dialog);
even the right-click Open path is unreliable for a fully unsealed bundle.
For the owner UAT on this machine it silently works — which is why it went
unnoticed.

## What the REA pass also verified (all clean)

- `audit_release_exposure` on rc-019755ba `RimLoc GUI.app`: **0 exposed**
  checks — 0 credential markers across scanned text members, 0 source maps
  / embedded sources, 0 debug companions (.pdb/.dSYM/.map), no Electron
  fuses surface (Tauri app → N/A). Rows `native.hardening` /
  `macho.hardened_runtime` were completed by the binary-hardening probe.
- `inspect_binary_hardening` on `rimloc-gui`: PIE enabled, stack execution
  disabled, `__stack_chk_fail` present (stack protector in use), code
  signature load command present. arm64e off (normal for Rust arm64).

## Remediation (verified)

One command before DMG creation:

```
codesign --force --sign - "RimLoc GUI.app"   # ad-hoc, no identity needed
codesign -v --strict "RimLoc GUI.app"        # must pass
```

Applied to a scratch copy: strict validation passes afterwards
(CodeDirectory `0x2(adhoc)`, resources sealed). Developer ID signing stays
an owner decision (distribution outside this machine); ad-hoc removes the
"damaged" class of failures and makes UAT copies verifiable.

## Guard

`testlab/release-guard.sh` now fails any macOS .app without a valid bundle
signature (check 1.5). Verified both directions against the staged RC:
unsigned → `GUARD FAILED` with the exact fix command; re-signed copy →
`GUARD PASSED`.

Packaging note: no bundling workflow exists on `main` (release workflows
parked, `ci.yml` only builds/tests). The `codesign` step belongs in
whatever assembles the next RC (tauri bundle output → sign → `hdiutil`),
and the guard now makes its absence a release blocker.
