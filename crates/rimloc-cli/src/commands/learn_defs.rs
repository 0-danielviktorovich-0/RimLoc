use crate::version::resolve_game_version_root;

#[allow(clippy::too_many_arguments)]
pub fn run_learn_defs(
    mod_root: std::path::PathBuf,
    dict: Vec<std::path::PathBuf>,
    model: Option<std::path::PathBuf>,
    ml_url: Option<String>,
    lang_dir: Option<String>,
    threshold: f32,
    out_dir: std::path::PathBuf,
    no_ml: bool,
    retrain: bool,
    min_len: Option<usize>,
    blacklist: Option<Vec<String>>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    let (scan_root, _) = resolve_game_version_root(&mod_root, game_version.as_deref())?;
    let opts = rimloc_services::learn::LearnOptions {
        mod_root: scan_root.clone(),
        defs_root: None,
        dict_files: dict,
        model_path: model,
        ml_url,
        lang_dir: lang_dir.unwrap_or_else(|| "English".to_string()),
        threshold,
        no_ml,
        retrain,
        min_len: min_len.unwrap_or(1),
        blacklist: blacklist.unwrap_or_default(),
        out_dir,
    };
    let res = rimloc_services::learn::learn_defs(&opts)?;
    crate::ui_out!(
        "learn-defs-summary",
        candidates = res.candidates.len(),
        accepted = res.accepted,
        missing = res.missing_path.display().to_string(),
        suggested = res.suggested_path.display().to_string()
    );
    Ok(())
}

