use crate::version::resolve_game_version_root;

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn run_import_po(
    po: std::path::PathBuf,
    out_xml: Option<std::path::PathBuf>,
    mod_root: Option<std::path::PathBuf>,
    lang: Option<String>,
    lang_dir: Option<String>,
    keep_empty: bool,
    dry_run: bool,
    backup: bool,
    single_file: bool,
    game_version: Option<String>,
    // New behavior flags
    // If true, print a summary of created/updated/skipped files and total keys written
    // (text only; JSON can be added later if needed)
    report: bool,
    // If true, skip writing files whose content would be identical (compare bytes)
    incremental: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "import_po_args", po = ?po, out_xml = ?out_xml, mod_root = ?mod_root, lang = ?lang, lang_dir = ?lang_dir, keep_empty = keep_empty, dry_run = dry_run, backup = backup, single_file = single_file, game_version = ?game_version);
    use std::fs;

    let mut entries = rimloc_import_po::read_po_entries(&po)?;
    tracing::debug!(event = "import_po_loaded", entries = entries.len());

    if !keep_empty {
        let before = entries.len();
        entries.retain(|e| !e.value.trim().is_empty());
        tracing::debug!(
            event = "import_po_filtered_empty",
            before = before,
            after = entries.len()
        );
        if entries.is_empty() {
            ui_info!("import-nothing-to-do");
            return Ok(());
        }
    }

    if let Some(out) = out_xml {
        if dry_run {
            ui_out!(
                "dry-run-would-write",
                count = entries.len(),
                path = out.display().to_string()
            );
            return Ok(());
        }

        if backup && out.exists() {
            let bak = out.with_extension("xml.bak");
            fs::copy(&out, &bak)?;
            tracing::warn!(event = "backup", from = %out.display(), to = %bak.display());
        }

        let pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
        rimloc_import_po::write_language_data_xml(&out, &pairs)?;
        ui_ok!("xml-saved", path = out.display().to_string());
        return Ok(());
    }

    let Some(base_root) = mod_root else {
        ui_info!("import-need-target");
        std::process::exit(2);
    };

    let (root, selected_version) = resolve_game_version_root(&base_root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "import_version_resolved", version = ver, path = %root.display());
    }

    let lang_folder = if let Some(dir) = lang_dir {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };
    tracing::debug!(event = "resolved_lang_folder", lang_folder = %lang_folder);

    if single_file {
        let out = root
            .join("Languages")
            .join(&lang_folder)
            .join("Keyed")
            .join("_Imported.xml");

        if dry_run {
            ui_out!(
                "dry-run-would-write",
                count = entries.len(),
                path = out.display().to_string()
            );
            return Ok(());
        }

        if backup && out.exists() {
            let bak = out.with_extension("xml.bak");
            fs::copy(&out, &bak)?;
            tracing::warn!(event = "backup", from = %out.display(), to = %bak.display());
        }

        let pairs: Vec<(String, String)> = entries.into_iter().map(|e| (e.key, e.value)).collect();
        rimloc_import_po::write_language_data_xml(&out, &pairs)?;
        ui_ok!("xml-saved", path = out.display().to_string());
        return Ok(());
    }

    use std::collections::HashMap;
    let re = regex::Regex::new(r"(?:^|[/\\])Languages[/\\]([^/\\]+)[/\\](?P<rel>.+?)(?::\d+)?$")
        .unwrap();
    let mut grouped: HashMap<std::path::PathBuf, Vec<(String, String)>> = HashMap::new();

    for e in entries {
        let rel = e
            .reference
            .as_ref()
            .and_then(|r| re.captures(r))
            .and_then(|c| c.name("rel").map(|m| std::path::PathBuf::from(m.as_str())))
            .unwrap_or_else(|| std::path::PathBuf::from("Keyed/_Imported.xml"));
        grouped.entry(rel).or_default().push((e.key, e.value));
    }

    if dry_run {
        ui_out!("import-dry-run-header");
        let mut keys_total = 0usize;
        let mut paths: Vec<_> = grouped.keys().cloned().collect();
        paths.sort();
        for rel in paths {
            let n = grouped.get(&rel).map(|v| v.len()).unwrap_or(0);
            keys_total += n;
            let full_path = root.join("Languages").join(&lang_folder).join(&rel);
            ui_out!(
                "import-dry-run-line",
                path = full_path.display().to_string(),
                n = n
            );
        }
        ui_out!("import-total-keys", n = keys_total);
        return Ok(());
    }

    // Summary counters
    let mut created_files = 0usize;
    let mut updated_files = 0usize;
    let mut skipped_files = 0usize;
    let mut keys_written = 0usize;

    for (rel, items) in grouped {
        let out_path = root.join("Languages").join(&lang_folder).join(&rel);
        if backup && out_path.exists() {
            let bak = out_path.with_extension("xml.bak");
            std::fs::copy(&out_path, &bak)?;
            tracing::warn!(event = "backup", from = %out_path.display(), to = %bak.display());
        }
        if incremental && out_path.exists() {
            // render new bytes and compare with existing file
            let new_bytes = rimloc_import_po::render_language_data_xml_bytes(&items)?;
            let old_bytes = std::fs::read(&out_path).unwrap_or_default();
            if old_bytes == new_bytes {
                skipped_files += 1;
                continue;
            }
        }

        let existed = out_path.exists();
        rimloc_import_po::write_language_data_xml(&out_path, &items)?;
        keys_written += items.len();
        if existed {
            updated_files += 1;
        } else {
            created_files += 1;
        }
    }

    ui_ok!("import-done", root = root.display().to_string());
    if report {
        ui_out!(
            "import-report-summary",
            created = created_files,
            updated = updated_files,
            skipped = skipped_files,
            keys = keys_written
        );
    }
    Ok(())
}
