use crate::version::resolve_game_version_root;

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

#[allow(clippy::too_many_arguments)]
pub fn run_diff_xml(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    baseline_po: Option<std::path::PathBuf>,
    format: String,
    strict: bool,
    out_dir: Option<std::path::PathBuf>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "diff_xml_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, baseline_po = ?baseline_po, format = %format, out_dir = ?out_dir, game_version = ?game_version);

    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "diff_version_resolved", version = ver, path = %scan_root.display());
    }

    // Resolve folders
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
    tracing::info!(event = "diff_lang_dirs", source = %src_dir, target = %trg_dir);
    let diff = rimloc_services::diff_xml(
        &scan_root,
        &src_dir,
        &trg_dir,
        baseline_po.as_deref(),
    )?;
    let any_diff = !diff.changed.is_empty() || !diff.only_in_translation.is_empty() || !diff.only_in_mod.is_empty();

    // Output
    if let Some(dir) = out_dir.as_ref() {
        use std::fs;
        use std::io::Write;
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
