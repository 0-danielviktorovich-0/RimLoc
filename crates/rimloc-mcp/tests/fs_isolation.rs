//! Fail-closed FS-isolation guarantees of the headless runtime
//! (`MCP_SERVER_PHASE1.md` headless model v1; independent-review audit
//! п.2 + п.4), driven at BOTH seams:
//!
//! - in-process: `Runtime::from_profile` / `Runtime::build` + the real
//!   `tools::call` layer — no `RIMLOC_DATA_DIR` process-global mutation,
//!   tests stay parallel-safe;
//! - spawned binary: the REAL `rimloc-mcp` with per-process env
//!   (`HOME` is redirected into a tempdir so even the DEFAULT profile
//!   resolves inside the test sandbox) — the env-sanitization ladder is
//!   verified end-to-end the way a client actually launches the server.
//!
//! The matrix (audit wording → test):
//! - default profile (no `RIMLOC_DATA_DIR`) → writes AND project-scoped
//!   reads refuse typed `profile_shared`; lock-free disk diagnostics
//!   (`status`, `capabilities`, `projects.list`, `jobs.status`) stay up;
//! - `RIMLOC_DATA_DIR=/` (canonicalizes to the volume root) → LOUD
//!   refusal + fallback to shared diagnostics; `/managed` is never
//!   created (no quiet write at the root);
//! - `RIMLOC_DATA_DIR=<regular file>` → refusal, file untouched;
//! - read-only profile dir → `Runtime::build` returns an io error (the
//!   spawned server exits before any protocol byte), never a panic;
//! - symlink wrappers → two processes reaching one root through
//!   different symlink paths contend for ONE canonical lock (the second
//!   gets `other_writer`), because `build` canonicalizes BEFORE
//!   acquiring;
//! - the shared runtime never writes the GUI profile at all: no lock
//!   file, no manager construction (a stale `running` job record stays
//!   `running` — `recover_interrupted_jobs` would relabel it).

use std::io::{BufRead, BufReader, Read, Write as _};
// Cross-platform test seams: the semantics under test (read-only refusal,
// symlink identity) are POSIX-first; Windows gets an honest readonly
// emulation and symlink coverage stays unix (CI windows runners cannot be
// relied upon for symlink privileges).
#[cfg(unix)]
use std::os::unix::fs::symlink;

fn set_readonly(dir: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).expect("chmod 555");
    }
    #[cfg(windows)]
    {
        let mut perms = std::fs::metadata(dir).expect("meta").permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(dir, perms).expect("set readonly");
    }
}

fn set_writable(dir: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).expect("chmod 755");
    }
    #[cfg(windows)]
    {
        let mut perms = std::fs::metadata(dir).expect("meta").permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(dir, perms).expect("clear readonly");
    }
}
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rimloc_mcp::error::RpcError;
use rimloc_mcp::runtime::{
    AppMode, ProfileResolution, Runtime, SharedProfileReason, WriterLock, CODE_PROFILE_SHARED,
    LOCK_FILE_NAME,
};
use rimloc_mcp::tools;
use rimloc_services::session::ProjectSessionManager;

/// The remediation phrase every shared-profile refusal must carry —
/// on the wire (error message) and in `rimloc.status`.
const ISOLATE_REMEDY: &str = "set RIMLOC_DATA_DIR to isolate";

/// The five project-scoped reads: they all construct a
/// `ProjectSessionManager` (whose constructor runs the
/// `recover_interrupted_jobs` WRITE), so shared mode refuses them
/// exactly like the write layer.
fn project_scoped_reads() -> Vec<(&'static str, Value)> {
    vec![
        ("rimloc.project.snapshot", json!({ "project_id": "proj-x" })),
        (
            "rimloc.entries.search",
            json!({ "project_id": "proj-x", "query": "fixture" }),
        ),
        (
            "rimloc.entry.context",
            json!({ "project_id": "proj-x", "file": "Languages/English/Keyed/Fixture.xml" }),
        ),
        ("rimloc.validation.run", json!({ "project_id": "proj-x" })),
        (
            "rimloc.export.preview",
            json!({ "project_id": "proj-x", "locale": "Russian" }),
        ),
    ]
}

/// The seven controlled writes. Arguments are SHAPE-VALID so the refusal
/// surfaces as typed `profile_shared` from the writer gate, not as an
/// earlier `invalid_params` — proving the gate, not the parser.
fn writes() -> Vec<(&'static str, Value)> {
    vec![
        ("rimloc.project.open", json!({ "project_id": "proj-x" })),
        (
            "rimloc.translation.propose",
            json!({
                "project_id": "proj-x",
                "session_epoch": 1,
                "locale": "Russian",
                "entries": [{ "identity": { "kind": "keyed", "key": "K" }, "text": "t" }]
            }),
        ),
        (
            "rimloc.translation.apply",
            json!({
                "project_id": "proj-x",
                "session_epoch": 1,
                "expected_revision": 1,
                "intents": []
            }),
        ),
        (
            "rimloc.ai_job.start",
            json!({
                "project_id": "proj-x",
                "session_epoch": 1,
                "locale": "Russian",
                "provider_instance_id": "prov-x"
            }),
        ),
        ("rimloc.ai_job.cancel", json!({ "job_id": "op-x" })),
        (
            "rimloc.ai_job.accept_selected",
            json!({ "job_id": "op-x", "session_epoch": 1, "selected": ["k"] }),
        ),
        (
            "rimloc.export.build",
            json!({
                "project_id": "proj-x",
                "session_epoch": 1,
                "out_dir": "/tmp/rimloc-fs-isolation-out",
                "locale": "Russian"
            }),
        ),
    ]
}

// ---------------------------------------------------------------------------
// In-process harness
// ---------------------------------------------------------------------------

fn call(rt: &Runtime, name: &str, args: Value) -> Value {
    tools::call(name, args, rt).unwrap_or_else(|err| panic!("`{name}` must succeed: {err:?}"))
}

fn call_err(rt: &Runtime, name: &str, args: Value) -> RpcError {
    tools::call(name, args, rt)
        .err()
        .unwrap_or_else(|| panic!("`{name}` must refuse"))
}

fn assert_profile_shared(err: &RpcError, tool: &str) {
    assert_eq!(err.code, CODE_PROFILE_SHARED, "{tool}: {err:?}");
    let data = err
        .data
        .as_ref()
        .unwrap_or_else(|| panic!("{tool}: the refusal must carry typed data"));
    assert_eq!(data["code"], "profile_shared", "{tool}");
    let message = &err.message;
    assert!(
        message.contains(ISOLATE_REMEDY),
        "{tool}: the message must carry the remediation: {message}"
    );
}

/// A shared-mode runtime over a SANDBOX root (never the real GUI
/// profile): `ProfileResolution::Shared` is the exact value
/// `from_env` produces for an unset variable, with the default root
/// pointed into the tempdir.
fn shared_runtime(managed: PathBuf) -> Runtime {
    Runtime::from_profile(ProfileResolution::Shared {
        reason: SharedProfileReason::DefaultProfile,
        managed_root: managed,
    })
    .expect("shared mode performs no filesystem work")
}

/// Platform volume root ("/" on unix, the system drive on windows): on
/// windows a bare "/" is not absolute and refuses at an earlier branch.
fn volume_root() -> PathBuf {
    if cfg!(windows) {
        let drive = std::env::var("SYSTEMDRIVE").unwrap_or_else(|_| "C:".to_string());
        PathBuf::from(format!("{drive}\\"))
    } else {
        PathBuf::from("/")
    }
}

fn fixture_mod() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testlab/fixtures/rimloc-wizard-mod")
}

/// Sorted file inventory of the managed root — the byte-level oracle for
/// «the shared runtime wrote nothing».
fn inventory(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root)
        .expect("managed root readable")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A durable job record in the `running` state — exactly what
/// `recover_interrupted_jobs` relabels to `interrupted` when a manager
/// is constructed. If it survives ANY shared-mode call unchanged, no
/// manager was constructed.
fn seed_running_job_record(managed: &Path, job_id: &str) -> PathBuf {
    let record = json!({
        "job_id": job_id,
        "project_id": "proj-x",
        "base_revision": 1,
        "locale": "Russian",
        "provider_id": "test-provider",
        "model": "test-model",
        "state": "running",
        "total": 3,
        "translated": 1,
        "failed": 0,
        "excluded_notranslate": 0,
        "rows": [],
        "rows_truncated": false
    });
    let path = managed.join(format!("{job_id}.job.json"));
    std::fs::write(&path, serde_json::to_vec_pretty(&record).expect("record"))
        .expect("running job record seed");
    path
}

// ---------------------------------------------------------------------------
// 1. Default profile (shared mode): refusals + diagnostics
// ---------------------------------------------------------------------------

#[test]
fn shared_default_profile_refuses_writes_and_project_reads_but_serves_diagnostics() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("managed");
    let rt = shared_runtime(managed.clone());

    assert!(rt.is_shared_profile());
    assert!(rt.is_diagnostics_only());
    assert_eq!(
        *rt.mode(),
        AppMode::SharedProfile {
            reason: SharedProfileReason::DefaultProfile
        }
    );

    // EVERY write refuses typed profile_shared — before any filesystem
    // work (the arguments are shape-valid, so the parser is passed).
    for (tool, args) in writes() {
        let err = call_err(&rt, tool, args.clone());
        assert_profile_shared(&err, tool);
    }

    // EVERY project-scoped read refuses the same way: the session open
    // would run the manager constructor's recovery write against the
    // live GUI's store.
    for (tool, args) in project_scoped_reads() {
        let err = call_err(&rt, tool, args.clone());
        assert_profile_shared(&err, tool);
    }

    // The lock-free disk diagnostics stay up. Seed a project and a job
    // record the way the GUI (or an earlier isolated writer) would have.
    let creator = ProjectSessionManager::new(&managed).expect("setup manager");
    creator
        .create(&fixture_mod(), None)
        .expect("fixture project seeds the sandbox profile");
    drop(creator);
    seed_running_job_record(&managed, "op-running-1");

    let status = call(&rt, "rimloc.status", json!({}));
    let sc = &status["structuredContent"];
    assert_eq!(sc["tools"]["writes_executable"], 0, "{sc}");
    assert_eq!(
        sc["tools"]["read_only_executable"], 5,
        "only the lock-free diagnostics + job-status alias execute: {sc}"
    );
    assert_eq!(sc["lock"]["held_by_us"], false);
    assert_ne!(
        sc["app"], "self",
        "shared mode must never claim to be the writer"
    );
    assert!(
        !status["content"][0]["text"]
            .as_str()
            .expect("summary")
            .is_empty(),
        "the text summary still answers"
    );

    let caps = call(&rt, "rimloc.capabilities", json!({}));
    assert_eq!(caps["structuredContent"]["contract_version"], 1);

    let list = call(&rt, "rimloc.projects.list", json!({}));
    let listed = list["structuredContent"]["projects"]
        .as_array()
        .expect("projects array");
    assert!(
        listed
            .iter()
            .any(|p| p["project_id"].as_str().unwrap_or("").starts_with("proj-")),
        "the disk-only list sees the seeded project: {list}"
    );

    let job = call(
        &rt,
        "rimloc.jobs.status",
        json!({ "job_id": "op-running-1" }),
    );
    assert_eq!(
        job["structuredContent"]["state"], "running",
        "the durable record answers lock-free, unmodified"
    );
}

// ---------------------------------------------------------------------------
// 2. The shared runtime NEVER writes the GUI profile
// ---------------------------------------------------------------------------

#[test]
fn shared_profile_never_writes_the_gui_profile() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("managed");
    std::fs::create_dir_all(&managed).expect("sandbox GUI profile");

    // Setup (the "GUI's" state): one project envelope + one RUNNING job
    // record + one interrupted record.
    let creator = ProjectSessionManager::new(&managed).expect("setup manager");
    creator
        .create(&fixture_mod(), None)
        .expect("fixture project seeds the sandbox profile");
    drop(creator);
    let running_record = seed_running_job_record(&managed, "op-running-2");
    seed_running_job_record(&managed, "op-interrupted-2");
    // Relabel the second record to `interrupted` by hand.
    let interrupted_path = managed.join("op-interrupted-2.job.json");
    let relabeled = std::fs::read_to_string(&interrupted_path)
        .expect("record")
        .replace("\"state\": \"running\"", "\"state\": \"interrupted\"");
    std::fs::write(&interrupted_path, relabeled).expect("relabel");

    let before = inventory(&managed);
    let rt = shared_runtime(managed.clone());

    // A full sweep of everything a client might try in shared mode.
    for (tool, args) in writes() {
        let _ = tools::call(tool, args.clone(), &rt);
    }
    for (tool, args) in project_scoped_reads() {
        let _ = tools::call(tool, args.clone(), &rt);
    }
    call(&rt, "rimloc.status", json!({}));
    call(&rt, "rimloc.capabilities", json!({}));
    call(&rt, "rimloc.projects.list", json!({}));
    call(
        &rt,
        "rimloc.jobs.status",
        json!({ "job_id": "op-running-2" }),
    );
    call(
        &rt,
        "rimloc.jobs.status",
        json!({ "job_id": "op-interrupted-2" }),
    );

    // Pin 1: the file inventory is byte-identical — no envelope, draft,
    // record or LOCK file was created, nothing was rewritten.
    assert_eq!(
        inventory(&managed),
        before,
        "the shared runtime must not add, remove or rewrite ANY file in the \
         GUI profile (a manager construction would rewrite the running record)"
    );

    // Pin 2: the running record still says `running` —
    // `recover_interrupted_jobs` (manager constructor) never ran.
    let on_disk: Value = serde_json::from_slice(&std::fs::read(&running_record).expect("record"))
        .expect("record parses");
    assert_eq!(on_disk["state"], "running");

    // Pin 3: the writer lock never materializes in the shared profile.
    assert!(
        !managed.join(LOCK_FILE_NAME).exists(),
        "shared mode must not take the writer lock"
    );
}

// ---------------------------------------------------------------------------
// 3. Sanitization ladder: volume root / regular file / read-only dir
// ---------------------------------------------------------------------------

#[test]
fn volume_root_data_dir_refuses_loudly_and_falls_back_to_shared() {
    // In-process form of the `RIMLOC_DATA_DIR=/` scenario: the root
    // degrades to SharedProfile with the refusal reason, and `/managed`
    // is never created.
    let managed_before = Path::new("/managed").exists();
    let rt = Runtime::from_profile(ProfileResolution::Isolated {
        data_dir: volume_root(),
        managed_root: PathBuf::from("/managed"),
    })
    .expect("a refused profile degrades to shared mode, not to an error");

    assert!(rt.is_shared_profile());
    assert_eq!(
        rt.shared_profile_reason(),
        Some(&SharedProfileReason::Refused {
            requested: volume_root().to_string_lossy().into_owned(),
            cause: "path canonicalizes to the volume root".to_string(),
        })
    );
    assert_eq!(
        Path::new("/managed").exists(),
        managed_before,
        "the refused profile must not materialize /managed"
    );

    // The degraded runtime still refuses writes typed.
    let err = call_err(
        &rt,
        "rimloc.project.open",
        json!({ "project_id": "proj-x" }),
    );
    assert_profile_shared(&err, "rimloc.project.open");
    let data = err.data.expect("data");
    assert_eq!(data["reason"], "profile_refused");
}

#[test]
fn regular_file_data_dir_refuses_and_stays_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("not-a-dir");
    std::fs::write(&file, b"precious bytes").expect("seed regular file");

    let rt = Runtime::from_profile(ProfileResolution::Isolated {
        data_dir: file.clone(),
        managed_root: file.join("managed"),
    })
    .expect("a refused profile degrades to shared mode, not to an error");

    assert!(rt.is_shared_profile());
    match rt.shared_profile_reason() {
        Some(SharedProfileReason::Refused { requested, cause }) => {
            assert_eq!(requested.as_str(), file.display().to_string());
            assert!(
                cause.contains("not a directory"),
                "the cause names the shape violation: {cause}"
            );
        }
        other => panic!("expected a refused profile, got {other:?}"),
    }

    // The file is untouched and no `managed` sibling appeared.
    assert_eq!(
        std::fs::read(&file).expect("file readable"),
        b"precious bytes",
        "the refused profile path must not be modified"
    );
    let siblings: Vec<String> = std::fs::read_dir(dir.path())
        .expect("tempdir readable")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        siblings,
        vec!["not-a-dir".to_string()],
        "nothing may be created next to the refused regular file: {siblings:?}"
    );
}

#[test]
fn read_only_profile_dir_is_an_io_error_not_a_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ro = dir.path().join("ro-profile");
    std::fs::create_dir_all(&ro).expect("ro dir");
    set_readonly(&ro);

    // If permissions are not enforced here (running as root), the
    // io-error refusal cannot be exercised — restore and skip honestly.
    if std::fs::write(ro.join("probe"), b"x").is_ok() {
        let _ = std::fs::remove_file(ro.join("probe"));
        set_writable(&ro);
        return;
    }

    // `Runtime::build` must return an io error — never panic — when the
    // managed root cannot be created/written.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Runtime::build(ro.join("managed"))
    }));
    match outcome {
        Ok(Err(err)) => {
            assert!(
                matches!(
                    err.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::Other
                ),
                "expected a permission-flavored io error, got: {err:?}"
            );
        }
        Ok(Ok(_)) => panic!("building on a read-only profile dir must fail"),
        Err(_) => panic!("Runtime::build panicked instead of returning an io error"),
    }

    // Nothing was written into the read-only dir.
    let entries: Vec<String> = std::fs::read_dir(&ro)
        .expect("ro dir readable")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.is_empty(),
        "the read-only dir must stay empty: {entries:?}"
    );

    set_writable(&ro);
}

// ---------------------------------------------------------------------------
// 4. Canonical lock: two symlink wrappers, ONE lock
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)] // symlink privileges are not guaranteed on windows CI
fn symlink_wrappers_share_one_canonical_lock() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real");
    std::fs::create_dir_all(real.join("managed")).expect("real managed root");
    let real_canonical = real.canonicalize().expect("canonical real root");

    // Two different symlink wrappers onto the SAME tree — the two
    // "processes" of the audit scenario.
    symlink(&real, dir.path().join("linkA")).expect("symlink A");
    symlink(&real, dir.path().join("linkB")).expect("symlink B");

    // Direct seam pin: `WriterLock::acquire` places the lock in the
    // directory AS GIVEN (the canonicalization point is `Runtime::build`)
    // — so through EITHER wrapper the file lands in the ONE real
    // directory: the wrapper path and the canonical path are the same
    // inode, and the lock is one file, not two.
    let lock = WriterLock::acquire(&dir.path().join("linkA").join("managed"))
        .expect("first wrapper acquires");
    assert!(
        real_canonical
            .join("managed")
            .join(LOCK_FILE_NAME)
            .is_file(),
        "the lock lands in the real (canonical) root directory"
    );
    drop(lock);

    // Process A enters through wrapper A. `build` canonicalizes BEFORE
    // acquiring — the reported root is the canonical one, and therefore
    // the lock path (`<root>/rimloc-mcp.lock`) is canonical by
    // construction.
    let a = Runtime::build(dir.path().join("linkA").join("managed")).expect("first writer wins");
    assert!(a.is_headless_writer());
    assert_eq!(
        a.managed_root(),
        real_canonical.join("managed").as_path(),
        "build must canonicalize the wrapper path before acquiring"
    );

    // The lock file exists ONCE, under the real (canonical) root.
    let locks_in_real: Vec<PathBuf> = std::fs::read_dir(real_canonical.join("managed"))
        .expect("managed readable")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n == LOCK_FILE_NAME))
        .collect();
    assert_eq!(
        locks_in_real.len(),
        1,
        "exactly ONE lock file in the canonical root: {locks_in_real:?}"
    );

    // Process B enters through wrapper B — the SAME tree, so it must see
    // the SAME lock and get other_writer, never a second writer.
    let b = Runtime::build(dir.path().join("linkB").join("managed")).expect("busy is not an error");
    assert!(
        b.is_other_writer(),
        "a second symlink wrapper must NOT yield a second writer"
    );
    match b.mode() {
        AppMode::OtherWriter { owner } => {
            assert_eq!(
                owner.pid,
                Some(std::process::id()),
                "the honest owner verdict crosses the wrapper"
            );
        }
        other => panic!("expected other_writer, got {other:?}"),
    }

    // RAII: when A drops the lock, a THIRD wrapper can enter — proof the
    // contenders were sharing one lock all along.
    drop(a);
    let c = Runtime::build(dir.path().join("linkB").join("managed"))
        .expect("lock is free again through any wrapper");
    assert!(c.is_headless_writer());
}

#[test]
#[cfg(unix)] // symlink privileges are not guaranteed on windows CI
fn from_profile_isolated_builds_a_writer_through_a_symlink_profile() {
    // The isolated path still works end-to-end: a symlinked data dir is
    // sanitized (it is a dir, not the root) and canonicalized into a
    // writer whose root is the canonical one.
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real-profile");
    std::fs::create_dir_all(&real).expect("real profile dir");
    symlink(&real, dir.path().join("profile-link")).expect("profile symlink");

    let link = dir.path().join("profile-link");
    let rt = Runtime::from_profile(ProfileResolution::Isolated {
        data_dir: link.clone(),
        managed_root: link.join("managed"),
    })
    .expect("sane profile builds");

    assert!(rt.is_headless_writer());
    let expected = real.canonicalize().expect("canonical").join("managed");
    assert_eq!(
        rt.managed_root(),
        expected.as_path(),
        "the writer root is the CANONICAL managed path, not the wrapper"
    );
}

// ---------------------------------------------------------------------------
// 5. Spawned binary: the real env → server behavior
// ---------------------------------------------------------------------------

/// One live stdio session with the real binary and a controlled
/// environment (the mcp_live.rs pattern, trimmed to what the
/// fs-isolation scenarios need).
struct SpawnedServer {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<Vec<String>>>,
    _stderr_thread: std::thread::JoinHandle<()>,
}

impl SpawnedServer {
    /// Spawn with `HOME` redirected into `home`, and `RIMLOC_DATA_DIR`
    /// removed (None) or set to `data_dir`.
    fn spawn(home: &Path, data_dir: Option<&Path>) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rimloc-mcp"));
        cmd.env("HOME", home)
            .env_remove("RIMLOC_DATA_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = data_dir {
            cmd.env("RIMLOC_DATA_DIR", dir);
        }
        let mut child = cmd.spawn().expect("rimloc-mcp binary spawns");
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        let stderr_raw = child.stderr.take().expect("stderr piped");
        let stderr: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&stderr);
        let drain = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr_raw);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => sink
                        .lock()
                        .expect("stderr sink poisoned")
                        .push(line.trim_end().to_string()),
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            stdout,
            stderr,
            _stderr_thread: drain,
        }
    }

    fn stderr_joined(&self) -> String {
        self.stderr.lock().expect("stderr sink").join("\n")
    }

    fn send(&mut self, id: u64, method: &str, params: Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin
            .write_all(
                serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": method,
                    "params": params,
                }))
                .expect("request serializes")
                .as_bytes(),
            )
            .and_then(|()| stdin.write_all(b"\n"))
            .and_then(|()| stdin.flush())
            .expect("request written");
    }

    /// Read stdout until the response with `id`; every line must be one
    /// JSON object (stdout purity is inherited from the transport).
    fn read_response(&mut self, id: u64) -> Value {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .expect("response line readable");
            assert!(bytes > 0, "server closed stdout before responding {id}");
            let msg: Value = serde_json::from_str(line.trim())
                .unwrap_or_else(|err| panic!("stdout carried non-JSON bytes: {err}: {line}"));
            if msg["id"] == id {
                return msg;
            }
        }
    }

    fn call_tool(&mut self, id: u64, name: &str, arguments: Value) -> Result<Value, Value> {
        self.send(
            id,
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        let resp = self.read_response(id);
        match resp.get("error") {
            Some(err) => Err(err.clone()),
            None => Ok(resp["result"]["structuredContent"].clone()),
        }
    }

    /// initialize + notifications/initialized (a TRUE notification — no
    /// id, no response), absorbing the init response.
    fn initialize(&mut self) {
        self.send(
            1,
            "initialize",
            json!({
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": { "name": "fs-isolation-test", "version": "0" }
            }),
        );
        let init = self.read_response(1);
        assert!(
            init.get("error").is_none(),
            "initialize must succeed: {init}"
        );
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .and_then(|()| stdin.flush())
            .expect("notification written");
    }

    fn shutdown(mut self) -> String {
        drop(self.stdin.take()); // EOF → clean exit
        let status = self.child.wait().expect("wait on rimloc-mcp");
        let stderr = self.stderr_joined();
        assert!(
            status.success(),
            "server exited with {status}; stderr:\n{stderr}"
        );
        stderr
    }
}

/// The default-profile managed root under a redirected HOME.
fn default_managed_under(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join("com.rimloc.gui")
        .join("managed")
}

#[test]
fn serve_without_rimloc_data_dir_is_fail_closed_shared_profile() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = default_managed_under(dir.path());

    let mut srv = SpawnedServer::spawn(dir.path(), None);
    srv.initialize();

    // rimloc.status reflects the mode honestly.
    let status = srv
        .call_tool(2, "rimloc.status", json!({}))
        .expect("status answers in shared mode");
    assert_eq!(
        status["tools"]["writes_executable"], 0,
        "shared mode: no writes execute: {status}"
    );
    assert_eq!(status["tools"]["read_only_executable"], 5);
    assert_eq!(status["lock"]["held_by_us"], false);
    assert_ne!(status["app"], "self");
    assert_eq!(status["profile_source"], "default_app_data");
    let status_text = serde_json::to_string(&status).expect("status serializes");
    assert!(
        status_text.contains(ISOLATE_REMEDY),
        "the status payload must carry the remediation: {status_text}"
    );

    // A write refuses typed profile_shared, live over the wire.
    let err = srv
        .call_tool(3, "rimloc.project.open", json!({ "project_id": "proj-x" }))
        .expect_err("write must refuse in shared mode");
    assert_eq!(err["code"], CODE_PROFILE_SHARED, "{err}");
    assert_eq!(err["data"]["code"], "profile_shared");
    assert!(
        err["message"]
            .as_str()
            .expect("message")
            .contains(ISOLATE_REMEDY),
        "{err}"
    );

    // Lock-free disk diagnostics stay up.
    let list = srv
        .call_tool(4, "rimloc.projects.list", json!({}))
        .expect("projects.list answers from disk");
    assert!(list["projects"].is_array(), "{list}");

    let stderr = srv.shutdown();

    // The default profile was never written: no managed dir, no lock.
    assert!(
        !managed.exists(),
        "shared mode must not create the default managed root"
    );
    assert!(
        stderr.contains("shared_profile mode"),
        "the startup banner must announce the shared mode: {stderr}"
    );
    assert!(
        stderr
            .to_lowercase()
            .contains(&ISOLATE_REMEDY.to_lowercase()),
        "the banner must carry the remediation: {stderr}"
    );
}

#[test]
fn serve_with_volume_root_data_dir_refuses_loudly_and_falls_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed_before = Path::new("/managed").exists();

    let mut srv = SpawnedServer::spawn(dir.path(), Some(&volume_root()));
    srv.initialize();

    let status = srv
        .call_tool(2, "rimloc.status", json!({}))
        .expect("the refused profile degrades to a serving diagnostics mode");
    assert_eq!(status["tools"]["writes_executable"], 0, "{status}");
    assert_eq!(status["lock"]["held_by_us"], false);
    assert_ne!(status["app"], "self");

    let err = srv
        .call_tool(3, "rimloc.project.open", json!({ "project_id": "proj-x" }))
        .expect_err("write must refuse after the profile refusal");
    assert_eq!(err["code"], CODE_PROFILE_SHARED, "{err}");
    assert_eq!(err["data"]["code"], "profile_shared");
    assert_eq!(
        err["data"]["reason"], "profile_refused",
        "the wire reason distinguishes refused profiles: {err}"
    );

    let stderr = srv.shutdown();

    // LOUD refusal on stderr, naming the cause…
    assert!(
        stderr.contains("refused") && stderr.contains("volume root"),
        "the volume-root refusal must be loud: {stderr}"
    );
    // …and NO quiet write at the root: /managed must not appear.
    assert_eq!(
        Path::new("/managed").exists(),
        managed_before,
        "RIMLOC_DATA_DIR=/ must never materialize /managed"
    );
}

#[test]
fn serve_with_regular_file_data_dir_refuses() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("regular.file");
    std::fs::write(&file, b"precious bytes").expect("seed regular file");

    let mut srv = SpawnedServer::spawn(dir.path(), Some(&file));
    srv.initialize();

    let status = srv
        .call_tool(2, "rimloc.status", json!({}))
        .expect("the refused profile degrades to a serving diagnostics mode");
    assert_eq!(status["tools"]["writes_executable"], 0, "{status}");

    let err = srv
        .call_tool(
            3,
            "rimloc.translation.apply",
            json!({
                "project_id": "proj-x",
                "session_epoch": 1,
                "expected_revision": 1,
                "intents": []
            }),
        )
        .expect_err("write must refuse after the profile refusal");
    assert_eq!(err["code"], CODE_PROFILE_SHARED, "{err}");
    assert_eq!(err["data"]["code"], "profile_shared");

    let stderr = srv.shutdown();

    assert!(
        stderr.contains("refused") && stderr.contains("not a directory"),
        "the non-directory refusal must be loud: {stderr}"
    );
    assert_eq!(
        std::fs::read(&file).expect("file readable"),
        b"precious bytes",
        "the refused regular file must be untouched"
    );
    // No `managed` was attempted next to the file.
    let siblings: Vec<String> = std::fs::read_dir(dir.path())
        .expect("tempdir readable")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(siblings, vec!["regular.file".to_string()], "{siblings:?}");
}

#[test]
fn serve_with_read_only_data_dir_fails_before_any_protocol_byte() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ro = dir.path().join("ro-profile");
    std::fs::create_dir_all(&ro).expect("ro dir");
    set_readonly(&ro);

    if std::fs::write(ro.join("probe"), b"x").is_ok() {
        // Permissions not enforced in this environment (root): the
        // io-error refusal cannot be exercised — restore and skip.
        let _ = std::fs::remove_file(ro.join("probe"));
        set_writable(&ro);
        return;
    }

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rimloc-mcp"));
    cmd.env("HOME", dir.path())
        .env("RIMLOC_DATA_DIR", &ro)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("binary spawns");
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("stdout piped")
        .read_to_string(&mut stdout)
        .expect("stdout read to EOF");
    let status = child.wait().expect("wait on rimloc-mcp");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("stderr piped")
        .read_to_string(&mut stderr)
        .expect("stderr read");

    set_writable(&ro);

    assert!(
        !status.success(),
        "a read-only profile must fail the startup, got success"
    );
    assert!(
        stdout.is_empty(),
        "no protocol byte may precede the failure: {stdout:?}"
    );
    assert!(
        stderr.contains("managed-root runtime failed to start"),
        "the io failure is reported honestly: {stderr}"
    );
    // The read-only dir stayed untouched.
    let entries: Vec<String> = std::fs::read_dir(&ro)
        .expect("ro dir readable")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.is_empty(),
        "no managed root was created: {entries:?}"
    );
}
