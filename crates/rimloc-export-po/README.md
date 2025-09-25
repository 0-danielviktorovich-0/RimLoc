# rimloc-export-po

PO exporter used by RimLoc to produce hand-off files for translators.

## Usage

```toml
[dependencies]
rimloc-export-po = "0.1.0"
rimloc-core = "0.1.0"
```

Write a `.po` file with header, references and msgctxt:

```rust
use rimloc_core::TransUnit;
use rimloc_export_po::write_po;
use std::path::Path;

fn main() -> color_eyre::Result<()> {
    let units = vec![TransUnit {
        key: "Greeting".into(),
        source: Some("Hello".into()),
        path: "/Mods/My/Languages/English/Keyed/A.xml".into(),
        line: Some(3),
    }];
    write_po(Path::new("./out.po"), &units, Some("ru"))?;
    Ok(())
}
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
