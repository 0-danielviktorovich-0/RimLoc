use crate::Result;
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

/// Read an existing Keyed/LanguageData XML file into a key -> value map for
/// import diffing. Delegates to the canonical Keyed reader so that entity
/// markup (`&lt;b&gt;...&lt;/b&gt;`) resolves to the same in-game text
/// (`<b>...</b>`) the PO side carries — otherwise every marked-up value would
/// look "changed" on each import.
fn parse_language_file_keys(
    path: &Path,
) -> std::io::Result<std::collections::BTreeMap<String, String>> {
    rimloc_parsers_xml::read_keyed_file_map(path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
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
    // H1: the language folder is joined into every output path - strict
    // form + containment BEFORE anything is planned or written.
    crate::util::ensure_lang_write_target(root, lang_folder)?;
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

    // Group by relative path from Languages/*. The `#:` reference comes
    // from a SHARED PO artifact, so every captured rel is validated before
    // it can become a write path: absolute/`..`/empty shapes are a typed
    // refusal (same discipline as `ensure_lang_write_target`), never a
    // silent fallback.
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    for e in entries {
        let rel = match e.reference.as_deref() {
            Some(r) => rimloc_import_po::rel_from_reference(r)?
                .unwrap_or_else(|| PathBuf::from("Keyed/_Imported.xml")),
            None => PathBuf::from("Keyed/_Imported.xml"),
        };
        grouped.entry(rel).or_default().push((e.key, e.value));
    }

    if dry_run {
        let lang_base = root.join("Languages").join(lang_folder);
        let mut files = Vec::new();
        let mut total = 0usize;
        let mut keys: Vec<_> = grouped.keys().cloned().collect();
        keys.sort();
        for rel in keys.into_iter() {
            let n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
            total += n;
            files.push((lang_base.join(rel), n));
        }
        return Ok((
            Some(ImportPlan {
                files,
                total_keys: total,
            }),
            None,
        ));
    }

    let lang_base = root.join("Languages").join(lang_folder);
    let mut created_files = 0usize;
    let mut updated_files = 0usize;
    let mut skipped_files = 0usize;
    let mut keys_written = 0usize;
    let mut files_stat: Vec<DFileStat> = Vec::new();

    for (rel, mut items) in grouped {
        let out_path = lang_base.join(&rel);
        // Canonical containment (defense in depth): the validated rel keeps
        // the spelling inside the language folder; this re-check on the
        // REAL filesystem view catches symlink aliases before any write.
        if !crate::util::is_within_allow(&out_path, &lang_base) {
            color_eyre::eyre::bail!(
                "import target `{}` resolves outside the language folder `{}`; refusing to write",
                out_path.display(),
                lang_base.display()
            );
        }
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
    // H1: strict form + containment before anything is planned or written.
    crate::util::ensure_lang_write_target(root, lang_folder)?;
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

    // Group by relative path from Languages/*. Same discipline as the
    // plan/apply twin above: the `#:` reference is shared-artifact input,
    // every captured rel is validated before it becomes a write path.
    let mut grouped: HashMap<PathBuf, Vec<(String, String)>> = HashMap::new();
    for e in entries {
        let rel = match e.reference.as_deref() {
            Some(r) => rimloc_import_po::rel_from_reference(r)?
                .unwrap_or_else(|| PathBuf::from("Keyed/_Imported.xml")),
            None => PathBuf::from("Keyed/_Imported.xml"),
        };
        grouped.entry(rel).or_default().push((e.key, e.value));
    }

    let lang_base = root.join("Languages").join(lang_folder);
    let total_files = grouped.len();
    let mut idx = 0usize;

    let mut created_files = 0usize;
    let mut updated_files = 0usize;
    let mut skipped_files = 0usize;
    let mut keys_written = 0usize;
    let mut files_stat: Vec<DFileStat> = Vec::new();

    for (rel, mut items) in grouped {
        let out_path = lang_base.join(&rel);
        // Canonical containment on the real filesystem view before any
        // write (defense in depth, see the twin loop above).
        if !crate::util::is_within_allow(&out_path, &lang_base) {
            color_eyre::eyre::bail!(
                "import target `{}` resolves outside the language folder `{}`; refusing to write",
                out_path.display(),
                lang_base.display()
            );
        }
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
