//! High-level orchestration layer over lower-level crates.
//! Intentionally thin: exposes stable functions used by CLI/GUI/LSP.

use std::path::{Path, PathBuf};

pub use rimloc_core::{Result, TransUnit};
pub use rimloc_validate::ValidationMessage;
pub use rimloc_export_po::PoStats as ExportPoStats;

/// Scan a RimWorld mod folder and return discovered translation units.
/// This wraps `rimloc_parsers_xml::scan_keyed_xml` to provide a stable entrypoint
/// for higher-level clients (CLI, GUI, LSP) without importing parser crates.
pub fn scan_units(root: &Path) -> Result<Vec<TransUnit>> {
    rimloc_parsers_xml::scan_keyed_xml(root)
}

fn is_under_languages_dir(path: &Path, lang_dir: &str) -> bool {
    let mut comps = path.components();
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("Languages") {
            if let Some(lang) = comps.next() {
                let lang_s = lang.as_os_str().to_string_lossy();
                return lang_s == lang_dir;
            }
            return false;
        }
    }
    false
}

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

/// Validate scanned units under a root with optional filtering by language folder/code.
pub fn validate_under_root(
    scan_root: &Path,
    source_lang: Option<&str>,
    source_lang_dir: Option<&str>,
) -> Result<Vec<ValidationMessage>> {
    let mut units = rimloc_parsers_xml::scan_keyed_xml(scan_root)?;
    if let Some(dir) = source_lang_dir {
        units.retain(|u| is_under_languages_dir(&u.path, dir));
    } else if let Some(code) = source_lang {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| is_under_languages_dir(&u.path, &dir));
    }
    let msgs = rimloc_validate::validate(&units)?;
    Ok(msgs)
}

/// Build translation mod from an existing Languages/<lang> tree under `from_root`.
/// Returns a list of files to write with number of keys; optionally writes when `write=true`.
pub fn build_from_root(
    from_root: &Path,
    out_mod: &Path,
    lang_folder: &str,
    versions: Option<&[String]>,
    write: bool,
    dedupe: bool,
) -> Result<(Vec<(PathBuf, usize)>, usize)> {
    use std::collections::{BTreeMap, HashSet};
    use std::fs;

    let mut grouped: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();
    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](.+)$").unwrap();
    let mut total_keys = 0usize;
    let units = rimloc_parsers_xml::scan_keyed_xml(from_root)?;
    for u in units {
        let path_str = u.path.to_string_lossy();
        if !(path_str.contains("/Languages/") || path_str.contains("\\Languages\\")) {
            continue;
        }
        if !(path_str.contains(&format!("/Languages/{}/", lang_folder))
            || path_str.contains(&format!("\\Languages\\{}\\", lang_folder)))
        {
            continue;
        }
        if let Some(vers) = versions {
            let mut matched = false;
            for ver in vers {
                if path_str.contains(&format!("/{}/", ver))
                    || path_str.contains(&format!("\\{}\\", ver))
                    || path_str.contains(&format!("/v{}/", ver))
                    || path_str.contains(&format!("\\v{}\\", ver))
                {
                    matched = true;
                    break;
                }
            }
            if !matched {
                continue;
            }
        }
        if let Some(src) = u.source.as_deref() {
            if let Some(caps) = re.captures(&path_str) {
                let rel = PathBuf::from(&caps[1]);
                grouped
                    .entry(rel)
                    .or_default()
                    .push((u.key, src.to_string()));
                total_keys += 1;
            }
        }
    }

    let mut files: Vec<(PathBuf, usize)> = Vec::new();
    for (rel, mut items) in grouped.into_iter() {
        if dedupe {
            let mut seen: HashSet<String> = HashSet::new();
            let mut outv: Vec<(String, String)> = Vec::new();
            for (k, v) in items.into_iter().rev() {
                if seen.insert(k.clone()) {
                    outv.push((k, v));
                }
            }
            outv.reverse();
            items = outv;
        }

        let full = out_mod.join("Languages").join(lang_folder).join(&rel);
        if write {
            rimloc_import_po::write_language_data_xml(&full, &items)?;
        }
        files.push((full, items.len()));
    }

    if write {
        // Ensure About/ exists but leave content generation to CLI/services caller
        let _ = std::fs::create_dir_all(out_mod.join("About"));
    }

    Ok((files, total_keys))
}

