#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn run_build_mod(
    po: std::path::PathBuf,
    out_mod: std::path::PathBuf,
    lang: String,
    name: String,
    package_id: String,
    rw_version: String,
    lang_dir: Option<String>,
    dry_run: bool,
) -> color_eyre::Result<()> {
    tracing::debug!(event = "build_mod_args", po = ?po, out_mod = ?out_mod, lang = %lang, name = %name, package_id = %package_id, rw_version = %rw_version, lang_dir = ?lang_dir, dry_run = dry_run);
    let lang_folder = lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang));

    if dry_run {
        let plan = rimloc_import_po::build_translation_mod_dry_run(
            &po,
            &out_mod,
            &lang_folder,
            &name,
            &package_id,
            &rw_version,
        )?;
        ui_out!("build-dry-run-header");
        ui_out!("build-name", value = plan.mod_name);
        ui_out!("build-package-id", value = plan.package_id);
        ui_out!("build-rw-version", value = plan.rw_version);
        ui_out!(
            "build-mod-folder",
            value = plan.out_mod.display().to_string()
        );
        ui_out!("build-language", value = plan.lang_dir);
        ui_out!("build-divider");
        for (path, n) in plan.files {
            ui_out!(
                "import-dry-run-line",
                path = path.display().to_string(),
                n = n
            );
        }
        ui_out!("build-divider");
        ui_out!("build-summary", n = plan.total_keys);
    } else {
        rimloc_import_po::build_translation_mod_with_langdir(
            &po,
            &out_mod,
            &lang_folder,
            &name,
            &package_id,
            &rw_version,
        )?;
        ui_ok!("build-done", out = out_mod.display().to_string());
    }
    Ok(())
}
