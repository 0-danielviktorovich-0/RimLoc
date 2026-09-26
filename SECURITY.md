# Security Policy

## Supported versions

RimLoc is a single-maintainer project without a backport policy. Security fixes land
on `main` and ship with the next release of `rimloc-cli` on crates.io. Older releases
and tags are not patched — please upgrade before reporting.

| Scope | Supported |
| --- | --- |
| Latest published release (`rimloc-cli --version` on crates.io) | yes |
| Current `main` branch | yes |
| Older releases / tags | no — upgrade first |

## Reporting a vulnerability

Please do **not** open a public issue with exploitation details (anything that can
overwrite data outside requested output paths, execute code, or leak local data).

Preferred channel — GitHub private vulnerability reporting:

1. Open <https://github.com/0-danielviktorovich-0/RimLoc/security/advisories/new>
   ("Report a vulnerability").
2. Include: `rimloc-cli --version`, OS, exact commands or GUI steps, a minimal
   reproduction, and relevant logs (`RIMLOC_LOG_DIR`, `RUST_LOG=debug`).

If the "Report a vulnerability" form is unavailable (the feature may not be enabled
yet), open a minimal public issue saying you have a security report and asking for a
private contact channel — without technical details.

What to expect: the project is maintained by one person in spare time, so there is no
guaranteed response window. Accepted reports are handled in a private advisory and
disclosed publicly with a patched release. Reports judged not to be vulnerabilities
may be redirected to the public issue tracker with your consent.

## Scope notes

- RimLoc reads RimWorld installations and mod folders as **input only** and writes
  translation output to the paths you give it (`--out-*` flags, project saves, the
  GUI). Writing to explicitly requested output paths is intended behavior, not a
  vulnerability. Reports are interesting when the tool writes outside the requested
  output paths or sends local data anywhere beyond this machine.
- Vulnerabilities in RimWorld itself, third-party mods, or the toolchain (Rust, npm,
  Tauri) belong to their respective upstream projects. Dependency advisories are
  tracked in CI via `cargo deny`.
