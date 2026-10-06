---
title: Legacy Scan Plugins
---

# Legacy scan plugins

RimLoc contains an **experimental legacy scan-plugin mechanism** that can extend XML scanning.

This is **not** the same thing as the new LocalizationAdapter architecture and must not be treated as the stable public extension API.

Historically, scan plugins could be loaded through <code>RIMLOC_PLUGINS</code> / <code>--with-plugins</code> and expose a C ABI returning JSON-compatible translation units.

That mechanism may remain useful for existing experiments, but new game/application integration should start from the [Localization Adapter guide](adapters.md).

## Why the distinction matters

A scan plugin only contributes extracted units. A full LocalizationAdapter may need:

- source discovery;
- versions/dependencies;
- existing-translation import;
- canonical provenance;
- adapter-specific validation;
- build/export;
- runtime acceptance.

Do not design new public integrations around an unversioned native dynamic-library ABI.

For historical implementation details, inspect <code>rimloc-plugin-api</code> and the existing plugin crates directly.
