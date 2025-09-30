use crate::version::resolve_game_version_root;
use rimloc_services::apply_diff_flags;

#[allow(clippy::too_many_arguments)]
pub fn run_diff_xml(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    defs_dir: Option<std::path::PathBuf>,
    defs_field: Vec<String>,
    defs_dict: Vec<std::path::PathBuf>,
    lang: Option<String>,
    lang_dir: Option<String>,
    baseline_po: Option<std::path::PathBuf>,
    format: String,
    strict: bool,
    apply_flags: bool,
    backup: bool,
    out_dir: Option<std::path::PathBuf>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "diff_xml_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, baseline_po = ?baseline_po, format = %format, out_dir = ?out_dir, game_version = ?game_version);
    let cfg = rimloc_config::load_config().unwrap_or_default();

    let effective_version = game_version.or(cfg.game_version.clone());
    let (scan_root, selected_version) =
        resolve_game_version_root(&root, effective_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "diff_version_resolved", version = ver, path = %scan_root.display());
    }

    // Resolve folders
    let src_dir = if let Some(dir) = source_lang_dir {
        dir
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        cfg.source_lang
            .as_deref()
            .map(rimloc_import_po::rimworld_lang_dir)
            .unwrap_or_else(|| "English".to_string())
    };
    let trg_dir = if let Some(dir) = lang_dir {
        dir
    } else if let Some(code) = lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        cfg.target_lang
            .as_deref()
            .map(rimloc_import_po::rimworld_lang_dir)
            .unwrap_or_else(|| "Russian".to_string())
    };
    tracing::info!(event = "diff_lang_dirs", source = %src_dir, target = %trg_dir);
    let defs_abs = defs_dir.as_ref().map(|p| {
        if p.is_absolute() {
            p.clone()
        } else {
            scan_root.join(p)
        }
    });
    // Merge defs_field from config if CLI didn't set
    let cfg = rimloc_config::load_config().unwrap_or_default();
    // Apply ENV gates from config defaults
    if cfg
        .scan
        .as_ref()
        .and_then(|s| s.no_inherit)
        .unwrap_or(false)
    {
        std::env::set_var("RIMLOC_INHERIT", "0");
    }
    if cfg
        .scan
        .as_ref()
        .and_then(|s| s.keyed_nested)
        .unwrap_or(false)
    {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
    }
    let mut cli_defs_field = defs_field;
    if cli_defs_field.is_empty() {
        if let Some(scan) = cfg.scan.as_ref() {
            if let Some(extra) = scan.defs_fields.clone() {
                cli_defs_field = extra;
            }
        }
    }
    // Merge dictionaries
    let mut dicts = Vec::new();
    dicts.push(rimloc_parsers_xml::load_embedded_defs_dict());
    if let Some(scan) = cfg.scan.as_ref() {
        if let Some(paths) = scan.defs_dicts.as_ref() {
            for p in paths {
                let pp = if p.starts_with('/') || p.contains(':') {
                    std::path::PathBuf::from(p)
                } else {
                    scan_root.join(p)
                };
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
                    dicts.push(d);
                }
            }
        }
    }
    for p in &defs_dict {
        let pp = if p.is_absolute() {
            p.clone()
        } else {
            scan_root.join(p)
        };
        if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
            dicts.push(d);
        }
    }
    let merged = rimloc_parsers_xml::merge_defs_dicts(&dicts);

    let diff = rimloc_services::diff_xml_with_defs_and_dict(
        &scan_root,
        &src_dir,
        &trg_dir,
        baseline_po.as_deref(),
        defs_abs.as_deref(),
        &merged.0,
        &cli_defs_field,
    )?;
    let any_diff = !diff.changed.is_empty()
        || !diff.only_in_translation.is_empty()
        || !diff.only_in_mod.is_empty();

    // Apply flags to translation XML if requested
    if apply_flags {
        let (f_cnt, u_cnt) = rimloc_services::apply_diff_flags(&scan_root, &trg_dir, &diff, backup)?;
        crate::ui_out!("diffxml-flags-applied", fuzzy = f_cnt, unused = u_cnt);
        if strict && any_diff {
            color_eyre::eyre::bail!("diffxml-nonempty");
        }
        return Ok(());
    }

    // Output
    if let Some(dir) = out_dir.as_ref() {
        use std::fs;
        // write via services util; no local writer needed
        fs::create_dir_all(dir)?;
        rimloc_services::write_diff_reports(dir, &diff)?;
        crate::ui_ok!("diffxml-saved", path = dir.display().to_string());
        if strict && any_diff {
            color_eyre::eyre::bail!("diffxml-nonempty");
        }
        return Ok(());
    }

    if format == "json" {
        #[derive(serde::Serialize)]
        struct DiffOut {
            changed: Vec<(String, String)>,
            only_in_translation: Vec<String>,
            only_in_mod: Vec<String>,
        }
        let out = DiffOut {
            changed: diff.changed.clone(),
            only_in_translation: diff.only_in_translation.clone(),
            only_in_mod: diff.only_in_mod.clone(),
        };
        serde_json::to_writer(std::io::stdout().lock(), &out)?;
        if strict && any_diff {
            color_eyre::eyre::bail!("diffxml-nonempty");
        }
        return Ok(());
    }

    // text summary
    crate::ui_out!(
        "diffxml-summary",
        changed = diff.changed.len(),
        only_trg = diff.only_in_translation.len(),
        only_src = diff.only_in_mod.len()
    );
    if strict && any_diff {
        color_eyre::eyre::bail!("diffxml-nonempty");
    }
    Ok(())
}
