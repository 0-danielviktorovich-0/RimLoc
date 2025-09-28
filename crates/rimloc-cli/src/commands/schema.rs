use std::fs;

pub fn run_schema(out_dir: std::path::PathBuf) -> color_eyre::Result<()> {
    let cfg = rimloc_config::load_config().unwrap_or_default();
    let out_dir = if out_dir.as_os_str().is_empty() {
        std::path::PathBuf::from(
            cfg.schema
                .and_then(|s| s.out_dir)
                .unwrap_or_else(|| "./docs/assets/schemas".to_string()),
        )
    } else {
        out_dir
    };
    fs::create_dir_all(&out_dir)?;
    macro_rules! dump {
        ($ty:ty, $name:literal) => {{
            let schema = schemars::schema_for!($ty);
            let path = out_dir.join($name);
            let f = std::fs::File::create(&path)?;
            serde_json::to_writer_pretty(f, &schema)?;
        }};
    }
    dump!(rimloc_domain::ScanUnit, "scan_unit.schema.json");
    dump!(rimloc_domain::ValidationMsg, "validation_msg.schema.json");
    dump!(rimloc_domain::ImportSummary, "import_summary.schema.json");
    dump!(rimloc_domain::DiffOutput, "diff_output.schema.json");
    dump!(rimloc_domain::HealthReport, "health_report.schema.json");
    dump!(rimloc_domain::AnnotatePlan, "annotate_plan.schema.json");
    crate::ui_ok!("schema-dumped", path = out_dir.display().to_string());
    Ok(())
}
