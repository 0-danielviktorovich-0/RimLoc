# rimloc-import-po

Import PO translations back into RimWorld mod XML (RimLoc).

## Usage

```toml
[dependencies]
rimloc-import-po = "0.1.0"
```

Read PO entries and write LanguageData XML:

```rust
use rimloc_import_po::{read_po_entries, write_language_data_xml};
use std::path::PathBuf;

fn main() -> color_eyre::Result<()> {
    let entries = read_po_entries(PathBuf::from("./ru.po").as_path())?;
    // Group entries by your own logic, or feed directly into writer
    let pairs: Vec<(String, String)> = entries
        .into_iter()
        .map(|e| (e.key, e.value))
        .collect();
    write_language_data_xml(PathBuf::from("./Keyed/_Imported.xml").as_path(), &pairs)?;
    Ok(())
}
```

## Links

- Docs: https://0-danielviktorovich-0.github.io/RimLoc/
- Repository: https://github.com/0-danielviktorovich-0/RimLoc
- License: GPL-3.0-only
