use crate::version::resolve_game_version_root;

pub fn run_lang_update(
    game_root: std::path::PathBuf,
    repo: String,
    branch: Option<String>,
    zip: Option<std::path::PathBuf>,
    source_lang_dir: Option<String>,
    target_lang_dir: Option<String>,
    dry_run: bool,
    backup: bool,
) -> color_eyre::Result<()> {
    // game_root is not versioned; resolve_game_version_root is not directly applicable
    let src_dir = source_lang_dir.unwrap_or_else(|| "Russian".to_string());
    let trg_dir = target_lang_dir.unwrap_or_else(|| "Russian (GitHub)".to_string());

    let (plan, summary) = rimloc_services::lang_update(
        &game_root,
        &repo,
        branch.as_deref(),
        zip.as_deref(),
        &src_dir,
        &trg_dir,
        dry_run,
        backup,
    )?;

    if let Some(p) = plan {
        crate::ui_out!("langupdate-dry-run-header");
        for f in p.files.iter() {
            crate::ui_out!(
                "langupdate-dry-run-line",
                path = format!("{}/{}", p.out_languages_dir.join(&p.target_lang_dir).display(), f.rel_path),
                size = (f.size as i64)
            );
        }
        crate::ui_out!(
            "langupdate-summary",
            files = (p.files.len() as i64),
            bytes = (p.total_bytes as i64),
            out = p.out_languages_dir.join(&p.target_lang_dir).display().to_string()
        );
        return Ok(());
    }

    if let Some(s) = summary {
        crate::ui_out!(
            "langupdate-summary",
            files = (s.files as i64),
            bytes = (s.bytes as i64),
            out = s.out_dir.display().to_string()
        );
        return Ok(());
    }
    Ok(())
}

