use rimloc_services::extras::compare::{CompareInputSet, CompareReport};
use std::collections::BTreeMap;

pub struct CompareSetArg {
    pub label: String,
    pub lang_dir: String,
}

/// Compare one or more target translations against the mod's source strings.
/// Reuses the same scan + language filtering as `coverage`.
#[allow(clippy::too_many_arguments)]
pub fn run_compare(
    root: std::path::PathBuf,
    source_lang_dir: String,
    sets: Vec<CompareSetArg>,
    glossary: Option<std::path::PathBuf>,
    out_json: Option<std::path::PathBuf>,
    out_md: Option<std::path::PathBuf>,
    format: String,
) -> color_eyre::Result<()> {
    let cfg = rimloc_config::load_config().unwrap_or_default();
    let (scan_root, _ver) = resolve_version(root, cfg.game_version.clone())?;

    let auto = rimloc_services::autodiscover_defs_context(&scan_root)?;
    let units = rimloc_services::scan_units_with_defs_and_dict(
        &scan_root,
        None,
        &auto
            .dict
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect(),
        &auto.extra_fields,
    )?;

    let src_dir = rimloc_services::normalize_lang_dir(&source_lang_dir);
    let source: Vec<rimloc_core::TransUnit> = units
        .iter()
        .filter(|u| rimloc_services::is_source_for_lang_dir(&u.path, &src_dir))
        .cloned()
        .collect();

    // Each set either lives in the scanned root (filter by folder name) or is
    // a full path to a Languages/<dir> of ANOTHER mod root (e.g. a copy with a
    // machine translation applied) — in that case scan that root separately.
    let mut input_sets = Vec::with_capacity(sets.len());
    for set in &sets {
        let set_path = std::path::Path::new(&set.lang_dir);
        let (scan_for_set, tgt_dir) = if set.lang_dir.contains('/') && set_path.is_dir() {
            // Full path form: <mod_root>/Languages/<dir>. Find the ancestor
            // literally named `Languages` and treat its parent as the root.
            let dir = set_path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
                .to_string();
            let languages_dir = set_path
                .ancestors()
                .find(|a| a.file_name().and_then(|f| f.to_str()) == Some("Languages"));
            let root = languages_dir
                .and_then(|l| l.parent())
                .map(|r| r.to_path_buf())
                .unwrap_or_else(|| scan_root.clone());
            (root, dir)
        } else {
            (
                scan_root.clone(),
                rimloc_services::normalize_lang_dir(&set.lang_dir),
            )
        };
        let set_units = if scan_for_set == scan_root {
            units.clone()
        } else {
            rimloc_services::scan_units_with_defs_and_dict(
                &scan_for_set,
                None,
                &auto2_dict(&scan_for_set)?,
                &[],
            )?
        };
        let target: Vec<rimloc_core::TransUnit> = set_units
            .into_iter()
            .filter(|u| rimloc_services::is_under_languages_dir(&u.path, &tgt_dir))
            .collect();
        input_sets.push(CompareInputSet {
            label: set.label.clone(),
            units: target,
        });
    }

    let glossary_terms: Option<BTreeMap<String, String>> = match &glossary {
        Some(p) => Some(serde_json::from_str(&std::fs::read_to_string(p)?)?),
        None => None,
    };

    let report: CompareReport =
        rimloc_services::extras::compare::compare(&source, &input_sets, glossary_terms.as_ref());

    if let Some(path) = &out_json {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let f = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(f, &report)?;
        ui_info!("compare-json-saved", path = path.display().to_string());
    }
    if let Some(path) = &out_md {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        rimloc_services::write_atomic(
            path,
            rimloc_services::extras::compare::report_markdown(&report).as_bytes(),
        )?;
        ui_info!("compare-md-saved", path = path.display().to_string());
    }
    if format == "json" && out_json.is_none() {
        serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
        println!();
    } else if format != "json" {
        println!(
            "{}",
            rimloc_services::extras::compare::report_markdown(&report)
        );
    }
    Ok(())
}

fn auto2_dict(
    root: &std::path::Path,
) -> color_eyre::Result<std::collections::HashMap<String, Vec<String>>> {
    let auto = rimloc_services::autodiscover_defs_context(root)?;
    Ok(auto
        .dict
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect())
}

fn resolve_version(
    root: std::path::PathBuf,
    game_version: Option<String>,
) -> color_eyre::Result<(std::path::PathBuf, Option<String>)> {
    crate::version::resolve_game_version_root(&root, game_version.as_deref())
}
