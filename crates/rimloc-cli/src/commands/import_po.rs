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
    format: String,
    // New behavior flags
    // If true, print a summary of created/updated/skipped files and total keys written
    // (text only; JSON can be added later if needed)
    report: bool,
    // If true, skip writing files whose content would be identical (compare bytes)
    incremental: bool,
    only_diff: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "import_po_args", po = ?po, out_xml = ?out_xml, mod_root = ?mod_root, lang = ?lang, lang_dir = ?lang_dir, keep_empty = keep_empty, dry_run = dry_run, backup = backup, single_file = single_file, game_version = ?game_version);
    // no local fs imports needed

    let cfg_all = rimloc_config::load_config().unwrap_or_default();
    let cfg_imp = cfg_all.import.unwrap_or_default();
    let eff_keep_empty = keep_empty || cfg_imp.keep_empty.unwrap_or(false);
    let eff_backup = backup || cfg_imp.backup.unwrap_or(false);
    let eff_single_file = single_file || cfg_imp.single_file.unwrap_or(false);
    let eff_incremental = incremental || cfg_imp.incremental.unwrap_or(false);
    let eff_only_diff = only_diff || cfg_imp.only_diff.unwrap_or(false);
    let eff_report = report || cfg_imp.report.unwrap_or(false);
    if let Some(out) = out_xml {
        let summary =
            rimloc_services::import_po_to_file(&po, &out, eff_keep_empty, dry_run, eff_backup)?;
        if dry_run {
            if format == "json" {
                #[derive(serde::Serialize)]
                struct Plan<'a> {
                    mode: &'a str,
                    files: Vec<(String, usize)>,
                    total_keys: usize,
                }
                let p = Plan {
                    mode: "dry_run",
                    files: summary
                        .files
                        .iter()
                        .map(|f| (f.path.clone(), f.keys))
                        .collect(),
                    total_keys: summary.keys,
                };
                serde_json::to_writer(std::io::stdout().lock(), &p)?;
            } else {
                for f in &summary.files {
                    ui_out!(
                        "dry-run-would-write",
                        count = f.keys,
                        path = f.path.as_str()
                    );
                }
            }
            return Ok(());
        }
        ui_ok!("xml-saved", path = out.display().to_string());
        if report && format == "json" {
            #[derive(serde::Serialize)]
            struct Out<'a> {
                mode: &'a str,
                created: usize,
                updated: usize,
                skipped: usize,
                keys: usize,
                files: Vec<(String, usize)>,
            }
            let files = summary
                .files
                .iter()
                .map(|f| (f.path.clone(), f.keys))
                .collect();
            let stats = Out {
                mode: "import",
                created: summary.created,
                updated: summary.updated,
                skipped: summary.skipped,
                keys: summary.keys,
                files,
            };
            serde_json::to_writer(std::io::stdout().lock(), &stats)?;
        }
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

    let lang_folder = if let Some(dir) = lang_dir.or(cfg_imp.lang_dir) {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };
    tracing::debug!(event = "resolved_lang_folder", lang_folder = %lang_folder);

    // Delegate to services for grouping/writing/statistics
    let (plan, summary) = rimloc_services::import_po_to_mod_tree(
        &po,
        &root,
        &lang_folder,
        eff_keep_empty,
        dry_run,
        eff_backup,
        eff_single_file,
        eff_incremental,
        eff_only_diff,
        eff_report,
    )?;
    if let Some(p) = plan {
        ui_out!("import-dry-run-header");
        if format == "json" {
            #[derive(serde::Serialize)]
            struct Plan<'a> {
                mode: &'a str,
                total_keys: usize,
                files: Vec<(String, usize)>,
            }
            let files = p
                .files
                .iter()
                .map(|(path, n)| (path.display().to_string(), *n))
                .collect();
            let json = Plan {
                mode: "dry_run",
                total_keys: p.total_keys,
                files,
            };
            serde_json::to_writer(std::io::stdout().lock(), &json)?;
        } else {
            for (path, n) in p.files {
                ui_out!(
                    "import-dry-run-line",
                    path = path.display().to_string(),
                    n = n
                );
            }
            ui_out!("import-total-keys", n = p.total_keys);
        }
        return Ok(());
    }

    if let Some(sum) = summary {
        ui_ok!("import-done", root = root.display().to_string());
        if report {
            if format == "json" {
                #[derive(serde::Serialize)]
                struct FileStat {
                    path: String,
                    keys: usize,
                    status: String,
                    added: Vec<String>,
                    changed: Vec<String>,
                }
                #[derive(serde::Serialize)]
                struct Summary {
                    mode: String,
                    created: usize,
                    updated: usize,
                    skipped: usize,
                    keys: usize,
                    files: Vec<FileStat>,
                }
                let files: Vec<FileStat> = sum
                    .files
                    .iter()
                    .map(|f| FileStat {
                        path: f.path.clone(),
                        keys: f.keys,
                        status: f.status.to_string(),
                        added: f.added.clone(),
                        changed: f.changed.clone(),
                    })
                    .collect();
                let out = Summary {
                    mode: "import".into(),
                    created: sum.created,
                    updated: sum.updated,
                    skipped: sum.skipped,
                    keys: sum.keys,
                    files,
                };
                serde_json::to_writer(std::io::stdout().lock(), &out)?;
            } else {
                ui_out!(
                    "import-report-summary",
                    created = sum.created,
                    updated = sum.updated,
                    skipped = sum.skipped,
                    keys = sum.keys
                );
            }
        }
        return Ok(());
    }
    Ok(())
}
// helper moved to rimloc-services
