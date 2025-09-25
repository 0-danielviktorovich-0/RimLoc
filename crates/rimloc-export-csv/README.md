# rimloc-export-csv

CSV exporter for the RimLoc toolkit.

## Usage

```toml
[dependencies]
rimloc-export-csv = "0.1.0"
rimloc-core = "0.1.0"
csv = "1"
```

Write a CSV with optional `lang` column:

```rust
use rimloc_core::TransUnit;
use rimloc_export_csv::write_csv;
use std::path::PathBuf;

fn main() -> color_eyre::Result<()> {
    let units = vec![TransUnit {
        key: "Greeting".into(),
        source: Some("Hello".into()),
        path: PathBuf::from("/Mods/My/Languages/English/Keyed/A.xml"),
        line: Some(3),
    }];
    let mut out = Vec::new();
    write_csv(&mut out, &units, Some("ru"))?;
    println!("{}", String::from_utf8(out).unwrap());
    Ok(())
}
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
