use crate::version::resolve_game_version_root;

pub fn run_learn_patches(
    mod_root: std::path::PathBuf,
    min_len: usize,
    out_json: Option<std::path::PathBuf>,
    game_version: Option<String>,
) -> color_eyre::Result<()> {
    let (scan_root, _) = resolve_game_version_root(&mod_root, game_version.as_deref())?;
    let cands = rimloc_services::learn::patches::scan_patches_texts(&scan_root, min_len)?;
    let out_dir = scan_root.join("learn_out");
    let out = out_json.unwrap_or_else(|| out_dir.join("patches_texts.json"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(&out)?;
    serde_json::to_writer_pretty(file, &cands)?;
    crate::ui_info!("scan-json-saved", path = out.display().to_string());

    // Also produce a suggested DefInjected XML using inferred keys
    use std::io::Write;
    let mut inferred: Vec<_> = cands
        .iter()
        .filter_map(|c| c.inferred.as_ref().map(|i| (i, &c.value)))
        .collect();
    inferred.sort_by(|a, b| {
        (a.0.def_type.as_str(), a.0.def_name.as_str(), a.0.field_path.as_str())
            .cmp(&(b.0.def_type.as_str(), b.0.def_name.as_str(), b.0.field_path.as_str()))
    });
    if !inferred.is_empty() {
        std::fs::create_dir_all(&out_dir)?;
        let sug = out_dir.join("_SuggestedFromPatches.xml");
        let mut f = std::fs::File::create(&sug)?;
        writeln!(f, "<LanguageData>")?;
        // We emit flat keys <DefName.path></DefName.path>
        // Group by def_type to help humans (as comments)
        let mut current_ty: Option<&str> = None;
        for (inf, val) in inferred {
            if current_ty.map(|t| t != inf.def_type.as_str()).unwrap_or(true) {
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
