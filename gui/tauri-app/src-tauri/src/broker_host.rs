//! GUI hosting of the rimloc-broker data-plane
//! (`docs/development/MCP_SERVER_PHASE2.md`, Phase 2): while THIS GUI
//! process runs, it is the ONE authoritative writer of the profile, and
//! external `rimloc-mcp` clients execute through the broker socket instead
//! of racing the shell's session manager.
//!
//! Minimal invasion, per the phase ask:
//! - the broker runs in its OWN thread inside [`rimloc_broker::server`]
//!   (accept loop + per-line handlers); `start_broker` only binds and
//!   returns — the shell's startup path gains no per-request work;
//! - the ops drive the LIVE `ContractState` manager, not a rival one:
//!   `ProjectSessionManager` is an `Arc`-based `Clone`
//!   (`rimloc-services/src/session.rs:101`), so the clone handed to
//!   [`rimloc_broker::ops::OpsImpl::over_shared_manager`] shares the
//!   session registry, provider settings and the keychain secret sink
//!   with every GUI command (`ContractState::with_manager` callers). No
//!   second manager is constructed anywhere;
//! - every broker event reaches the frontend as the Tauri event
//!   [`BROKER_CHANGED_EVENT`] carrying the design payload
//!   `{project_id, revision, session_epoch, kind}`
//!   ([`rimloc_broker::protocol::RevisionChangedPayload`]) — the same
//!   `emit` style the shell already uses (`main.rs` `window.emit`);
//! - startup lives in the `.setup` hook (`main.rs`), gated by
//!   `RIMLOC_BROKER=0`; ANY failure is a stderr diagnostic and the GUI
//!   lives on — the broker is an enhancement, never a launch dependency;
//! - before binding, the shell checks `<managed>/rimloc-mcp.lock`
//!   (`crates/rimloc-mcp/src/runtime.rs:53`): a lock whose owner is not
//!   PROVABLY dead means a live headless writer owns the profile, so the
//!   broker does not start (the design's lock→socket-bind order). The
//!   headless writer removes its lock on clean exit and the NEXT GUI
//!   start hosts the broker; until then MCP clients keep falling back to
//!   the Phase 1 headless rules on their side.
//!
//! Deliberately NOT shared (process state by design, honest through the
//! durable `<job_id>.job.json` records — `rimloc-broker` ops.rs module
//! docs): the AI-job registry. Jobs started through the broker are
//! visible to the broker's own `ai_job.*` methods and to the durable
//! records; the GUI's `AiJobs` state keeps only the jobs IT started.
//!
//! Windows: the broker crate is a Unix-socket design in Phase 2 (0600
//! UDS in the profile; named pipe is Phase 3) — this module and the
//! `rimloc-broker` dependency are `cfg(unix)`-gated, and this shell keeps
//! building without the broker there.

/// Tauri event name for the frontend (MCP_SERVER_PHASE2.md «React sync»:
/// «emit("broker-changed", {project_id, revision, session_epoch, kind})»).
pub const BROKER_CHANGED_EVENT: &str = "broker-changed";

/// Windows/no-unix stub: Phase 2 has no Windows transport by design.
#[cfg(not(unix))]
pub fn start_broker(_app: tauri::AppHandle) {
    eprintln!(
        "rimloc-broker: no unix transport on this platform; broker not hosted (Phase 3 pending)"
    );
}

#[cfg(unix)]
mod host {
    use std::path::Path;
    use std::sync::{Arc, OnceLock};

    use tauri::{AppHandle, Emitter, Manager};

    use rimloc_broker::ops::OpsImpl;
    use rimloc_broker::protocol::{BrokerEvent, RevisionChangedPayload, EVENT_REVISION_CHANGED};
    use rimloc_broker::server::{BrokerServer, BrokerStartError};

    use crate::broker_host::BROKER_CHANGED_EVENT;
    use crate::contract_adapter::ContractState;

    /// Dev-log event tags for the broker's stderr diagnostics. These are
    /// dev/diagnostic lines (not localized UI text), emitted as a machine
    /// tag with a detail value via `{}`-formatter `eprintln!` — the shape
    /// the no-hardcoded-user-strings guard accepts for structured dev
    /// logs (the `main.rs` DEV_LOG_* idiom) — so the diagnostics stay on
    /// stderr without failing CI.
    const DEV_LOG_BROKER_DISABLED: &str =
        "rimloc-broker: disabled by RIMLOC_BROKER=0; the shell runs without it";
    const DEV_LOG_BROKER_STATE_MISSING: &str =
        "rimloc-broker: contract state is not managed; broker not started";
    const DEV_LOG_BROKER_SERVING: &str = "rimloc-broker: serving 0600 unix socket at";
    const DEV_LOG_BROKER_START_FAILED: &str =
        "rimloc-broker: broker start failed, the shell continues without it:";

    /// The `<managed>/rimloc-mcp.lock` name — a pinned profile constant of
    /// `crates/rimloc-mcp/src/runtime.rs:53`. The GUI crate does not (and
    /// must not) depend on rimloc-mcp, so the name is mirrored here.
    const WRITER_LOCK_FILE_NAME: &str = "rimloc-mcp.lock";

    /// Managed keep-alive: dropping [`BrokerServer`] stops the accept loop
    /// and removes the socket file, so the server must live as Tauri state
    /// for the whole app lifetime (Tauri drops managed state with the app;
    /// a killed process leaves a stale node that the startup connect-probe
    /// of the NEXT run unlinks — server.rs «Live-vs-stale probe»).
    struct BrokerHostState {
        _server: BrokerServer,
    }

    /// What the writer lock says about its holder (`pid=`/`boot=` lines,
    /// runtime.rs `LockOwner` shape).
    pub(crate) struct WriterLockOwner {
        pub pid: Option<u32>,
        pub boot: Option<String>,
    }

    /// Per-process boot token, the runtime.rs `boot_token` mirror: two
    /// processes that somehow observed the same pid still differ here.
    fn boot_token() -> &'static String {
        static BOOT: OnceLock<String> = OnceLock::new();
        BOOT.get_or_init(|| {
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            format!("{millis:x}-{:x}", std::process::id())
        })
    }

    /// `Some(owner)` = a headless writer lock exists whose owner is not
    /// PROVABLY dead → the broker must not start (the design: «занят и
    /// владелец жив (pid+boot валиден) → брокер НЕ стартует»).
    ///
    /// Provably dead = the recycled-pid case of
    /// `crates/rimloc-mcp/src/runtime.rs:186` mirrored exactly: the file
    /// records OUR pid with a DIFFERENT boot token. Everything else —
    /// including an unparsable or pid-less file — is reported as live,
    /// never guessed away (runtime.rs «Stale-lock honesty»: a kill -9
    /// leftover is recovered by the recycled-pid rule or manual removal,
    /// and the file is closed by the live writer on clean exit).
    pub(crate) fn live_headless_writer(managed_root: &Path) -> Option<WriterLockOwner> {
        let path = managed_root.join(WRITER_LOCK_FILE_NAME);
        if !path.exists() {
            return None;
        }
        let mut owner = WriterLockOwner {
            pid: None,
            boot: None,
        };
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines() {
                if let Some(pid) = line.strip_prefix("pid=") {
                    owner.pid = pid.trim().parse::<u32>().ok();
                } else if let Some(boot) = line.strip_prefix("boot=") {
                    owner.boot = Some(boot.trim().to_string());
                }
            }
        }
        let recycled_dead = owner.pid == Some(std::process::id())
            && owner.boot.as_deref() != Some(boot_token().as_str());
        if recycled_dead {
            None
        } else {
            Some(owner)
        }
    }

    /// Host the broker over the live contract state. Called from the
    /// `.setup` hook; never panics, never blocks the shell: every failure
    /// path is a stderr diagnostic followed by a plain return.
    pub fn start_broker(app: AppHandle) {
        // Opt-out switch (the phase ask): RIMLOC_BROKER=0 disables.
        if std::env::var("RIMLOC_BROKER").as_deref() == Ok("0") {
            eprintln!("{}", DEV_LOG_BROKER_DISABLED);
            return;
        }
        // The SAME managed root the contract commands resolve (the ask:
        // default_managed_root, contract_adapter.rs:132).
        let managed = crate::contract_adapter::default_managed_root();

        // Stale-lock gate BEFORE the socket bind (design: порядок
        // lock→socket-bind).
        if let Some(owner) = live_headless_writer(&managed) {
            eprintln!(
                "rimloc-broker: a live headless writer owns `{}` (pid={:?} boot={:?}); \
                 broker not started — the lock clears when that writer exits cleanly, \
                 the next GUI start hosts the broker",
                managed.join(WRITER_LOCK_FILE_NAME).display(),
                owner.pid,
                owner.boot,
            );
            return;
        }

        // Ops over THE live ContractState manager: the clone shares the
        // session registry (Arc inner), provider settings and the secret
        // sink — a missing state is a wiring bug, reported, never a panic.
        let ops = match app.try_state::<ContractState>() {
            Some(state) => OpsImpl::over_shared_manager(state.with_manager(|m| m.clone())),
            None => {
                eprintln!("{}", DEV_LOG_BROKER_STATE_MISSING);
                return;
            }
        };

        // Host half of the event fan-out: forward every
        // revision_changed payload to the frontend as `broker-changed`
        // (the payload shape is the design's, protocol.rs
        // RevisionChangedPayload).
        let emitter_app = app.clone();
        let notify: Arc<dyn Fn(BrokerEvent) + Send + Sync> = Arc::new(move |event: BrokerEvent| {
            if event.event != EVENT_REVISION_CHANGED {
                return;
            }
            match serde_json::from_value::<RevisionChangedPayload>(event.payload) {
                Ok(payload) => {
                    let _ = emitter_app.emit(BROKER_CHANGED_EVENT, payload);
                }
                Err(err) => {
                    eprintln!(
                        "rimloc-broker: malformed {EVENT_REVISION_CHANGED} payload dropped: {err}"
                    );
                }
            }
        });

        match BrokerServer::start(&managed, Arc::new(ops), notify) {
            Ok(server) => {
                let socket = server.socket_path().display().to_string();
                app.manage(BrokerHostState { _server: server });
                eprintln!("{} {}", DEV_LOG_BROKER_SERVING, socket);
            }
            // A live broker on this profile (second GUI process — the
            // documented unsupported case): the existing one wins.
            Err(BrokerStartError::AlreadyRunning { socket_path }) => {
                eprintln!(
                    "rimloc-broker: another live broker already serves `{}`; not starting twice",
                    socket_path.display()
                );
            }
            // ANY other failure: diagnostic, the GUI lives (the ask:
            // «ошибка старта брокера НЕ роняет GUI»).
            Err(err) => {
                eprintln!("{} {}", DEV_LOG_BROKER_START_FAILED, err);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::PathBuf;
        use std::sync::Mutex;

        use serde_json::json;

        use rimloc_broker::ops::{ApplyParams, BrokerOps, ProjectOpenParams, ProjectRefParams};
        use rimloc_services::contract::TranslationIntent;
        use rimloc_services::session::ProjectSessionManager;

        /// The pinned in-repo canary mod (About + 8 English Keyed rows) —
        /// the same fixture the broker crate suite pins.
        fn fixture_mod() -> PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../testlab/fixtures/rimloc-wizard-mod")
        }

        /// The phase ask's integration scenario: `OpsImpl` over a
        /// tempdir-based SHARED manager — one full
        /// create-project → apply → snapshot round trip, no window, no
        /// socket (the socket layer is covered by the crate's own unit
        /// tests). The assertion that matters beyond the round trip: the
        /// HOST-side manager (the original, not the clone) sees the
        /// applied revision — proving the broker drove the SAME live
        /// session registry, which is the single-writer point of the
        /// `over_shared_manager` hosting.
        #[test]
        fn ops_over_shared_tempdir_manager_create_apply_snapshot() {
            let dir = tempfile::tempdir().expect("tempdir");
            let managed = dir.path().join("profile").join("managed");
            std::fs::create_dir_all(&managed).expect("managed root");

            // The host side: one live manager (the ContractState
            // stand-in) creates the project.
            let manager = ProjectSessionManager::new(&managed).expect("manager");
            let created = manager
                .create(&fixture_mod(), None)
                .expect("fixture mod scans into a managed project");
            let project_id = created.project_id;

            // The broker side: OpsImpl over a CLONE of that manager, with
            // the emitter bound (no window needed for this seam).
            let ops = OpsImpl::over_shared_manager(manager.clone());
            let kinds: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let sink = kinds.clone();
            ops.bind_emitter(Arc::new(move |event: BrokerEvent| {
                assert_eq!(event.event, EVENT_REVISION_CHANGED);
                let kind = event.payload["kind"].as_str().expect("kind").to_string();
                sink.lock().expect("kinds").push(kind);
            }));

            // 1) project.open mints the write coordinates.
            let opened = ops
                .project_open(ProjectOpenParams {
                    project_id: project_id.clone(),
                })
                .expect("open");
            let epoch = opened["session_epoch"].as_u64().expect("epoch");
            let revision = opened["revision"].as_u64().expect("revision");
            assert_eq!(opened["project_id"], project_id.as_str());
            assert_eq!(
                opened["counts"]["entries"], 8,
                "the canary has 8 keyed rows"
            );

            // 2) translation.apply through the broker surface.
            let intents: Vec<TranslationIntent> = serde_json::from_value(json!([{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "Привет мир"
            }]))
            .expect("intent shape");
            let applied = ops
                .translation_apply(ApplyParams {
                    project_id: project_id.clone(),
                    session_epoch: epoch,
                    expected_revision: revision,
                    intents,
                    origin: None,
                })
                .expect("apply");
            assert_eq!(
                applied["revision"].as_u64(),
                Some(revision + 1),
                "apply bumps the durable revision: {applied}"
            );

            // 3) snapshot through the broker ops AND through the HOST's
            // own manager — both must report the SAME live revision.
            let snap = ops
                .project_snapshot(ProjectRefParams {
                    project_id: project_id.clone(),
                    session_epoch: Some(epoch),
                })
                .expect("snapshot");
            assert_eq!(snap["revision"].as_u64(), Some(revision + 1));
            assert_eq!(snap["counts"]["translated"], 1);
            let host_view = manager
                .snapshot(&project_id)
                .expect("host manager reads the same live session");
            assert_eq!(host_view.revision, revision + 1);

            // The mutation events the host callback would have received,
            // in order.
            let seen = kinds.lock().expect("kinds");
            assert_eq!(*seen, vec!["project_open".to_string(), "apply".to_string()]);
        }

        /// The stale-lock gate: absent → pass; a live (unprovably-dead)
        /// owner → block with the parsed owner; the recycled-pid case
        /// (OUR pid, foreign boot token) → provably dead → pass.
        #[test]
        fn writer_lock_gate_blocks_live_owner_and_lets_recycled_pid_pass() {
            let dir = tempfile::tempdir().expect("tempdir");
            let managed = dir.path().join("managed");
            std::fs::create_dir_all(&managed).expect("managed root");

            // No lock file at all → nothing blocks the broker.
            assert!(live_headless_writer(&managed).is_none());

            // A foreign-pid lock: the live-headless-writer case — blocks,
            // and the owner is parsed for the warning message.
            let lock = managed.join(WRITER_LOCK_FILE_NAME);
            std::fs::write(&lock, "pid=1\nboot=other-process-boot\n").expect("lock");
            let owner = live_headless_writer(&managed).expect("a foreign owner blocks");
            assert_eq!(owner.pid, Some(1));
            assert_eq!(owner.boot.as_deref(), Some("other-process-boot"));

            // The recycled-pid case: OUR pid with a DIFFERENT boot token —
            // the one provably-dead shape (runtime.rs:186 rule mirrored).
            std::fs::write(
                &lock,
                format!(
                    "pid={}\nboot=previous-boot-of-same-pid\n",
                    std::process::id()
                ),
            )
            .expect("lock");
            assert!(
                live_headless_writer(&managed).is_none(),
                "a recycled pid is provably dead and must not block"
            );
        }
    }
}

#[cfg(unix)]
pub use host::start_broker;
