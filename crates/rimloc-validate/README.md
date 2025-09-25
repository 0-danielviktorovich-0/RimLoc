# rimloc-validate

Validation helpers for RimLoc to catch duplicate keys and placeholder mismatches.

## Usage

```toml
[dependencies]
rimloc-validate = "0.1.0"
rimloc-core = "0.1.0"
```

Validate scanned units:

```rust
use rimloc_core::TransUnit;
use rimloc_validate::validate;
use std::path::PathBuf;

fn main() -> color_eyre::Result<()> {
    let units = vec![TransUnit {
        key: "Greeting".into(),
        source: Some("Hello %s".into()),
        path: PathBuf::from("/Mods/My/Languages/English/Keyed/A.xml"),
        line: Some(3),
    }];
    let msgs = validate(&units)?;
    for m in msgs {
        eprintln!("{}:{} [{}] {}", m.path, m.line.unwrap_or(0), m.kind, m.message);
    }
    Ok(())
}
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
