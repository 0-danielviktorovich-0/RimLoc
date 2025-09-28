use crate::version::resolve_game_version_root;
use std::io::IsTerminal;
use std::path::Path;
use walkdir::WalkDir;

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
    mut tm_roots: Vec<std::path::PathBuf>,
    game_version: Option<String>,
    include_all_versions: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "export_po_args", root = ?root, out_po = ?out_po, lang = ?lang, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, tm_roots = ?tm_roots, game_version = ?game_version, include_all_versions = include_all_versions);
    let cfg = rimloc_config::load_config().unwrap_or_default();

    let effective_version = game_version.or(cfg.game_version.clone());
    let (scan_root, selected_version) = if include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, effective_version.as_deref())?
    };
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "export_version_resolved", version = ver, path = %scan_root.display());
    }

    // If no CLI tm-roots provided, fallback to config export.tm_root
    let cfg_tm_root: Option<std::path::PathBuf> = rimloc_config::load_config()
        .ok()
        .and_then(|c| c.export.and_then(|e| e.tm_root))
        .map(std::path::PathBuf::from);
    if tm_roots.is_empty() {
        if let Some(one) = cfg_tm_root {
            tm_roots.push(one);
        }
    }

    let auto = rimloc_services::autodiscover_defs_context(&scan_root)?;

    let effective_source_lang = source_lang.clone().or(cfg.source_lang.clone());
    let stats = rimloc_services::export_po_with_tm(
        &scan_root,
        &out_po,
        lang.as_deref(),
        effective_source_lang.as_deref(),
        source_lang_dir.as_deref(),
        if tm_roots.is_empty() {
            None
        } else {
            Some(&tm_roots)
        },
    )?;
    ui_ok!("export-po-saved", path = out_po.display().to_string());
    if !tm_roots.is_empty() {
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

    fn has_definj_files(dir: &Path) -> bool {
        if !dir.exists() {
            return false;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("xml"))
                    .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    let src_dir = if let Some(dir) = source_lang_dir.as_deref() {
        dir.to_string()
    } else if let Some(code) = effective_source_lang.clone() {
        rimloc_import_po::rimworld_lang_dir(&code)
    } else {
        "English".to_string()
    };
    let english_definj = scan_root
        .join("Languages")
        .join(&src_dir)
        .join("DefInjected");
    if !has_definj_files(&english_definj) {
        let suggested = scan_root.join("_learn").join("suggested.xml");
        if suggested.exists() {
            ui_warn!(
                "export-po-missing-definj-suggested",
                path = suggested.display().to_string(),
                lang_dir = src_dir
            );
        } else if let Some(first) = auto.learned_sources.first() {
            ui_warn!(
                "export-po-missing-definj-learned",
                path = first.display().to_string()
            );
        } else {
            ui_warn!("export-po-missing-definj-generate", lang_dir = src_dir);
        }
    }
    Ok(())
}
