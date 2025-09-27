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

    let stats = rimloc_services::export_po_with_tm(
        &scan_root,
        &out_po,
        lang.as_deref(),
        source_lang.as_deref(),
        source_lang_dir.as_deref(),
        tm_root.as_deref(),
    )?;
    ui_ok!("export-po-saved", path = out_po.display().to_string());
    if tm_root.is_some() {
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
