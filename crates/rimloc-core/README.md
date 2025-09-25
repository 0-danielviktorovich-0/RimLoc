# rimloc-core

Core data types and utilities used across the RimLoc toolkit.

## Usage

Add to Cargo.toml:

```toml
[dependencies]
rimloc-core = "0.1.0"
```

Parse a minimal PO string and work with `TransUnit`/`PoEntry`:

```rust
use rimloc_core::{parse_simple_po, TransUnit};

fn main() -> color_eyre::Result<()> {
    // Minimal PO with two entries
    let po = r#"
msgid "Hello"
msgstr "Привет"

msgid "Bye"
msgstr "Пока"
"#;
    let entries = parse_simple_po(po)?;
    assert_eq!(entries.len(), 2);

    // Build a TransUnit for downstream exporters
    let unit = TransUnit {
        key: "Greeting".into(),
        source: Some("Hello".into()),
        path: "Mods/My/Languages/English/Keyed/A.xml".into(),
        line: Some(3),
    };
    println!("{unit:?}");
    Ok(())
}
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
