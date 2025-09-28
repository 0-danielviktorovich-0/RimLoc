use crate::version::resolve_game_version_root;

#[allow(clippy::too_many_arguments)]
pub fn run_init(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    overwrite: bool,
    dry_run: bool,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "init_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, overwrite = overwrite, dry_run = dry_run, game_version = ?game_version);

    let cfg = rimloc_config::load_config().unwrap_or_default();
    let (scan_root, selected_version) =
        resolve_game_version_root(&root, game_version.or(cfg.game_version.clone()).as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "init_version_resolved", version = ver, path = %scan_root.display());
    }

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
    let plan = rimloc_services::make_init_plan(&scan_root, &src_dir, &trg_dir)?;
    if dry_run {
        for f in &plan.files {
            crate::ui_out!(
                "dry-run-would-write",
                path = f.path.display().to_string(),
                count = f.keys
            );
        }
        crate::ui_out!("init-summary", count = 0i64, lang = plan.language.as_str());
        return Ok(());
    }
    let files_written = rimloc_services::write_init_plan(&plan, overwrite, false)?;
    crate::ui_out!(
        "init-summary",
        count = files_written,
        lang = plan.language.as_str()
    );
    Ok(())
}
