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
    game_version: Option<String>,
    include_all_versions: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "export_po_args", root = ?root, out_po = ?out_po, lang = ?lang, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, game_version = ?game_version, include_all_versions = include_all_versions);

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

    let filtered: Vec<_> = units
        .into_iter()
        .filter(|u| is_under_languages_dir(&u.path, &src_dir))
        .collect();
    tracing::info!(event = "export_units", count = filtered.len(), source_dir = %src_dir);

    rimloc_export_po::write_po(&out_po, &filtered, lang.as_deref())?;
    ui_ok!("export-po-saved", path = out_po.display().to_string());
    Ok(())
}
