#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console on Windows in release

use chrono::Utc;
use rimloc_gui_lib::contract_adapter;

// Enforcement + smoke tests (dead since 7fc196f restored as compile-wired).
#[cfg(test)]
mod tests;
use color_eyre::eyre::WrapErr;
use rimloc_domain::{ScanUnit, SCHEMA_VERSION};
use rimloc_export_csv as export_csv;
use rimloc_export_xliff::write_xliff_12 as svc_export_xliff;
use rimloc_import_xliff::xliff_to_language_data as svc_import_xliff;
use rimloc_services::keyed_merge::merge_keyed as svc_merge_keyed;
use rimloc_services::plugins as svc_plugins;
use rimloc_services::validate::coverage_report as svc_coverage;
use rimloc_services::{
    annotate_apply, annotate_dry_run_plan, build_from_po_with_progress, diff_xml,
    diff_xml_with_defs, import_po_to_mod_tree, import_po_to_mod_tree_with_progress, lang_update,
    make_init_plan, validate_placeholders_cross_language, validate_under_root,
    validate_under_root_with_defs, validate_under_root_with_defs_and_fields, write_init_plan,
    xml_health_scan,
};
use rimloc_services::{autodiscover_defs_context, learn};
use rimloc_services::{MorphOptions, MorphProvider};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tauri::{Manager, State, Window};
use tauri_plugin_dialog::DialogExt;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error, Serialize)]
#[error("{message}")]
pub struct ApiError {
    pub message: String,
}

impl From<color_eyre::Report> for ApiError {
    fn from(err: color_eyre::Report) -> Self {
        ApiError {
            message: format!("{err}"),
        }
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError {
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError {
            message: err.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScanRequest {
    root: String,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    include_all_versions: bool,
    #[serde(default)]
    out_json: Option<String>,
    #[serde(default)]
    out_csv: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    source_lang: Option<String>,
    #[serde(default)]
    source_lang_dir: Option<String>,
    #[serde(default)]
    defs_root: Option<String>,
    #[serde(default)]
    extra_fields: Option<Vec<String>>,
    #[serde(default)]
    defs_dicts: Option<Vec<String>>,
    #[serde(default)]
    type_schema: Option<String>,
    #[serde(default)]
    keyed_nested: bool,
    #[serde(default)]
    no_inherit: bool,
    #[serde(default)]
    with_plugins: bool,
    #[serde(default)]
    with_patches: bool,
    #[serde(default)]
    patch_min_len: Option<usize>,
    #[serde(default)]
    patch_strict_xpath: bool,
    #[serde(default)]
    parallel: bool,
    #[serde(default)]
    fuzzy: bool,
    #[serde(default)]
    use_en_comments: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScanUnitView {
    key: String,
    source: String,
    path: String,
    line: Option<usize>,
    kind: ScanKind,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResponse {
    root: String,
    resolved_root: String,
    game_version: Option<String>,
    total: usize,
    keyed: usize,
    def_injected: usize,
    saved_json: Option<String>,
    saved_csv: Option<String>,
    units: Vec<ScanUnitView>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LearnDefsResponse {
    resolved_root: String,
    game_version: Option<String>,
    out_dir: String,
    missing_path: String,
    suggested_path: String,
    learned_path: String,
    candidates: usize,
    accepted: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportPoResponse {
    resolved_root: String,
    game_version: Option<String>,
    out_po: String,
    total: usize,
    tm_filled: usize,
    tm_coverage_pct: u32,
    warning: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum ScanKind {
    Keyed,
    DefInjected,
    Other,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LogEvent {
    level: String,
    message: String,
}

#[derive(Debug)]
struct LogState {
    path: PathBuf,
}

fn append_log(path: &Path, level: &str, message: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    rotate_if_needed(path);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let ts = Utc::now().to_rfc3339();
        let tid = format!("{:?}", std::thread::current().id());
        let _ = writeln!(
            f,
            "[{}][{}] {}: {}",
            ts,
            tid,
            level.to_uppercase(),
            message.replace('\n', " ")
        );
    }
}

fn rotate_if_needed(path: &Path) {
    if let Ok(meta) = std::fs::metadata(path) {
        let max = 5 * 1024 * 1024; // 5 MB
        if meta.len() > max {
            if let Some(dir) = path.parent() {
                let rotated = dir.join(format!("gui-{}.log", Utc::now().format("%Y%m%d-%H%M%S")));
                let _ = std::fs::rename(path, rotated);
            }
        }
    }
}

fn emit_log(window: &Window, state: &State<LogState>, level: &str, message: impl Into<String>) {
    let msg: String = message.into();
    let _ = window.emit(
        "log",
        LogEvent {
            level: level.to_string(),
            message: msg.clone(),
        },
    );
    append_log(&state.path, level, &msg);
}

#[tauri::command]
fn log_message(
    window: Window,
    state: State<LogState>,
    level: String,
    message: String,
) -> Result<(), ApiError> {
    // Accept logs from the frontend and persist alongside backend logs
    emit_log(&window, &state, &level, message);
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    action: String,
    step: String,
    message: Option<String>,
    pct: Option<u32>,
}

fn emit_progress(
    window: &Window,
    state: &State<LogState>,
    action: &str,
    step: &str,
    message: Option<String>,
    pct: Option<u32>,
) {
    let _ = window.emit(
        "progress",
        ProgressEvent {
            action: action.to_string(),
            step: step.to_string(),
            message: message.clone(),
            pct,
        },
    );
    if let Some(msg) = message {
        append_log(
            &state.path,
            "DEBUG",
            &format!("[{}] {} {}%", action, step, pct.unwrap_or(0)),
        );
        append_log(&state.path, "DEBUG", &msg);
    }
}

// --- Plugins ---
#[derive(Debug, Deserialize)]
struct LoadPluginRequest {
    path: String,
}

#[derive(Debug, Serialize)]
struct LoadPluginResponse {
    id: String,
}

/// Paths from which dynamic scan plugins may be loaded from the UI.
/// Plugin loading executes native code, so it is opt-in: the absolute library
/// path must be listed in ~/.rimloc/plugins-allow.json as {"allow": ["…"]}.
fn plugin_allowlist() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let cfg_path = home.join(".rimloc").join("plugins-allow.json");
    let Ok(raw) = std::fs::read_to_string(cfg_path) else {
        return Vec::new();
    };
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("allow").and_then(|a| a.as_array()).cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
fn load_plugin_cmd(
    _window: Window,
    _state: State<LogState>,
    request: LoadPluginRequest,
) -> Result<LoadPluginResponse, ApiError> {
    let p = PathBuf::from(&request.path);
    let allowed_ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e, "dylib" | "so" | "dll"))
        .unwrap_or(false);
    if !allowed_ext {
        return Err(ApiError {
            message: "Dynamic plugins must be .dylib/.so/.dll".into(),
        });
    }
    let canon = p.canonicalize().map_err(|e| ApiError {
        message: format!("Plugin path: {e}"),
    })?;
    if !plugin_allowlist().contains(&canon) {
        return Err(ApiError {
            message: format!(
                "Plugin loading is disabled for this path. Native plugins execute code, so \
                 each one must be allowlisted in ~/.rimloc/plugins-allow.json \
                 ({{\"allow\": [\"{}\"]}}) and RimLoc restarted.",
                canon.display()
            ),
        });
    }
    svc_plugins::load_dynamic_plugin(&p).map_err(|e| ApiError {
        message: format!("{e}"),
    })?;
    // Return filename as id; registry stores Arc<dyn ParserPlugin> without direct IDs for dyn plugins
    let id = p
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("dyn")
        .to_string();
    Ok(LoadPluginResponse { id })
}

#[derive(Debug, Serialize)]
struct ListPluginsResponse {
    ids: Vec<String>,
}

#[tauri::command]
fn list_plugins_cmd(
    _window: Window,
    _state: State<LogState>,
) -> Result<ListPluginsResponse, ApiError> {
    let ids: Vec<String> = svc_plugins::iter()
        .into_iter()
        .map(|p| p.id().to_string())
        .collect();
    Ok(ListPluginsResponse { ids })
}

fn write_profile(
    state: &State<LogState>,
    command: &str,
    start: std::time::Instant,
    extra: serde_json::Value,
) {
    let duration_ms = start.elapsed().as_millis() as u64;
    let entry = serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "command": command,
        "duration_ms": duration_ms,
        "extra": extra,
    });
    if let Some(dir) = state.path.parent() {
        let file = dir.join("profile.jsonl");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(file) {
            let _ = writeln!(f, "{}", entry);
        }
    }
}

#[derive(Debug, Deserialize)]
struct MergeKeyedRequest {
    root: String,
    #[serde(default)]
    source_lang_dir: String,
    target_lang_dir: String,
    #[serde(default)]
    out_dir: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MergeKeyedResponse {
    files: usize,
    keys_total: usize,
    reused: usize,
    unused: usize,
    out_hint: String,
}

#[tauri::command]
fn merge_keyed_gui(
    _window: Window,
    _state: State<LogState>,
    request: MergeKeyedRequest,
) -> Result<MergeKeyedResponse, ApiError> {
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        return Err(ApiError {
            message: format!("Path not found: {}", root.display()),
        });
    }
    let out_dir = request.out_dir.as_deref().map(PathBuf::from);
    if let Some(o) = &out_dir {
        ensure_caller_path_absolute("out_dir", o)?;
    }
    let stats = svc_merge_keyed(
        &root,
        &request.source_lang_dir,
        &request.target_lang_dir,
        out_dir.as_deref(),
    )
    .map_err(|e| ApiError {
        message: format!("{e}"),
    })?;
    let out_hint = out_dir.map(|p| p.display().to_string()).unwrap_or_else(|| {
        root.join("Languages")
            .join(&request.target_lang_dir)
            .join("Keyed")
            .display()
            .to_string()
    });
    Ok(MergeKeyedResponse {
        files: stats.files,
        keys_total: stats.keys_total,
        reused: stats.reused,
        unused: stats.unused,
        out_hint,
    })
}

#[derive(Debug, Deserialize)]
struct CoverageRequest {
    root: String,
    source_lang_dir: String,
    target_lang_dir: String,
    #[serde(default)]
    defs_root: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CoverageResponse {
    source_total: usize,
    target_total: usize,
    translated: usize,
    missing: usize,
}

#[tauri::command]
fn coverage_gui(
    _window: Window,
    _state: State<LogState>,
    request: CoverageRequest,
) -> Result<CoverageResponse, ApiError> {
    let root = PathBuf::from(&request.root);
    let defs = request.defs_root.as_deref().map(PathBuf::from);
    let rep = svc_coverage(
        &root,
        &request.source_lang_dir,
        &request.target_lang_dir,
        defs.as_deref(),
    )
    .map_err(|e| ApiError {
        message: format!("{e}"),
    })?;
    Ok(CoverageResponse {
        source_total: rep.source_total,
        target_total: rep.target_total,
        translated: rep.translated,
        missing: rep.missing,
    })
}

#[derive(Debug, Deserialize)]
struct ExportXliffRequest {
    root: String,
    out_xlf: String,
    #[serde(default)]
    source_lang_dir: Option<String>,
    #[serde(default)]
    lang: Option<String>,
}

#[tauri::command]
fn export_xliff_gui(
    _window: Window,
    _state: State<LogState>,
    request: ExportXliffRequest,
) -> Result<String, ApiError> {
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        return Err(ApiError {
            message: format!("Path not found: {}", root.display()),
        });
    }
    let mut units = rimloc_services::scan::scan_units(&root).map_err(|e| ApiError {
        message: format!("{e}"),
    })?;
    let src = request.source_lang_dir.as_deref().unwrap_or("English");
    units.retain(|u| rimloc_services::is_under_languages_dir(&u.path, src));
    let out = PathBuf::from(&request.out_xlf);
    ensure_caller_path_absolute("out_xlf", &out)?;
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let lang = request.lang.as_deref().unwrap_or("ru");
    svc_export_xliff(&out, &units, "en", lang).map_err(|e| ApiError {
        message: format!("{e}"),
    })?;
    Ok(out.display().to_string())
}

#[derive(Debug, Deserialize)]
struct ImportXliffRequest {
    xlf: String,
    out_xml: String,
}

#[tauri::command]
fn import_xliff_gui(
    _window: Window,
    _state: State<LogState>,
    request: ImportXliffRequest,
) -> Result<String, ApiError> {
    let xlf = PathBuf::from(&request.xlf);
    let out = PathBuf::from(&request.out_xml);
    ensure_caller_path_absolute("out_xml", &out)?;
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    svc_import_xliff(&out, &xlf).map_err(|e| ApiError {
        message: format!("{e}"),
    })?;
    Ok(out.display().to_string())
}

#[derive(Debug, Deserialize)]
struct LearnDefsRequest {
    root: String,
    #[serde(default)]
    out_dir: Option<String>,
    #[serde(default)]
    lang_dir: Option<String>,
    #[serde(default)]
    threshold: Option<f32>,
    #[serde(default)]
    game_version: Option<String>,
    // Advanced options
    #[serde(default)]
    defs_root: Option<String>,
    #[serde(default)]
    dict_files: Option<Vec<String>>,
    #[serde(default)]
    model_path: Option<String>,
    #[serde(default)]
    ml_url: Option<String>,
    #[serde(default)]
    no_ml: bool,
    #[serde(default)]
    retrain: bool,
    #[serde(default)]
    learned_out: Option<String>,
    #[serde(default)]
    retrain_dict: Option<String>,
    #[serde(default)]
    min_len: Option<usize>,
    #[serde(default)]
    blacklist: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ExportPoRequest {
    root: String,
    out_po: String,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    pot: bool,
    #[serde(default)]
    source_lang: Option<String>,
    #[serde(default)]
    source_lang_dir: Option<String>,
    #[serde(default)]
    tm_roots: Option<Vec<String>>,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    include_all_versions: bool,
    // New advanced options (mirror scan/validate)
    #[serde(default)]
    defs_root: Option<String>,
    #[serde(default)]
    extra_fields: Option<Vec<String>>,
    #[serde(default)]
    defs_dicts: Option<Vec<String>>,
    #[serde(default)]
    defs_type_schema: Option<String>,
    #[serde(default)]
    type_schema: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ValidateRequest {
    root: String,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    source_lang: Option<String>,
    #[serde(default)]
    source_lang_dir: Option<String>,
    #[serde(default)]
    defs_root: Option<String>,
    #[serde(default)]
    extra_fields: Option<Vec<String>>,
    #[serde(default)]
    out_json: Option<String>,
    #[serde(default)]
    include_all_versions: bool,
    #[serde(default)]
    compare_placeholders: bool,
    #[serde(default)]
    compare_lists: bool,
    #[serde(default)]
    report_orphans: bool,
    #[serde(default)]
    target_lang: Option<String>,
    #[serde(default)]
    target_lang_dir: Option<String>,
    #[serde(default)]
    defs_dicts: Option<Vec<String>>,
    #[serde(default)]
    defs_type_schema: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidateResponse {
    resolved_root: String,
    game_version: Option<String>,
    total: usize,
    errors: usize,
    warnings: usize,
    infos: usize,
    messages: Vec<ValidationMessageView>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ValidationMessageView {
    kind: String,
    key: String,
    path: String,
    line: Option<usize>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct XmlHealthRequest {
    root: String,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    lang_dir: Option<String>,
    #[serde(default)]
    out_json: Option<String>,
    #[serde(default)]
    strict: bool,
    #[serde(default)]
    only: Option<Vec<String>>,
    #[serde(default)]
    except: Option<Vec<String>>,
}

// --- Strings inventory ---
#[derive(Debug, Deserialize)]
struct ScanStringsRequest {
    root: String,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    include_all_versions: bool,
    #[serde(default)]
    lang_dir: Option<String>,
    #[serde(default)]
    out_json: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StringLineView {
    path: String,
    line: usize,
    text: String,
    lang_dir: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanStringsResponse {
    resolved_root: String,
    game_version: Option<String>,
    total: usize,
    items: Vec<StringLineView>,
    saved_json: Option<String>,
}

#[tauri::command]
fn scan_strings_gui(
    _window: Window,
    _state: State<LogState>,
    request: ScanStringsRequest,
) -> Result<ScanStringsResponse, ApiError> {
    use walkdir::WalkDir;
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = if request.include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, request.game_version.as_deref())?
    };
    let mut items: Vec<StringLineView> = Vec::new();
    for entry in WalkDir::new(&scan_root).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        let is_txt = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("txt"))
            .unwrap_or(false);
        if !is_txt {
            continue;
        }
        let s = p.to_string_lossy();
        if !(s.contains("/Languages/") || s.contains("\\Languages\\")) {
            continue;
        }
        if !(s.contains("/Strings/") || s.contains("\\Strings\\")) {
            continue;
        }
        if let Some(dir) = request.lang_dir.as_deref() {
            if !(s.contains(&format!("/Languages/{dir}/"))
                || s.contains(&format!("\\Languages\\{}\\", dir)))
            {
                continue;
            }
        }
        let content = match std::fs::read_to_string(p) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for (idx, line) in content.lines().enumerate() {
            let line_no = idx + 1;
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            let snippet = if t.len() > 200 {
                format!("{}…", &t[..200])
            } else {
                t.to_string()
            };
            items.push(StringLineView {
                path: p.display().to_string(),
                line: line_no,
                text: snippet,
                lang_dir: request.lang_dir.clone(),
            });
        }
    }
    items.sort_by_key(|a| (a.path.clone(), a.line));
    let saved_json = if let Some(path) = request.out_json.as_ref() {
        let path = make_absolute(&scan_root, Path::new(path));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = File::create(&path)?;
        serde_json::to_writer_pretty(file, &items)?;
        Some(path.display().to_string())
    } else {
        None
    };
    Ok(ScanStringsResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        total: items.len(),
        items,
        saved_json,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct XmlHealthResponse {
    resolved_root: String,
    game_version: Option<String>,
    checked: usize,
    issues: Vec<rimloc_services::HealthIssue>,
}

#[derive(Debug, Deserialize)]
struct ImportPoRequest {
    root: String,
    po_path: String,
    #[serde(default)]
    out_xml: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    lang_dir: Option<String>,
    #[serde(default)]
    keep_empty: bool,
    #[serde(default)]
    backup: bool,
    #[serde(default)]
    single_file: bool,
    #[serde(default)]
    incremental: bool,
    #[serde(default)]
    only_diff: bool,
    #[serde(default)]
    report: bool,
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportPoResponse {
    resolved_root: String,
    game_version: Option<String>,
    lang_dir: String,
    created: usize,
    updated: usize,
    skipped: usize,
    keys: usize,
}

#[derive(Debug, Deserialize)]
struct BuildModRequest {
    po_path: String,
    out_mod: String,
    lang_dir: String,
    name: String,
    package_id: String,
    rw_version: String,
    #[serde(default)]
    dedupe: bool,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    from_root: Option<String>,
    #[serde(default)]
    from_game_versions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildModResponse {
    out_mod: String,
    files: usize,
    total_keys: usize,
}

#[derive(Debug, Deserialize)]
struct DiffXmlRequest {
    root: String,
    #[serde(default)]
    game_version: Option<String>,
    source_lang_dir: String,
    target_lang_dir: String,
    #[serde(default)]
    baseline_po: Option<String>,
    #[serde(default)]
    defs_root: Option<String>,
    #[serde(default)]
    out_json: Option<String>,
    #[serde(default)]
    defs_dicts: Option<Vec<String>>,
    #[serde(default)]
    extra_fields: Option<Vec<String>>,
    #[serde(default)]
    type_schema: Option<String>,
    #[serde(default)]
    out_dir: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiffXmlResponse {
    resolved_root: String,
    game_version: Option<String>,
    only_in_mod: Vec<String>,
    only_in_translation: Vec<String>,
    changed: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
struct LangUpdateRequest {
    root: String,
    repo: String,
    #[serde(default)]
    branch: Option<String>,
    #[serde(default)]
    zip_path: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
    source_lang_dir: String,
    target_lang_dir: String,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    backup: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LangUpdateResponse {
    files: usize,
    bytes: u64,
    out_dir: String,
}

#[derive(Debug, Deserialize)]
struct AnnotateRequest {
    root: String,
    source_lang_dir: String,
    target_lang_dir: String,
    #[serde(default)]
    comment_prefix: Option<String>,
    #[serde(default)]
    strip: bool,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    backup: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnotateResponse {
    processed: usize,
    annotated: usize,
}

#[derive(Debug, Deserialize)]
struct InitRequest {
    root: String,
    source_lang_dir: String,
    target_lang_dir: String,
    #[serde(default)]
    overwrite: bool,
    #[serde(default)]
    dry_run: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitResponse {
    files: usize,
    out_language: String,
}

#[tauri::command]
fn get_app_info() -> Result<AppInfo, ApiError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct DebugOptions {
    #[serde(default)]
    backtrace: Option<bool>,
    #[serde(default)]
    min_level: Option<String>,
}

#[tauri::command]
fn set_debug_options(state: State<LogState>, opts: DebugOptions) -> Result<String, ApiError> {
    if let Some(bt) = opts.backtrace {
        std::env::set_var("RUST_BACKTRACE", if bt { "1" } else { "0" });
        append_log(
            &state.path,
            "INFO",
            &format!("debug: backtrace set to {}", bt),
        );
    }
    if let Some(level) = opts.min_level {
        append_log(
            &state.path,
            "INFO",
            &format!("debug: min_level hint {}", level),
        );
    }
    Ok("ok".into())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiagnosticsInfo {
    app_version: String,
    os: String,
    arch: String,
    log_path: String,
    tauri_version: String,
}

#[tauri::command]
fn get_diagnostics(state: State<LogState>) -> Result<DiagnosticsInfo, ApiError> {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    Ok(DiagnosticsInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os,
        arch,
        log_path: state.path.display().to_string(),
        tauri_version: tauri::VERSION.to_string(),
    })
}

#[derive(Debug, Deserialize)]
struct CollectDiagRequest {
    #[serde(default)]
    out_path: Option<String>,
}

#[tauri::command]
fn collect_diagnostics_via_dialog(
    window: Window,
    state: State<LogState>,
    req: CollectDiagRequest,
) -> Result<Option<String>, ApiError> {
    let base = state
        .path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(std::env::temp_dir);
    let default_path = req
        .out_path
        .unwrap_or_else(|| base.join("diagnostics.txt").display().to_string());
    let mut builder = window.dialog().file();
    let dp = PathBuf::from(&default_path);
    if let Some(dir) = dp.parent() {
        if dir.is_dir() {
            builder = builder.set_directory(dir);
        }
    }
    if let Some(name) = dp.file_name() {
        builder = builder.set_file_name(name.to_string_lossy().to_string());
    }
    let Some(picked) = builder.blocking_save_file() else {
        return Ok(None);
    };
    let out = PathBuf::from(picked.simplified().to_string());
    let resolved = out.display().to_string();
    write_diagnostics(&state, &out)?;
    Ok(Some(resolved))
}

fn write_diagnostics(state: &LogState, out: &Path) -> Result<(), ApiError> {
    let mut buf = String::new();
    buf.push_str(&format!(
        "Diagnostics generated at: {}\n",
        Utc::now().to_rfc3339()
    ));
    buf.push_str(&format!("App version: {}\n", env!("CARGO_PKG_VERSION")));
    buf.push_str(&format!(
        "OS: {}\nArch: {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    buf.push_str(&format!("Tauri: {}\n", tauri::VERSION));
    buf.push_str(&format!("Log file: {}\n\n", state.path.display()));
    if let Ok(log) = std::fs::read_to_string(&state.path) {
        buf.push_str("=== Last 500 lines of gui.log ===\n");
        let lines: Vec<&str> = log.lines().collect();
        let start = lines.len().saturating_sub(500);
        for l in &lines[start..] {
            buf.push_str(l);
            buf.push('\n');
        }
    }
    if let Some(parent) = out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(out, buf.as_bytes())?;
    Ok(())
}

#[tauri::command]
fn simulate_error() -> Result<(), ApiError> {
    Err(ApiError {
        message: "Simulated error for debug".into(),
    })
}

#[tauri::command]
fn simulate_panic() -> Result<(), ApiError> {
    std::panic::panic_any(0u8);
}

#[tauri::command]
fn scan_mod(
    window: Window,
    state: State<LogState>,
    request: ScanRequest,
) -> Result<ScanResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "scan_mod: root={} all={} gv={:?}",
            request.root, request.include_all_versions, request.game_version
        ),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!(
            "scan: root={} include_all_versions={}",
            request.root, request.include_all_versions
        ),
    );
    emit_progress(
        &window,
        &state,
        "scan",
        "start",
        Some("Scanning…".to_string()),
        Some(0),
    );
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(
            &window,
            &state,
            "error",
            format!("scan: path not found: {}", root.display()),
        );
        return Err(ApiError {
            message: format!("Path not found: {}", root.display()),
        });
    }
    if request.include_all_versions {
        let res = run_scan(&root, None, &request);
        if let Ok(ref r) = res {
            emit_log(&window, &state, "info", format!("scan finished (all versions): total={} keyed={} definj={} saved_json={:?} saved_csv={:?}", r.total, r.keyed, r.def_injected, r.saved_json, r.saved_csv));
            emit_progress(
                &window,
                &state,
                "scan",
                "done",
                Some("Scan finished".to_string()),
                Some(100),
            );
        }
        if let Ok(ref r) = res {
            write_profile(
                &state,
                "scan_mod",
                t0,
                serde_json::json!({"total": r.total, "keyed": r.keyed, "def_injected": r.def_injected}),
            );
        }
        res
    } else {
        let (resolved, version) =
            resolve_game_version_root(&root, request.game_version.as_deref())?;
        let res = run_scan(&resolved, version.as_deref(), &request);
        if let Ok(ref r) = res {
            emit_log(&window, &state, "info", format!("scan finished: {} → total={} keyed={} definj={} saved_json={:?} saved_csv={:?}", resolved.display(), r.total, r.keyed, r.def_injected, r.saved_json, r.saved_csv));
            emit_progress(
                &window,
                &state,
                "scan",
                "done",
                Some("Scan finished".to_string()),
                Some(100),
            );
            write_profile(
                &state,
                "scan_mod",
                t0,
                serde_json::json!({"total": r.total, "keyed": r.keyed, "def_injected": r.def_injected}),
            );
        }
        res
    }
}

fn run_scan(
    scan_root: &Path,
    version: Option<&str>,
    request: &ScanRequest,
) -> Result<ScanResponse, ApiError> {
    // Advanced scan mirrors CLI logic where possible
    use std::collections::{BTreeSet, HashMap};
    let defs_abs = request
        .defs_root
        .as_deref()
        .map(|p| make_absolute(scan_root, Path::new(p)));
    let auto = autodiscover_defs_context(scan_root).wrap_err("discover defs context")?;
    let mut extra_fields: Vec<String> = auto.extra_fields.clone();
    if let Some(ref fields) = request.extra_fields {
        extra_fields.extend(fields.clone());
    }
    extra_fields.sort();
    extra_fields.dedup();

    // Merge dicts: auto + files + schema-as-dict
    let mut dict_sets: HashMap<String, BTreeSet<String>> = auto
        .dict
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();
    let mut merge_dict = |map: HashMap<String, Vec<String>>| {
        for (k, v) in map {
            dict_sets.entry(k).or_default().extend(v);
        }
    };
    if let Some(list) = request.defs_dicts.as_ref() {
        for p in list {
            let pp = make_absolute(scan_root, Path::new(p));
            if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
                merge_dict(d.0);
            }
        }
    }
    if let Some(schema) = request.type_schema.as_deref() {
        let pp = make_absolute(scan_root, Path::new(schema));
        if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) {
            merge_dict(d.0);
        }
    }
    let merged: HashMap<String, Vec<String>> = dict_sets
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();

    // Mirror CLI env gates
    if request.keyed_nested {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
    }
    if request.fuzzy {
        std::env::set_var("RIMLOC_FUZZY", "1");
    }
    // no_inherit is present in ScanRequest outer options (handled in frontend; if needed place here too)

    // Env gates for inheritance/nested keyed
    if request.no_inherit {
        std::env::set_var("RIMLOC_INHERIT", "0");
    }
    if request.keyed_nested {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
    }
    if request.parallel {
        std::env::set_var("RIMLOC_PARALLEL", "1");
    }

    // Perform scan
    let mut units = rimloc_services::scan_units_with_defs_and_dict(
        scan_root,
        defs_abs.as_deref(),
        &merged,
        &extra_fields,
    )
    .wrap_err("scan units (defs+dict)")?;
    if request.with_plugins {
        let _ = rimloc_services::plugins::load_plugins_from_env();
        let default_dir = scan_root.join("plugins");
        let _ = rimloc_services::plugins::load_dynamic_plugins_from(&default_dir);
        if let Ok(mut extra) = rimloc_services::plugins::run_scan_plugins(scan_root) {
            units.append(&mut extra);
        }
    }
    if request.with_patches {
        let min_len = request.patch_min_len.unwrap_or(1);
        if let Ok(mut extra) =
            rimloc_services::scan_patches_as_units(scan_root, min_len, request.patch_strict_xpath)
        {
            units.append(&mut extra);
        }
    }

    // Optional language filter
    if let Some(dir) = request.source_lang_dir.as_deref() {
        units.retain(|u| rimloc_services::is_under_languages_dir(&u.path, dir));
    } else if let Some(code) = request.source_lang.as_deref() {
        let dir = rimloc_import_po::rimworld_lang_dir(code);
        units.retain(|u| rimloc_services::is_under_languages_dir(&u.path, &dir));
    }
    let mut keyed = 0usize;
    let mut def_injected = 0usize;
    let mut mapped: Vec<ScanUnitView> = Vec::with_capacity(units.len());
    for unit in &units {
        let kind = classify_unit(&unit.path);
        match kind {
            ScanKind::Keyed => keyed += 1,
            ScanKind::DefInjected => def_injected += 1,
            ScanKind::Other => {}
        }
        mapped.push(ScanUnitView {
            key: unit.key.clone(),
            source: unit.source.clone().unwrap_or_default(),
            path: unit.path.display().to_string(),
            line: unit.line,
            kind,
        });
    }

    let saved_json = if let Some(path) = request.out_json.as_ref() {
        let path = make_absolute(scan_root, Path::new(path));
        write_scan_json(&path, &units)?;
        Some(path.display().to_string())
    } else {
        None
    };

    let saved_csv = if let Some(path) = request.out_csv.as_ref() {
        let path = make_absolute(scan_root, Path::new(path));
        write_scan_csv(&path, &units, request.lang.as_deref())?;
        Some(path.display().to_string())
    } else {
        None
    };

    Ok(ScanResponse {
        root: request.root.clone(),
        resolved_root: scan_root.display().to_string(),
        game_version: version.map(ToString::to_string),
        total: units.len(),
        keyed,
        def_injected,
        saved_json,
        saved_csv,
        units: mapped,
    })
}

fn write_scan_json(path: &Path, units: &[rimloc_services::TransUnit]) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let payload: Vec<ScanUnit> = units
        .iter()
        .map(|u| ScanUnit {
            schema_version: SCHEMA_VERSION,
            path: u.path.display().to_string(),
            line: u.line,
            key: u.key.clone(),
            value: u.source.clone(),
        })
        .collect();
    serde_json::to_writer_pretty(file, &payload).map_err(ApiError::from)
}

fn write_scan_csv(
    path: &Path,
    units: &[rimloc_services::TransUnit],
    lang: Option<&str>,
) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    export_csv::write_csv(file, units, lang).map_err(ApiError::from)
}

fn make_absolute(base: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base.join(candidate)
    }
}

/// LEGACY hardening (RC K4, 2026-09-27): the contract surface refuses
/// CWD-relative caller paths as a typed `invalid_output_path` BEFORE any
/// filesystem work — a relative path silently lands wherever the app was
/// launched from, and resolving one on the caller's behalf is exactly how
/// the built-app `…/RimLoc-Export/…` incident happened. Legacy write
/// commands that take a caller-chosen out path now enforce the same form
/// policy: absolute only, never resolved, never CWD-dependent.
fn ensure_caller_path_absolute(field: &str, p: &Path) -> Result<(), ApiError> {
    if p.is_absolute() {
        Ok(())
    } else {
        Err(ApiError {
            message: format!(
                "{field} must be an absolute path (got `{}`); the legacy surface does not resolve relative paths against the process working directory",
                p.display()
            ),
        })
    }
}

fn classify_unit(path: &Path) -> ScanKind {
    let path_str = path.to_string_lossy();
    if path_str.contains("/Keyed/") || path_str.contains("\\Keyed\\") {
        ScanKind::Keyed
    } else if path_str.contains("/DefInjected/") || path_str.contains("\\DefInjected\\") {
        ScanKind::DefInjected
    } else {
        ScanKind::Other
    }
}

#[tauri::command]
fn learn_defs(
    window: Window,
    state: State<LogState>,
    request: LearnDefsRequest,
) -> Result<LearnDefsResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "learn_defs: root={} gv={:?}",
            request.root, request.game_version
        ),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!("learn: root={}", request.root),
    );
    emit_progress(
        &window,
        &state,
        "learn",
        "start",
        Some("Preparing…".to_string()),
        Some(0),
    );
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(
            &window,
            &state,
            "error",
            format!("learn: path not found: {}", root.display()),
        );
        return Err(ApiError {
            message: format!("Path not found: {}", root.display()),
        });
    }
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let out_dir_raw = request
        .out_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("_learn"));
    let out_dir = make_absolute(&scan_root, &out_dir_raw);
    std::fs::create_dir_all(&out_dir)?;

    emit_progress(
        &window,
        &state,
        "learn",
        "discover",
        Some("Discovering context…".to_string()),
        Some(10),
    );
    let auto = autodiscover_defs_context(&scan_root).wrap_err("discover defs context")?;
    let defs_root = request
        .defs_root
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let dict_files: Vec<PathBuf> = if let Some(list) = request.dict_files.as_ref() {
        list.iter()
            .map(|p| make_absolute(&scan_root, Path::new(p)))
            .collect()
    } else {
        auto.dict_sources.clone()
    };
    let model_path = request
        .model_path
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let learned_out = request
        .learned_out
        .as_deref()
        .map(|p| make_absolute(&out_dir, Path::new(p)));
    let retrain_dict = request
        .retrain_dict
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let opts = learn::LearnOptions {
        mod_root: scan_root.clone(),
        defs_root,
        dict_files,
        model_path,
        ml_url: request.ml_url.clone(),
        lang_dir: request
            .lang_dir
            .clone()
            .unwrap_or_else(|| "English".to_string()),
        threshold: request.threshold.unwrap_or(0.8),
        no_ml: request.no_ml,
        retrain: request.retrain,
        retrain_dict,
        min_len: request.min_len.unwrap_or(1),
        blacklist: request.blacklist.clone().unwrap_or_default(),
        out_dir: out_dir.clone(),
        learned_out,
    };

    emit_log(
        &window,
        &state,
        "debug",
        format!("learn options: out_dir={}", out_dir.display()),
    );
    emit_progress(
        &window,
        &state,
        "learn",
        "learn",
        Some("Learning templates…".to_string()),
        Some(60),
    );
    let result = learn::learn_defs(&opts).wrap_err("learn defs")?;
    let learned_path = out_dir.join("learned_defs.json");
    emit_progress(
        &window,
        &state,
        "learn",
        "done",
        Some("Learn finished".to_string()),
        Some(100),
    );

    let response = LearnDefsResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        out_dir: out_dir.display().to_string(),
        missing_path: result.missing_path.display().to_string(),
        suggested_path: result.suggested_path.display().to_string(),
        learned_path: learned_path.display().to_string(),
        candidates: result.candidates.len(),
        accepted: result.accepted,
    };
    emit_log(
        &window,
        &state,
        "info",
        format!(
            "learn finished: accepted {}/{} → out_dir={}",
            response.accepted, response.candidates, response.out_dir
        ),
    );
    write_profile(
        &state,
        "learn_defs",
        t0,
        serde_json::json!({"accepted": response.accepted, "candidates": response.candidates}),
    );
    Ok(response)
}

#[tauri::command]
fn export_po(
    window: Window,
    state: State<LogState>,
    request: ExportPoRequest,
) -> Result<ExportPoResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!("export_po: root={} out_po={}", request.root, request.out_po),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!("export_po: root={} out_po={}", request.root, request.out_po),
    );
    emit_progress(
        &window,
        &state,
        "export",
        "start",
        Some("Exporting…".to_string()),
        Some(0),
    );
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(
            &window,
            &state,
            "error",
            format!("export_po: path not found: {}", root.display()),
        );
        return Err(ApiError {
            message: format!("Path not found: {}", root.display()),
        });
    }
    let (scan_root, version) = if request.include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, request.game_version.as_deref())?
    };
    let out_po_path = make_absolute(&scan_root, Path::new(&request.out_po));
    if let Some(parent) = out_po_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tm_paths: Option<Vec<PathBuf>> = request.tm_roots.as_ref().map(|roots| {
        roots
            .iter()
            .map(|r| make_absolute(&scan_root, Path::new(r)))
            .collect()
    });

    emit_progress(
        &window,
        &state,
        "export",
        "collect",
        Some("Collecting units…".to_string()),
        Some(20),
    );

    // Advanced: allow explicit Defs dir and custom dict/fields like scan
    let defs_abs = request
        .defs_root
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let auto = autodiscover_defs_context(&scan_root).wrap_err("discover defs context")?;
    let mut extra_fields: Vec<String> = auto.extra_fields.clone();
    if let Some(ref fields) = request.extra_fields {
        extra_fields.extend(fields.clone());
    }
    extra_fields.sort();
    extra_fields.dedup();

    use std::collections::{BTreeSet, HashMap};
    let mut dict_sets: HashMap<String, BTreeSet<String>> = auto
        .dict
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();
    let mut merge_dict = |map: HashMap<String, Vec<String>>| {
        for (k, v) in map {
            dict_sets.entry(k).or_default().extend(v);
        }
    };
    if let Some(list) = request.defs_dicts.as_ref() {
        for p in list {
            let pp = make_absolute(&scan_root, Path::new(p));
            if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
                merge_dict(d.0);
            }
        }
    }
    let type_schema = request
        .defs_type_schema
        .as_deref()
        .or(request.type_schema.as_deref());
    if let Some(schema) = type_schema {
        let pp = make_absolute(&scan_root, Path::new(schema));
        if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) {
            merge_dict(d.0);
        }
    }
    let merged: HashMap<String, Vec<String>> = dict_sets
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect();

    // Source dir for filtering + target path mapping of DefInjected
    let source_dir: String = request
        .source_lang_dir
        .clone()
        .or_else(|| {
            request
                .source_lang
                .clone()
                .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
        })
        .unwrap_or_else(|| "English".to_string());

    // Gate I2 consolidation: ONE PO-inventory builder (services) — the
    // previous inline copy silently dropped TKey units (0 scan_defs_tkey
    // calls). Services owns Keyed + Defs-by-dictionary + TKey + TM prefill.
    let _ = &defs_abs;
    let _ = &merged;
    let _ = &extra_fields;
    let effective_source_lang = request.source_lang.clone();
    let stats = rimloc_services::export_po_with_tm(
        &scan_root,
        &out_po_path,
        if request.pot {
            None
        } else {
            request.lang.as_deref()
        },
        effective_source_lang.as_deref(),
        Some(&source_dir),
        tm_paths.as_deref(),
    )
    .wrap_err("export po")?;

    let tm_coverage_pct = if stats.total == 0 {
        0
    } else {
        ((stats.tm_filled as f64 / stats.total as f64) * 100.0).round() as u32
    };

    let warning = def_injected_warning(&scan_root, &source_dir);
    if let Some(ref w) = warning {
        emit_log(&window, &state, "warn", w.clone());
    }
    emit_progress(
        &window,
        &state,
        "export",
        "done",
        Some("Export finished".to_string()),
        Some(100),
    );

    let resp = ExportPoResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        out_po: out_po_path.display().to_string(),
        total: stats.total,
        tm_filled: stats.tm_filled,
        tm_coverage_pct,
        warning,
    };
    emit_log(
        &window,
        &state,
        "info",
        format!(
            "export finished: {} → total={} tm_filled={} ({}%)",
            resp.out_po, resp.total, resp.tm_filled, resp.tm_coverage_pct
        ),
    );
    write_profile(
        &state,
        "export_po",
        t0,
        serde_json::json!({"total": resp.total, "tm_filled": resp.tm_filled}),
    );
    Ok(resp)
}

// Local helper: derive DefInjected target path for a given Defs file

#[tauri::command]
fn validate_mod(
    window: Window,
    state: State<LogState>,
    request: ValidateRequest,
) -> Result<ValidateResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "validate_mod: root={} defs_root={:?}",
            request.root, request.defs_root
        ),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!("validate: root={}", request.root),
    );
    emit_progress(
        &window,
        &state,
        "validate",
        "start",
        Some("Validating…".to_string()),
        Some(0),
    );
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = if request.include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, request.game_version.as_deref())?
    };
    let defs_root = request
        .defs_root
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));

    // If dicts and/or type schema provided, merge dicts and call validate_with_defs_and_dict
    let mut msgs_raw = if request
        .defs_dicts
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || request.defs_type_schema.as_ref().is_some()
    {
        let mut dicts: Vec<rimloc_parsers_xml::DefsDict> = Vec::new();
        dicts.push(rimloc_parsers_xml::load_embedded_defs_dict());
        if let Some(list) = request.defs_dicts.as_ref() {
            for p in list {
                let pp = make_absolute(&scan_root, Path::new(p));
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
                    dicts.push(d);
                }
            }
        }
        if let Some(schema) = request.defs_type_schema.as_deref() {
            let pp = make_absolute(&scan_root, Path::new(schema));
            if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) {
                dicts.push(d);
            }
        }
        let merged = rimloc_parsers_xml::merge_defs_dicts(&dicts);
        rimloc_services::validate_under_root_with_defs_and_dict(
            &scan_root,
            request.source_lang.as_deref(),
            request.source_lang_dir.as_deref(),
            defs_root.as_deref(),
            &merged.0,
            request.extra_fields.as_deref().unwrap_or(&Vec::new()),
        )?
    } else if let Some(fields) = request.extra_fields.as_ref() {
        validate_under_root_with_defs_and_fields(
            &scan_root,
            request.source_lang.as_deref(),
            request.source_lang_dir.as_deref(),
            defs_root.as_deref(),
            fields,
        )?
    } else if request.defs_root.is_some() {
        validate_under_root_with_defs(
            &scan_root,
            request.source_lang.as_deref(),
            request.source_lang_dir.as_deref(),
            defs_root.as_deref(),
        )?
    } else {
        validate_under_root(
            &scan_root,
            request.source_lang.as_deref(),
            request.source_lang_dir.as_deref(),
        )?
    };
    if request.compare_placeholders {
        let src_dir = request
            .source_lang_dir
            .clone()
            .or_else(|| {
                request
                    .source_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "English".to_string());
        let tgt_dir = request
            .target_lang_dir
            .clone()
            .or_else(|| {
                request
                    .target_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "Russian".to_string());
        if let Ok(mut extra) = validate_placeholders_cross_language(
            &scan_root,
            &src_dir,
            &tgt_dir,
            defs_root.as_deref(),
        ) {
            msgs_raw.append(&mut extra);
        }
    }
    if request.compare_lists {
        let src_dir = request
            .source_lang_dir
            .clone()
            .or_else(|| {
                request
                    .source_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "English".to_string());
        let tgt_dir = request
            .target_lang_dir
            .clone()
            .or_else(|| {
                request
                    .target_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "Russian".to_string());
        if let Ok(mut extra) = rimloc_services::validate::validate_lists_cross_language(
            &scan_root,
            &src_dir,
            &tgt_dir,
            defs_root.as_deref(),
        ) {
            msgs_raw.append(&mut extra);
        }
    }
    if request.report_orphans {
        let src_dir = request
            .source_lang_dir
            .clone()
            .or_else(|| {
                request
                    .source_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "English".to_string());
        let tgt_dir = request
            .target_lang_dir
            .clone()
            .or_else(|| {
                request
                    .target_lang
                    .clone()
                    .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
            })
            .unwrap_or_else(|| "Russian".to_string());
        if let Ok(mut extra) = rimloc_services::validate::validate_orphans_cross_language(
            &scan_root,
            &src_dir,
            &tgt_dir,
            defs_root.as_deref(),
        ) {
            msgs_raw.append(&mut extra);
        }
    }
    let msgs: Vec<ValidationMessageView> = msgs_raw
        .into_iter()
        .map(|m| ValidationMessageView {
            kind: m.kind,
            key: m.key,
            path: m.path,
            line: m.line,
            message: m.message,
        })
        .collect();
    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut infos = 0usize;
    for m in &msgs {
        match m.kind.as_str() {
            "error" => errors += 1,
            "warn" | "warning" => warnings += 1,
            _ => infos += 1,
        }
    }
    emit_progress(
        &window,
        &state,
        "validate",
        "done",
        Some("Validation finished".to_string()),
        Some(100),
    );
    if let Some(out) = request.out_json.as_deref() {
        let path = make_absolute(&scan_root, Path::new(out));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, serde_json::to_vec_pretty(&msgs).unwrap_or_default());
    }
    write_profile(
        &state,
        "validate",
        t0,
        serde_json::json!({"total": msgs.len(), "errors": errors, "warnings": warnings, "infos": infos }),
    );
    Ok(ValidateResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        total: msgs.len(),
        errors,
        warnings,
        infos,
        messages: msgs,
    })
}

#[tauri::command]
fn xml_health(
    window: Window,
    state: State<LogState>,
    request: XmlHealthRequest,
) -> Result<XmlHealthResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "xml_health: root={} lang_dir={:?}",
            request.root, request.lang_dir
        ),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!("xml_health: root={}", request.root),
    );
    emit_progress(
        &window,
        &state,
        "health",
        "start",
        Some("Checking XML…".to_string()),
        Some(0),
    );
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let lang_dir = request
        .lang_dir
        .clone()
        .or_else(|| {
            request
                .lang
                .clone()
                .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
        })
        .unwrap_or_else(|| "English".to_string());
    let mut report = xml_health_scan(&scan_root, Some(&lang_dir)).wrap_err("xml health")?;
    // Optional filtering like CLI options
    if let Some(only) = request.only.as_ref() {
        if !only.is_empty() {
            report
                .issues
                .retain(|i| only.iter().any(|k| k.eq_ignore_ascii_case(&i.category)));
        }
    }
    if let Some(except) = request.except.as_ref() {
        if !except.is_empty() {
            report
                .issues
                .retain(|i| !except.iter().any(|k| k.eq_ignore_ascii_case(&i.category)));
        }
    }
    if request.strict && !report.issues.is_empty() {
        emit_log(
            &window,
            &state,
            "warn",
            format!("XML health strict: {} issues detected", report.issues.len()),
        );
    }
    emit_progress(
        &window,
        &state,
        "health",
        "done",
        Some("XML check finished".to_string()),
        Some(100),
    );
    let out = XmlHealthResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        checked: report.checked,
        issues: report.issues,
    };
    if let Some(path_str) = request.out_json.as_deref() {
        let p = make_absolute(&scan_root, Path::new(path_str));
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&p, serde_json::to_vec_pretty(&out).unwrap_or_default());
    }
    write_profile(
        &state,
        "xml_health",
        t0,
        serde_json::json!({"checked": out.checked, "issues": out.issues.len()}),
    );
    Ok(out)
}

#[tauri::command]
fn import_po(
    window: Window,
    state: State<LogState>,
    request: ImportPoRequest,
) -> Result<ImportPoResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!("import_po: root={} po={}", request.root, request.po_path),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!("import_po: root={} po={}", request.root, request.po_path),
    );
    emit_progress(
        &window,
        &state,
        "import",
        "start",
        Some("Importing PO…".to_string()),
        Some(0),
    );
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let po_path = make_absolute(&scan_root, Path::new(&request.po_path));
    let lang_dir = request
        .lang_dir
        .clone()
        .or_else(|| {
            request
                .lang
                .clone()
                .map(|c| rimloc_import_po::rimworld_lang_dir(&c))
        })
        .unwrap_or_else(|| "English".to_string());
    let summary = if let Some(out_xml) = request.out_xml.as_deref() {
        let outp = make_absolute(&scan_root, Path::new(out_xml));
        rimloc_services::import_po_to_file(
            &po_path,
            &outp,
            request.keep_empty,
            request.dry_run,
            request.backup,
        )
        .wrap_err("import po to file")?
    } else if request.dry_run {
        let (_plan, summary) = import_po_to_mod_tree(
            &po_path,
            &scan_root,
            &lang_dir,
            request.keep_empty,
            true,
            request.backup,
            request.single_file,
            request.incremental,
            request.only_diff,
            request.report,
        )?;
        summary.unwrap_or(rimloc_services::ImportSummary {
            mode: "dry_run".into(),
            created: 0,
            updated: 0,
            skipped: 0,
            keys: 0,
            files: vec![],
        })
    } else {
        import_po_to_mod_tree_with_progress(
            &po_path,
            &scan_root,
            &lang_dir,
            request.keep_empty,
            request.backup,
            request.single_file,
            request.incremental,
            request.only_diff,
            request.report,
            |cur, total, path| {
                emit_progress(
                    &window,
                    &state,
                    "import",
                    "file",
                    Some(path.display().to_string()),
                    Some(((cur as f64 / total as f64) * 100.0).round() as u32),
                );
            },
        )
        .wrap_err("import po")?
    };
    emit_progress(
        &window,
        &state,
        "import",
        "done",
        Some("Import finished".to_string()),
        Some(100),
    );
    let resp = ImportPoResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        lang_dir,
        created: summary.created,
        updated: summary.updated,
        skipped: summary.skipped,
        keys: summary.keys,
    };
    write_profile(
        &state,
        "import_po",
        t0,
        serde_json::json!({"created": resp.created, "updated": resp.updated, "skipped": resp.skipped, "keys": resp.keys}),
    );
    Ok(resp)
}

#[tauri::command]
fn build_mod(
    window: Window,
    state: State<LogState>,
    request: BuildModRequest,
) -> Result<BuildModResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!("build_mod: po={} out={}", request.po_path, request.out_mod),
    );
    let t0 = std::time::Instant::now();
    emit_log(
        &window,
        &state,
        "info",
        format!("build_mod from PO: {}", request.po_path),
    );
    emit_progress(
        &window,
        &state,
        "build",
        "start",
        Some("Building mod…".to_string()),
        Some(0),
    );
    let out = PathBuf::from(&request.out_mod);
    let mut files_count = 0usize;
    let mut total_keys = 0usize;
    if let Some(from_root) = request.from_root.as_deref() {
        let root = PathBuf::from(from_root);
        let versions = request.from_game_versions.as_deref();
        if request.dry_run {
            let (files, total) = rimloc_services::build_from_root(
                &root,
                &out,
                &request.lang_dir,
                versions,
                false,
                request.dedupe,
            )?;
            files_count = files.len();
            total_keys = total;
        } else {
            let (files, total) = rimloc_services::build_from_root_with_progress(
                &root,
                &out,
                &request.lang_dir,
                versions,
                true,
                request.dedupe,
                |cur, total, path| {
                    emit_progress(
                        &window,
                        &state,
                        "build",
                        "file",
                        Some(path.display().to_string()),
                        Some(((cur as f64 / total as f64) * 100.0).round() as u32),
                    );
                },
            )?;
            files_count = files.len();
            total_keys = total;
        }
    } else if request.dry_run {
        let po = PathBuf::from(&request.po_path);
        let plan = rimloc_services::build_from_po_dry_run(
            &po,
            &out,
            &request.lang_dir,
            &request.name,
            &request.package_id,
            &request.rw_version,
            request.dedupe,
        )?;
        files_count = plan.files.len();
        total_keys = plan.total_keys;
    } else {
        let po = PathBuf::from(&request.po_path);
        build_from_po_with_progress(
            &po,
            &out,
            &request.lang_dir,
            &request.name,
            &request.package_id,
            &request.rw_version,
            request.dedupe,
            |cur, total, path| {
                files_count = total;
                emit_progress(
                    &window,
                    &state,
                    "build",
                    "file",
                    Some(path.display().to_string()),
                    Some(((cur as f64 / total as f64) * 100.0).round() as u32),
                );
            },
        )
        .wrap_err("build mod from po")?;
    }
    emit_progress(
        &window,
        &state,
        "build",
        "done",
        Some("Build finished".to_string()),
        Some(100),
    );
    let resp = BuildModResponse {
        out_mod: out.display().to_string(),
        files: files_count,
        total_keys,
    };
    write_profile(
        &state,
        "build_mod",
        t0,
        serde_json::json!({"files": resp.files }),
    );
    Ok(resp)
}

fn def_injected_warning(scan_root: &Path, source_dir: &str) -> Option<String> {
    let definj = scan_root
        .join("Languages")
        .join(source_dir)
        .join("DefInjected");
    if has_any_xml(&definj) {
        None
    } else {
        Some(format!(
            "Languages/{source_dir}/DefInjected NOT found → export will include only Keyed. You can copy _learn/suggested.xml into DefInjected."
        ))
    }
}

fn has_any_xml(dir: &Path) -> bool {
    if !dir.exists() {
        return false;
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("xml"))
                .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn resolve_game_version_root(
    base: &Path,
    requested: Option<&str>,
) -> Result<(PathBuf, Option<String>), ApiError> {
    if is_version_directory(base) {
        let name = base
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string());
        return Ok((base.to_path_buf(), name));
    }

    let languages = list_version_directories(base)?;

    if let Some(req) = requested {
        if let Some(path) = find_version_directory(base, req) {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
            return Ok((path, name));
        }
        return Err(ApiError {
            message: format!(
                "Requested version '{req}' not found under {}",
                base.display()
            ),
        });
    }

    if languages.is_empty() {
        return Ok((base.to_path_buf(), None));
    }

    let mut entries = languages;
    entries.sort_by(|a, b| {
        let len_cmp = a.components.len().cmp(&b.components.len());
        if len_cmp != std::cmp::Ordering::Equal {
            return len_cmp;
        }
        a.components.cmp(&b.components)
    });

    if let Some(entry) = entries.last() {
        return Ok((entry.path.clone(), Some(entry.name.clone())));
    }

    Ok((base.to_path_buf(), None))
}

// --- Validate PO (GUI) ---
#[derive(Debug, Deserialize)]
struct ValidatePoRequest {
    po_path: String,
    #[serde(default)]
    #[allow(dead_code)]
    strict: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidatePoMismatch {
    context: Option<String>,
    reference: Option<String>,
    msgid: String,
    msgstr: String,
    expected_placeholders: Vec<String>,
    got_placeholders: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ValidatePoResponse {
    checked: usize,
    mismatches: Vec<ValidatePoMismatch>,
}

fn extract_placeholders_cli_like(s: &str) -> BTreeSet<String> {
    // same patterns as CLI: %.. and {...}
    static RE_PCT: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
    static RE_BRACE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
    let mut set = BTreeSet::new();
    let re1 = RE_PCT.get_or_init(|| regex::Regex::new(r"%(\d+\$)?0?\d*[sdif]").unwrap());
    for m in re1.find_iter(s) {
        set.insert(m.as_str().to_string());
    }
    let re2 = RE_BRACE.get_or_init(|| regex::Regex::new(r"\{[^}]+\}").unwrap());
    for m in re2.find_iter(s) {
        set.insert(m.as_str().to_string());
    }
    set
}

#[tauri::command]
fn validate_po_gui(
    _window: Window,
    _state: State<LogState>,
    req: ValidatePoRequest,
) -> Result<ValidatePoResponse, ApiError> {
    use std::io::BufRead;
    let file = std::fs::File::open(&req.po_path).map_err(ApiError::from)?;
    let rdr = std::io::BufReader::new(file);
    let mut ctx: Option<String> = None;
    let mut id = String::new();
    let mut strv = String::new();
    let mut reference: Option<String> = None;
    enum Mode {
        None,
        InId,
        InStr,
    }
    let mut mode = Mode::None;
    let mut entries: Vec<(Option<String>, String, String, Option<String>)> = Vec::new();
    let push =
        |ctx: &mut Option<String>,
         id: &mut String,
         strv: &mut String,
         reference: &mut Option<String>,
         entries: &mut Vec<(Option<String>, String, String, Option<String>)>| {
            if !id.is_empty() || !strv.is_empty() || ctx.is_some() || reference.is_some() {
                entries.push((
                    ctx.clone(),
                    std::mem::take(id),
                    std::mem::take(strv),
                    reference.clone(),
                ));
                *ctx = None;
                *reference = None;
            }
        };
    for line in rdr.lines() {
        let t = line.map_err(ApiError::from)?.trim().to_string();
        if t.is_empty() {
            push(&mut ctx, &mut id, &mut strv, &mut reference, &mut entries);
            mode = Mode::None;
            continue;
        }
        if let Some(rest) = t.strip_prefix("#:") {
            reference = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgctxt ") {
            push(&mut ctx, &mut id, &mut strv, &mut reference, &mut entries);
            ctx = Some(unquote(rest));
            mode = Mode::None;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgid ") {
            push(&mut ctx, &mut id, &mut strv, &mut reference, &mut entries);
            id = unquote(rest);
            mode = Mode::InId;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgstr ") {
            strv = unquote(rest);
            mode = Mode::InStr;
            continue;
        }
        if matches!(mode, Mode::InId | Mode::InStr) && t.starts_with('"') {
            let chunk = unquote(&t);
            match mode {
                Mode::InId => id.push_str(&chunk),
                Mode::InStr => strv.push_str(&chunk),
                Mode::None => {}
            }
            continue;
        }
    }
    push(&mut ctx, &mut id, &mut strv, &mut reference, &mut entries);

    let mut mismatches: Vec<ValidatePoMismatch> = Vec::new();
    let mut checked = 0usize;
    for (ctx, msgid, msgstr, reference) in entries {
        if msgid.is_empty() {
            continue;
        }
        if msgstr.trim().is_empty() {
            continue;
        }
        checked += 1;
        let src_ph = extract_placeholders_cli_like(&msgid);
        let dst_ph = extract_placeholders_cli_like(&msgstr);
        if src_ph != dst_ph {
            mismatches.push(ValidatePoMismatch {
                context: ctx,
                reference,
                msgid,
                msgstr,
                expected_placeholders: src_ph.into_iter().collect(),
                got_placeholders: dst_ph.into_iter().collect(),
            });
        }
    }
    Ok(ValidatePoResponse {
        checked,
        mismatches,
    })
}

fn unquote(s: &str) -> String {
    let raw = s.trim().trim_start_matches('"').trim_end_matches('"');
    let mut out = String::new();
    let mut it = raw.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' {
            if let Some(n) = it.next() {
                out.push(match n {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    x => x,
                });
            } else {
                out.push('\\');
            }
        } else {
            out.push(c);
        }
    }
    out
}

// --- Load TM (baseline PO + TM roots with .po files) ---
#[derive(Debug, Deserialize)]
struct LoadTmRequest {
    #[serde(default)]
    baseline_po: Option<String>,
    #[serde(default)]
    tm_roots: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TmEntry {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TmBundle {
    by_key: Vec<TmEntry>,
    by_id: Vec<TmEntry>,
}

struct PoEntry {
    ctx: Option<String>,
    id: String,
    val: String,
}

fn parse_po_entries(path: &Path) -> Result<Vec<PoEntry>, ApiError> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).map_err(ApiError::from)?;
    let rdr = std::io::BufReader::new(file);
    let mut ctx: Option<String> = None;
    let mut id = String::new();
    let mut strv = String::new();
    let mut _reference: Option<String> = None;
    enum Mode {
        None,
        InId,
        InStr,
    }
    let mut mode = Mode::None;
    let mut out: Vec<PoEntry> = Vec::new();
    let push = |ctx: &mut Option<String>,
                id: &mut String,
                strv: &mut String,
                _reference: &mut Option<String>,
                out: &mut Vec<PoEntry>| {
        if !id.is_empty() || !strv.is_empty() || ctx.is_some() || _reference.is_some() {
            out.push(PoEntry {
                ctx: ctx.clone(),
                id: std::mem::take(id),
                val: std::mem::take(strv),
            });
        }
        *ctx = None;
        *_reference = None;
    };
    for line in rdr.lines() {
        let t = line.map_err(ApiError::from)?.trim().to_string();
        if t.is_empty() {
            push(&mut ctx, &mut id, &mut strv, &mut _reference, &mut out);
            mode = Mode::None;
            continue;
        }
        if let Some(rest) = t.strip_prefix("#:") {
            _reference = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgctxt ") {
            push(&mut ctx, &mut id, &mut strv, &mut _reference, &mut out);
            ctx = Some(unquote(rest));
            mode = Mode::None;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgid ") {
            id = unquote(rest);
            mode = Mode::InId;
            continue;
        }
        if let Some(rest) = t.strip_prefix("msgstr ") {
            strv = unquote(rest);
            mode = Mode::InStr;
            continue;
        }
        if matches!(mode, Mode::InId | Mode::InStr) && t.starts_with('"') {
            let chunk = unquote(&t);
            match mode {
                Mode::InId => id.push_str(&chunk),
                Mode::InStr => strv.push_str(&chunk),
                Mode::None => {}
            }
            continue;
        }
    }
    push(&mut ctx, &mut id, &mut strv, &mut _reference, &mut out);
    Ok(out)
}

#[tauri::command]
fn load_tm(
    _window: Window,
    _state: State<LogState>,
    req: LoadTmRequest,
) -> Result<TmBundle, ApiError> {
    let mut by_key: HashMap<String, String> = HashMap::new();
    let mut by_id: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(p) = req.baseline_po.as_deref() {
        let abs = PathBuf::from(p);
        if abs.is_file() {
            if let Ok(entries) = parse_po_entries(&abs) {
                for e in entries {
                    if let Some(ctx) = e.ctx.as_ref() {
                        if !ctx.is_empty() && !e.val.trim().is_empty() {
                            by_key.entry(ctx.clone()).or_insert(e.val.clone());
                        }
                    }
                    if !e.id.trim().is_empty() && !e.val.trim().is_empty() {
                        by_id.entry(e.id.clone()).or_default().push(e.val.clone());
                    }
                }
            }
        }
    }
    if let Some(roots) = req.tm_roots.as_ref() {
        for root in roots {
            let base = PathBuf::from(root);
            if !base.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(&base)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let is_po = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("po"))
                    .unwrap_or(false);
                if !is_po {
                    continue;
                }
                if let Ok(entries) = parse_po_entries(p) {
                    for e in entries {
                        if let Some(ctx) = e.ctx.as_ref() {
                            if !ctx.is_empty() && !e.val.trim().is_empty() {
                                by_key.entry(ctx.clone()).or_insert(e.val.clone());
                            }
                        }
                        if !e.id.trim().is_empty() && !e.val.trim().is_empty() {
                            let arr = by_id.entry(e.id.clone()).or_default();
                            if !arr.contains(&e.val) {
                                arr.push(e.val.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    let mut by_key_list: Vec<TmEntry> = by_key
        .into_iter()
        .map(|(k, v)| TmEntry { key: k, value: v })
        .collect();
    by_key_list.sort_by(|a, b| a.key.cmp(&b.key));
    let mut by_id_list: Vec<TmEntry> = Vec::new();
    for (k, arr) in by_id.into_iter() {
        for v in arr {
            by_id_list.push(TmEntry {
                key: k.clone(),
                value: v,
            });
        }
    }
    by_id_list.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(TmBundle {
        by_key: by_key_list,
        by_id: by_id_list,
    })
}

// --- Learn Patches (scan Patches/ texts) ---
#[derive(Debug, Deserialize)]
struct LearnPatchesRequest {
    root: String,
    #[serde(default)]
    min_len: Option<usize>,
    #[serde(default)]
    out_json: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LearnPatchesResponse {
    out_json: String,
    suggested_xml: Option<String>,
    total: usize,
}

#[tauri::command]
fn learn_patches_cmd(
    _window: Window,
    _state: State<LogState>,
    req: LearnPatchesRequest,
) -> Result<LearnPatchesResponse, ApiError> {
    let root = PathBuf::from(&req.root);
    let (scan_root, _version) = resolve_game_version_root(&root, req.game_version.as_deref())?;
    let min_len = req.min_len.unwrap_or(1);
    let cands = rimloc_services::learn::patches::scan_patches_texts(&scan_root, min_len)
        .wrap_err("scan patches")?;
    let out_dir = scan_root.join("learn_out");
    let out_json = req
        .out_json
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| out_dir.join("patches_texts.json"));
    ensure_caller_path_absolute("out_json", &out_json)?;
    if let Some(parent) = out_json.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let f = std::fs::File::create(&out_json)?;
    serde_json::to_writer_pretty(f, &cands).ok();
    // Also suggested XML if there are inferred entries
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
    let mut suggested: Option<PathBuf> = None;
    if !inferred.is_empty() {
        std::fs::create_dir_all(&out_dir).ok();
        let sug = out_dir.join("_SuggestedFromPatches.xml");
        let mut f = std::fs::File::create(&sug)?;
        use std::io::Write;
        writeln!(f, "<LanguageData>")?;
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
        suggested = Some(sug);
    }
    Ok(LearnPatchesResponse {
        out_json: out_json.display().to_string(),
        suggested_xml: suggested.map(|p| p.display().to_string()),
        total: cands.len(),
    })
}

// --- Load CLI FTL (best-effort simple parser) ---
#[tauri::command]
fn get_cli_i18n(lang: String) -> Result<std::collections::HashMap<String, String>, ApiError> {
    use std::collections::HashMap;
    use std::io::BufRead;
    let here = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ftl = here
        .join("../../../crates/rimloc-cli/i18n")
        .join(&lang)
        .join("rimloc.ftl");
    let mut map: HashMap<String, String> = HashMap::new();
    if !ftl.exists() {
        return Ok(map);
    }
    let file = std::fs::File::open(&ftl)?;
    let rdr = std::io::BufReader::new(file);
    let re = regex::Regex::new(r"^\s*([A-Za-z0-9_.-]+)\s*=\s*(.+)$").unwrap();
    for line in rdr.lines() {
        let l = line?;
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some(cap) = re.captures(s) {
            let key = cap.get(1).unwrap().as_str().to_string();
            let val = cap.get(2).unwrap().as_str().to_string();
            map.insert(key, val);
        }
    }
    Ok(map)
}

#[derive(Debug, Clone)]
struct VersionEntry {
    name: String,
    components: Vec<u32>,
    path: PathBuf,
}

fn is_version_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .and_then(parse_version_components)
        .is_some()
}

fn list_version_directories(base: &Path) -> Result<Vec<VersionEntry>, ApiError> {
    let mut entries = Vec::new();
    let read_dir = match std::fs::read_dir(base) {
        Ok(iter) => iter,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
        Err(err) => {
            return Err(ApiError {
                message: err.to_string(),
            })
        }
    };
    for entry in read_dir {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name_os = entry.file_name();
        let Some(name) = name_os.to_str() else {
            continue;
        };
        if let Some(components) = parse_version_components(name) {
            entries.push(VersionEntry {
                name: name.to_string(),
                components,
                path: entry.path(),
            });
        }
    }
    Ok(entries)
}

fn parse_version_components(name: &str) -> Option<Vec<u32>> {
    let trimmed = name.trim_start_matches('v');
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for part in trimmed.split('.') {
        if part.is_empty() {
            return None;
        }
        let value: u32 = part.parse().ok()?;
        parts.push(value);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

fn find_version_directory(base: &Path, requested: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let normalized = requested.trim_start_matches('v');
    if requested.starts_with('v') {
        candidates.push(requested.trim().to_string());
        candidates.push(normalized.to_string());
    } else {
        candidates.push(normalized.to_string());
        candidates.push(format!("v{normalized}"));
    }
    for name in candidates {
        if name.is_empty() {
            continue;
        }
        let candidate = base.join(&name);
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

/// Commands registered in the PRODUCTION live entry: contract + SAFE
/// read-only legacy extras. Mirrors the live generate_handler! list below —
/// src/tests.rs binds this constant to the enforcement assertions.
pub const LIVE_COMMANDS: &[&str] = &[
    // binding contract (wave 2)
    "contract_handshake",
    "project_create",
    "project_open",
    "project_list",
    "project_snapshot",
    "project_apply_intents",
    "project_refresh",
    "project_cancel_next",
    // final night wave: validate/build/diagnostics over the contract
    "project_validate",
    "project_export",
    "project_build_mod",
    "project_diagnose",
    // safe read-only legacy extras
    "get_app_info",
    "scan_mod",
    "scan_strings_gui",
    "validate_mod",
    "validate_po_gui",
    "xml_health",
    "coverage_gui",
    "diff_xml_cmd",
    "get_cli_i18n",
    "pick_directory",
    // selfloc entry (mandate D): resolve the app-bundled UI catalog dir —
    // read-only shell extra, same class as pick_directory.
    "selfloc_catalog_dir",
];

/// PRIVILEGED legacy commands (source-tree writes, arbitrary open, plugin
/// loading, provider invocation, raw diagnostics): registered ONLY when
/// RIMLOC_LEGACY_COMMANDS=1 — never in the production live entry
/// (lead decision 033 #4). The legacy 3-file UI degrades without these;
/// accepted (the new GUI journey is the product target).
pub const LEGACY_PRIVILEGED_COMMANDS: &[&str] = &[
    "apply_translation",
    "save_text_via_dialog",
    "open_path",
    "load_plugin_cmd",
    "list_plugins_cmd",
    "log_message",
    "set_debug_options",
    "get_diagnostics",
    "collect_diagnostics_via_dialog",
    "simulate_error",
    "simulate_panic",
    "morph_cmd",
    "learn_defs",
    "learn_keyed_cmd",
    "learn_patches_cmd",
    "init_lang_cmd",
    "lang_update_cmd",
    "annotate_cmd",
    "export_po",
    "import_po",
    "build_mod",
    "export_xliff_gui",
    "import_xliff_gui",
    "merge_keyed_gui",
    "get_log_info",
    "dump_schemas",
    "get_profile",
    // Night audit P1-2: load_tm reads ANY caller-chosen baseline PO and
    // walkdir-walks ANY caller-chosen tm_roots with no root containment or
    // symlink checks — an arbitrary-read surface, same class as open_path.
    // Frontend-v2 never calls it; it returns with root guards when the TM
    // slice lands over the contract.
    "load_tm",
];

fn legacy_commands_enabled() -> bool {
    std::env::var("RIMLOC_LEGACY_COMMANDS").as_deref() == Ok("1")
}

/// DEV-ONLY: claim user-initiated activity for the whole process lifetime so
/// App Nap never suspends an off-screen automation instance. A suspended
/// process stops answering AXWindows entirely, which kills AXPress-driven
/// journeys ~10-30s after launch (measured 2026-09-27: AXWindows empties
/// mid-journey and never revives). The activity token and its reason string
/// are intentionally leaked: the claim lives as long as the process.
#[cfg(debug_assertions)]
#[cfg(target_os = "macos")]
fn dev_disable_app_nap() {
    extern "C" {
        fn sel_registerName(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        fn objc_getClass(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        // One declaration for both call shapes: on arm64 the callee reads
        // only the registers it needs, extra args in x2/x3 are ignored.
        fn objc_msgSend(
            receiver: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            options: u64,
            reason: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
        fn CFStringCreateWithCString(
            alloc: *mut std::ffi::c_void,
            c_str: *const std::os::raw::c_char,
            encoding: u32,
        ) -> *mut std::ffi::c_void;
    }
    const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;
    // NSActivityUserInitiated = idleSystemSleepDisabled | userInitiated
    const NS_ACTIVITY_USER_INITIATED: u64 = (1 << 20) | (1 << 15);
    unsafe {
        let cls = objc_getClass(c"NSProcessInfo".as_ptr());
        if cls.is_null() {
            return;
        }
        let info = objc_msgSend(
            cls,
            sel_registerName(c"processInfo".as_ptr()),
            0,
            std::ptr::null_mut(),
        );
        if info.is_null() {
            return;
        }
        let reason = CFStringCreateWithCString(
            std::ptr::null_mut(),
            c"rimloc off-screen UI automation".as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );
        let _activity = objc_msgSend(
            info,
            sel_registerName(c"beginActivityWithOptions:reason:".as_ptr()),
            NS_ACTIVITY_USER_INITIATED,
            reason,
        );
    }
}

/// DEV-ONLY: [NSApp accessibilityActivate] — starts the app's accessibility — starts the app's accessibility
/// server deterministically. Off-screen the AX bridge is lazy and sometimes
/// never hydrates from client queries alone, which would break AXPress
/// automation.
#[cfg(debug_assertions)]
#[cfg(target_os = "macos")]
fn dev_accessibility_activate() {
    // Plain-C AX client query aimed at our own pid: forces the ApplicationServices
    // accessibility machinery to initialize without any ObjC exception risk
    // (every call reports errors by code).
    extern "C" {
        fn getpid() -> i32;
        fn AXUIElementCreateApplication(pid: i32) -> *mut std::ffi::c_void;
        fn CFStringCreateWithCString(
            alloc: *mut std::ffi::c_void,
            c_str: *const std::os::raw::c_char,
            encoding: u32,
        ) -> *mut std::ffi::c_void;
        fn AXUIElementCopyAttributeValue(
            el: *mut std::ffi::c_void,
            attr: *mut std::ffi::c_void,
            out: *mut *mut std::ffi::c_void,
        ) -> i32;
        fn CFRelease(cf: *mut std::ffi::c_void);
    }
    const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;
    unsafe {
        let pid = getpid();
        let el = AXUIElementCreateApplication(pid);
        if el.is_null() {
            return;
        }
        let attr = CFStringCreateWithCString(
            std::ptr::null_mut(),
            c"AXWindows".as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );
        let mut out: *mut std::ffi::c_void = std::ptr::null_mut();
        let _err = AXUIElementCopyAttributeValue(el, attr, &mut out);
        if !out.is_null() {
            CFRelease(out);
        }
        if !attr.is_null() {
            CFRelease(attr);
        }
        CFRelease(el);
    }
}

/// DEV-ONLY: [NSApplication sharedApplication] for the setup hooks.
/// Kept unused for now: the documented entry point for upcoming setup hooks.
#[cfg(debug_assertions)]
#[cfg(target_os = "macos")]
#[allow(dead_code)]
unsafe fn shared_app() -> *mut std::ffi::c_void {
    extern "C" {
        fn sel_registerName(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        fn objc_getClass(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        // Same unified 4-arg shape as in dev_disable_app_nap: on arm64 the
        // callee reads only the registers it needs, extra args are ignored,
        // and one signature across the crate avoids redeclaration errors.
        fn objc_msgSend(
            receiver: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            options: u64,
            reason: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }
    let cls = objc_getClass(c"NSApplication".as_ptr());
    if cls.is_null() {
        return std::ptr::null_mut();
    }
    let sel = sel_registerName(c"sharedApplication".as_ptr());
    objc_msgSend(cls, sel, 0, std::ptr::null_mut())
}

/// DEV-ONLY diagnostics: ObjC class name of the underlying NSWindow.
#[cfg(debug_assertions)]
#[cfg(target_os = "macos")]
fn ns_window_class_name(window: &tauri::WebviewWindow) -> &'static str {
    extern "C" {
        fn object_getClassName(obj: *mut std::ffi::c_void) -> *const std::os::raw::c_char;
    }
    match window.ns_window() {
        Ok(p) if !p.is_null() => unsafe {
            let name = object_getClassName(p);
            if name.is_null() {
                "<null>"
            } else {
                std::ffi::CStr::from_ptr(name)
                    .to_string_lossy()
                    .into_owned()
                    .leak() as &str
                /* dev-only diagnostics leak: one string per process */
            }
        },
        _ => "<no-ns-window>",
    }
}

/// DEV-ONLY: make the off-screen window look "alive" to WebKit (macOS).
/// An ordered window parked outside display geometry renders its first
/// frames, but the first delivered mouse event makes WKWebView re-evaluate
/// view visibility (occlusion / key-window activity state) and freeze the
/// page: the layer goes black and no further frames are committed.
/// Fix: override the window's visibility truth-tellers — occlusionState,
/// isKeyWindow, isMainWindow, isVisible, canBecomeKeyWindow — to report a
/// normal on-screen key window. Class-scoped (single window in-process),
/// dev-only automation mode.
#[cfg(debug_assertions)]
#[cfg(target_os = "macos")]
fn fake_window_visibility(window: &tauri::WebviewWindow) -> bool {
    // Staged via RIMLOC_FAKE (comma list) for experiment control:
    //   occlusion  — -occlusionState always reports NSWindowOcclusionStateVisible
    //   key        — isKeyWindow/isMainWindow/isVisible/canBecomeKeyWindow → YES
    //   app        — [NSApp isActive] → YES
    //   firstmouse — acceptsFirstMouse: → YES on the webview class and the
    //                NSView base: for an inactive app NSWindow consumes the
    //                first click as an "activation click" unless the hit view
    //                accepts it (wry's accept_first_mouse default is NO), and
    //                an off-screen window can never become active, so every
    //                synthetic click would be swallowed without this.
    let stage = std::env::var("RIMLOC_FAKE").unwrap_or_else(|_| "occlusion".to_string());
    let want = |k: &str| stage.split(',').any(|s| s.trim() == k);
    extern "C" {
        fn sel_registerName(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        fn class_replaceMethod(
            cls: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            imp: *mut std::ffi::c_void,
            types: *const std::os::raw::c_char,
        ) -> *mut std::ffi::c_void;
        fn object_getClass(obj: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn objc_getClass(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        // no-arg method returning id (used for [NSApplication sharedApplication]);
        // the real objc_msgSend symbol, given a Rust-friendly alias-free name.
        // Same unified 4-arg shape as in dev_disable_app_nap: on arm64 the
        // callee reads only the registers it needs, extra args are ignored.
        fn objc_msgSend(
            receiver: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            options: u64,
            reason: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }
    // extern "C" fns returning NSUInteger / BOOL (arm64: x0 / w0).
    extern "C" fn ret_occlusion_visible(
        _self: *mut std::ffi::c_void,
        _cmd: *mut std::ffi::c_void,
    ) -> u64 {
        let _ = (_self, _cmd);
        2 // NSWindowOcclusionStateVisible
    }
    extern "C" fn ret_true(_self: *mut std::ffi::c_void, _cmd: *mut std::ffi::c_void) -> u8 {
        let _ = (_self, _cmd);
        1
    }
    type OcclusionImp = extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> u64;
    type BoolImp = extern "C" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> u8;
    let ns_window = match window.ns_window() {
        Ok(p) if !p.is_null() => p,
        _ => return false,
    };
    let occ_imp: OcclusionImp = ret_occlusion_visible;
    let bool_imp: BoolImp = ret_true;
    // IMPs are passed to the ObjC runtime as raw pointers; a plain `as` cast
    // is the clippy-preferred alternative to `mem::transmute` here.
    let occ_imp_ptr = occ_imp as *mut std::ffi::c_void;
    let bool_imp_ptr = bool_imp as *mut std::ffi::c_void;
    unsafe {
        let cls = object_getClass(ns_window);
        let mut window_lies: Vec<(&str, &str)> = Vec::new();
        if want("occlusion") {
            window_lies.push(("occlusionState", "Q@:"));
        }
        if want("key") {
            window_lies.extend([
                ("isKeyWindow", "c@:"),
                ("isMainWindow", "c@:"),
                ("isVisible", "c@:"),
                ("canBecomeKeyWindow", "c@:"),
            ]);
        }
        for (name, types) in window_lies {
            let sel_name = match std::ffi::CString::new(name) {
                Ok(s) => s,
                Err(_) => return false,
            };
            let sel = sel_registerName(sel_name.as_ptr());
            let t = match std::ffi::CString::new(types) {
                Ok(s) => s,
                Err(_) => return false,
            };
            let imp = if types == "Q@:" {
                occ_imp_ptr
            } else {
                bool_imp_ptr
            };
            class_replaceMethod(cls, sel, imp, t.as_ptr());
        }
        // WebKit drops web mouse events while the owning application is not
        // active ([NSApp isActive]). Lie the same way for the automation run:
        // replace -isActive on the NSApplication class (single instance).
        if want("app") {
            let nsapp_cls = objc_getClass(c"NSApplication".as_ptr());
            if !nsapp_cls.is_null() {
                let sel_shared = sel_registerName(c"sharedApplication".as_ptr());
                let nsapp = objc_msgSend(nsapp_cls, sel_shared, 0, std::ptr::null_mut());
                if !nsapp.is_null() {
                    let sel_active = sel_registerName(c"isActive".as_ptr());
                    let t = match std::ffi::CString::new("c@:") {
                        Ok(s) => s,
                        Err(_) => return false,
                    };
                    class_replaceMethod(
                        object_getClass(nsapp),
                        sel_active,
                        bool_imp_ptr,
                        t.as_ptr(),
                    );
                }
            }
        }
        // First-click swallowing: see the stage docs above.
        if want("firstmouse") {
            let sel_fm = sel_registerName(c"acceptsFirstMouse:".as_ptr());
            let t = match std::ffi::CString::new("c@:@") {
                Ok(s) => s,
                Err(_) => return false,
            };
            // the webview's own class (wry's WryWebView override wins over base)
            if let Ok(ns_view) = window.ns_view() {
                if !ns_view.is_null() {
                    class_replaceMethod(object_getClass(ns_view), sel_fm, bool_imp_ptr, t.as_ptr());
                }
            }
            // and the NSView base, for private subviews that hitTest may return
            let nsv_cls = objc_getClass(c"NSView".as_ptr());
            if !nsv_cls.is_null() {
                class_replaceMethod(nsv_cls, sel_fm, bool_imp_ptr, t.as_ptr());
            }
        }
        true
    }
}

/// DEV-ONLY: make the window immune to AppKit constrainFrameRect: (macOS).
/// Modern AppKit clamps ANY frame change of a visible (ordered) window back
/// into the union of screens, including raw setFrameOrigin: and borderless
/// windows — so the window can never be parked off-screen while clickable.
/// Fix: replace constrainFrameRect:toScreen: on the window's own class with
/// an identity trampoline. Class identity is untouched (object_setClass was
/// tried and aborted AppKit's NSDynamicProperties assertion); the process
/// has a single Tauri window, so the class-wide scope is acceptable for a
/// dev-only automation mode.
#[cfg(debug_assertions)]
#[cfg(target_os = "macos")]
fn disable_window_frame_constrain(window: &tauri::WebviewWindow) -> bool {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NsRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    extern "C" {
        fn sel_registerName(name: *const std::os::raw::c_char) -> *mut std::ffi::c_void;
        fn class_replaceMethod(
            cls: *mut std::ffi::c_void,
            sel: *mut std::ffi::c_void,
            imp: *mut std::ffi::c_void,
            types: *const std::os::raw::c_char,
        ) -> *mut std::ffi::c_void;
        fn object_getClass(obj: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }
    // - (NSRect)constrainFrameRect:(NSRect)frame toScreen:(NSScreen *)screen
    // Identity trampoline: NSRect HFA passes/returns via d0-d3 on arm64.
    extern "C" fn constrain_identity(
        _self: *mut std::ffi::c_void,
        _cmd: *mut std::ffi::c_void,
        frame: NsRect,
        _screen: *mut std::ffi::c_void,
    ) -> NsRect {
        let _ = (_self, _cmd, _screen);
        frame
    }
    let ns_window = match window.ns_window() {
        Ok(p) if !p.is_null() => p,
        _ => return false,
    };
    unsafe {
        let sel = match std::ffi::CString::new("constrainFrameRect:toScreen:") {
            Ok(s) => s,
            Err(_) => return false,
        };
        let selector = sel_registerName(sel.as_ptr());
        if selector.is_null() {
            return false;
        }
        let types = match std::ffi::CString::new(
            "{NSRect={NSPoint=dd}{NSSize=dd}}@0:0{NSRect={NSPoint=dd}{NSSize=dd}}@:",
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        type ConstrainImp = extern "C" fn(
            *mut std::ffi::c_void,
            *mut std::ffi::c_void,
            NsRect,
            *mut std::ffi::c_void,
        ) -> NsRect;
        let imp: ConstrainImp = constrain_identity;
        // NULL previous IMP means the method was absent on this class and has
        // been ADDED (class_addMethod semantics) — e.g. the window class is a
        // KVO subclass and constrainFrameRect: lives on a superclass. Both
        // outcomes install the identity trampoline on this class.
        let _prev = class_replaceMethod(
            object_getClass(ns_window),
            selector,
            imp as *mut std::ffi::c_void,
            types.as_ptr(),
        );
        true
    }
}

/// Dev-log event tags for stderr diagnostics. These are dev/diagnostic lines
/// (not localized UI text), emitted as machine tag + detail via `{}`-formatter
/// `eprintln!` — the shape the no-hardcoded-user-strings guard accepts for
/// structured dev logs — so the diagnostics stay on stderr without failing CI.
const DEV_LOG_CONTRACT_ROOT_INIT_FAILED: &str = "contract_root_init_failed";
// Used only inside the attribute-gated off-screen setup block below, so the
// constant must live under the same gate (otherwise it is dead code in
// release/non-macOS profiles).
#[cfg(all(debug_assertions, target_os = "macos"))]
const DEV_LOG_WINDOW_ORIGIN_INVALID: &str = "window_origin_env_invalid";

fn main() {
    let _ = color_eyre::install();
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    let builder = match contract_adapter::attach_contract(
        builder,
        contract_adapter::default_managed_root(),
    ) {
        Ok(b) => b,
        Err(e) => {
            // Dev-log: this fires before any window exists, so stderr is the
            // only channel (see DEV_LOG_* note above).
            eprintln!("{} {}", DEV_LOG_CONTRACT_ROOT_INIT_FAILED, e);
            return;
        }
    };
    let builder = if legacy_commands_enabled() {
        builder.invoke_handler(tauri::generate_handler![
            // binding contract (always registered)
            rimloc_gui_lib::contract_adapter::contract_handshake,
            rimloc_gui_lib::contract_adapter::project_create,
            rimloc_gui_lib::contract_adapter::project_open,
            rimloc_gui_lib::contract_adapter::project_list,
            rimloc_gui_lib::contract_adapter::project_snapshot,
            rimloc_gui_lib::contract_adapter::project_apply_intents,
            rimloc_gui_lib::contract_adapter::project_refresh,
            rimloc_gui_lib::contract_adapter::project_cancel_next,
            rimloc_gui_lib::contract_adapter::project_validate,
            rimloc_gui_lib::contract_adapter::project_export,
            rimloc_gui_lib::contract_adapter::project_build_mod,
            rimloc_gui_lib::contract_adapter::project_diagnose,
            // legacy surface (operator opt-in only, RIMLOC_LEGACY_COMMANDS=1)
            get_app_info,
            scan_mod,
            learn_defs,
            export_po,
            validate_mod,
            xml_health,
            import_po,
            build_mod,
            diff_xml_cmd,
            lang_update_cmd,
            annotate_cmd,
            init_lang_cmd,
            get_log_info,
            pick_directory,
            save_text_via_dialog,
            log_message,
            open_path,
            set_debug_options,
            get_diagnostics,
            collect_diagnostics_via_dialog,
            simulate_error,
            simulate_panic,
            morph_cmd,
            learn_keyed_cmd,
            dump_schemas,
            get_profile,
            validate_po_gui,
            learn_patches_cmd,
            get_cli_i18n,
            apply_translation,
            load_tm,
            scan_strings_gui,
            load_plugin_cmd,
            list_plugins_cmd,
            coverage_gui,
            export_xliff_gui,
            import_xliff_gui,
            merge_keyed_gui,
            // selfloc entry: safe read-only shell extra (live too)
            selfloc_catalog_dir
        ])
    } else {
        builder.invoke_handler(tauri::generate_handler![
            // binding contract (wave 2) — production live entry
            rimloc_gui_lib::contract_adapter::contract_handshake,
            rimloc_gui_lib::contract_adapter::project_create,
            rimloc_gui_lib::contract_adapter::project_open,
            rimloc_gui_lib::contract_adapter::project_list,
            rimloc_gui_lib::contract_adapter::project_snapshot,
            rimloc_gui_lib::contract_adapter::project_apply_intents,
            rimloc_gui_lib::contract_adapter::project_refresh,
            rimloc_gui_lib::contract_adapter::project_cancel_next,
            // final night wave: validate/build/diagnostics over the contract
            rimloc_gui_lib::contract_adapter::project_validate,
            rimloc_gui_lib::contract_adapter::project_export,
            rimloc_gui_lib::contract_adapter::project_build_mod,
            rimloc_gui_lib::contract_adapter::project_diagnose,
            // safe read-only legacy extras (until contract analogs land)
            get_app_info,
            scan_mod,
            scan_strings_gui,
            validate_mod,
            validate_po_gui,
            xml_health,
            coverage_gui,
            diff_xml_cmd,
            get_cli_i18n,
            pick_directory,
            // selfloc entry (mandate D): resolve the app-bundled UI catalog
            // dir — read-only shell extra, same class as pick_directory.
            selfloc_catalog_dir
        ])
    };
    builder
        .setup(|app| {
            // Prepare log + profile paths
            let base = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| dirs::data_dir().unwrap_or_else(std::env::temp_dir));
            // keep our fixed app folder name for consistency across OSes
            let logs_dir = base.join("RimLoc").join("logs");
            let log_path = logs_dir.join("gui.log");
            let profile_path = logs_dir.join("profile.jsonl");
            let _ = std::fs::create_dir_all(&logs_dir);
            // Write a startup banner
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = writeln!(f, "=== RimLoc GUI start v{} ===", env!("CARGO_PKG_VERSION"));
            }
            // ensure profile file exists
            let _ = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&profile_path);
            app.manage(LogState {
                path: log_path.clone(),
            });
            let main_window = app.get_webview_window("main");
            if let Some(window) = main_window {
                // DEV-ONLY background automation: RIMLOC_WINDOW_ORIGIN="x,y"
                // relocates the window off-screen (e.g. "-3000,-3000") so an
                // automation driver can click it without ever appearing on the
                // owner's display. Guarded by cfg!(debug_assertions): release
                // builds ignore the variable entirely.
                //
                // RIMLOC_WINDOW_MOVE selects the relocation method:
                //   "borderless" — set_decorations(false) first: AppKit's
                //     default constrainFrameRect: does not clamp borderless
                //     windows, so the window stays ordered (WebContent keeps
                //     rendering, synthetic events hit-test normally) while
                //     fully off-screen. Title bar loss is dev-only cosmetics.
                //   "tauri" — tauri set_position() → setFrameTopLeftPoint:,
                //     which constrainFrameRect: clamps back on-screen (kept
                //     only as the documented-clamped baseline).
                //   "hide" — orderOut then tauri set_position (ordered-out
                //     windows move freely, but WKWebView event routing to a
                //     hidden window is not guaranteed).
                //
                // Attribute-gated (NOT cfg!()): cfg!() is a runtime check and
                // its body compiles in every profile, while the hooks called
                // inside are #[cfg]-removed from release/non-macOS builds —
                // that combination once broke `cargo check --release` with
                // six E0425s. The attribute removes the whole block where the
                // hooks do not exist.
                #[cfg(all(debug_assertions, target_os = "macos"))]
                if cfg!(debug_assertions) {
                    if let Ok(origin) = std::env::var("RIMLOC_WINDOW_ORIGIN") {
                        let parsed = origin.split_once(',').and_then(|(a, b)| {
                            let x = a.trim().parse::<f64>().ok()?;
                            let y = b.trim().parse::<f64>().ok()?;
                            Some((x, y))
                        });
                        match parsed {
                            Some((x, y)) => {
                                let mode = std::env::var("RIMLOC_WINDOW_MOVE")
                                    .unwrap_or_else(|_| "swizzle".to_string());
                                let moved = match mode.as_str() {
                                    "swizzle" => {
                                        let swizzled = disable_window_frame_constrain(&window);
                                        let faked = fake_window_visibility(&window);
                                        let pos = window
                                            .set_position(tauri::LogicalPosition::new(x, y))
                                            .is_ok();
                                        eprintln!(
                                            "rimloc-gui: swizzle swizzled={swizzled} faked={faked} \
                                             pos={pos} outer={:?} size={:?} class={}",
                                            window.outer_position(),
                                            window.outer_size(),
                                            ns_window_class_name(&window),
                                        );
                                        swizzled && faked && pos
                                    }
                                    _ => false,
                                };
                                if !moved {
                                    let _ = window
                                        .set_position(tauri::LogicalPosition::new(x, y));
                                }
                                eprintln!(
                                    "rimloc-gui: RIMLOC_WINDOW_ORIGIN=({x},{y}) mode={mode} moved={moved}"
                                );
                                // App Nap opt-out FIRST: a napped
                                // process stops answering AXWindows and
                                // the AX tree dies mid-journey.
                                dev_disable_app_nap();
                                // Make the app's accessibility server
                                // start deterministically: off-screen the
                                // AX bridge is lazy and sometimes never
                                // hydrates on client queries alone, which
                                // breaks AXPress-driven automation.
                                dev_accessibility_activate();
                                // Keep the window parked: some WebKit
                                // interactions (AXPress navigation,
                                // scroll-to-reveal) nudge the window frame
                                // after the initial move.
                                let park = window.clone();
                                std::thread::spawn(move || {
                                    // macOS 27 drops off-screen windows from
                                    // the app's AXWindows report seconds after
                                    // launch (measured 2026-09-27) and a
                                    // same-position re-assert is a no-op that
                                    // does NOT re-register. A REAL 2px frame
                                    // change re-registers the window with AX,
                                    // so park ticks alternate x by 2px and run
                                    // every second — the driver's 1.5s poll
                                    // then always finds a live AX window.
                                    let mut tick: u32 = 0;
                                    loop {
                                        std::thread::sleep(
                                            std::time::Duration::from_millis(1000),
                                        );
                                        tick += 1;
                                        // keep re-asserting the AX registration:
                                        // WebKit page loads can drop it, and an
                                        // unhydrated AX bridge breaks automation
                                        dev_accessibility_activate();
                                        let px =
                                            if tick.is_multiple_of(2) { x } else { x - 2.0 };
                                        let _ = park.set_position(
                                            tauri::LogicalPosition::new(px, y),
                                        );
                                    }
                                });
                            }
                            None => {
                                // Dev-log (see DEV_LOG_* note above): bad env
                                // payload in the dev-only off-screen mode.
                                let detail = format!("origin={origin:?} want=\"x,y\"");
                                eprintln!("{} {}", DEV_LOG_WINDOW_ORIGIN_INVALID, detail);
                            }
                        }
                    }
                }
                let _ = window.emit(
                    "app-info",
                    AppInfo {
                        version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                );
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogInfo {
    log_path: String,
}

#[tauri::command]
fn get_log_info(state: State<LogState>) -> Result<LogInfo, ApiError> {
    Ok(LogInfo {
        log_path: state.path.display().to_string(),
    })
}

#[tauri::command]
fn pick_directory(window: Window, initial: Option<String>) -> Result<Option<String>, ApiError> {
    let mut builder = window.dialog().file();
    if let Some(init) = initial.as_deref() {
        let p = PathBuf::from(init);
        builder = builder.set_directory(p);
    }
    let picked = builder
        .blocking_pick_folder()
        .map(|p| p.simplified().to_string());
    Ok(picked)
}

/// Self-localization entry (mandate D): resolve the app-bundled RimLoc UI
/// catalog as an ORDINARY project source directory. Thin shell over
/// [`rimloc_gui_lib::selfloc_catalog::resolve_catalog_dir`] (resource
/// candidates → idempotent app-data copy → project dir whose basename IS the
/// project display name "RimLoc UI (en)"). Read-only extra: no dialog, no
/// caller-chosen paths — total failure is a typed refusal, never a guess.
#[tauri::command]
fn selfloc_catalog_dir(app: tauri::AppHandle) -> Result<String, ApiError> {
    use tauri::Manager;
    let resource_dir = app.path().resource_dir().ok();
    let app_data = app.path().app_data_dir().unwrap_or_else(|_| {
        dirs::data_dir()
            .map(|d| d.join("com.rimloc.gui"))
            .unwrap_or_else(std::env::temp_dir)
    });
    rimloc_gui_lib::selfloc_catalog::resolve_catalog_dir(resource_dir, &app_data)
        .map_err(|e| ApiError { message: e })
}

/// Save arbitrary text, but the destination is always confirmed by the user
/// through the native save dialog opened on the Rust side. The WebView never
/// supplies a writable path (an XSS could otherwise write anywhere).
#[tauri::command]
fn save_text_via_dialog(
    window: Window,
    default_path: Option<String>,
    content: String,
) -> Result<Option<String>, ApiError> {
    let mut builder = window.dialog().file();
    if let Some(d) = default_path.as_deref() {
        let pb = PathBuf::from(d);
        if let Some(dir) = pb.parent() {
            if dir.is_dir() {
                builder = builder.set_directory(dir);
            }
        }
        if let Some(name) = pb.file_name() {
            builder = builder.set_file_name(name.to_string_lossy().to_string());
        }
    }
    let Some(picked) = builder.blocking_save_file() else {
        return Ok(None);
    };
    let p = PathBuf::from(picked.simplified().to_string());
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, content.as_bytes())?;
    Ok(Some(p.display().to_string()))
}

#[tauri::command]
fn open_path(path: String) -> Result<(), ApiError> {
    let p = PathBuf::from(&path);
    // Never hand WebView-provided strings to a shell interpreter; `open` uses
    // LaunchServices/ShellExecute/xdg-open directly, so path metacharacters are
    // inert. Existence check keeps the command from being probed blindly.
    if !p.exists() {
        return Err(ApiError {
            message: format!("Path does not exist: {path}"),
        });
    }
    open::that_detached(&p).map_err(|e| ApiError {
        message: format!("Failed to open path: {e}"),
    })?;
    Ok(())
}
#[tauri::command]
fn diff_xml_cmd(
    window: Window,
    state: State<LogState>,
    request: DiffXmlRequest,
) -> Result<DiffXmlResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "diff_xml_cmd: root={} src={} trg={}",
            request.root, request.source_lang_dir, request.target_lang_dir
        ),
    );
    let t0 = std::time::Instant::now();
    append_log(
        &state.path,
        "INFO",
        &format!(
            "diff_xml: root={} src={} trg={}",
            request.root, request.source_lang_dir, request.target_lang_dir
        ),
    );
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let baseline = request
        .baseline_po
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let defs = request
        .defs_root
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let out = if request
        .defs_dicts
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || request.type_schema.is_some()
    {
        // Build merged dicts (embedded + files + type schema) and run dict-based diff
        let mut dicts: Vec<rimloc_parsers_xml::DefsDict> = Vec::new();
        dicts.push(rimloc_parsers_xml::load_embedded_defs_dict());
        if let Some(list) = request.defs_dicts.as_ref() {
            for p in list {
                let pp = make_absolute(&scan_root, Path::new(p));
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) {
                    dicts.push(d);
                }
            }
        }
        if let Some(schema) = request.type_schema.as_deref() {
            let pp = make_absolute(&scan_root, Path::new(schema));
            if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) {
                dicts.push(d);
            }
        }
        let merged = rimloc_parsers_xml::merge_defs_dicts(&dicts);
        rimloc_services::diff_xml_with_defs_and_dict(
            &scan_root,
            &request.source_lang_dir,
            &request.target_lang_dir,
            baseline.as_deref(),
            defs.as_deref(),
            &merged.0,
            request.extra_fields.as_deref().unwrap_or(&Vec::new()),
        )?
    } else if let Some(fields) = request.extra_fields.as_ref() {
        rimloc_services::diff_xml_with_defs_and_fields(
            &scan_root,
            &request.source_lang_dir,
            &request.target_lang_dir,
            baseline.as_deref(),
            defs.as_deref(),
            fields,
        )?
    } else if defs.is_some() {
        diff_xml_with_defs(
            &scan_root,
            &request.source_lang_dir,
            &request.target_lang_dir,
            baseline.as_deref(),
            defs.as_deref(),
        )?
    } else {
        diff_xml(
            &scan_root,
            &request.source_lang_dir,
            &request.target_lang_dir,
            baseline.as_deref(),
        )?
    };
    let resp = DiffXmlResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        only_in_mod: out.only_in_mod,
        only_in_translation: out.only_in_translation,
        changed: out.changed,
    };
    if let Some(p) = request.out_json.as_deref() {
        let path = make_absolute(&scan_root, Path::new(p));
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, serde_json::to_vec_pretty(&resp).unwrap_or_default());
    }
    if let Some(dir) = request.out_dir.as_deref() {
        let out_dir = make_absolute(&scan_root, Path::new(dir));
        let diff = rimloc_domain::DiffOutput {
            changed: resp.changed.clone(),
            only_in_mod: resp.only_in_mod.clone(),
            only_in_translation: resp.only_in_translation.clone(),
        };
        let _ = rimloc_services::write_diff_reports(&out_dir, &diff);
    }
    write_profile(
        &state,
        "diff_xml",
        t0,
        serde_json::json!({"only_in_mod": resp.only_in_mod.len(), "only_in_translation": resp.only_in_translation.len(), "changed": resp.changed.len()}),
    );
    Ok(resp)
}

#[tauri::command]
fn lang_update_cmd(
    window: Window,
    state: State<LogState>,
    request: LangUpdateRequest,
) -> Result<LangUpdateResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "lang_update_cmd: root={} repo={} branch={:?}",
            request.root, request.repo, request.branch
        ),
    );
    let _ = &request.game_version; // mark as used
    let t0 = std::time::Instant::now();
    append_log(
        &state.path,
        "INFO",
        &format!(
            "lang_update: repo={} src={} trg={}",
            request.repo, request.source_lang_dir, request.target_lang_dir
        ),
    );
    // Expecting game root (folder containing Data/)
    let mut scan_root = PathBuf::from(&request.root);
    // macOS: allow selecting the .app bundle; resolve to Contents/Resources if needed
    #[cfg(target_os = "macos")]
    {
        let direct = scan_root.join("Data").join("Core").join("Languages");
        let bundled = scan_root
            .join("Contents")
            .join("Resources")
            .join("Data")
            .join("Core")
            .join("Languages");
        if !direct.exists() && bundled.exists() {
            scan_root = scan_root.join("Contents").join("Resources");
        }
    }
    let zip_path = request
        .zip_path
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)));
    let (_plan, summary) = lang_update(
        &scan_root,
        &request.repo,
        request.branch.as_deref(),
        zip_path.as_deref(),
        &request.source_lang_dir,
        &request.target_lang_dir,
        request.dry_run,
        request.backup,
    )?;
    if let Some(s) = summary {
        let resp = LangUpdateResponse {
            files: s.files,
            bytes: s.bytes,
            out_dir: s.out_dir.display().to_string(),
        };
        write_profile(
            &state,
            "lang_update",
            t0,
            serde_json::json!({"files": resp.files, "bytes": resp.bytes }),
        );
        Ok(resp)
    } else {
        let resp = LangUpdateResponse {
            files: 0,
            bytes: 0,
            out_dir: scan_root.join("Data/Core/Languages").display().to_string(),
        };
        write_profile(
            &state,
            "lang_update",
            t0,
            serde_json::json!({"files": 0, "bytes": 0 }),
        );
        Ok(resp)
    }
}

// --- Morph (CLI parity) ---
#[derive(Debug, Deserialize)]
struct MorphRequest {
    root: String,
    target_lang_dir: String,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    filter_key_regex: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    cache_size: Option<usize>,
    #[serde(default)]
    pymorphy_url: Option<String>,
    #[serde(default)]
    morpher_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MorphResponse {
    processed: usize,
    lang: String,
    warn_no_morpher: bool,
    warn_no_pymorphy: bool,
}

#[tauri::command]
fn morph_cmd(
    _window: Window,
    state: State<LogState>,
    request: MorphRequest,
) -> Result<MorphResponse, ApiError> {
    append_log(
        &state.path,
        "INFO",
        &format!(
            "morph: target={} provider={:?}",
            request.target_lang_dir, request.provider
        ),
    );
    let root = PathBuf::from(&request.root);
    let prov = match request.provider.as_deref() {
        Some("morpher") | Some("MorpherApi") => MorphProvider::MorpherApi,
        Some("pymorphy") | Some("Pymorphy2") => MorphProvider::Pymorphy2,
        _ => MorphProvider::Dummy,
    };
    if let Some(tok) = request.morpher_token.as_deref() {
        std::env::set_var("MORPHER_TOKEN", tok);
    }
    let opts = MorphOptions {
        provider: prov,
        target_lang_dir: request.target_lang_dir.clone(),
        filter_key_regex: request.filter_key_regex.clone(),
        limit: request.limit,
        timeout_ms: request.timeout_ms.unwrap_or(1500),
        cache_size: request.cache_size.unwrap_or(1024),
        pymorphy_url: request.pymorphy_url.clone(),
    };
    let res = rimloc_services::morph_generate(&root, &opts).wrap_err("morph generate")?;
    Ok(MorphResponse {
        processed: res.processed,
        lang: res.lang,
        warn_no_morpher: res.warn_no_morpher,
        warn_no_pymorphy: res.warn_no_pymorphy,
    })
}

// --- Learn Keyed ---
#[derive(Debug, Deserialize)]
struct LearnKeyedRequest {
    root: String,
    source_lang_dir: Option<String>,
    target_lang_dir: Option<String>,
    #[serde(default)]
    game_version: Option<String>,
    #[serde(default)]
    dict_files: Option<Vec<String>>,
    #[serde(default)]
    min_len: Option<usize>,
    #[serde(default)]
    blacklist: Option<Vec<String>>,
    #[serde(default)]
    must_contain_letter: bool,
    #[serde(default)]
    exclude_substr: Option<Vec<String>>,
    #[serde(default)]
    threshold: Option<f32>,
    #[serde(default)]
    out_dir: Option<String>,
    #[serde(default)]
    from_defs_special: bool,
    #[serde(default)]
    ml_url: Option<String>,
    #[serde(default)]
    no_ml: bool,
    #[serde(default)]
    retrain: bool,
    #[serde(default)]
    retrain_dict: Option<String>,
    #[serde(default)]
    learned_out: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LearnKeyedResponse {
    processed: usize,
    suggested: String,
    missing: String,
}

#[tauri::command]
fn learn_keyed_cmd(
    _window: Window,
    state: State<LogState>,
    request: LearnKeyedRequest,
) -> Result<LearnKeyedResponse, ApiError> {
    append_log(
        &state.path,
        "INFO",
        &format!(
            "learn_keyed: src={:?} trg={:?}",
            request.source_lang_dir, request.target_lang_dir
        ),
    );
    let root = PathBuf::from(&request.root);
    let (scan_root, _version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let out_dir = request
        .out_dir
        .as_deref()
        .map(|p| make_absolute(&scan_root, Path::new(p)))
        .unwrap_or_else(|| scan_root.join("_learn"));
    std::fs::create_dir_all(&out_dir)?;

    // load dicts
    let mut dicts = Vec::new();
    if let Some(files) = request.dict_files.as_ref() {
        for f in files {
            let pp = PathBuf::from(f);
            if let Ok(d) = rimloc_services::learn::keyed::load_keyed_dict_from_file(&pp) {
                dicts.push(d);
            }
        }
    }
    let src_dir = request
        .source_lang_dir
        .clone()
        .unwrap_or_else(|| "English".to_string());
    let trg_dir = request
        .target_lang_dir
        .clone()
        .unwrap_or_else(|| "Russian".to_string());
    if request.from_defs_special {
        std::env::set_var("RIMLOC_LEARN_KEYED_FROM_DEFS", "1");
    }
    // Classifier
    let mut classifier: Box<dyn rimloc_services::learn::ml::Classifier> = if request.no_ml {
        Box::new(rimloc_services::learn::ml::DummyClassifier::new(1.0))
    } else if let Some(url) = &request.ml_url {
        Box::new(rimloc_services::learn::ml::RestClassifier::new(url.clone()))
    } else {
        Box::new(rimloc_services::learn::ml::DummyClassifier::new(0.9))
    };
    let missing = rimloc_services::learn::keyed::learn_keyed(
        &scan_root,
        &src_dir,
        &trg_dir,
        &dicts,
        request.min_len.unwrap_or(1),
        &request.blacklist.clone().unwrap_or_default(),
        request.must_contain_letter,
        &request.exclude_substr.clone().unwrap_or_default(),
        request.threshold.unwrap_or(0.8),
        classifier.as_mut(),
    )?;

    let learned_out = request
        .learned_out
        .as_deref()
        .map(|p| make_absolute(&out_dir, Path::new(p)));
    // Save learned set for audit
    {
        #[derive(serde::Serialize)]
        #[allow(non_snake_case)]
        struct Row<'a> {
            key: &'a str,
            value: &'a str,
            confidence: f32,
            sourceFile: String,
            learnedAt: String,
        }
        let now = chrono::Utc::now().to_rfc3339();
        let rows: Vec<Row> = missing
            .iter()
            .map(|c| Row {
                key: &c.key,
                value: &c.value,
                confidence: c.confidence.unwrap_or(1.0),
                sourceFile: c.source_file.display().to_string(),
                learnedAt: now.clone(),
            })
            .collect();
        let path = learned_out.unwrap_or_else(|| out_dir.join("learned_keyed.json"));
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, &rows)?;
    }

    // Retrain: update a dict file if requested
    if request.retrain {
        let out_path = if let Some(p) = request.retrain_dict.as_deref() {
            make_absolute(&scan_root, Path::new(p))
        } else {
            out_dir.join("keyed_dict.updated.json")
        };
        #[derive(serde::Serialize)]
        struct KD {
            include: Vec<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            exclude: Option<Vec<String>>,
        }
        let mut include: Vec<String> = Vec::new();
        for c in &missing {
            include.push(format!("^{}$", regex::escape(&c.key)));
        }
        include.sort();
        include.dedup();
        let file = std::fs::File::create(out_path)?;
        serde_json::to_writer_pretty(
            file,
            &KD {
                include,
                exclude: None,
            },
        )?;
    }

    let miss = out_dir.join("missing_keyed.json");
    rimloc_services::learn::keyed::write_keyed_missing_json(&miss, &missing)?;
    let sug = out_dir.join("_SuggestedKeyed.xml");
    rimloc_services::learn::keyed::write_keyed_suggested_xml(&sug, &missing)?;
    Ok(LearnKeyedResponse {
        processed: missing.len(),
        suggested: sug.display().to_string(),
        missing: miss.display().to_string(),
    })
}

// --- Dump JSON Schemas ---
#[derive(Debug, Deserialize)]
struct DumpSchemasRequest {
    out_dir: String,
}
#[tauri::command]
fn dump_schemas(
    _window: Window,
    _state: State<LogState>,
    req: DumpSchemasRequest,
) -> Result<String, ApiError> {
    use std::fs;
    let out_dir = PathBuf::from(&req.out_dir);
    ensure_caller_path_absolute("out_dir", &out_dir)?;
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
    Ok(out_dir.display().to_string())
}

// --- Profiler: return last N entries of profile.jsonl with per-command summary ---
#[derive(Debug, Deserialize)]
struct ProfileRequest {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileEntryOut {
    ts: String,
    command: String,
    duration_ms: u64,
    extra: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileCommandRow {
    command: String,
    count: usize,
    total_ms: u64,
    max_ms: u64,
    avg_ms: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileResponse {
    entries: Vec<ProfileEntryOut>,
    per_command: Vec<ProfileCommandRow>,
}

#[tauri::command]
fn get_profile(state: State<LogState>, req: ProfileRequest) -> Result<ProfileResponse, ApiError> {
    let lim = req.limit.unwrap_or(200).min(5000);
    let mut entries: Vec<ProfileEntryOut> = Vec::new();
    if let Some(dir) = state.path.parent() {
        let file = dir.join("profile.jsonl");
        if file.exists() {
            let content = std::fs::read_to_string(&file)?;
            for line in content.lines().rev().take(lim) {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                    let ts = v
                        .get("ts")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let command = v
                        .get("command")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let duration_ms = v.get("duration_ms").and_then(|x| x.as_u64()).unwrap_or(0);
                    let extra = v.get("extra").cloned().unwrap_or(serde_json::json!({}));
                    entries.push(ProfileEntryOut {
                        ts,
                        command,
                        duration_ms,
                        extra,
                    });
                }
            }
            entries.reverse();
        }
    }
    use std::collections::BTreeMap;
    let mut map: BTreeMap<String, (usize, u64, u64)> = BTreeMap::new();
    for e in &entries {
        let ent = map.entry(e.command.clone()).or_insert((0, 0, 0));
        ent.0 += 1;
        ent.1 += e.duration_ms;
        if e.duration_ms > ent.2 {
            ent.2 = e.duration_ms;
        }
    }
    let mut per_command: Vec<ProfileCommandRow> = map
        .into_iter()
        .map(|(k, (c, tot, max))| ProfileCommandRow {
            command: k,
            count: c,
            total_ms: tot,
            max_ms: max,
            avg_ms: if c == 0 {
                0.0
            } else {
                (tot as f64) / (c as f64)
            },
        })
        .collect();
    per_command.sort_by(|a, b| {
        b.avg_ms
            .partial_cmp(&a.avg_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(ProfileResponse {
        entries,
        per_command,
    })
}

#[tauri::command]
fn annotate_cmd(
    window: Window,
    state: State<LogState>,
    request: AnnotateRequest,
) -> Result<AnnotateResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "annotate_cmd: root={} src={} trg={} dry_run={}",
            request.root, request.source_lang_dir, request.target_lang_dir, request.dry_run
        ),
    );
    let t0 = std::time::Instant::now();
    append_log(
        &state.path,
        "INFO",
        &format!(
            "annotate: src={} trg={}",
            request.source_lang_dir, request.target_lang_dir
        ),
    );
    let root = PathBuf::from(&request.root);
    if request.dry_run {
        let plan = annotate_dry_run_plan(
            &root,
            &request.source_lang_dir,
            &request.target_lang_dir,
            request.comment_prefix.as_deref().unwrap_or("//"),
            request.strip,
        )?;
        let resp = AnnotateResponse {
            processed: plan.processed,
            annotated: plan.total_add,
        };
        write_profile(
            &state,
            "annotate_preview",
            t0,
            serde_json::json!({"processed": resp.processed, "annotated": resp.annotated }),
        );
        Ok(resp)
    } else {
        let s = annotate_apply(
            &root,
            &request.source_lang_dir,
            &request.target_lang_dir,
            request.comment_prefix.as_deref().unwrap_or("//"),
            request.strip,
            false,
            request.backup,
        )?;
        let resp = AnnotateResponse {
            processed: s.processed,
            annotated: s.annotated,
        };
        write_profile(
            &state,
            "annotate_apply",
            t0,
            serde_json::json!({"processed": resp.processed, "annotated": resp.annotated }),
        );
        Ok(resp)
    }
}

#[tauri::command]
fn init_lang_cmd(
    window: Window,
    state: State<LogState>,
    request: InitRequest,
) -> Result<InitResponse, ApiError> {
    emit_log(
        &window,
        &state,
        "debug",
        format!(
            "init_lang_cmd: root={} src={} trg={} overwrite={} dry_run={}",
            request.root,
            request.source_lang_dir,
            request.target_lang_dir,
            request.overwrite,
            request.dry_run
        ),
    );
    let t0 = std::time::Instant::now();
    let root = PathBuf::from(&request.root);
    let plan = make_init_plan(&root, &request.source_lang_dir, &request.target_lang_dir)?;
    let files = write_init_plan(&plan, request.overwrite, request.dry_run)?;
    let resp = InitResponse {
        files,
        out_language: plan.language,
    };
    write_profile(
        &state,
        "init_lang",
        t0,
        serde_json::json!({"files": resp.files }),
    );
    Ok(resp)
}
#[derive(Debug, Deserialize)]
struct ApplyTranslationRequest {
    root: String,
    #[serde(default)]
    lang: Option<String>,
    #[serde(default)]
    lang_dir: Option<String>,
    key: String,
    value: String,
    #[serde(default)]
    file: Option<String>,
}
#[derive(Debug, Serialize)]
struct ApplyTranslationResponse {
    out_path: String,
    total_keys: usize,
}

fn read_language_pairs(path: &Path) -> std::io::Result<Vec<(String, String)>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let s = std::fs::read_to_string(path)?;
    let doc = roxmltree::Document::parse(&s).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut out = Vec::new();
    let root = doc.root_element();
    for child in root.children().filter(|n| n.is_element()) {
        let name = child.tag_name().name().to_string();
        let val = child.text().unwrap_or("").to_string();
        out.push((name, val));
    }
    Ok(out)
}

#[tauri::command]
fn apply_translation(
    request: ApplyTranslationRequest,
) -> Result<ApplyTranslationResponse, ApiError> {
    let root = PathBuf::from(&request.root);
    let lang_dir = request
        .lang_dir
        .clone()
        .or_else(|| {
            request
                .lang
                .as_ref()
                .map(|c| rimloc_import_po::rimworld_lang_dir(c))
        })
        .unwrap_or_else(|| "Russian".to_string());
    // LEGACY hardening (RC K4, P1-2 class): the lang dir is joined into the
    // output path (`Languages/<dir>/Keyed/_Edited.xml`) — `Path::join` with
    // an absolute string replaces the whole prefix and `..` escapes the
    // mod. The strict folder form is the same predicate the services layer
    // enforces on every other mod-tree write; enforce it here too, before
    // any path is built.
    if !rimloc_services::lang_dir_form_ok(&lang_dir) {
        return Err(ApiError {
            message: format!(
                "lang_dir `{lang_dir}` is malformed: expected a plain language-folder name (letters, digits, `_`, `-`)"
            ),
        });
    }
    let out_path = if let Some(f) = request.file.as_deref() {
        make_absolute(&root, Path::new(f))
    } else {
        root.join("Languages")
            .join(&lang_dir)
            .join("Keyed")
            .join("_Edited.xml")
    };
    let mut pairs = read_language_pairs(&out_path).unwrap_or_default();
    // update/insert
    let mut found = false;
    for (k, v) in pairs.iter_mut() {
        if k == &request.key {
            *v = request.value.clone();
            found = true;
            break;
        }
    }
    if !found {
        pairs.push((request.key.clone(), request.value.clone()));
    }
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    rimloc_import_po::write_language_data_xml(&out_path, &pairs).map_err(ApiError::from)?;
    Ok(ApplyTranslationResponse {
        out_path: out_path.display().to_string(),
        total_keys: pairs.len(),
    })
}
