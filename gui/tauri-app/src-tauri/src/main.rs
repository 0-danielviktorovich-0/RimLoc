#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console on Windows in release

use rimloc_domain::{DiffOutput, HealthReport, ScanUnit};
use serde::Serialize;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug, Serialize)]
#[error("{message}")]
pub struct ApiError { pub message: String }

impl From<color_eyre::Report> for ApiError {
  fn from(e: color_eyre::Report) -> Self { ApiError { message: format!("{e}") } }
}

#[tauri::command]
fn api_scan(root: String) -> Result<Vec<ScanUnit>, ApiError> {
  let units = rimloc_services::scan_units(PathBuf::from(root).as_path())?;
  Ok(units)
}

#[tauri::command]
fn api_validate(root: String, source_lang: Option<String>, source_lang_dir: Option<String>) -> Result<Vec<rimloc_services::ValidationMessage>, ApiError> {
  let msgs = rimloc_services::validate_under_root(
    PathBuf::from(root).as_path(),
    source_lang.as_deref(),
    source_lang_dir.as_deref(),
  )?;
  Ok(msgs)
}

#[tauri::command]
fn api_export_po(root: String, out_po: String, lang: Option<String>, source_lang: Option<String>, source_lang_dir: Option<String>, tm_roots: Option<Vec<String>>) -> Result<(), ApiError> {
  let tm_paths: Option<Vec<PathBuf>> = tm_roots.map(|v| v.into_iter().map(PathBuf::from).collect());
  rimloc_services::export_po_with_tm(
    PathBuf::from(&root).as_path(),
    PathBuf::from(&out_po).as_path(),
    lang.as_deref(),
    source_lang.as_deref(),
    source_lang_dir.as_deref(),
    tm_paths.as_ref().map(|v| v.as_slice()),
  )?;
  Ok(())
}

#[tauri::command]
fn api_import_po_dry(po: String, mod_root: String, lang: Option<String>, lang_dir: Option<String>, keep_empty: bool, single_file: bool, game_version: Option<String>, only_diff: bool) -> Result<rimloc_services::ImportPlan, ApiError> {
  let (_plan, summary) = rimloc_services::import_po_to_mod_tree(
    PathBuf::from(po).as_path(),
    PathBuf::from(mod_root).as_path(),
    &lang_dir.unwrap_or_else(|| lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)).unwrap_or_else(|| "Russian".to_string())),
    keep_empty,
    true,
    false,
    single_file,
    true,
    only_diff,
    false,
  )?;
  if let Some(plan) = _plan { Ok(plan) } else { Err(ApiError { message: "no plan generated".into() }) }
}

#[tauri::command]
fn api_build_mod_dry(po: Option<String>, out_mod: String, lang: String, from_root: Option<String>, from_game_version: Option<Vec<String>>, name: Option<String>, package_id: Option<String>, rw_version: Option<String>, lang_dir: Option<String>, dedupe: bool) -> Result<rimloc_services::BuildPlan, ApiError> {
  if let Some(root) = from_root {
    // We don't have a dry-run planner for from_root; reuse import dry-run style by scanning files
    let (files, total_keys) = rimloc_services::build_from_root(
      PathBuf::from(root).as_path(),
      PathBuf::from(&out_mod).as_path(),
      &lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang)),
      from_game_version.as_deref(),
      false,
      dedupe,
    )?;
    return Ok(rimloc_services::BuildPlan { out_mod: PathBuf::from(out_mod), files, total_keys, mod_name: name.unwrap_or_else(|| "RimLoc Translation".into()), package_id: package_id.unwrap_or_else(|| "yourname.rimloc.translation".into()), rw_version: rw_version.unwrap_or_else(|| "1.5".into()), lang_dir: lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang)) });
  }
  let po = po.ok_or_else(|| ApiError { message: "po is required when from_root is not set".into() })?;
  let plan = rimloc_services::build_from_po_dry_run(
    PathBuf::from(&po).as_path(),
    PathBuf::from(&out_mod).as_path(),
    &lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang)),
    &name.unwrap_or_else(|| "RimLoc Translation".into()),
    &package_id.unwrap_or_else(|| "yourname.rimloc.translation".into()),
    &rw_version.unwrap_or_else(|| "1.5".into()),
    dedupe,
  )?;
  Ok(plan)
}

#[tauri::command]
fn api_diff_xml(root: String, source_lang: Option<String>, source_lang_dir: Option<String>, lang: Option<String>, lang_dir: Option<String>, baseline_po: Option<String>) -> Result<DiffOutput, ApiError> {
  let cfg_src = source_lang_dir.or_else(|| source_lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "English".to_string());
  let cfg_trg = lang_dir.or_else(|| lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "Russian".to_string());
  let diff = rimloc_services::diff_xml(
    PathBuf::from(&root).as_path(),
    &cfg_src,
    &cfg_trg,
    baseline_po.as_deref().map(PathBuf::from).as_deref(),
  )?;
  Ok(diff)
}

#[tauri::command]
fn api_diff_save_reports(root: String, source_lang: Option<String>, source_lang_dir: Option<String>, lang: Option<String>, lang_dir: Option<String>, baseline_po: Option<String>, out_dir: String) -> Result<String, ApiError> {
  let cfg_src = source_lang_dir.or_else(|| source_lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "English".to_string());
  let cfg_trg = lang_dir.or_else(|| lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "Russian".to_string());
  let diff = rimloc_services::diff_xml(
    PathBuf::from(&root).as_path(),
    &cfg_src,
    &cfg_trg,
    baseline_po.as_deref().map(PathBuf::from).as_deref(),
  )?;
  let out = PathBuf::from(&out_dir);
  std::fs::create_dir_all(&out).map_err(|e| ApiError { message: e.to_string() })?;
  rimloc_services::write_diff_reports(&out, &diff)?;
  Ok(out.display().to_string())
}

#[tauri::command]
fn api_xml_health(root: String, lang_dir: Option<String>) -> Result<HealthReport, ApiError> {
  let rep = rimloc_services::xml_health_scan(PathBuf::from(root).as_path(), lang_dir.as_deref(), None, None)?;
  Ok(rep)
}

#[derive(Serialize)]
struct LangUpdateDryLine { path: String, size: u64 }
#[derive(Serialize)]
struct LangUpdateDry { files: Vec<LangUpdateDryLine>, total: u64, out: String }

#[tauri::command]
fn api_lang_update_dry(game_root: String, repo: Option<String>, branch: Option<String>, zip: Option<String>, source_lang_dir: Option<String>, target_lang_dir: Option<String>) -> Result<LangUpdateDry, ApiError> {
  let repo = repo.unwrap_or_else(|| "Ludeon/RimWorld-ru".into());
  let src = source_lang_dir.unwrap_or_else(|| "Russian".into());
  let trg = target_lang_dir.unwrap_or_else(|| "Russian (GitHub)".into());
  let (plan, _summary) = rimloc_services::lang_update(
    PathBuf::from(&game_root).as_path(),
    &repo,
    branch.as_deref(),
    zip.as_deref().map(PathBuf::from).as_deref(),
    &src,
    &trg,
    true,
    false,
  )?;
  let p = plan.ok_or_else(|| ApiError { message: "no plan".into() })?;
  Ok(LangUpdateDry { files: p.files.into_iter().map(|f| LangUpdateDryLine { path: format!("{}/{}", p.out_languages_dir.join(&p.target_lang_dir).display(), f.rel_path), size: f.size }).collect(), total: p.total_bytes, out: p.out_languages_dir.join(&p.target_lang_dir).display().to_string() })
}

// Apply actions
#[tauri::command]
fn api_import_po_apply(po: String, mod_root: String, lang: Option<String>, lang_dir: Option<String>, keep_empty: bool, single_file: bool, incremental: bool, only_diff: bool, report: bool, backup: bool) -> Result<rimloc_services::ImportSummary, ApiError> {
  let (_plan, summary) = rimloc_services::import_po_to_mod_tree(
    PathBuf::from(po).as_path(),
    PathBuf::from(mod_root).as_path(),
    &lang_dir.unwrap_or_else(|| lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)).unwrap_or_else(|| "Russian".to_string())),
    keep_empty,
    false,
    backup,
    single_file,
    incremental,
    only_diff,
    report,
  )?;
  summary.ok_or_else(|| ApiError { message: "no summary".into() })
}

#[tauri::command]
fn api_build_mod_apply(po: Option<String>, out_mod: String, lang: String, from_root: Option<String>, from_game_version: Option<Vec<String>>, name: Option<String>, package_id: Option<String>, rw_version: Option<String>, lang_dir: Option<String>, dedupe: bool) -> Result<String, ApiError> {
  if let Some(root) = from_root {
    let _ = rimloc_services::build_from_root(
      PathBuf::from(root).as_path(),
      PathBuf::from(&out_mod).as_path(),
      &lang_dir.clone().unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang)),
      from_game_version.as_deref(),
      true,
      dedupe,
    )?;
    return Ok(out_mod);
  }
  let po = po.ok_or_else(|| ApiError { message: "po is required when from_root is not set".into() })?;
  rimloc_services::build_from_po_execute(
    PathBuf::from(&po).as_path(),
    PathBuf::from(&out_mod).as_path(),
    &lang_dir.unwrap_or_else(|| rimloc_import_po::rimworld_lang_dir(&lang)),
    &name.unwrap_or_else(|| "RimLoc Translation".into()),
    &package_id.unwrap_or_else(|| "yourname.rimloc.translation".into()),
    &rw_version.unwrap_or_else(|| "1.5".into()),
    dedupe,
  )?;
  Ok(out_mod)
}

#[tauri::command]
fn api_lang_update_apply(game_root: String, repo: Option<String>, branch: Option<String>, zip: Option<String>, source_lang_dir: Option<String>, target_lang_dir: Option<String>, backup: bool) -> Result<String, ApiError> {
  let repo = repo.unwrap_or_else(|| "Ludeon/RimWorld-ru".into());
  let src = source_lang_dir.unwrap_or_else(|| "Russian".into());
  let trg = target_lang_dir.unwrap_or_else(|| "Russian (GitHub)".into());
  let (_plan, summary) = rimloc_services::lang_update(
    PathBuf::from(&game_root).as_path(),
    &repo,
    branch.as_deref(),
    zip.as_deref().map(PathBuf::from).as_deref(),
    &src,
    &trg,
    false,
    backup,
  )?;
  let sum = summary.ok_or_else(|| ApiError { message: "no summary".into() })?;
  Ok(sum.out_dir.display().to_string())
}

// Annotate
#[tauri::command]
fn api_annotate_dry(root: String, source_lang: Option<String>, source_lang_dir: Option<String>, lang: Option<String>, lang_dir: Option<String>, comment_prefix: Option<String>, strip: bool) -> Result<rimloc_services::AnnotatePlan, ApiError> {
  let cfg_src = source_lang_dir.or_else(|| source_lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "English".to_string());
  let cfg_trg = lang_dir.or_else(|| lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "Russian".to_string());
  let prefix = comment_prefix.unwrap_or_else(|| "EN:".into());
  let plan = rimloc_services::annotate_dry_run_plan(PathBuf::from(root).as_path(), &cfg_src, &cfg_trg, &prefix, strip)?;
  Ok(plan)
}

#[tauri::command]
fn api_annotate_apply(root: String, source_lang: Option<String>, source_lang_dir: Option<String>, lang: Option<String>, lang_dir: Option<String>, comment_prefix: Option<String>, strip: bool, backup: bool) -> Result<rimloc_services::AnnotateSummary, ApiError> {
  let cfg_src = source_lang_dir.or_else(|| source_lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "English".to_string());
  let cfg_trg = lang_dir.or_else(|| lang.map(|c| rimloc_import_po::rimworld_lang_dir(&c)) ).unwrap_or_else(|| "Russian".to_string());
  let prefix = comment_prefix.unwrap_or_else(|| "EN:".into());
  let sum = rimloc_services::annotate_apply(PathBuf::from(root).as_path(), &cfg_src, &cfg_trg, &prefix, strip, false, backup)?;
  Ok(sum)
}

// Schema dump
#[tauri::command]
fn api_schema_dump(out_dir: String) -> Result<String, ApiError> {
  let out_dir = PathBuf::from(out_dir);
  std::fs::create_dir_all(&out_dir).map_err(|e| ApiError { message: e.to_string() })?;
  macro_rules! dump {
    ($ty:ty, $name:literal) => {{
      let schema = schemars::schema_for!($ty);
      let path = out_dir.join($name);
      let f = std::fs::File::create(&path).map_err(|e| ApiError { message: e.to_string() })?;
      serde_json::to_writer_pretty(f, &schema).map_err(|e| ApiError { message: e.to_string() })?;
    }};
  }
  dump!(rimloc_domain::ScanUnit, "scan_unit.schema.json");
  dump!(rimloc_domain::ValidationMsg, "validation_msg.schema.json");
  dump!(rimloc_domain::ImportSummary, "import_summary.schema.json");
  dump!(rimloc_domain::DiffOutput, "diff_output.schema.json");
  dump!(rimloc_domain::HealthReport, "health_report.schema.json");
  dump!(rimloc_domain::AnnotatePlan, "annotate_plan.schema.json");
  Ok(out_dir.display().to_string())
}

#[tauri::command]
fn api_save_text(path: String, contents: String) -> Result<String, ApiError> {
  std::fs::write(&path, contents).map_err(|e| ApiError { message: e.to_string() })?;
  Ok(path)
}

#[tauri::command]
fn api_app_version() -> Result<String, ApiError> { Ok(env!("CARGO_PKG_VERSION").to_string()) }

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn version_nonempty() {
    let v = api_app_version().unwrap();
    assert!(!v.is_empty());
  }
}

// Morph
#[tauri::command]
fn api_morph(root: String, provider: Option<String>, lang: Option<String>, lang_dir: Option<String>, filter_key_regex: Option<String>, limit: Option<usize>, game_version: Option<String>, timeout_ms: Option<u64>, cache_size: Option<usize>, pymorphy_url: Option<String>) -> Result<rimloc_services::MorphResult, ApiError> {
  let opts = rimloc_services::MorphOptions {
    root: PathBuf::from(root),
    provider,
    lang,
    lang_dir,
    filter_key_regex,
    limit,
    game_version,
    timeout_ms,
    cache_size,
    pymorphy_url,
  };
  let res = rimloc_services::morph_generate(opts)?;
  Ok(res)
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      api_scan,
      api_validate,
      api_export_po,
      api_import_po_dry,
      api_build_mod_dry,
      api_diff_xml,
      api_diff_save_reports,
      api_xml_health,
      api_lang_update_dry,
      api_save_text,
      api_app_version,
      api_import_po_apply,
      api_build_mod_apply,
      api_lang_update_apply,
      api_annotate_dry,
      api_annotate_apply,
      api_schema_dump,
      api_morph,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
