use crate::version::resolve_game_version_root;
use regex::Regex;

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

fn rel_from_languages(path_str: &str) -> Option<String> {
    static REL_FROM_LANGUAGES: once_cell::sync::OnceCell<Regex> = once_cell::sync::OnceCell::new();
    let re = REL_FROM_LANGUAGES
        .get_or_init(|| Regex::new(r"(?:^|[/\\])Languages[/\\][^/\\]+[/\\](.+)$").unwrap());
    re.captures(path_str)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn run_init(
    root: std::path::PathBuf,
    source_lang: Option<String>,
    source_lang_dir: Option<String>,
    lang: Option<String>,
    lang_dir: Option<String>,
    overwrite: bool,
    dry_run: bool,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "init_args", root = ?root, source_lang = ?source_lang, source_lang_dir = ?source_lang_dir, lang = ?lang, lang_dir = ?lang_dir, overwrite = overwrite, dry_run = dry_run, game_version = ?game_version);

    let (scan_root, selected_version) = resolve_game_version_root(&root, game_version.as_deref())?;
    if let Some(ver) = selected_version.as_deref() {
        tracing::info!(event = "init_version_resolved", version = ver, path = %scan_root.display());
    }

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
    let plan = rimloc_services::make_init_plan(&scan_root, &src_dir, &trg_dir)?;
    if dry_run {
        for f in &plan.files {
            crate::ui_out!("dry-run-would-write", path = f.path.display().to_string(), count = f.keys);
        }
        crate::ui_out!("init-summary", count = 0i64, lang = plan.language.as_str());
        return Ok(());
    }
    let files_written = rimloc_services::write_init_plan(&plan, overwrite, false)?;
    crate::ui_out!("init-summary", count = files_written, lang = plan.language.as_str());
    Ok(())
}
