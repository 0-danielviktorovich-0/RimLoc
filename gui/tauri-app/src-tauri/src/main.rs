#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console on Windows in release

use color_eyre::eyre::WrapErr;
use rimloc_domain::{ScanUnit, SCHEMA_VERSION};
use rimloc_export_csv as export_csv;
use rimloc_services::{autodiscover_defs_context, export_po_with_tm, learn};
use rimloc_services::{
    validate_under_root, validate_under_root_with_defs, validate_under_root_with_defs_and_fields,
    xml_health_scan, import_po_to_mod_tree_with_progress, build_from_po_with_progress,
    diff_xml, diff_xml_with_defs, lang_update, annotate_dry_run_plan, annotate_apply,
    make_init_plan, write_init_plan, import_po_to_mod_tree, validate_placeholders_cross_language,
};
use rimloc_services::{MorphOptions, MorphProvider};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
use tauri::{Manager, State, Window};
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;
use thiserror::Error;
use walkdir::WalkDir;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Utc;

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
        let _ = writeln!(f, "[{}][{}] {}: {}", ts, tid, level.to_uppercase(), message.replace('\n', " "));
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
fn log_message(window: Window, state: State<LogState>, level: String, message: String) -> Result<(), ApiError> {
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

fn emit_progress(window: &Window, state: &State<LogState>, action: &str, step: &str, message: Option<String>, pct: Option<u32>) {
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
        append_log(&state.path, "DEBUG", &format!("[{}] {} {}%", action, step, pct.unwrap_or(0)));
        append_log(&state.path, "DEBUG", &msg);
    }
}

fn write_profile(state: &State<LogState>, command: &str, start: std::time::Instant, extra: serde_json::Value) {
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
        append_log(&state.path, "INFO", &format!("debug: backtrace set to {}", bt));
    }
    if let Some(level) = opts.min_level {
        append_log(&state.path, "INFO", &format!("debug: min_level hint {}", level));
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
struct CollectDiagRequest { #[serde(default)] out_path: Option<String> }

#[tauri::command]
fn collect_diagnostics(state: State<LogState>, req: CollectDiagRequest) -> Result<String, ApiError> {
    let base = state.path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::env::temp_dir());
    let out = req.out_path.map(PathBuf::from).unwrap_or_else(|| base.join("diagnostics.txt"));
    let mut buf = String::new();
    buf.push_str(&format!("Diagnostics generated at: {}\n", Utc::now().to_rfc3339()));
    buf.push_str(&format!("App version: {}\n", env!("CARGO_PKG_VERSION")));
    buf.push_str(&format!("OS: {}\nArch: {}\n", std::env::consts::OS, std::env::consts::ARCH));
    buf.push_str(&format!("Tauri: {}\n", tauri::VERSION));
    buf.push_str(&format!("Log file: {}\n\n", state.path.display()));
    if let Ok(log) = std::fs::read_to_string(&state.path) {
        buf.push_str("=== Last 500 lines of gui.log ===\n");
        let lines: Vec<&str> = log.lines().collect();
        let start = lines.len().saturating_sub(500);
        for l in &lines[start..] { buf.push_str(l); buf.push('\n'); }
    }
    if let Some(parent) = out.parent() { let _ = std::fs::create_dir_all(parent); }
    std::fs::write(&out, buf.as_bytes())?;
    Ok(out.display().to_string())
}

#[tauri::command]
fn simulate_error() -> Result<(), ApiError> { Err(ApiError { message: "Simulated error for debug".into() }) }

#[tauri::command]
fn simulate_panic() -> Result<(), ApiError> { std::panic::panic_any(0u8); }

#[tauri::command]
fn scan_mod(window: Window, state: State<LogState>, request: ScanRequest) -> Result<ScanResponse, ApiError> {
    println!("scan_mod invoked: root={} all={} gv={:?}", request.root, request.include_all_versions, request.game_version);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("scan: root={} include_all_versions={}", request.root, request.include_all_versions));
    emit_progress(&window, &state, "scan", "start", Some("Scanning…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(&window, &state, "error", format!("scan: path not found: {}", root.display()));
        return Err(ApiError { message: format!("Path not found: {}", root.display()) });
    }
    if request.include_all_versions {
        let res = run_scan(&root, None, &request);
        if let Ok(ref r) = res {
            emit_log(&window, &state, "info", format!("scan finished (all versions): total={} keyed={} definj={} saved_json={:?} saved_csv={:?}", r.total, r.keyed, r.def_injected, r.saved_json, r.saved_csv));
            emit_progress(&window, &state, "scan", "done", Some("Scan finished".to_string()), Some(100));
        }
        if let Ok(ref r) = res {
            write_profile(&state, "scan_mod", t0, serde_json::json!({"total": r.total, "keyed": r.keyed, "def_injected": r.def_injected}));
        }
        res
    } else {
        let (resolved, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
        let res = run_scan(&resolved, version.as_deref(), &request);
        if let Ok(ref r) = res {
            emit_log(&window, &state, "info", format!("scan finished: {} → total={} keyed={} definj={} saved_json={:?} saved_csv={:?}", resolved.display(), r.total, r.keyed, r.def_injected, r.saved_json, r.saved_csv));
            emit_progress(&window, &state, "scan", "done", Some("Scan finished".to_string()), Some(100));
            write_profile(&state, "scan_mod", t0, serde_json::json!({"total": r.total, "keyed": r.keyed, "def_injected": r.def_injected}));
        }
        res
    }
}

fn run_scan(scan_root: &Path, version: Option<&str>, request: &ScanRequest) -> Result<ScanResponse, ApiError> {
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

    // Env gates for inheritance/nested keyed
    if request.no_inherit {
        std::env::set_var("RIMLOC_INHERIT", "0");
    }
    if request.keyed_nested {
        std::env::set_var("RIMLOC_KEYED_NESTED", "1");
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

fn write_scan_csv(path: &Path, units: &[rimloc_services::TransUnit], lang: Option<&str>) -> Result<(), ApiError> {
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
fn learn_defs(window: Window, state: State<LogState>, request: LearnDefsRequest) -> Result<LearnDefsResponse, ApiError> {
    println!("learn_defs invoked: root={} gv={:?}", request.root, request.game_version);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("learn: root={}", request.root));
    emit_progress(&window, &state, "learn", "start", Some("Preparing…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(&window, &state, "error", format!("learn: path not found: {}", root.display()));
        return Err(ApiError { message: format!("Path not found: {}", root.display()) });
    }
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let out_dir_raw = request
        .out_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("_learn"));
    let out_dir = make_absolute(&scan_root, &out_dir_raw);
    std::fs::create_dir_all(&out_dir)?;

    emit_progress(&window, &state, "learn", "discover", Some("Discovering context…".to_string()), Some(10));
    let auto = autodiscover_defs_context(&scan_root).wrap_err("discover defs context")?;
    let opts = learn::LearnOptions {
        mod_root: scan_root.clone(),
        defs_root: None,
        dict_files: auto.dict_sources.clone(),
        model_path: None,
        ml_url: None,
        lang_dir: request
            .lang_dir
            .clone()
            .unwrap_or_else(|| "English".to_string()),
        threshold: request.threshold.unwrap_or(0.8),
        no_ml: true,
        retrain: false,
        retrain_dict: None,
        min_len: 1,
        blacklist: Vec::new(),
        out_dir: out_dir.clone(),
        learned_out: None,
    };

    emit_log(&window, &state, "debug", format!("learn options: out_dir={}", out_dir.display()));
    emit_progress(&window, &state, "learn", "learn", Some("Learning templates…".to_string()), Some(60));
    let result = learn::learn_defs(&opts).wrap_err("learn defs")?;
    let learned_path = out_dir.join("learned_defs.json");
    emit_progress(&window, &state, "learn", "done", Some("Learn finished".to_string()), Some(100));

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
    emit_log(&window, &state, "info", format!("learn finished: accepted {}/{} → out_dir={}", response.accepted, response.candidates, response.out_dir));
    write_profile(&state, "learn_defs", t0, serde_json::json!({"accepted": response.accepted, "candidates": response.candidates}));
    Ok(response)
}

#[tauri::command]
fn export_po(window: Window, state: State<LogState>, request: ExportPoRequest) -> Result<ExportPoResponse, ApiError> {
    println!("export_po invoked: root={} out_po={}", request.root, request.out_po);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("export_po: root={} out_po={}", request.root, request.out_po));
    emit_progress(&window, &state, "export", "start", Some("Exporting…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(&window, &state, "error", format!("export_po: path not found: {}", root.display()));
        return Err(ApiError { message: format!("Path not found: {}", root.display()) });
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

    emit_progress(&window, &state, "export", "collect", Some("Collecting units…".to_string()), Some(20));
    let stats = export_po_with_tm(
        &scan_root,
        &out_po_path,
        if request.pot { None } else { request.lang.as_deref() },
        request.source_lang.as_deref(),
        request.source_lang_dir.as_deref(),
        tm_paths.as_ref().map(|v| v.as_slice()),
    )
    .wrap_err("export po")?;

    let tm_coverage_pct = if stats.total == 0 {
        0
    } else {
        ((stats.tm_filled as f64 / stats.total as f64) * 100.0).round() as u32
    };

    let source_dir = request
        .source_lang_dir
        .clone()
        .or_else(|| request.source_lang.clone().map(|c| rimloc_import_po::rimworld_lang_dir(&c)))
        .unwrap_or_else(|| "English".to_string());
    let warning = def_injected_warning(&scan_root, &source_dir);
    if let Some(ref w) = warning {
        emit_log(&window, &state, "warn", w.clone());
    }
    emit_progress(&window, &state, "export", "done", Some("Export finished".to_string()), Some(100));

    let resp = ExportPoResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        out_po: out_po_path.display().to_string(),
        total: stats.total,
        tm_filled: stats.tm_filled,
        tm_coverage_pct,
        warning,
    };
    emit_log(&window, &state, "info", format!("export finished: {} → total={} tm_filled={} ({}%)", resp.out_po, resp.total, resp.tm_filled, resp.tm_coverage_pct));
    write_profile(&state, "export_po", t0, serde_json::json!({"total": resp.total, "tm_filled": resp.tm_filled}));
    Ok(resp)
}

#[tauri::command]
fn validate_mod(window: Window, state: State<LogState>, request: ValidateRequest) -> Result<ValidateResponse, ApiError> {
    println!("validate_mod invoked: root={} defs_root={:?}", request.root, request.defs_root);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("validate: root={}", request.root));
    emit_progress(&window, &state, "validate", "start", Some("Validating…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = if request.include_all_versions {
        (root.clone(), None)
    } else {
        resolve_game_version_root(&root, request.game_version.as_deref())?
    };
    let defs_root = request.defs_root.as_deref().map(|p| make_absolute(&scan_root, Path::new(p)));

    // If dicts and/or type schema provided, merge dicts and call validate_with_defs_and_dict
    let mut msgs_raw = if request.defs_dicts.as_ref().map(|v| !v.is_empty()).unwrap_or(false)
        || request.defs_type_schema.as_ref().is_some()
    {
        let mut dicts: Vec<rimloc_parsers_xml::DefsDict> = Vec::new();
        dicts.push(rimloc_parsers_xml::load_embedded_defs_dict());
        if let Some(list) = request.defs_dicts.as_ref() {
            for p in list {
                let pp = make_absolute(&scan_root, Path::new(p));
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) { dicts.push(d); }
            }
        }
        if let Some(schema) = request.defs_type_schema.as_deref() {
            let pp = make_absolute(&scan_root, Path::new(schema));
            if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) { dicts.push(d); }
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
            .or_else(|| request.source_lang.clone().map(|c| rimloc_import_po::rimworld_lang_dir(&c)))
            .unwrap_or_else(|| "English".to_string());
        let tgt_dir = request
            .target_lang_dir
            .clone()
            .or_else(|| request.target_lang.clone().map(|c| rimloc_import_po::rimworld_lang_dir(&c)))
            .unwrap_or_else(|| "Russian".to_string());
        if let Ok(mut extra) = validate_placeholders_cross_language(&scan_root, &src_dir, &tgt_dir, defs_root.as_deref()) {
            msgs_raw.append(&mut extra);
        }
    }
    let msgs: Vec<ValidationMessageView> = msgs_raw
        .into_iter()
        .map(|m| ValidationMessageView { kind: m.kind, key: m.key, path: m.path, line: m.line, message: m.message })
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
    emit_progress(&window, &state, "validate", "done", Some("Validation finished".to_string()), Some(100));
    if let Some(out) = request.out_json.as_deref() {
        let path = make_absolute(&scan_root, Path::new(out));
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(&path, serde_json::to_vec_pretty(&msgs).unwrap_or_default());
    }
    write_profile(&state, "validate", t0, serde_json::json!({"total": msgs.len(), "errors": errors, "warnings": warnings, "infos": infos }));
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
fn xml_health(window: Window, state: State<LogState>, request: XmlHealthRequest) -> Result<XmlHealthResponse, ApiError> {
    println!("xml_health invoked: root={} lang_dir={:?}", request.root, request.lang_dir);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("xml_health: root={}", request.root));
    emit_progress(&window, &state, "health", "start", Some("Checking XML…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let lang_dir = request
        .lang_dir
        .clone()
        .or_else(|| request.lang.clone().map(|c| rimloc_import_po::rimworld_lang_dir(&c)))
        .unwrap_or_else(|| "English".to_string());
    let mut report = xml_health_scan(&scan_root, Some(&lang_dir)).wrap_err("xml health")?;
    // Optional filtering like CLI options
    if let Some(only) = request.only.as_ref() {
        if !only.is_empty() {
            report.issues.retain(|i| only.iter().any(|k| k.eq_ignore_ascii_case(&i.category)));
        }
    }
    if let Some(except) = request.except.as_ref() {
        if !except.is_empty() {
            report.issues.retain(|i| !except.iter().any(|k| k.eq_ignore_ascii_case(&i.category)));
        }
    }
    if request.strict && !report.issues.is_empty() {
        emit_log(&window, &state, "warn", format!("XML health strict: {} issues detected", report.issues.len()));
    }
    emit_progress(&window, &state, "health", "done", Some("XML check finished".to_string()), Some(100));
    let out = XmlHealthResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        checked: report.checked,
        issues: report.issues,
    };
    if let Some(path_str) = request.out_json.as_deref() {
        let p = make_absolute(&scan_root, Path::new(path_str));
        if let Some(parent) = p.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(&p, serde_json::to_vec_pretty(&out).unwrap_or_default());
    }
    write_profile(&state, "xml_health", t0, serde_json::json!({"checked": out.checked, "issues": out.issues.len()}));
    Ok(out)
}

#[tauri::command]
fn import_po(window: Window, state: State<LogState>, request: ImportPoRequest) -> Result<ImportPoResponse, ApiError> {
    println!("import_po invoked: root={} po={}", request.root, request.po_path);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("import_po: root={} po={}", request.root, request.po_path));
    emit_progress(&window, &state, "import", "start", Some("Importing PO…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let po_path = make_absolute(&scan_root, Path::new(&request.po_path));
    let lang_dir = request
        .lang_dir
        .clone()
        .or_else(|| request.lang.clone().map(|c| rimloc_import_po::rimworld_lang_dir(&c)))
        .unwrap_or_else(|| "English".to_string());
    let summary = if let Some(out_xml) = request.out_xml.as_deref() {
        let outp = make_absolute(&scan_root, Path::new(out_xml));
        rimloc_services::import_po_to_file(&po_path, &outp, request.keep_empty, request.dry_run, request.backup)
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
        summary.unwrap_or(rimloc_services::ImportSummary { mode: "dry_run".into(), created: 0, updated: 0, skipped: 0, keys: 0, files: vec![] })
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
                emit_progress(&window, &state, "import", "file", Some(path.display().to_string()), Some(((cur as f64 / total as f64) * 100.0).round() as u32));
            },
        )
        .wrap_err("import po")?
    };
    emit_progress(&window, &state, "import", "done", Some("Import finished".to_string()), Some(100));
    let resp = ImportPoResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        lang_dir,
        created: summary.created,
        updated: summary.updated,
        skipped: summary.skipped,
        keys: summary.keys,
    };
    write_profile(&state, "import_po", t0, serde_json::json!({"created": resp.created, "updated": resp.updated, "skipped": resp.skipped, "keys": resp.keys}));
    Ok(resp)
}

#[tauri::command]
fn build_mod(window: Window, state: State<LogState>, request: BuildModRequest) -> Result<BuildModResponse, ApiError> {
    println!("build_mod invoked: po={} out={}", request.po_path, request.out_mod);
    let t0 = std::time::Instant::now();
    emit_log(&window, &state, "info", format!("build_mod from PO: {}", request.po_path));
    emit_progress(&window, &state, "build", "start", Some("Building mod…".to_string()), Some(0));
    let out = PathBuf::from(&request.out_mod);
    let mut files_count = 0usize;
    let mut total_keys = 0usize;
    if let Some(from_root) = request.from_root.as_deref() {
        let root = PathBuf::from(from_root);
        let versions = request.from_game_versions.as_ref().map(|v| v.as_slice());
        if request.dry_run {
            let (files, total) = rimloc_services::build_from_root(&root, &out, &request.lang_dir, versions, false, request.dedupe)?;
            files_count = files.len();
            total_keys = total;
        } else {
            let (files, total) = rimloc_services::build_from_root_with_progress(&root, &out, &request.lang_dir, versions, true, request.dedupe, |cur, total, path| {
                emit_progress(&window, &state, "build", "file", Some(path.display().to_string()), Some(((cur as f64 / total as f64) * 100.0).round() as u32));
            })?;
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
                emit_progress(&window, &state, "build", "file", Some(path.display().to_string()), Some(((cur as f64 / total as f64) * 100.0).round() as u32));
            },
        )
        .wrap_err("build mod from po")?;
    }
    emit_progress(&window, &state, "build", "done", Some("Build finished".to_string()), Some(100));
    let resp = BuildModResponse { out_mod: out.display().to_string(), files: files_count, total_keys };
    write_profile(&state, "build_mod", t0, serde_json::json!({"files": resp.files }));
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
        if entry.file_type().is_file() {
            if entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("xml"))
                .unwrap_or(false)
            {
                return true;
            }
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
        Err(err) => return Err(ApiError {
            message: err.to_string(),
        }),
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
    if parts.is_empty() { None } else { Some(parts) }
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

fn main() {
    let _ = color_eyre::install();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Prepare log + profile paths
            let base = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| dirs::data_dir().unwrap_or_else(|| std::env::temp_dir()));
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
            let _ = OpenOptions::new().create(true).append(true).open(&profile_path);
            app.manage(LogState { path: log_path.clone() });
            let main_window = app.get_webview_window("main");
            if let Some(window) = main_window {
                let _ = window.emit("app-info", AppInfo {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            save_text_file,
            log_message,
            open_path,
            set_debug_options,
            get_diagnostics,
            collect_diagnostics,
            simulate_error,
            simulate_panic
            ,morph_cmd
            ,learn_keyed_cmd
            ,dump_schemas
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogInfo { log_path: String }

#[tauri::command]
fn get_log_info(state: State<LogState>) -> Result<LogInfo, ApiError> {
    Ok(LogInfo { log_path: state.path.display().to_string() })
}

#[tauri::command]
fn pick_directory(window: Window, initial: Option<String>) -> Result<Option<String>, ApiError> {
    let mut builder = window.dialog().file();
    if let Some(init) = initial.as_deref() {
        let p = PathBuf::from(init);
        builder = builder.set_directory(p);
    }
    let picked = builder.blocking_pick_folder().map(|p| p.simplified().to_string());
    Ok(picked)
}

#[tauri::command]
fn save_text_file(path: String, content: String) -> Result<String, ApiError> {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
    std::fs::write(&p, content.as_bytes())?;
    Ok(p.display().to_string())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), ApiError> {
    let p = PathBuf::from(path);
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(&p).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(&p).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/C", "start", "", &p.display().to_string()]).spawn()?;
    }
    Ok(())
}
#[tauri::command]
fn diff_xml_cmd(_window: Window, state: State<LogState>, request: DiffXmlRequest) -> Result<DiffXmlResponse, ApiError> {
    println!("diff_xml_cmd invoked: root={} src={} trg={}", request.root, request.source_lang_dir, request.target_lang_dir);
    let t0 = std::time::Instant::now();
    append_log(&state.path, "INFO", &format!("diff_xml: root={} src={} trg={}", request.root, request.source_lang_dir, request.target_lang_dir));
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let baseline = request.baseline_po.as_deref().map(|p| make_absolute(&scan_root, Path::new(p)));
    let defs = request.defs_root.as_deref().map(|p| make_absolute(&scan_root, Path::new(p)));
    let out = if request.defs_dicts.as_ref().map(|v| !v.is_empty()).unwrap_or(false)
        || request.type_schema.is_some()
    {
        // Build merged dicts (embedded + files + type schema) and run dict-based diff
        let mut dicts: Vec<rimloc_parsers_xml::DefsDict> = Vec::new();
        dicts.push(rimloc_parsers_xml::load_embedded_defs_dict());
        if let Some(list) = request.defs_dicts.as_ref() {
            for p in list {
                let pp = make_absolute(&scan_root, Path::new(p));
                if let Ok(d) = rimloc_parsers_xml::load_defs_dict_from_file(&pp) { dicts.push(d); }
            }
        }
        if let Some(schema) = request.type_schema.as_deref() {
            let pp = make_absolute(&scan_root, Path::new(schema));
            if let Ok(d) = rimloc_parsers_xml::load_type_schema_as_dict(&pp) { dicts.push(d); }
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
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(&path, serde_json::to_vec_pretty(&resp).unwrap_or_default());
    }
    write_profile(&state, "diff_xml", t0, serde_json::json!({"only_in_mod": resp.only_in_mod.len(), "only_in_translation": resp.only_in_translation.len(), "changed": resp.changed.len()}));
    Ok(resp)
}

#[tauri::command]
fn lang_update_cmd(_window: Window, state: State<LogState>, request: LangUpdateRequest) -> Result<LangUpdateResponse, ApiError> {
    println!("lang_update_cmd invoked: root={} repo={} branch={:?}", request.root, request.repo, request.branch);
    let _ = &request.game_version; // mark as used
    let t0 = std::time::Instant::now();
    append_log(&state.path, "INFO", &format!("lang_update: repo={} src={} trg={}", request.repo, request.source_lang_dir, request.target_lang_dir));
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
    let zip_path = request.zip_path.as_deref().map(|p| make_absolute(&scan_root, Path::new(p)));
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
        let resp = LangUpdateResponse { files: s.files, bytes: s.bytes, out_dir: s.out_dir.display().to_string() };
        write_profile(&state, "lang_update", t0, serde_json::json!({"files": resp.files, "bytes": resp.bytes }));
        Ok(resp)
    } else {
        let resp = LangUpdateResponse { files: 0, bytes: 0, out_dir: scan_root.join("Data/Core/Languages").display().to_string() };
        write_profile(&state, "lang_update", t0, serde_json::json!({"files": 0, "bytes": 0 }));
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
struct MorphResponse { processed: usize, lang: String, warn_no_morpher: bool, warn_no_pymorphy: bool }

#[tauri::command]
fn morph_cmd(_window: Window, state: State<LogState>, request: MorphRequest) -> Result<MorphResponse, ApiError> {
    append_log(&state.path, "INFO", &format!("morph: target={} provider={:?}", request.target_lang_dir, request.provider));
    let root = PathBuf::from(&request.root);
    let prov = match request.provider.as_deref() {
        Some("morpher") | Some("MorpherApi") => MorphProvider::MorpherApi,
        Some("pymorphy") | Some("Pymorphy2") => MorphProvider::Pymorphy2,
        _ => MorphProvider::Dummy,
    };
    if let Some(tok) = request.morpher_token.as_deref() { std::env::set_var("MORPHER_TOKEN", tok); }
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
    Ok(MorphResponse { processed: res.processed, lang: res.lang, warn_no_morpher: res.warn_no_morpher, warn_no_pymorphy: res.warn_no_pymorphy })
}

// --- Learn Keyed ---
#[derive(Debug, Deserialize)]
struct LearnKeyedRequest {
    root: String,
    source_lang_dir: Option<String>,
    target_lang_dir: Option<String>,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LearnKeyedResponse { processed: usize, suggested: String, missing: String }

#[tauri::command]
fn learn_keyed_cmd(_window: Window, state: State<LogState>, request: LearnKeyedRequest) -> Result<LearnKeyedResponse, ApiError> {
    append_log(&state.path, "INFO", &format!("learn_keyed: src={:?} trg={:?}", request.source_lang_dir, request.target_lang_dir));
    let root = PathBuf::from(&request.root);
    let scan_root = root.clone();
    let out_dir = request.out_dir.as_deref().map(PathBuf::from).unwrap_or_else(|| scan_root.join("_learn"));
    std::fs::create_dir_all(&out_dir)?;

    // load dicts
    let mut dicts = Vec::new();
    if let Some(files) = request.dict_files.as_ref() {
        for f in files {
            let pp = PathBuf::from(f);
            if let Ok(d) = rimloc_services::learn::keyed::load_keyed_dict_from_file(&pp) { dicts.push(d); }
        }
    }
    let src_dir = request.source_lang_dir.clone().unwrap_or_else(|| "English".to_string());
    let trg_dir = request.target_lang_dir.clone().unwrap_or_else(|| "Russian".to_string());
    if request.from_defs_special { std::env::set_var("RIMLOC_LEARN_KEYED_FROM_DEFS", "1"); }
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
        &mut rimloc_services::learn::ml::DummyClassifier::new(0.9),
    )?;

    let miss = out_dir.join("missing_keyed.json");
    rimloc_services::learn::keyed::write_keyed_missing_json(&miss, &missing)?;
    let sug = out_dir.join("_SuggestedKeyed.xml");
    rimloc_services::learn::keyed::write_keyed_suggested_xml(&sug, &missing)?;
    Ok(LearnKeyedResponse { processed: missing.len(), suggested: sug.display().to_string(), missing: miss.display().to_string() })
}

// --- Dump JSON Schemas ---
#[derive(Debug, Deserialize)]
struct DumpSchemasRequest { out_dir: String }
#[tauri::command]
fn dump_schemas(_window: Window, _state: State<LogState>, req: DumpSchemasRequest) -> Result<String, ApiError> {
    use std::fs;
    let out_dir = PathBuf::from(&req.out_dir);
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

#[tauri::command]
fn annotate_cmd(_window: Window, state: State<LogState>, request: AnnotateRequest) -> Result<AnnotateResponse, ApiError> {
    println!("annotate_cmd invoked: root={} src={} trg={} dry_run={}", request.root, request.source_lang_dir, request.target_lang_dir, request.dry_run);
    let t0 = std::time::Instant::now();
    append_log(&state.path, "INFO", &format!("annotate: src={} trg={}", request.source_lang_dir, request.target_lang_dir));
    let root = PathBuf::from(&request.root);
    if request.dry_run {
        let plan = annotate_dry_run_plan(&root, &request.source_lang_dir, &request.target_lang_dir, request.comment_prefix.as_deref().unwrap_or("//"), request.strip)?;
        let resp = AnnotateResponse { processed: plan.processed, annotated: plan.total_add };
        write_profile(&state, "annotate_preview", t0, serde_json::json!({"processed": resp.processed, "annotated": resp.annotated }));
        Ok(resp)
    } else {
        let s = annotate_apply(&root, &request.source_lang_dir, &request.target_lang_dir, request.comment_prefix.as_deref().unwrap_or("//"), request.strip, false, request.backup)?;
        let resp = AnnotateResponse { processed: s.processed, annotated: s.annotated };
        write_profile(&state, "annotate_apply", t0, serde_json::json!({"processed": resp.processed, "annotated": resp.annotated }));
        Ok(resp)
    }
}

#[tauri::command]
fn init_lang_cmd(_window: Window, state: State<LogState>, request: InitRequest) -> Result<InitResponse, ApiError> {
    println!("init_lang_cmd invoked: root={} src={} trg={} overwrite={} dry_run={}", request.root, request.source_lang_dir, request.target_lang_dir, request.overwrite, request.dry_run);
    let t0 = std::time::Instant::now();
    let root = PathBuf::from(&request.root);
    let plan = make_init_plan(&root, &request.source_lang_dir, &request.target_lang_dir)?;
    let files = write_init_plan(&plan, request.overwrite, request.dry_run)?;
    let resp = InitResponse { files, out_language: plan.language };
    write_profile(&state, "init_lang", t0, serde_json::json!({"files": resp.files }));
    Ok(resp)
}
