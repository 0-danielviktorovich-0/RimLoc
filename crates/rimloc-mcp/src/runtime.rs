//! Process runtime behind tool execution (`MCP_SERVER_PHASE1.md`, headless
//! model v1): the managed-root profile, the fail-closed profile modes, the
//! exclusive writer lock, and the typed seams every handler goes through.
//!
//! # Profile resolution (fail-closed, audit п.2 / п.4)
//!
//! `RIMLOC_DATA_DIR=<dir>` selects `<dir>/managed` — the SAME rule the GUI
//! shell uses (`contract_adapter::default_managed_root`), so an MCP process
//! and a live GUI with the same variable are fully isolated together.
//! `resolve_profile()` turns the environment into a `ProfileResolution`,
//! and EVERY other outcome is fail-closed into `AppMode::SharedProfile` —
//! diagnostics only, no writer:
//!
//! - **Variable unset / empty** → the default app-data profile of
//!   `com.rimloc.gui`. A live GUI owns this profile and never takes
//!   `rimloc-mcp.lock`, so the lock proves nothing here: the runtime never
//!   enters `HeadlessWriter` on it. Writes and project-scoped reads refuse
//!   typed `profile_shared`; lock-free disk diagnostics stay up.
//! - **Refused variable** → loud stderr refusal + the same shared
//!   diagnostics mode, never a silent write into the refused tree. The
//!   sanitization ladder (`refused_profile_cause`): the value must be an
//!   absolute traversal-free path; it must not exist as a non-directory
//!   (a regular file is a refusal); and a path that canonicalizes to the
//!   volume root (`/`) is refused — an isolation request pointing at `/`
//!   must not quietly materialize `/managed` at the root.
//! - **Explicit, sane variable** → `Runtime::build` creates
//!   `<dir>/managed`, canonicalizes it, and takes the exclusive lock. Only
//!   here does the process become `HeadlessWriter`.
//!
//! The default branch mirrors `dirs::data_dir()` without adding the
//! dependency (the Phase 1 proposal prefers zero new deps for the
//! hand-rolled stdio loop).
//!
//! # Exclusive writer lock (isolated profiles only)
//!
//! `Runtime::build` canonicalizes the managed root BEFORE acquiring, so
//! `rimloc-mcp.lock` always lives on the CANONICAL path: two processes
//! reaching the same root through different symlink wrappers must contend
//! for ONE lock file, not two (`WriterLock::acquire`, pinned by
//! `tests/fs_isolation.rs`). The lock is `O_EXCL` (`create_new`) on
//! `<managed_root>/rimloc-mcp.lock`, held open for the process lifetime;
//! the file records `pid=` and a per-process boot token. A busy lock puts
//! the runtime into `other_writer` mode — `rimloc.status` answers
//! honestly, write tools refuse, and project-scoped reads refuse too:
//! opening a session means constructing a `ProjectSessionManager`, whose
//! constructor re-labels stale `running` job records on disk
//! (`recover_interrupted_jobs`, called from
//! `rimloc-services/src/session.rs` `build`) — a WRITE a second writer's
//! live store must never see. The shared-profile mode refuses the same
//! operations for the same reason. Lock-free diagnostics that read the
//! store straight from disk (`rimloc.projects.list`, `rimloc.jobs.status`)
//! stay available in both degraded modes, because managed envelopes and
//! job records are written atomically (tmp+rename).
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
///
/// The env value is operator input (the client launching this process owns
/// the machine), but it is still validated before any filesystem work: an
/// RIMLOC_DATA_DIR that is relative or carries `..` components is refused
/// loudly and the shared diagnostics mode is used — a mis-set variable
/// must never silently point the writer at an unexpected tree.
pub fn managed_root_for_env() -> PathBuf {
    match resolve_profile() {
        ProfileResolution::Isolated { managed_root, .. } => managed_root,
        ProfileResolution::Shared { managed_root, .. } => managed_root,
    }
}

/// Why a runtime ended up in the shared diagnostics-only profile mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedProfileReason {
    /// `RIMLOC_DATA_DIR` is not set: the default app-data profile is the
    /// live RimLoc GUI's own profile, and the GUI never takes
    /// `rimloc-mcp.lock` — the writer lock proves nothing on it, so the
    /// runtime is fail-closed into diagnostics-only (audit п.2).
    DefaultProfile,
    /// `RIMLOC_DATA_DIR` was set but refused by the sanitization ladder
    /// (audit п.4): volume-root path, non-directory path, relative or
    /// traversal-carrying path. The refusal is loud on stderr; the
    /// runtime degrades to shared diagnostics instead of writing anywhere
    /// near the refused tree.
    Refused { requested: String, cause: String },
}

/// The startup profile decision (`resolve_profile`): either an explicit,
/// sanitized `RIMLOC_DATA_DIR` to open exclusively, or the shared
/// diagnostics-only mode with the reason and the default root the
/// lock-free disk diagnostics read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileResolution {
    /// Explicit, sanitized profile: create + canonicalize + lock
    /// `<managed_root>` (a `HeadlessWriter` is possible).
    Isolated {
        data_dir: PathBuf,
        managed_root: PathBuf,
    },
    /// Shared with the live GUI (or the requested profile was refused):
    /// diagnostics only — no lock, no directory creation, no manager
    /// construction; writes and project-scoped reads refuse
    /// `profile_shared`. Disk diagnostics read `<managed_root>`, which is
    /// always the DEFAULT GUI profile root, never the refused path.
    Shared {
        reason: SharedProfileReason,
        managed_root: PathBuf,
    },
}

/// Read the `RIMLOC_DATA_DIR` environment variable and decide the startup
/// profile. Every input that fails the sanitization ladder is refused
/// LOUDLY (stderr) and degrades to `ProfileResolution::Shared` — the
/// variable can downgrade the runtime to diagnostics, never upgrade it
/// onto an unvalidated tree. `doctor` and `serve` both consult this, so
/// the reported verdict is exactly the enforced one.
pub fn resolve_profile() -> ProfileResolution {
    let explicit = match std::env::var("RIMLOC_DATA_DIR") {
        Ok(dir) if !dir.is_empty() => Some(dir),
        _ => None,
    };
    let Some(dir) = explicit else {
        return ProfileResolution::Shared {
            reason: SharedProfileReason::DefaultProfile,
            managed_root: default_profile_managed_root(),
        };
    };
    let candidate = PathBuf::from(&dir);
    if let Some(cause) = refused_profile_cause(&candidate) {
        crate::stderr_log(&format!(
            "RIMLOC_DATA_DIR `{dir}` refused: {cause}; falling back to the \
             shared-profile diagnostics mode (set RIMLOC_DATA_DIR to isolate)"
        ));
        return ProfileResolution::Shared {
            reason: SharedProfileReason::Refused {
                requested: dir,
                cause,
            },
            managed_root: default_profile_managed_root(),
        };
    }
    ProfileResolution::Isolated {
        managed_root: candidate.join("managed"),
        data_dir: candidate,
    }
}

/// The sanitization ladder for an explicit profile path (audit п.4).
/// `None` — the path may be opened as the isolated data dir; `Some(cause)`
/// — refuse loudly and go shared. Read-only checks only: creation of the
/// tree stays with `Runtime::build`, which surfaces real I/O failures as
/// typed errors instead of guesses.
pub fn refused_profile_cause(candidate: &Path) -> Option<String> {
    if !candidate.is_absolute()
        || candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Some("not an absolute traversal-free path".to_string());
    }
    match std::fs::metadata(candidate) {
        Ok(meta) if !meta.is_dir() => {
            return Some("path exists and is not a directory".to_string());
        }
        Ok(_) => {}
        // Does not exist yet — `Runtime::build` creates it (or fails
        // honestly on a real I/O error: read-only parent, dangling
        // symlink, …).
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Some(format!("path is unreadable: {err}")),
    }
    // The volume-root rule needs the RESOLVED path: `/`, `//`, `/./` and a
    // symlink to `/` all canonicalize to `/`. `Path::parent()` is `None`
    // exactly for filesystem roots.
    if let Ok(canonical) = candidate.canonicalize() {
        if canonical.parent().is_none() {
            return Some("path canonicalizes to the volume root".to_string());
        }
    }
    None
}

/// The default profile root the diagnostics read from in shared mode —
/// the same tree a live GUI uses without `RIMLOC_DATA_DIR`. Never created
/// by this process: shared mode performs no filesystem writes at all.
pub fn default_profile_managed_root() -> PathBuf {
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
    /// This process holds the exclusive writer lock on an ISOLATED
    /// profile (explicit, sanitized `RIMLOC_DATA_DIR`).
    HeadlessWriter,
    /// Another process holds the lock. Write tools and project-scoped
    /// reads refuse; lock-free disk diagnostics stay available.
    OtherWriter { owner: LockOwner },
    /// The profile is shared with the live RimLoc GUI (default profile,
    /// or the requested one was refused — see `SharedProfileReason`).
    /// Fail-closed diagnostics-only (audit п.2 / п.4): NO writer lock is
    /// taken (the GUI does not take one, so holding ours would prove
    /// nothing and only write into the GUI's tree), writes and
    /// project-scoped reads refuse typed `profile_shared`; lock-free disk
    /// diagnostics stay available.
    SharedProfile { reason: SharedProfileReason },
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
    /// profile first via `from_env`; tests inject a tempdir). The root is
    /// created, CANONICALIZED, and only then locked — the lock file always
    /// lives on the canonical path, so two processes reaching one root
    /// through different symlink wrappers contend for ONE lock.
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
    /// Unset/refused variables degrade to the shared diagnostics-only
    /// mode — they can never silently become a writer (module docs).
    pub fn from_env() -> std::io::Result<Self> {
        Self::from_profile(resolve_profile())
    }

    /// Construct the runtime from an explicit profile decision — the
    /// production `from_env` path minus the environment read. The
    /// `Isolated` arm re-runs the sanitization ladder (defense in depth:
    /// nothing can bypass `refused_profile_cause` into `build`), the
    /// `Shared` arm performs NO filesystem work at all.
    pub fn from_profile(profile: ProfileResolution) -> std::io::Result<Self> {
        match profile {
            ProfileResolution::Shared {
                reason,
                managed_root,
            } => Ok(Self::shared_profile(reason, managed_root)),
            ProfileResolution::Isolated {
                data_dir,
                managed_root,
            } => {
                if let Some(cause) = refused_profile_cause(&data_dir) {
                    crate::stderr_log(&format!(
                        "RIMLOC_DATA_DIR `{}` refused: {cause}; falling back to the \
                         shared-profile diagnostics mode (set RIMLOC_DATA_DIR to isolate)",
                        data_dir.display()
                    ));
                    return Ok(Self::shared_profile(
                        SharedProfileReason::Refused {
                            requested: data_dir.display().to_string(),
                            cause,
                        },
                        default_profile_managed_root(),
                    ));
                }
                Self::build(managed_root)
            }
        }
    }

    /// Diagnostics-only runtime for a profile the live GUI owns (or a
    /// refused one): no lock, no directory creation, no manager
    /// construction — every write and project-scoped read refuses typed
    /// `profile_shared`; the lock-free disk diagnostics read
    /// `<managed_root>` straight from disk. Production reaches this
    /// through `from_profile`/`from_env`; public so tests (and future
    /// embedders) can build the exact served state.
    pub fn shared_profile(reason: SharedProfileReason, managed_root: impl Into<PathBuf>) -> Self {
        Self {
            managed_root: managed_root.into(),
            mode: AppMode::SharedProfile { reason },
            ai_jobs: AiJobRegistryDebug::default(),
            _lock: None,
        }
    }

    pub fn managed_root(&self) -> &Path {
        &self.managed_root
    }

    pub fn mode(&self) -> &AppMode {
        &self.mode
    }

    /// This process holds the exclusive writer lock on an isolated
    /// profile.
    pub fn is_headless_writer(&self) -> bool {
        matches!(self.mode, AppMode::HeadlessWriter)
    }

    /// The runtime is NOT the writer of this root: either another process
    /// holds the exclusive lock, or the profile is shared with the live
    /// GUI / was refused (fail-closed). In this state write tools and
    /// project-scoped reads refuse, and the handlers that CAN answer do
    /// so lock-free from disk — that is exactly how the tool layer routes
    /// its diagnostics (`projects.list`, `jobs.status`).
    pub fn is_other_writer(&self) -> bool {
        !matches!(self.mode, AppMode::HeadlessWriter)
    }

    /// Precise predicate for the shared-profile mode (default/refused
    /// profile) as opposed to a contended lock.
    pub fn is_shared_profile(&self) -> bool {
        matches!(self.mode, AppMode::SharedProfile { .. })
    }

    pub fn shared_profile_reason(&self) -> Option<&SharedProfileReason> {
        match &self.mode {
            AppMode::SharedProfile { reason } => Some(reason),
            _ => None,
        }
    }

    /// Umbrella for honest status reporting: the runtime answers
    /// diagnostics only.
    pub fn is_diagnostics_only(&self) -> bool {
        self.is_other_writer()
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

/// JSON-RPC server-range code for the shared-profile refusal
/// (`data.code == "profile_shared"`). Declared here because
/// `error::codes` is a shared file this change does not touch; the value
/// stays inside the reserved -32000..-32099 range and distinct from
/// NOT_IMPLEMENTED (-32001), CONTRACT_ERROR (-32002) and OTHER_WRITER
/// (-32003) — pinned by the unit test below, mirroring error.rs's test.
pub const CODE_PROFILE_SHARED: i64 = -32004;

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

/// Refuse an operation that needs the writer machinery while the runtime
/// is not the writer of the managed root. Shared-profile refusals get
/// their own typed code — the remediation is different (isolate the
/// profile) from a contended lock (wait for the other process).
pub fn shared_profile_error(tool: &str, reason: &SharedProfileReason) -> RpcError {
    let (reason_code, detail) = match reason {
        SharedProfileReason::DefaultProfile => ("default_profile_shared", String::new()),
        SharedProfileReason::Refused { requested, cause } => (
            "profile_refused",
            format!(" — RIMLOC_DATA_DIR `{requested}` was refused: {cause}"),
        ),
    };
    RpcError::new(
        CODE_PROFILE_SHARED,
        format!(
            "`{tool}` needs an isolated managed root: this process runs on the \
             profile shared with the live RimLoc GUI, which does not take the \
             `rimloc-mcp.lock`{detail}; set RIMLOC_DATA_DIR to isolate"
        ),
    )
    .with_data(serde_json::json!({
        "code": "profile_shared",
        "tool": tool,
        "reason": reason_code,
    }))
}

/// Refuse an operation that needs the writer machinery while another
/// process holds the exclusive lock.
pub fn other_writer_error(tool: &str, rt: &Runtime) -> RpcError {
    let owner = match &rt.mode {
        AppMode::OtherWriter { owner } => owner.clone(),
        // Unreachable through `require_writer` (shared refusals take the
        // `profile_shared` path first); kept exhaustive with an honest
        // "unknown holder".
        AppMode::SharedProfile { .. } => LockOwner {
            pid: None,
            boot: None,
        },
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

/// The writer-machinery gate EVERY write tool and project-scoped read
/// goes through, before any filesystem work. Two degraded modes refuse
/// here: another process holds the lock (`other_writer_active`), or the
/// profile is shared with the live GUI / was refused (`profile_shared`) —
/// opening a session would run the manager constructor's recovery write
/// (`recover_interrupted_jobs`) against a store this process must never
/// mutate (module docs).
pub fn require_writer(rt: &Runtime, tool: &str) -> Result<(), RpcError> {
    match &rt.mode {
        AppMode::HeadlessWriter => Ok(()),
        AppMode::SharedProfile { reason } => Err(shared_profile_error(tool, reason)),
        AppMode::OtherWriter { .. } => Err(other_writer_error(tool, rt)),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The shared-profile refusal code mirrors the error.rs discipline:
    /// inside the JSON-RPC server-defined range and distinct from every
    /// other server code — clients switch on them.
    #[test]
    fn profile_shared_code_is_in_range_and_distinct() {
        assert!(
            (-32099..=-32000).contains(&CODE_PROFILE_SHARED),
            "CODE_PROFILE_SHARED must live in the reserved range"
        );
        for code in [
            codes::NOT_IMPLEMENTED,
            codes::CONTRACT_ERROR,
            codes::OTHER_WRITER,
        ] {
            assert_ne!(CODE_PROFILE_SHARED, code, "server codes must stay distinct");
        }
    }

    #[test]
    fn shared_profile_error_carries_the_isolation_remedy() {
        let err = shared_profile_error("rimloc.project.open", &SharedProfileReason::DefaultProfile);
        assert_eq!(err.code, CODE_PROFILE_SHARED);
        let data = err.data.as_ref().expect("typed data");
        assert_eq!(data["code"], "profile_shared");
        assert_eq!(data["reason"], "default_profile_shared");
        assert_eq!(data["tool"], "rimloc.project.open");
        assert!(
            err.message.contains("set RIMLOC_DATA_DIR to isolate"),
            "the message must carry the remediation: {err:?}"
        );

        let refused = SharedProfileReason::Refused {
            requested: "/".to_string(),
            cause: "path canonicalizes to the volume root".to_string(),
        };
        let err = shared_profile_error("rimloc.export.build", &refused);
        let data = err.data.as_ref().expect("typed data");
        assert_eq!(data["reason"], "profile_refused");
        assert!(
            err.message.contains("canonicalizes to the volume root"),
            "the refusal cause rides the message: {err:?}"
        );
    }

    /// The platform volume root: `/` on unix, the system drive on windows
    /// (e.g. `C:\`). Ladder tests use it so the "canonicalizes to the
    /// volume root" branch is exercised on every OS.
    #[cfg(test)]
    fn volume_root() -> PathBuf {
        if cfg!(windows) {
            let drive = std::env::var("SYSTEMDRIVE").unwrap_or_else(|_| "C:".to_string());
            PathBuf::from(format!("{drive}\\"))
        } else {
            PathBuf::from("/")
        }
    }

    #[test]
    fn sanitization_ladder_refuses_relative_traversal_and_root() {
        // Pure shape checks first (no filesystem access).
        assert_eq!(
            refused_profile_cause(Path::new("relative/dir")).as_deref(),
            Some("not an absolute traversal-free path")
        );
        assert_eq!(
            refused_profile_cause(Path::new("/a/../escape")).as_deref(),
            Some("not an absolute traversal-free path")
        );
        // The volume root itself — /, // and /./ all canonicalize to /.
        assert_eq!(
            refused_profile_cause(Path::new("/")).as_deref(),
            Some("path canonicalizes to the volume root")
        );
        assert_eq!(
            refused_profile_cause(Path::new("/./")).as_deref(),
            Some("path canonicalizes to the volume root")
        );
    }

    #[test]
    fn sanitization_ladder_refuses_regular_files_and_passes_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("regular.file");
        std::fs::write(&file, b"not a directory").expect("seed file");
        assert_eq!(
            refused_profile_cause(&file).as_deref(),
            Some("path exists and is not a directory")
        );
        // A real directory passes; a missing path passes (build creates it).
        assert_eq!(refused_profile_cause(dir.path()), None);
        assert_eq!(refused_profile_cause(&dir.path().join("not-yet")), None);
    }

    #[test]
    fn from_profile_fails_closed_on_refused_profiles() {
        // A hand-assembled Isolated cannot bypass the ladder: the volume
        // root degrades to SharedProfile, and `/managed` is never created.
        assert!(!Path::new("/managed").exists(), "precondition for the pin");
        let rt = Runtime::from_profile(ProfileResolution::Isolated {
            data_dir: PathBuf::from("/"),
            managed_root: PathBuf::from("/managed"),
        })
        .expect("refusal degrades to shared, not an error");
        assert!(rt.is_shared_profile());
        assert_eq!(
            rt.shared_profile_reason(),
            Some(&SharedProfileReason::Refused {
                requested: "/".to_string(),
                cause: "path canonicalizes to the volume root".to_string(),
            })
        );
        assert!(
            !Path::new("/managed").exists(),
            "the refused profile must not be created"
        );
    }

    #[test]
    fn from_profile_isolated_still_builds_a_writer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let data = dir.path().join("profile");
        let rt = Runtime::from_profile(ProfileResolution::Isolated {
            managed_root: data.join("managed"),
            data_dir: data.clone(),
        })
        .expect("sane isolated profile builds");
        assert!(rt.is_headless_writer());
        assert_eq!(
            rt.managed_root(),
            std::fs::canonicalize(data.join("managed"))
                .expect("canonical")
                .as_path(),
            "the runtime root is the canonical managed path"
        );
    }

    #[test]
    fn shared_profile_runtime_never_touches_the_filesystem() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("gui-managed");
        let rt = Runtime::shared_profile(SharedProfileReason::DefaultProfile, root.clone());
        assert!(rt.is_shared_profile());
        assert!(rt.is_diagnostics_only());
        assert!(rt.is_other_writer(), "disk-diagnostic routing must engage");
        assert!(!root.exists(), "shared mode creates nothing");
    }

    #[test]
    fn require_writer_refuses_shared_typed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rt = Runtime::shared_profile(
            SharedProfileReason::DefaultProfile,
            dir.path().join("managed"),
        );
        let err = require_writer(&rt, "rimloc.translation.apply").expect_err("refused");
        assert_eq!(err.code, CODE_PROFILE_SHARED);
        assert_eq!(err.data.expect("data")["code"], "profile_shared");
    }
}
