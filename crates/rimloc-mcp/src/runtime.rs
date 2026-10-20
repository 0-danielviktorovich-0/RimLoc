//! Process runtime behind tool execution (`MCP_SERVER_PHASE1.md`, headless
//! model v1): the managed-root profile, the exclusive writer lock, and the
//! typed seams every read-only handler goes through.
//!
//! Profile: `RIMLOC_DATA_DIR=<dir>` selects `<dir>/managed` — the SAME
//! rule the GUI shell uses (`contract_adapter::default_managed_root`), so
//! an MCP process and a live GUI either share one profile or are fully
//! isolated together. Without the variable the OS app-data dir of
//! `com.rimloc.gui` is used. The default branch mirrors `dirs::data_dir()`
//! without adding the dependency (the Phase 1 proposal prefers zero new
//! deps for the hand-rolled stdio loop).
//!
//! Exclusive writer lock: `O_EXCL` (`create_new`) on
//! `<managed_root>/rimloc-mcp.lock`, holding the file open for the
//! process lifetime; the file records `pid=` and a per-process boot token.
//! A busy lock puts the runtime into `other_writer` mode — `rimloc.status`
//! answers honestly, write tools refuse, and project-scoped reads refuse
//! too: opening a session means constructing a `ProjectSessionManager`,
//! whose constructor re-labels stale `running` job records on disk
//! (`recover_interrupted_jobs`, called from
//! `rimloc-services/src/session.rs` `build`) — a WRITE a second writer's
//! live store must never see. Lock-free diagnostics that read the store
//! straight from disk (`rimloc.projects.list`, `rimloc.jobs.status`) stay
//! available, because managed envelopes and job records are written
//! atomically (tmp+rename).
//!
//! Stale-lock honesty: a crashed owner would leave the file behind. The
//! only provably-dead case detectable in `std` is a recycled pid: when the
//! file records OUR pid with a DIFFERENT boot token, the previous holder
//! of that pid is gone and the lock is stolen once. Every other busy state
//! is reported, never guessed away.
//!
//! Statelessness: the runtime holds NO project sessions. Handlers that
//! need trusted state construct a fresh `ProjectSessionManager` per call,
//! `open` the project, run the read, and drop the manager — no cached
//! sessions between requests (session.rs has no close API, so dropping the
//! manager IS closing the session). The single exception is the AI-job
//! registry (`ai_jobs`): jobs are process state by design (the services
//! `missing_job` wording says it verbatim — "jobs live in the app
//! process"), and the durable `<job_id>.job.json` records are the
//! restart-honest fallback.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rimloc_services::contract::{ContractError, ProjectListReport, ProjectSummary};

use crate::error::{codes, RpcError};

/// Lock file name inside the managed root. Distinct from every managed
/// artifact (`*.rimloc.json`, `*.job.json`, `settings/`).
pub const LOCK_FILE_NAME: &str = "rimloc-mcp.lock";

/// Managed-root profile resolution — mirrors the GUI shell so both share
/// one projects universe under the same `RIMLOC_DATA_DIR`.
pub fn managed_root_for_env() -> PathBuf {
    if let Ok(dir) = std::env::var("RIMLOC_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir).join("managed");
        }
    }
    match os_data_dir() {
        Some(base) => base.join("com.rimloc.gui").join("managed"),
        // `dirs::data_dir()` falls back to the temp dir too; keep the same
        // last resort so the server still runs in stripped environments.
        None => std::env::temp_dir().join("com.rimloc.gui").join("managed"),
    }
}

/// Mirror of `dirs::data_dir()` in std-only form (macOS / Windows / XDG).
fn os_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
        })
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            if Path::new(&xdg).is_absolute() {
                return Some(PathBuf::from(xdg));
            }
        }
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
    }
}

// ---------------------------------------------------------------------------
// Exclusive writer lock
// ---------------------------------------------------------------------------

/// What is known about the current lock holder, parsed from the lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockOwner {
    pub pid: Option<u32>,
    pub boot: Option<String>,
}

/// Lock acquisition failure: busy (another writer) or an I/O error.
#[derive(Debug)]
pub enum LockError {
    Busy { owner: LockOwner },
    Io(std::io::Error),
}

/// Per-process boot token: two processes that somehow observed the same pid
/// (recycled pid) still differ here. Same shape as the ai_job boot nonce.
fn boot_token() -> &'static str {
    static BOOT: OnceLock<String> = OnceLock::new();
    BOOT.get_or_init(|| {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        format!("{millis:x}-{:x}", std::process::id())
    })
}

fn read_lock_owner(path: &Path) -> LockOwner {
    let mut owner = LockOwner {
        pid: None,
        boot: None,
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return owner;
    };
    for line in text.lines() {
        if let Some(pid) = line.strip_prefix("pid=") {
            owner.pid = pid.trim().parse::<u32>().ok();
        } else if let Some(boot) = line.strip_prefix("boot=") {
            owner.boot = Some(boot.trim().to_string());
        }
    }
    owner
}

/// The held exclusive writer lock. Dropping it removes the file (the serve
/// loop exits by returning, so RAII runs; a kill -9 leaves the file — the
/// recycled-pid rule above and manual removal are the recovery paths).
#[derive(Debug)]
pub struct WriterLock {
    path: PathBuf,
}

impl WriterLock {
    /// Try to become THE writer of `managed_root`.
    pub fn acquire(managed_root: &Path) -> Result<Self, LockError> {
        std::fs::create_dir_all(managed_root).map_err(LockError::Io)?;
        let path = managed_root.join(LOCK_FILE_NAME);
        let boot = boot_token();
        // At most one steal-and-retry: only the recycled-pid case qualifies.
        for attempt in 0..2 {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut file) => {
                    // Contents are diagnostics for humans and the steal rule;
                    // a failed write cannot un-own the O_EXCL create.
                    let _ = writeln!(file, "pid={}", std::process::id());
                    let _ = writeln!(file, "boot={boot}");
                    let _ = file.flush();
                    return Ok(Self { path });
                }
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    let owner = read_lock_owner(&path);
                    let recycled_pid = owner.pid == Some(std::process::id())
                        && owner.boot.as_deref() != Some(boot);
                    if recycled_pid && attempt == 0 {
                        let _ = std::fs::remove_file(&path);
                        continue;
                    }
                    return Err(LockError::Busy { owner });
                }
                Err(err) => return Err(LockError::Io(err)),
            }
        }
        Err(LockError::Busy {
            owner: read_lock_owner(&path),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Runtime
// ---------------------------------------------------------------------------

/// Who owns the managed root right now — reported verbatim by
/// `rimloc.status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    /// This process holds the exclusive writer lock.
    HeadlessWriter,
    /// Another process holds it. Write tools and project-scoped reads
    /// refuse; lock-free disk diagnostics stay available.
    OtherWriter { owner: LockOwner },
}

/// Everything tool handlers need: the resolved managed root, the lock
/// verdict, and the process-owned AI-job registry. Project sessions stay
/// uncached (fresh manager per call); the job registry is the ONE piece of
/// process state, exactly like the GUI's Tauri state — a job started by
/// `rimloc.ai_job.start` stays visible to `status`/`cancel`/`accept_selected`
/// for the process lifetime, and its durable record outlives the process.
#[derive(Debug)]
pub struct Runtime {
    managed_root: PathBuf,
    mode: AppMode,
    ai_jobs: AiJobRegistryDebug,
    /// Kept alive for the process lifetime; removal happens on drop.
    _lock: Option<WriterLock>,
}

/// Newtype only so `Runtime` keeps its derived `Debug`: the job registry
/// itself carries no Debug impl, and its contents (live job handles) are
/// better inspected through the job-status tools anyway.
#[derive(Default)]
struct AiJobRegistryDebug(rimloc_services::ai_job::AiJobRegistry);

impl std::fmt::Debug for AiJobRegistryDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AiJobRegistry(..)")
    }
}

impl Runtime {
    /// Build against an explicit managed root (production resolves the
    /// profile first, tests inject a tempdir).
    pub fn build(managed_root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let managed_root = managed_root.into();
        // The manager would create the root too; doing it here keeps the
        // lock file path well-defined before the first handler call.
        std::fs::create_dir_all(&managed_root)?;
        let managed_root = std::fs::canonicalize(&managed_root)?;
        match WriterLock::acquire(&managed_root) {
            Ok(lock) => Ok(Self {
                managed_root,
                mode: AppMode::HeadlessWriter,
                ai_jobs: AiJobRegistryDebug::default(),
                _lock: Some(lock),
            }),
            Err(LockError::Busy { owner }) => Ok(Self {
                managed_root,
                mode: AppMode::OtherWriter { owner },
                ai_jobs: AiJobRegistryDebug::default(),
                _lock: None,
            }),
            Err(LockError::Io(err)) => Err(err),
        }
    }

    /// Production entry: resolve the `RIMLOC_DATA_DIR` profile and build.
    pub fn from_env() -> std::io::Result<Self> {
        Self::build(managed_root_for_env())
    }

    pub fn managed_root(&self) -> &Path {
        &self.managed_root
    }

    pub fn mode(&self) -> &AppMode {
        &self.mode
    }

    pub fn is_other_writer(&self) -> bool {
        matches!(self.mode, AppMode::OtherWriter { .. })
    }

    /// The process-owned AI-job registry (`start` inserts here,
    /// `status`/`cancel`/`accept_selected` read here; the durable record is
    /// the restart-honest fallback the job tools already consult).
    pub fn ai_jobs(&self) -> &rimloc_services::ai_job::AiJobRegistry {
        &self.ai_jobs.0
    }
}

// ---------------------------------------------------------------------------
// Typed seams used by every handler
// ---------------------------------------------------------------------------

/// Map a services contract refusal onto the JSON-RPC surface: one server
/// code for "the contract refused the operation", the STABLE snake_case
/// contract code in `data.code` (the MCP layer never invents contract
/// codes — error.rs module docs).
pub fn contract_error(err: ContractError, tool: &str) -> RpcError {
    RpcError::new(codes::CONTRACT_ERROR, err.to_string()).with_data(serde_json::json!({
        "code": err.code.as_str(),
        "tool": tool,
        "details": err.details,
    }))
}

/// Refuse an operation that needs the writer machinery while another
/// process holds the exclusive lock.
pub fn other_writer_error(tool: &str, rt: &Runtime) -> RpcError {
    let owner = match &rt.mode {
        AppMode::OtherWriter { owner } => owner.clone(),
        AppMode::HeadlessWriter => LockOwner {
            pid: Some(std::process::id()),
            boot: Some(boot_token().to_string()),
        },
    };
    RpcError::new(
        codes::OTHER_WRITER,
        format!(
            "another process holds the exclusive writer lock on `{}`; \
             `{tool}` needs session machinery that would write to the managed root",
            rt.managed_root.display()
        ),
    )
    .with_data(serde_json::json!({
        "code": "other_writer_active",
        "tool": tool,
        "owner_pid": owner.pid,
    }))
}

/// Internal refusal when the runtime was never built (protocol tests
/// construct a bare `Server`); never expected in a served session.
pub fn runtime_unavailable(tool: &str) -> RpcError {
    RpcError::internal(format!(
        "server runtime is not initialized; `{tool}` cannot run"
    ))
    .with_data(serde_json::json!({ "code": "runtime_unavailable", "tool": tool }))
}

/// A read-only tool answer: structured output plus the short text summary
/// (the ask: «structuredContent + краткое text-резюме»). Shape follows the
/// MCP `CallToolResult`: `content[0].text` for the model,
/// `structuredContent` for the client.
pub fn tool_ok(
    structured: serde_json::Value,
    summary: String,
) -> Result<serde_json::Value, RpcError> {
    Ok(serde_json::json!({
        "content": [ { "type": "text", "text": summary } ],
        "structuredContent": structured,
    }))
}

/// Parse `tools/call` arguments into a typed struct — serde refusals
/// (wrong types, negative integers where unsigned is declared) become
/// typed `invalid_params`, never panics.
pub fn parse_args<T: serde::de::DeserializeOwned>(
    tool: &str,
    arguments: &serde_json::Value,
) -> Result<T, RpcError> {
    serde_json::from_value(arguments.clone()).map_err(|err| {
        RpcError::invalid_params(format!("invalid arguments for `{tool}`: {err}"))
            .with_data(serde_json::json!({ "code": "invalid_arguments", "tool": tool }))
    })
}

/// Project-scoped reads in `other_writer` mode refuse here, before any
/// filesystem work: opening a session would run the manager constructor's
/// recovery write against a live writer's store (module docs).
pub fn require_writer(rt: &Runtime, tool: &str) -> Result<(), RpcError> {
    if rt.is_other_writer() {
        return Err(other_writer_error(tool, rt));
    }
    Ok(())
}

/// Statelessness seam: fresh manager per call (`ProjectSessionManager::new`
/// has no session-close API, so dropping the manager closes the session).
/// Constructor I/O failures are typed internal errors.
pub fn new_manager(
    rt: &Runtime,
    tool: &str,
) -> Result<rimloc_services::session::ProjectSessionManager, RpcError> {
    rimloc_services::session::ProjectSessionManager::new(rt.managed_root()).map_err(|err| {
        RpcError::internal(format!("session manager init failed for `{tool}`: {err}"))
            .with_data(serde_json::json!({ "code": "session_init_failed", "tool": tool }))
    })
}

/// The disk-only list view for `other_writer` mode: the same scan
/// `ProjectSessionManager::list_report` runs (managed envelopes are atomic,
/// so the disk truth is consistent), minus the in-memory half we cannot
/// have without the writer lock. Unloadable diagnostics included —
/// corruption must never look like "the project is gone".
pub fn list_from_disk(managed_root: &Path) -> ProjectListReport {
    let suffix = ".rimloc.json";
    let mut projects: Vec<ProjectSummary> = Vec::new();
    let mut unloadable = Vec::new();
    let Ok(entries) = std::fs::read_dir(managed_root) else {
        return ProjectListReport {
            projects,
            unloadable,
        };
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(id) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(suffix))
            .map(str::to_owned)
        else {
            continue;
        };
        match rimloc_services::project_store::load_project_with_meta(&path) {
            Ok(loaded) => {
                let name = loaded
                    .meta
                    .display_name
                    .clone()
                    .or_else(|| {
                        loaded
                            .meta
                            .source_root
                            .as_deref()
                            .map(Path::new)
                            .and_then(Path::file_name)
                            .map(|n| n.to_string_lossy().to_string())
                    })
                    .unwrap_or_else(|| id.clone());
                projects.push(ProjectSummary {
                    project_id: id,
                    name,
                    revision: loaded.meta.revision.unwrap_or(1),
                    target_version: loaded.project.context.target_version.clone(),
                });
            }
            Err(err) => {
                let reason = if err.to_string().contains("schema_version") {
                    "schema_version"
                } else {
                    "corrupt_project"
                };
                unloadable.push(rimloc_services::contract::UnloadableProject {
                    project_id: id,
                    reason: reason.to_string(),
                });
            }
        }
    }
    projects.sort_by(|a, b| a.project_id.cmp(&b.project_id));
    ProjectListReport {
        projects,
        unloadable,
    }
}
