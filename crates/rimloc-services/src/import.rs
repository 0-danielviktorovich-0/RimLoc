use crate::Result;
use quick_xml::{events::Event, Reader};
use rimloc_domain::{ImportFileStat as DFileStat, ImportSummary as DSummary};
use std::path::{Path, PathBuf};

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

pub type ImportSummary = DSummary;

fn parse_language_file_keys(
    path: &Path,
) -> std::io::Result<std::collections::HashMap<String, String>> {
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
            mode: "import".into(),
            created: if out_xml.exists() { 0 } else { 1 },
            updated: if out_xml.exists() { 1 } else { 0 },
            skipped: 0,
            keys: entries.len(),
            files: vec![DFileStat {
                path: out_xml.display().to_string(),
                keys: entries.len(),
                status: "planned".into(),
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
        mode: "import".into(),
        created: if out_xml.exists() { 0 } else { 1 },
        updated: if out_xml.exists() { 1 } else { 0 },
        skipped: 0,
        keys: pairs.len(),
        files: vec![DFileStat {
            path: out_xml.display().to_string(),
            keys: pairs.len(),
            status: "updated".into(),
            added: Vec::new(),
            changed: Vec::new(),
        }],
    })
}

/// Import a PO file into a mod tree under `root/Languages/<lang_folder>`.
/// When `single_file` is true, writes everything into `Keyed/_Imported.xml`.
/// Returns a plan on dry-run, or a summary after applying.
#[allow(clippy::too_many_arguments)]
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
            return Ok((
                None,
                Some(ImportSummary {
                    mode: "import".into(),
                    created: 0,
                    updated: 0,
                    skipped: 0,
                    keys: 0,
                    files: vec![],
                }),
            ));
        }
    }

    if single_file {
        let out = root
            .join("Languages")
            .join(lang_folder)
            .join("Keyed")
            .join("_Imported.xml");
        if dry_run {
            return Ok((
                Some(ImportPlan {
                    files: vec![(out.clone(), entries.len())],
                    total_keys: entries.len(),
                }),
                None,
            ));
        }
        if backup && out.exists() {
            let _ = std::fs::copy(&out, out.with_extension("xml.bak"));
        }
        let pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
        let bytes = rimloc_import_po::render_language_data_xml_bytes(&pairs)?;
        crate::util::write_atomic(&out, &bytes)?;
        return Ok((
            None,
            Some(ImportSummary {
                mode: "import".into(),
                created: (!out.exists()) as usize,
                updated: out.exists() as usize,
                skipped: 0,
                keys: pairs.len(),
                files: vec![DFileStat {
                    path: out.display().to_string(),
                    keys: pairs.len(),
                    status: "updated".into(),
                    added: vec![],
                    changed: vec![],
                }],
            }),
        ));
    }

    // Group by relative path from Languages/*
    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\]([^/\\]+)[/\\](?P<rel>.+?)(?::\d+)?$")
        .unwrap();
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
        return Ok((
            Some(ImportPlan {
                files,
                total_keys: total,
            }),
            None,
        ));
    }

    let mut created_files = 0usize;
    let mut updated_files = 0usize;
    let mut skipped_files = 0usize;
    let mut keys_written = 0usize;
    let mut files_stat: Vec<DFileStat> = Vec::new();

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
                        if old != v {
                            changed.push(k.clone());
                        }
                    } else {
                        added.push(k.clone());
                    }
                }
                (added, changed)
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };

        if incremental && out_path.exists() {
            let new_bytes = rimloc_import_po::render_language_data_xml_bytes(&items)?;
            let old_bytes = std::fs::read(&out_path).unwrap_or_default();
            if old_bytes == new_bytes {
                skipped_files += 1;
                files_stat.push(DFileStat {
                    path: out_path.display().to_string(),
                    keys: items.len(),
                    status: "skipped".into(),
                    added: vec![],
                    changed: vec![],
                });
                continue;
            }
        }

        let existed = out_path.exists();
        if only_diff && existed {
            let old_map = parse_language_file_keys(&out_path).unwrap_or_default();
            items.retain(|(k, v)| old_map.get(k).map(|ov| ov != v).unwrap_or(true));
            if items.is_empty() {
                skipped_files += 1;
                files_stat.push(DFileStat {
                    path: out_path.display().to_string(),
                    keys: 0,
                    status: "skipped".into(),
                    added: vec![],
                    changed: vec![],
                });
                continue;
            }
        }

        let bytes = rimloc_import_po::render_language_data_xml_bytes(&items)?;
        crate::util::write_atomic(&out_path, &bytes)?;
        keys_written += items.len();
        if existed {
            updated_files += 1;
            files_stat.push(DFileStat {
                path: out_path.display().to_string(),
                keys: items.len(),
                status: "updated".into(),
                added: added_keys,
                changed: changed_keys,
            });
        } else {
            created_files += 1;
            files_stat.push(DFileStat {
                path: out_path.display().to_string(),
                keys: items.len(),
                status: "created".into(),
                added: added_keys,
                changed: changed_keys,
            });
        }
    }

    Ok((
        None,
        Some(ImportSummary {
            mode: "import".into(),
            created: created_files,
            updated: updated_files,
            skipped: skipped_files,
            keys: keys_written,
            files: files_stat,
        }),
    ))
}

/// Apply import with per-file progress callback (current, total, path)
#[allow(clippy::too_many_arguments)]
pub fn import_po_to_mod_tree_with_progress(
    po: &Path,
    root: &Path,
    lang_folder: &str,
    keep_empty: bool,
    backup: bool,
    single_file: bool,
    incremental: bool,
    only_diff: bool,
    report: bool,
    mut progress: impl FnMut(usize, usize, &Path),
) -> Result<ImportSummary> {
    use std::collections::HashMap;
    let mut entries = rimloc_import_po::read_po_entries(po)?;
    if !keep_empty {
        entries.retain(|e| !e.value.trim().is_empty());
        if entries.is_empty() {
            return Ok(ImportSummary {
                mode: "import".into(),
                created: 0,
                updated: 0,
                skipped: 0,
                keys: 0,
                files: vec![],
            });
        }
    }

    if single_file {
        let out = root
            .join("Languages")
            .join(lang_folder)
            .join("Keyed")
            .join("_Imported.xml");
        if backup && out.exists() {
            let _ = std::fs::copy(&out, out.with_extension("xml.bak"));
        }
        let pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
        let bytes = rimloc_import_po::render_language_data_xml_bytes(&pairs)?;
        crate::util::write_atomic(&out, &bytes)?;
        progress(1, 1, &out);
        return Ok(ImportSummary {
            mode: "import".into(),
            created: (!out.exists()) as usize,
            updated: out.exists() as usize,
            skipped: 0,
            keys: pairs.len(),
            files: vec![DFileStat {
                path: out.display().to_string(),
                keys: pairs.len(),
                status: "updated".into(),
                added: vec![],
                changed: vec![],
            }],
        });
    }

    // Group by relative path from Languages/*
    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\]([^/\\]+)[/\\](?P<rel>.+?)(?::\d+)?$")
        .unwrap();
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

    let total_files = grouped.len();
    let mut idx = 0usize;

    let mut created_files = 0usize;
    let mut updated_files = 0usize;
    let mut skipped_files = 0usize;
    let mut keys_written = 0usize;
    let mut files_stat: Vec<DFileStat> = Vec::new();

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
                        if old != v {
                            changed.push(k.clone());
                        }
                    } else {
                        added.push(k.clone());
                    }
                }
                (added, changed)
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };

        if incremental && out_path.exists() {
            let new_bytes = rimloc_import_po::render_language_data_xml_bytes(&items)?;
            let old_bytes = std::fs::read(&out_path).unwrap_or_default();
            if old_bytes == new_bytes {
                skipped_files += 1;
                files_stat.push(DFileStat {
                    path: out_path.display().to_string(),
                    keys: items.len(),
                    status: "skipped".into(),
                    added: vec![],
                    changed: vec![],
                });
                idx += 1;
                progress(idx, total_files, &out_path);
                continue;
            }
        }

        let existed = out_path.exists();
        if only_diff && existed {
            let old_map = parse_language_file_keys(&out_path).unwrap_or_default();
            items.retain(|(k, v)| old_map.get(k).map(|ov| ov != v).unwrap_or(true));
            if items.is_empty() {
                skipped_files += 1;
                files_stat.push(DFileStat {
                    path: out_path.display().to_string(),
                    keys: 0,
                    status: "skipped".into(),
                    added: vec![],
                    changed: vec![],
                });
                idx += 1;
                progress(idx, total_files, &out_path);
                continue;
            }
        }

        let bytes = rimloc_import_po::render_language_data_xml_bytes(&items)?;
        crate::util::write_atomic(&out_path, &bytes)?;
        keys_written += items.len();
        if existed {
            updated_files += 1;
            files_stat.push(DFileStat {
                path: out_path.display().to_string(),
                keys: items.len(),
                status: "updated".into(),
                added: added_keys,
                changed: changed_keys,
            });
        } else {
            created_files += 1;
            files_stat.push(DFileStat {
                path: out_path.display().to_string(),
                keys: items.len(),
                status: "created".into(),
                added: added_keys,
                changed: changed_keys,
            });
        }
        idx += 1;
        progress(idx, total_files, &out_path);
    }

    Ok(ImportSummary {
        mode: "import".into(),
        created: created_files,
        updated: updated_files,
        skipped: skipped_files,
        keys: keys_written,
        files: files_stat,
    })
}
