---
title: Scan Plugins
---

# Scan Plugins

RimLoc can extend scanning beyond XML via dynamically loaded plugins.

## Loading

- Set `RIMLOC_PLUGINS` to a list of directories (colon on Unix, semicolon on Windows) with plugin libraries.
- Or place plugin libraries under `<mod root>/plugins`.
- Run `rimloc-cli scan --with-plugins --root <PATH>` to load and execute plugins.

## ABI

Each plugin must export a C symbol:

```
extern "C" fn rimloc_plugin_scan_json(root: *const c_char) -> *mut c_char
```

It receives a UTF‑8 path to the scan root and must return a UTF‑8 JSON string with an array of objects compatible with `TransUnit` fields: `{ key, source, path, line }`.

For static plugins within the same process, implement `rimloc_plugin_api::ScanPlugin` and register via `rimloc_services::plugins::register_scan_plugin(...)`.

## Example

See `crates/rimloc-plugin-jsonftl` for a simple `.ftl` parser plugin.

