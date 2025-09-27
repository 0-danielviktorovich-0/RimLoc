use crate::{util::is_under_languages_dir, ExportPoStats, Result};
use std::path::Path;

/// Export to PO with optional TM, filtering by source lang or explicit folder name.
pub fn export_po_with_tm(
    scan_root: &Path,
    out_po: &Path,
    lang: Option<&str>,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    tm_root: Option<&Path>,
) -> Result<ExportPoStats> {
    let units = rimloc_parsers_xml::scan_keyed_xml(scan_root)?;

    let src_dir: String = if let Some(dir) = source_lang_dir {
        dir.to_string()
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(code)
    } else {
        "English".to_string()
    };

    let mut filtered: Vec<_> = units
        .into_iter()
        .filter(|u| is_under_languages_dir(&u.path, &src_dir))
        .collect();
    filtered.sort_by(|a, b| {
        (
            a.path.to_string_lossy(),
            a.line.unwrap_or(0),
            a.key.as_str(),
        )
            .cmp(&(
                b.path.to_string_lossy(),
                b.line.unwrap_or(0),
                b.key.as_str(),
            ))
    });

    let tm_map: Option<std::collections::HashMap<String, String>> = if let Some(tm_path) = tm_root
    {
        match rimloc_parsers_xml::scan_keyed_xml(tm_path) {
            Ok(units) => {
                let mut map = std::collections::HashMap::<String, String>::new();
                for u in units {
                    if let Some(val) = u.source.as_deref() {
                        let v = val.trim();
                        if !v.is_empty() {
                            map.entry(u.key).or_insert_with(|| v.to_string());
                        }
                    }
                }
                Some(map)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let stats = rimloc_export_po::write_po_with_tm(out_po, &filtered, lang, tm_map.as_ref())?;
    Ok(stats)
}

