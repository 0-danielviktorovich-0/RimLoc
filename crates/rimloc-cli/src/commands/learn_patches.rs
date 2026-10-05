use crate::version::resolve_game_version_root;

pub fn run_learn_patches(
    mod_root: std::path::PathBuf,
    min_len: usize,
    out_json: Option<std::path::PathBuf>,
    strict_xpath: bool,
    emit_keys_only: bool,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    let (scan_root, _) = resolve_game_version_root(&mod_root, game_version.as_deref())?;
    if strict_xpath {
        std::env::set_var("RIMLOC_PATCH_STRICT_XPATH", "1");
    }
    let cands = rimloc_services::learn::patches::scan_patches_texts(&scan_root, min_len)?;
    let out_dir = scan_root.join("learn_out");
    let out = out_json.unwrap_or_else(|| out_dir.join("patches_texts.json"));
    // Canonical write guard (rust/path-injection chokepoint): the CLI
    // resolves a relative argument against the CWD EXPLICITLY, then the
    // guard proves the real (symlink-resolved) location and the write goes
    // to the returned path. No protected roots here BY DESIGN: `learn_out`
    // inside the scanned mod tree is this command's product.
    let out = rimloc_services::resolve_cli_out_path(&out)?;
    let out = rimloc_services::ensure_free_output_path(&out, &[])?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&out)?;
    if emit_keys_only {
        let keys: Vec<String> = cands
            .iter()
            .filter_map(|c| {
                c.inferred
                    .as_ref()
                    .map(|i| format!("{}.{}", i.def_name, i.field_path))
            })
            .collect();
        serde_json::to_writer_pretty(file, &keys)?;
    } else {
        serde_json::to_writer_pretty(file, &cands)?;
    }
    crate::ui_info!("scan-json-saved", path = out.display().to_string());

    // Also produce a suggested DefInjected XML using inferred keys
    use std::io::Write;
    let mut inferred: Vec<_> = cands
        .iter()
        .filter_map(|c| c.inferred.as_ref().map(|i| (i, &c.value)))
        .collect();
    inferred.sort_by(|a, b| {
        (
            a.0.def_type.as_str(),
            a.0.def_name.as_str(),
            a.0.field_path.as_str(),
        )
            .cmp(&(
                b.0.def_type.as_str(),
                b.0.def_name.as_str(),
                b.0.field_path.as_str(),
            ))
    });
    if !inferred.is_empty() {
        // Same guard as the JSON output above; the canonical `sug` parent
        // replaces the raw `out_dir` for mkdir, so staging happens next to
        // the real destination, not next to a symlinked spelling of it.
        let sug = rimloc_services::resolve_cli_out_path(&out_dir.join("_SuggestedFromPatches.xml"))?;
        let sug = rimloc_services::ensure_free_output_path(&sug, &[])?;
        let out_dir = sug
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or(out_dir);
        std::fs::create_dir_all(&out_dir)?;
        let mut f = std::fs::File::create(&sug)?;
        writeln!(f, "<LanguageData>")?;
        // We emit flat keys <DefName.path></DefName.path>
        // Group by def_type to help humans (as comments)
        let mut current_ty: Option<&str> = None;
        for (inf, val) in inferred {
            if current_ty
                .map(|t| t != inf.def_type.as_str())
                .unwrap_or(true)
            {
                current_ty = Some(&inf.def_type);
                writeln!(f, "  <!-- {} -->", inf.def_type)?;
            }
            let key = format!("{}.{}", inf.def_name, inf.field_path);
            let en = rimloc_services::learn::export::escape_xml_comment(val);
            writeln!(f, "  <!-- EN: {} -->", en)?;
            writeln!(f, "  <{}></{}>", key, key)?;
        }
        writeln!(f, "</LanguageData>")?;
        crate::ui_info!("scan-json-saved", path = sug.display().to_string());
    }
    Ok(())
}
