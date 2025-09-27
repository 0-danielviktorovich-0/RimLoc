//! High-level orchestration layer over lower-level crates.
//! Intentionally thin: exposes stable functions used by CLI/GUI/LSP.

use std::path::{Path, PathBuf};

pub use rimloc_core::{Result, TransUnit};
pub use rimloc_validate::ValidationMessage;
pub use rimloc_export_po::PoStats as ExportPoStats;
use quick_xml::{events::Event, Reader};

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

#[derive(Debug, Clone)]
pub struct ImportPlan {
    pub files: Vec<(PathBuf, usize)>,
    pub total_keys: usize,
}

#[derive(Debug, Clone)]
pub struct FileStat {
    pub path: PathBuf,
    pub keys: usize,
    pub status: &'static str, // created/updated/skipped
    pub added: Vec<String>,
    pub changed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub keys: usize,
    pub files: Vec<FileStat>,
}

fn parse_language_file_keys(path: &Path) -> std::io::Result<std::collections::HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut key: Option<String> = None;
    let mut acc = std::collections::HashMap::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                stack.push(name.clone());
                if stack.len() == 2 && !name.is_empty() {
                    key = Some(name);
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() == 2 {
                    key = None;
                }
                stack.pop();
            }
            Ok(Event::Text(t)) => {
                if stack.len() == 2 {
                    if let Some(k) = key.as_ref() {
                        let v = t
                            .unescape()
                            .unwrap_or_else(|_| {
                                std::borrow::Cow::Owned(
                                    String::from_utf8_lossy(t.as_ref()).into_owned(),
                                )
                            })
                            .to_string();
                        acc.insert(k.clone(), v);
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                if stack.len() == 1 {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    acc.insert(name, String::new());
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(acc)
}

/// Import a PO file into a single XML file at `out_xml`.
pub fn import_po_to_file(
    po: &Path,
    out_xml: &Path,
    keep_empty: bool,
    dry_run: bool,
    backup: bool,
) -> Result<ImportSummary> {
    let mut entries = rimloc_import_po::read_po_entries(po)?;
    if !keep_empty {
        entries.retain(|e| !e.value.trim().is_empty());
    }

    if dry_run {
        return Ok(ImportSummary {
            created: if out_xml.exists() { 0 } else { 1 },
            updated: if out_xml.exists() { 1 } else { 0 },
            skipped: 0,
            keys: entries.len(),
            files: vec![FileStat {
                path: out_xml.to_path_buf(),
                keys: entries.len(),
                status: "planned",
                added: Vec::new(),
                changed: Vec::new(),
            }],
        });
    }

    if backup && out_xml.exists() {
        let bak = out_xml.with_extension("xml.bak");
        std::fs::copy(out_xml, &bak)?;
    }
    let pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
    rimloc_import_po::write_language_data_xml(out_xml, &pairs)?;

    Ok(ImportSummary {
        created: if out_xml.exists() { 0 } else { 1 },
        updated: if out_xml.exists() { 1 } else { 0 },
        skipped: 0,
        keys: pairs.len(),
        files: vec![FileStat {
            path: out_xml.to_path_buf(),
            keys: pairs.len(),
            status: "updated",
            added: Vec::new(),
            changed: Vec::new(),
        }],
    })
}

/// Import a PO file into a mod tree under `root/Languages/<lang_folder>`.
/// When `single_file` is true, writes everything into `Keyed/_Imported.xml`.
/// Returns a plan on dry-run, or a summary after applying.
pub fn import_po_to_mod_tree(
    po: &Path,
    root: &Path,
    lang_folder: &str,
    keep_empty: bool,
    dry_run: bool,
    backup: bool,
    single_file: bool,
    incremental: bool,
    only_diff: bool,
    report: bool,
) -> Result<(Option<ImportPlan>, Option<ImportSummary>)> {
    use std::collections::HashMap;
    let mut entries = rimloc_import_po::read_po_entries(po)?;
    if !keep_empty {
        entries.retain(|e| !e.value.trim().is_empty());
        if entries.is_empty() {
            return Ok((None, Some(ImportSummary { created: 0, updated: 0, skipped: 0, keys: 0, files: vec![] })));
        }
    }

    if single_file {
        let out = root.join("Languages").join(lang_folder).join("Keyed").join("_Imported.xml");
        if dry_run {
            return Ok((
                Some(ImportPlan { files: vec![(out.clone(), entries.len())], total_keys: entries.len() }),
                None,
            ));
        }
        if backup && out.exists() { let _ = std::fs::copy(&out, out.with_extension("xml.bak")); }
        let pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
        rimloc_import_po::write_language_data_xml(&out, &pairs)?;
        return Ok((None, Some(ImportSummary { created: (!out.exists()) as usize, updated: out.exists() as usize, skipped: 0, keys: pairs.len(), files: vec![FileStat { path: out, keys: pairs.len(), status: "updated", added: vec![], changed: vec![] }] })));
    }

    // Group by relative path from Languages/*
    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\]([^/\\]+)[/\\](?P<rel>.+?)(?::\d+)?$").unwrap();
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    for e in entries {
        let rel = e
            .reference
            .as_ref()
            .and_then(|r| re.captures(r))
            .and_then(|c| c.name("rel").map(|m| PathBuf::from(m.as_str())))
            .unwrap_or_else(|| PathBuf::from("Keyed/_Imported.xml"));
        grouped.entry(rel).or_default().push((e.key, e.value));
    }

    if dry_run {
        let mut files = Vec::new();
        let mut total = 0usize;
        let mut keys: Vec<_> = grouped.keys().cloned().collect();
        keys.sort();
        for rel in keys.into_iter() {
            let n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
            total += n;
            files.push((root.join("Languages").join(lang_folder).join(rel), n));
        }
        return Ok((Some(ImportPlan { files, total_keys: total }), None));
    }

    let mut created_files = 0usize;
    let mut updated_files = 0usize;
    let mut skipped_files = 0usize;
    let mut keys_written = 0usize;
    let mut files_stat: Vec<FileStat> = Vec::new();

    for (rel, mut items) in grouped {
        let out_path = root.join("Languages").join(lang_folder).join(&rel);
        if backup && out_path.exists() {
            let _ = std::fs::copy(&out_path, out_path.with_extension("xml.bak"));
        }

        let (added_keys, changed_keys) = if report && out_path.exists() {
            if let Ok(old_map) = parse_language_file_keys(&out_path) {
                let mut added = Vec::new();
                let mut changed = Vec::new();
                for (k, v) in &items {
                    if let Some(old) = old_map.get(k) {
                        if old != v { changed.push(k.clone()); }
                    } else { added.push(k.clone()); }
                }
                (added, changed)
            } else { (Vec::new(), Vec::new()) }
        } else { (Vec::new(), Vec::new()) };

        if incremental && out_path.exists() {
            let new_bytes = rimloc_import_po::render_language_data_xml_bytes(&items)?;
            let old_bytes = std::fs::read(&out_path).unwrap_or_default();
            if old_bytes == new_bytes {
                skipped_files += 1;
                files_stat.push(FileStat { path: out_path, keys: items.len(), status: "skipped", added: vec![], changed: vec![] });
                continue;
            }
        }

        let existed = out_path.exists();
        if only_diff && existed {
            let old_map = parse_language_file_keys(&out_path).unwrap_or_default();
            items.retain(|(k, v)| old_map.get(k).map(|ov| ov != v).unwrap_or(true));
            if items.is_empty() {
                skipped_files += 1;
                files_stat.push(FileStat { path: out_path, keys: 0, status: "skipped", added: vec![], changed: vec![] });
                continue;
            }
        }

        rimloc_import_po::write_language_data_xml(&out_path, &items)?;
        keys_written += items.len();
        if existed {
            updated_files += 1;
            files_stat.push(FileStat { path: out_path, keys: items.len(), status: "updated", added: added_keys, changed: changed_keys });
        } else {
            created_files += 1;
            files_stat.push(FileStat { path: out_path, keys: items.len(), status: "created", added: added_keys, changed: changed_keys });
        }
    }

    Ok((None, Some(ImportSummary { created: created_files, updated: updated_files, skipped: skipped_files, keys: keys_written, files: files_stat })))
}

/// Build a translation mod from a PO file (wrappers around importer crate).
pub struct BuildPlan {
    pub mod_name: String,
    pub package_id: String,
    pub rw_version: String,
    pub out_mod: PathBuf,
    pub lang_dir: String,
    pub files: Vec<(PathBuf, usize)>,
    pub total_keys: usize,
}

pub fn build_from_po_dry_run(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
) -> Result<BuildPlan> {
    let plan = rimloc_import_po::build_translation_mod_dry_run_opts(
        po,
        out_mod,
        lang_folder,
        name,
        package_id,
        rw_version,
        dedupe,
    )?;
    Ok(BuildPlan {
        mod_name: plan.mod_name,
        package_id: plan.package_id,
        rw_version: plan.rw_version,
        out_mod: plan.out_mod,
        lang_dir: plan.lang_dir,
        files: plan.files,
        total_keys: plan.total_keys,
    })
}

pub fn build_from_po_execute(
    po: &Path,
    out_mod: &Path,
    lang_folder: &str,
    name: &str,
    package_id: &str,
    rw_version: &str,
    dedupe: bool,
) -> Result<()> {
    rimloc_import_po::build_translation_mod_with_langdir_opts(
        po,
        out_mod,
        lang_folder,
        name,
        package_id,
        rw_version,
        dedupe,
    )
}

