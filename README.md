# RimLoc

[English](README.md) | [Русский](docs/readme/README.ru.md)

[![Build](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/build.yml/badge.svg)](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/build.yml) [![Crates.io](https://img.shields.io/crates/v/rimloc)](https://crates.io/crates/rimloc) [![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/) [![License](https://img.shields.io/badge/license-GNU%20GPL-blue)](LICENSE)

RimLoc is a Rust-based toolkit for RimWorld localization and mod translation management.  
It helps modders scan XML, validate translation quality, and export/import to PO/CSV across Linux, macOS, and Windows.

## Installation

```bash
cargo install rimloc-cli
```

## Features

- Scan RimWorld XML and extract translation units  
- Validate duplicates, empty strings, placeholders  
- Export/Import to PO / CSV  
- CLI localized with Fluent (English + Russian)  

## Example

```
rimloc-cli scan --root ./TestMod
```

### Demo (asciinema)

[![asciicast](https://asciinema.org/a/your-demo-id.svg)](https://asciinema.org/a/your-demo-id)

### Screenshot

![CLI validation example](docs/readme/demo-validation.png)

<!-- TODO: Add screenshot or asciinema demo of CLI output once available -->

## Documentation

👉 Full docs: [RimLoc Docs](https://0-danielviktorovich-0.github.io/RimLoc/)

---

## License

GNU GPL — see [LICENSE](LICENSE).