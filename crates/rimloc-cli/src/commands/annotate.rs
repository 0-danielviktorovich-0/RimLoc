use crate::version::resolve_game_version_root;

#[allow(clippy::too_many_arguments)]
pub fn run_annotate(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    comment_prefix: Option<String>,
    dry_run: bool,
    backup: bool,
    strip: bool,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "annotate_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, dry_run = dry_run, backup = backup, strip = strip, game_version = ?game_version);

    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "annotate_version_resolved", version = ver, path = %scan_root.display());
    }

    let cfg = rimloc_config::load_config().unwrap_or_default();
    let src_dir = if let Some(dir) = source_lang_dir.or(cfg.source_lang.clone()) {
        dir
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "English".to_string()
    };
    let trg_dir = if let Some(dir) = lang_dir.or(cfg.target_lang.clone()) {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };
    let prefix = comment_prefix
        .or_else(|| {
            rimloc_config::load_config()
                .ok()
                .and_then(|c| c.annotate.and_then(|a| a.comment_prefix))
        })
        .unwrap_or_else(|| "EN:".to_string());
    let cfg_ann = rimloc_config::load_config()
        .unwrap_or_default()
        .annotate
        .unwrap_or_default();
    let eff_strip = strip || cfg_ann.strip.unwrap_or(false);
    let eff_backup = backup || cfg_ann.backup.unwrap_or(false);
    if dry_run {
        // Detailed dry-run plan with per-file counts (limit to 100 for readability)
        let plan = rimloc_services::annotate_dry_run_plan(
            &scan_root, &src_dir, &trg_dir, &prefix, eff_strip,
        )?;
        let limit = cfg.list_limit.unwrap_or(100usize);
        for (idx, f) in plan.files.iter().enumerate() {
            if idx >= limit {
                break;
            }
            crate::ui_out!(
                "annotate-dry-run-line",
                path = f.path.as_str(),
                add = (f.add as i64),
                strip = (f.strip as i64)
            );
        }
        crate::ui_out!(
            "annotate-summary",
            processed = (plan.processed as i64),
            annotated = (plan.total_add as i64)
        );
        return Ok(());
    }
    let summary = rimloc_services::annotate_apply(
        &scan_root, &src_dir, &trg_dir, &prefix, eff_strip, false, eff_backup,
    )?;
    crate::ui_out!(
        "annotate-summary",
        processed = summary.processed,
        annotated = summary.annotated
    );
    Ok(())
}
