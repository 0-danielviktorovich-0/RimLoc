use rimloc_services::extras::version_diff::{
    report_markdown, version_diff_scan, VersionDiffScanOptions,
};

pub fn run_version_diff(
    root: std::path::PathBuf,
    from: String,
    to: String,
    format: String,
    out_json: Option<std::path::PathBuf>,
    out_md: Option<std::path::PathBuf>,
) -> color_eyre::Result<()> {
    // Per-version roots resolve via the standard game-version layout; the
    // scan+filter pipeline lives in services::version_diff_scan (shared
    // with the GUI contract_version_diff — one diff semantic everywhere).
    let cfg = rimloc_config::load_config().unwrap_or_default();
    let (dir_a, _) = crate::version::resolve_game_version_root(&root, Some(&from))?;
    let (dir_b, _) = crate::version::resolve_game_version_root(&root, Some(&to))?;
    let report = version_diff_scan(
        &dir_a,
        &dir_b,
        &VersionDiffScanOptions {
            source_lang: cfg.source_lang.clone(),
            from_label: Some(from.clone()),
            to_label: Some(to.clone()),
        },
    )?;

    if let Some(path) = &out_json {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        serde_json::to_writer_pretty(std::fs::File::create(path)?, &report)?;
        ui_info!("vdiff-json-saved", path = path.display().to_string());
    }
    if let Some(path) = &out_md {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        rimloc_services::write_atomic(path, report_markdown(&report).as_bytes())?;
        ui_info!("vdiff-md-saved", path = path.display().to_string());
    }
    match format.as_str() {
        "json" if out_json.is_none() => {
            serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
            println!();
        }
        "json" => {}
        _ => println!("{}", report_markdown(&report)),
    }
    Ok(())
}
