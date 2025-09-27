use crate::version::resolve_game_version_root;

#[allow(dead_code)]
fn is_under_languages_dir(path: &std::path::Path, lang_dir: &str) -> bool {
    let mut comps = path.components();
    while let Some(c) = comps.next() {
        let s = c.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("Languages") {
            if let Some(lang) = comps.next() {
                let lang_s = lang.as_os_str().to_string_lossy();
                return lang_s == lang_dir;
            }
            return false;
        }
    }
    false
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn run_export_po(
    root: std::path::PathBuf,
    out_po: std::path::PathBuf,
    lang: Option<String>,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    tm_root: Option<std::path::PathBuf>,
    game_version: Option<String>,
    include_all_versions: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "export_po_args", root = ?root, out_po = ?out_po, lang = ?lang, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, tm_root = ?tm_root, game_version = ?game_version, include_all_versions = include_all_versions);

    let (scan_root, selected_version) = if include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, game_version.as_deref())?
    };
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "export_version_resolved", version = ver, path = %scan_root.display());
    }

    let units = rimloc_parsers_xml::scan_keyed_xml(&scan_root)?;

    let src_dir = if let Some(dir) = source_lang_dir {
        dir
    } else if let Some(code) = source_lang {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "English".to_string()
    };
    tracing::info!(event = "export_from", source_dir = %src_dir);

    let mut filtered: Vec<_> = units
        .into_iter()
        .filter(|u| is_under_languages_dir(&u.path, &src_dir))
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
    tracing::info!(event = "export_units", count = filtered.len(), source_dir = %src_dir);

    // Build TM map if requested
    let tm_map: Option<std::collections::HashMap<String, String>> = if let Some(tm_path) = tm_root {
        match rimloc_parsers_xml::scan_keyed_xml(&tm_path) {
            Ok(units) => {
                let mut map = std::collections::HashMap::<String, String>::new();
                for u in units {
                    if let Some(val) = u.source.as_deref() {
                        let v = val.trim();
                        if !v.is_empty() {
                            map.entry(u.key).or_insert_with(|| v.to_string());
                        }
                    }
                }
                tracing::info!(event = "export_tm_loaded", entries = map.len());
                Some(map)
            }
            Err(e) => {
                tracing::warn!(event = "export_tm_failed", error = ?e);
                None
            }
        }
    } else {
        None
    };

    let stats =
        rimloc_export_po::write_po_with_tm(&out_po, &filtered, lang.as_deref(), tm_map.as_ref())?;
    ui_ok!("export-po-saved", path = out_po.display().to_string());
    if tm_map.is_some() {
        let pct: u32 = if stats.total == 0 {
            0
        } else {
            ((stats.tm_filled as f64 / stats.total as f64) * 100.0).round() as u32
        };
        ui_info!(
            "export-po-tm-coverage",
            total = stats.total,
            filled = stats.tm_filled,
            pct = pct
        );
    }
    Ok(())
}
