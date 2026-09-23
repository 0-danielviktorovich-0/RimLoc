use rimloc_services::extras::version_diff::{report_markdown, version_diff, VersionDiffReport};

pub fn run_version_diff(
    root: std::path::PathBuf,
    from: String,
    to: String,
    format: String,
    out_json: Option<std::path::PathBuf>,
    out_md: Option<std::path::PathBuf>,
) -> color_eyre::Result<()> {
    let scan_version = |ver: &str| -> color_eyre::Result<Vec<rimloc_core::TransUnit>> {
        let (dir, _) = crate::version::resolve_game_version_root(&root, Some(ver))?;
        let auto = rimloc_services::autodiscover_defs_context(&dir)?;
        let units = rimloc_services::scan_units_with_defs_and_dict(
            &dir,
            None,
            &auto
                .dict
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            &auto.extra_fields,
        )?;
        // Source-language units only (Defs + Languages/<source>).
        let cfg = rimloc_config::load_config().unwrap_or_default();
        let src_dir =
            rimloc_import_po::rimworld_lang_dir(cfg.source_lang.as_deref().unwrap_or("English"));
        Ok(units
            .into_iter()
            .filter(|u| rimloc_services::is_source_for_lang_dir(&u.path, &src_dir))
            .collect())
    };

    let units_a = scan_version(&from)?;
    let units_b = scan_version(&to)?;
    let report: VersionDiffReport = version_diff(&units_a, &units_b, &from, &to);

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
