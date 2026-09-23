use rimloc_services::extras::wordinfo;

pub fn run_wordinfo(
    root: std::path::PathBuf,
    lang: String,
    out_scaffold: Option<std::path::PathBuf>,
    format: String,
) -> color_eyre::Result<()> {
    // Accept folder names ("Russian (Русский)") and codes ("ru"): a code maps
    // through the game's folder-name table, an exact existing folder wins.
    let lang_dir = if root.join("Languages").join(&lang).is_dir() {
        lang.clone()
    } else {
        rimloc_import_po::rimworld_lang_dir(&lang)
    };
    let diag = wordinfo::diagnose(&root, &lang_dir)?;

    match diag.capability {
        wordinfo::WordInfoCapability::None => {
            ui_info!("wordinfo-not-applicable", lang = lang.clone());
            return Ok(());
        }
        wordinfo::WordInfoCapability::Full => {}
    }

    if let Some(path) = &out_scaffold {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        rimloc_services::write_atomic(path, wordinfo::scaffold_case(&diag).as_bytes())?;
        ui_info!("wordinfo-scaffold-saved", path = path.display().to_string());
    }

    if format == "json" {
        serde_json::to_writer_pretty(std::io::stdout().lock(), &diag)?;
        println!();
    } else {
        ui_info!(
            "wordinfo-summary",
            covered = diag.covered_labels,
            missing = diag.missing_labels.len()
        );
        use std::io::Write as _;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        for label in diag.missing_labels.iter().take(20) {
            let _ = writeln!(lock, "  {label}");
        }
        let rest = diag.missing_labels.len().saturating_sub(20);
        if rest > 0 {
            ui_info!("wordinfo-more", count = rest);
        }
    }
    Ok(())
}
