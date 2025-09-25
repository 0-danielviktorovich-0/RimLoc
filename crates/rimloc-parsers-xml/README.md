# rimloc-parsers-xml

XML ingestion and parsing for RimWorld localization data (RimLoc).

## Usage

```toml
[dependencies]
rimloc-parsers-xml = "0.1.0"
```

Scan `Languages/*/Keyed/*.xml` under a mod root:

```rust
use rimloc_parsers_xml::scan_keyed_xml;
use std::path::Path;

fn main() -> color_eyre::Result<()> {
    let units = scan_keyed_xml(Path::new("./Mods/MyMod"))?;
    for u in units.iter().take(5) {
        println!("{} => {:?}", u.key, u.source);
    }
    Ok(())
}
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
