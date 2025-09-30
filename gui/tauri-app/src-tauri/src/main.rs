#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console on Windows in release

use color_eyre::eyre::WrapErr;
use rimloc_domain::{ScanUnit, SCHEMA_VERSION};
use rimloc_export_csv as export_csv;
use rimloc_services::{autodiscover_defs_context, export_po_with_tm, learn, scan_units_auto};
use rimloc_services::{
    validate_under_root, validate_under_root_with_defs, validate_under_root_with_defs_and_fields,
    xml_health_scan, import_po_to_mod_tree_with_progress, build_from_po_with_progress,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
use tauri::{Manager, State, Window};
use tauri::Emitter;
use thiserror::Error;
use walkdir::WalkDir;
use std::fs::OpenOptions;
use std::io::Write;

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
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{}: {}", level.to_uppercase(), message.replace('\n', " "));
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
    source_lang: Option<String>,
    #[serde(default)]
    source_lang_dir: Option<String>,
    #[serde(default)]
    tm_roots: Option<Vec<String>>,
    #[serde(default)]
    game_version: Option<String>,
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildModResponse {
    out_mod: String,
    files: usize,
    total_keys: usize,
}

#[tauri::command]
fn get_app_info() -> Result<AppInfo, ApiError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
fn scan_mod(window: Window, state: State<LogState>, request: ScanRequest) -> Result<ScanResponse, ApiError> {
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
        res
    } else {
        let (resolved, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
        let res = run_scan(&resolved, version.as_deref(), &request);
        if let Ok(ref r) = res {
            emit_log(&window, &state, "info", format!("scan finished: {} → total={} keyed={} definj={} saved_json={:?} saved_csv={:?}", resolved.display(), r.total, r.keyed, r.def_injected, r.saved_json, r.saved_csv));
            emit_progress(&window, &state, "scan", "done", Some("Scan finished".to_string()), Some(100));
        }
        res
    }
}

fn run_scan(scan_root: &Path, version: Option<&str>, request: &ScanRequest) -> Result<ScanResponse, ApiError> {
    let units = scan_units_auto(scan_root).wrap_err("scan units")?;
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
    Ok(response)
}

#[tauri::command]
fn export_po(window: Window, state: State<LogState>, request: ExportPoRequest) -> Result<ExportPoResponse, ApiError> {
    emit_log(&window, &state, "info", format!("export_po: root={} out_po={}", request.root, request.out_po));
    emit_progress(&window, &state, "export", "start", Some("Exporting…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    if !root.exists() {
        emit_log(&window, &state, "error", format!("export_po: path not found: {}", root.display()));
        return Err(ApiError { message: format!("Path not found: {}", root.display()) });
    }
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
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
        request.lang.as_deref(),
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
    Ok(resp)
}

#[tauri::command]
fn validate_mod(window: Window, state: State<LogState>, request: ValidateRequest) -> Result<ValidateResponse, ApiError> {
    emit_log(&window, &state, "info", format!("validate: root={}", request.root));
    emit_progress(&window, &state, "validate", "start", Some("Validating…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let defs_root = request.defs_root.as_deref().map(|p| make_absolute(&scan_root, Path::new(p)));
    let msgs_raw = if let Some(fields) = request.extra_fields.as_ref() {
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
    emit_log(&window, &state, "info", format!("xml_health: root={}", request.root));
    emit_progress(&window, &state, "health", "start", Some("Checking XML…".to_string()), Some(0));
    let root = PathBuf::from(&request.root);
    let (scan_root, version) = resolve_game_version_root(&root, request.game_version.as_deref())?;
    let lang_dir = request
        .lang_dir
        .clone()
        .or_else(|| request.lang.clone().map(|c| rimloc_import_po::rimworld_lang_dir(&c)))
        .unwrap_or_else(|| "English".to_string());
    let report = xml_health_scan(&scan_root, Some(&lang_dir)).wrap_err("xml health")?;
    emit_progress(&window, &state, "health", "done", Some("XML check finished".to_string()), Some(100));
    Ok(XmlHealthResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        checked: report.checked,
        issues: report.issues,
    })
}

#[tauri::command]
fn import_po(window: Window, state: State<LogState>, request: ImportPoRequest) -> Result<ImportPoResponse, ApiError> {
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
    let summary = import_po_to_mod_tree_with_progress(
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
    .wrap_err("import po")?;
    emit_progress(&window, &state, "import", "done", Some("Import finished".to_string()), Some(100));
    Ok(ImportPoResponse {
        resolved_root: scan_root.display().to_string(),
        game_version: version,
        lang_dir,
        created: summary.created,
        updated: summary.updated,
        skipped: summary.skipped,
        keys: summary.keys,
    })
}

#[tauri::command]
fn build_mod(window: Window, state: State<LogState>, request: BuildModRequest) -> Result<BuildModResponse, ApiError> {
    emit_log(&window, &state, "info", format!("build_mod from PO: {}", request.po_path));
    emit_progress(&window, &state, "build", "start", Some("Building mod…".to_string()), Some(0));
    let po = PathBuf::from(&request.po_path);
    let out = PathBuf::from(&request.out_mod);
    let mut files_count = 0usize;
    let mut total_keys = 0usize;
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
    // Estimate total keys by scanning generated mod quickly (optional)
    // Skipping heavy scan; we leave total_keys = 0 for now as a placeholder.
    emit_progress(&window, &state, "build", "done", Some("Build finished".to_string()), Some(100));
    Ok(BuildModResponse { out_mod: out.display().to_string(), files: files_count, total_keys })
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
            // Prepare log file path in OS-specific data directory
            let base = dirs::data_dir().unwrap_or_else(|| std::env::temp_dir());
            let log_path = base.join("RimLoc").join("logs").join("gui.log");
            if let Some(parent) = log_path.parent() { let _ = std::fs::create_dir_all(parent); }
            // Write a startup banner
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let _ = writeln!(f, "=== RimLoc GUI start v{} ===", env!("CARGO_PKG_VERSION"));
            }
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
            get_log_info,
            pick_directory,
            save_text_file,
            log_message,
            open_path
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
fn pick_directory(_initial: Option<String>) -> Result<Option<String>, ApiError> {
    // Tauri v2: use JS dialog plugin. Backend picker removed to avoid extra deps.
    Err(ApiError { message: "Directory dialog is not available via backend on Tauri v2".into() })
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
