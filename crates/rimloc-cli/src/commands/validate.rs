use crate::version::resolve_game_version_root;

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn run_validate(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    defs_dir: Option<std::path::PathBuf>,
    defs_field: Vec<String>,
    defs_dict: Vec<std::path::PathBuf>,
    format: String,
    game_version: Option<String>,
    include_all_versions: bool,
    use_color: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "validate_args", root = ?root, game_version = ?game_version, include_all_versions = include_all_versions);

    let cfg = rimloc_config::load_config().unwrap_or_default();
    let effective_version = game_version.or(cfg.game_version.clone());
    let (scan_root, selected_version) = if include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, effective_version.as_deref())?
    };
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "validate_version_resolved", version = ver, path = %scan_root.display());
    }

    let defs_abs = defs_dir
        .as_ref()
        .map(|p| if p.is_absolute() { p.clone() } else { scan_root.join(p) });
    // Merge defs_field from config if CLI didn't set
    let cfg = rimloc_config::load_config().unwrap_or_default();
    let mut cli_defs_field = defs_field;
    if cli_defs_field.is_empty() {
        if let Some(ref scan) = cfg.scan {
            if let Some(extra) = scan.defs_fields.clone() { cli_defs_field = extra; }
        }
    }
    // Merge dictionaries
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

    let msgs = rimloc_services::validate_under_root_with_defs_and_dict(
        &scan_root,
        source_lang.or(cfg.source_lang).as_deref(),
        source_lang_dir.as_deref(),
        defs_abs.as_deref(),
        &merged.0,
        &cli_defs_field,
    )?;
    if format == "json" {
        #[derive(serde::Serialize)]
        struct JsonMsg<'a> {
            schema_version: u32,
            kind: &'a str,
            key: &'a str,
            path: &'a str,
            line: Option<usize>,
            message: &'a str,
        }
        let items: Vec<JsonMsg> = msgs
            .iter()
            .map(|m| JsonMsg {
                schema_version: crate::OUTPUT_SCHEMA_VERSION,
                kind: m.kind.as_str(),
                key: m.key.as_str(),
                path: m.path.as_str(),
                line: m.line,
                message: m.message.as_str(),
            })
            .collect();
        serde_json::to_writer(std::io::stdout().lock(), &items)?;
        return Ok(());
    }
    if msgs.is_empty() {
        if use_color {
            use owo_colors::OwoColorize;
            println!("{} {}", "✔".green(), tr!("validate-clean"));
        } else {
            println!("✔ {}", tr!("validate-clean"));
        }
    } else {
        for m in msgs {
            if !use_color {
                println!(
                    "[{}] {} ({}:{}) — {}",
                    m.kind,
                    m.key,
                    m.path,
                    m.line.unwrap_or(0),
                    m.message
                );
            } else {
                use owo_colors::OwoColorize;
                let tag = match m.kind.as_str() {
                    "duplicate" => "⚠",
                    "empty" => "✖",
                    "placeholder-check" => "ℹ",
                    _ => "•",
                };
                let plain_kind_token = m.kind.as_str();
                println!(
                    "{} [{}] {} ({}:{}) — {}",
                    tag,
                    plain_kind_token,
                    m.key.green(),
                    m.path.blue(),
                    m.line.unwrap_or(0).to_string().magenta(),
                    m.message
                );
            }
        }
    }
    Ok(())
}
