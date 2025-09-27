use crate::version::resolve_game_version_root;
use std::io::IsTerminal;

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
pub fn run_scan(
    root: std::path::PathBuf,
    out_csv: Option<std::path::PathBuf>,
    out_json: Option<std::path::PathBuf>,
    lang: Option<String>,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    defs_dir: Option<std::path::PathBuf>,
    defs_field: Vec<String>,
    defs_dict: Vec<std::path::PathBuf>,
    format: String,
    game_version: Option<String>,
    include_all_versions: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(
        event = "scan_args",
        root = ?root,
        out_csv = ?out_csv,
        out_json = ?out_json,
        lang = ?lang,
        format = %format,
        game_version = ?game_version,
        include_all_versions = include_all_versions
    );

    let (scan_root, selected_version) = if include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, game_version.as_deref())?
    };
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "scan_version_resolved", version = ver, path = %scan_root.display());
    }

    let defs_abs = defs_dir
        .as_ref()
        .map(|p| if p.is_absolute() { p.clone() } else { scan_root.join(p) });
    // Merge defs_field from config if not provided on CLI
    let cfg = rimloc_config::load_config().unwrap_or_default();
    let mut cli_defs_field = defs_field;
    if cli_defs_field.is_empty() {
        if let Some(ref scan) = cfg.scan {
            if let Some(extra) = scan.defs_fields.clone() { cli_defs_field = extra; }
        }
    }
    // Load dictionaries (embedded + config + CLI)
    let mut dicts = Vec::new();
    dicts.push(rimloc_parsers_xml::load_embedded_defs_dict());
    if let Some(scan) = cfg.scan.as_ref() {
        if let Some(paths) = scan.defs_dicts.as_ref() {
            for p in paths {
                let pp = if p.starts_with('/') || p.contains(':') { std::path::PathBuf::from(p) } else { scan_root.join(p) };
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) { dicts.push(d); }
            }
        }
    }
    for p in &defs_dict {
        let pp = if p.is_absolute() { p.clone() } else { scan_root.join(p) };
        if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) { dicts.push(d); }
    }
    let merged = rimloc_parsers_xml::merge_defs_dicts(&dicts);

    let mut units = rimloc_services::scan_units_with_defs_and_dict(
        &scan_root,
        defs_abs.as_deref(),
        &merged.0,
        &cli_defs_field,
    )?;

    fn is_source_for_lang_dir(path: &std::path::Path, lang_dir: &str) -> bool {
        // Languages/<dir>
        let mut comps = path.components();
        while let Some(c) = comps.next() {
            let s = c.as_os_str().to_string_lossy();
            if s.eq_ignore_ascii_case("Languages") {
                if let Some(lang) = comps.next() {
                    let lang_s = lang.as_os_str().to_string_lossy();
                    if lang_s == lang_dir { return true; }
                }
                break;
            }
        }
        // English also includes Defs/*
        if lang_dir.eq_ignore_ascii_case("English") {
            let s = path.to_string_lossy();
            if s.contains("/Defs/") || s.contains("\\Defs\\") { return true; }
        }
        false
    }

    let units = if let Some(dir) = source_lang_dir.clone() {
        let before = units.len();
        let mut filtered: Vec<_> = units
            .into_iter()
            .filter(|u| is_source_for_lang_dir(&u.path, dir.as_str()))
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
        tracing::info!(event = "scan_filtered_by_dir", before = before, after = filtered.len(), source_lang_dir = %dir);
        filtered
    } else if let Some(code) = source_lang.clone() {
        let dir = rimloc_import_po::rimworld_lang_dir(&code);
        let before = units.len();
        let mut filtered: Vec<_> = units
            .into_iter()
            .filter(|u| is_source_for_lang_dir(&u.path, dir.as_str()))
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
        tracing::info!(event = "scan_filtered_by_code", source_lang = %code, source_dir = %dir, before = before, after = filtered.len());
        filtered
    } else {
        units.sort_by(|a, b| {
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
        units
    };

    match format.as_str() {
        "csv" => {
            if out_json.is_some() {
                return Err(color_eyre::eyre::eyre!(
                    "--out-json is only supported when --format json"
                ));
            }
            if let Some(path) = out_csv {
                let file = std::fs::File::create(&path)?;
                rimloc_export_csv::write_csv(file, &units, lang.as_deref())?;
                ui_info!("scan-csv-saved", path = path.display().to_string());
            } else {
                if std::io::stdout().is_terminal() {
                    ui_info!("scan-csv-stdout");
                }
                let stdout = std::io::stdout();
                let lock = stdout.lock();
                rimloc_export_csv::write_csv(lock, &units, lang.as_deref())?;
            }
        }
        "json" => {
            #[derive(serde::Serialize)]
            struct JsonUnit<'a> {
                schema_version: u32,
                path: String,
                line: Option<usize>,
                key: &'a str,
                value: Option<&'a str>,
            }
            let items: Vec<JsonUnit<'_>> = units
                .iter()
                .map(|u| JsonUnit {
                    schema_version: crate::OUTPUT_SCHEMA_VERSION,
                    path: u.path.display().to_string(),
                    line: u.line,
                    key: u.key.as_str(),
                    value: u.source.as_deref(),
                })
                .collect();

            if let Some(path) = out_json {
                let file = std::fs::File::create(&path)?;
                serde_json::to_writer_pretty(file, &items)?;
                ui_info!("scan-json-saved", path = path.display().to_string());
            } else {
                if std::io::stdout().is_terminal() {
                    ui_info!("scan-json-stdout");
                }
                serde_json::to_writer(std::io::stdout().lock(), &items)?;
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
