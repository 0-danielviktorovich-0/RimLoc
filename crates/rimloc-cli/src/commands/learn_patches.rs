use crate::version::resolve_game_version_root;

pub fn run_learn_patches(
    mod_root: std::path::PathBuf,
    min_len: usize,
    out_json: Option<std::path::PathBuf>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    let (scan_root, _) = resolve_game_version_root(&mod_root, game_version.as_deref())?;
    let cands = rimloc_services::learn::patches::scan_patches_texts(&scan_root, min_len)?;
    let out = out_json.unwrap_or_else(|| scan_root.join("learn_out").join("patches_texts.json"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&out)?;
    serde_json::to_writer_pretty(file, &cands)?;
    crate::ui_info!("scan-json-saved", path = out.display().to_string());
    Ok(())
}

