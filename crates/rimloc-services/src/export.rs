use crate::{
    autodiscover_defs_context, scan_defs_with_meta, util::is_under_languages_dir, ExportPoStats,
    Result,
};
use rimloc_parsers_xml::DefsMetaUnit;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Export to PO with optional TM, filtering by source lang or explicit folder name.
pub fn export_po_with_tm(
    scan_root: &Path,
    out_po: &Path,
    lang: Option<&str>,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
    tm_roots: Option<&[std::path::PathBuf]>,
) -> Result<ExportPoStats> {
    let units = rimloc_parsers_xml::scan_keyed_xml(scan_root)?;
    let auto = autodiscover_defs_context(scan_root)?;

    let src_dir: String = if let Some(dir) = source_lang_dir {
        dir.to_string()
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(code)
    } else {
        "English".to_string()
    };
    let mut english_map: HashMap<String, rimloc_core::TransUnit> = HashMap::new();
    for u in units
        .into_iter()
        .filter(|u| is_under_languages_dir(&u.path, &src_dir))
    {
        english_map.insert(u.key.clone(), u);
    }

    let defs_meta: Vec<DefsMetaUnit> =
        scan_defs_with_meta(scan_root, None, &auto.dict, &auto.extra_fields)?;
    for meta in defs_meta {
        let key = meta.unit.key.clone();
        let source = meta.unit.source.clone();
        if source.is_none() {
            continue;
        }
        let target_path = {
            let file_name = meta
                .unit
                .path
                .file_name()
                .map(|s| s.to_os_string())
                .unwrap_or_else(|| std::ffi::OsString::from("Defs.xml"));
            PathBuf::from(scan_root)
                .join("Languages")
                .join(&src_dir)
                .join("DefInjected")
                .join(&meta.def_type)
                .join(file_name)
        };
        let entry = english_map
            .entry(key.clone())
            .or_insert_with(|| rimloc_core::TransUnit {
                key: key.clone(),
                source: source.clone(),
                path: target_path.clone(),
                line: None,
            });
        if entry
            .source
            .as_ref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            entry.source = source.clone();
        }
        if !entry.path.to_string_lossy().contains("/DefInjected/")
            && !entry.path.to_string_lossy().contains("\\DefInjected\\")
        {
            entry.path = target_path;
            entry.line = None;
        }
    }

    let mut filtered: Vec<_> = english_map.into_values().collect();
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

    let tm_map: Option<std::collections::HashMap<String, String>> = match tm_roots {
        None => None,
        Some([]) => None,
        Some(roots) => {
            let mut map = std::collections::HashMap::<String, String>::new();
            for tm_path in roots {
                if let Ok(units) = rimloc_parsers_xml::scan_keyed_xml(tm_path) {
                    for u in units {
                        if let Some(val) = u.source.as_deref() {
                            let v = val.trim();
                            if !v.is_empty() {
                                // last wins across multiple TM roots
                                map.insert(u.key, v.to_string());
                            }
                        }
                    }
                }
            }
            Some(map)
        }
    };

    let stats = rimloc_export_po::write_po_with_tm(out_po, &filtered, lang, tm_map.as_ref())?;
    Ok(stats)
}
