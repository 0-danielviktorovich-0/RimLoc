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

    let src_dir = if let Some(dir) = source_lang_dir {
        dir
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "English".to_string()
    };
    let trg_dir = if let Some(dir) = lang_dir {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "Russian".to_string()
    };
    let prefix = comment_prefix.unwrap_or_else(|| "EN:".to_string());
    if dry_run {
        // services annotate skips writing when dry_run, but we want per-file messages; keep single-line notification per CLI contract
    }
    let summary = rimloc_services::annotate_apply(
        &scan_root,
        &src_dir,
        &trg_dir,
        &prefix,
        strip,
        dry_run,
        backup,
    )?;
    crate::ui_out!("annotate-summary", processed = summary.processed, annotated = summary.annotated);
    Ok(())
}
