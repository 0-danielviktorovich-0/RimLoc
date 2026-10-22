#![cfg(unix)] // real Unix-socket broker transport; windows transport is Phase 3

//! Phase 2 broker_client mode end-to-end
//! (`docs/development/MCP_SERVER_PHASE2.md` «Тесты» п.2 — integration:
//! GUIless-брокер + mcp client): a REAL `rimloc-broker` server on a
//! tempdir profile + the REAL `rimloc-mcp` binary
//! (`CARGO_BIN_EXE_rimloc-mcp`) speaking stdio, no mocks anywhere.
//!
//! Scenario:
//! - the spawned server PROBES the live broker at startup and reports
//!   `rimloc.status` honestly: app=broker_gui, mode=broker_client,
//!   writer=gui_process, no writer lock taken;
//! - `projects.list` and `project.snapshot` answer THROUGH the proxy;
//! - `translation.apply` through the proxy really mutates the project
//!   (verified against the store by an independent manager);
//! - the broker-side PAID-GUARD answers through the proxy: a non-local
//!   provider instance refuses `paid_providers_disabled` (mapped to
//!   contract_error, NOT profile_shared — the -32004 numeric collision
//!   is handled);
//! - kill the broker mid-session → the next proxied call answers a TYPED
//!   `broker_unavailable` (-32005), the process does not panic, and
//!   `rimloc.status` keeps answering;
//! - a DEAD broker at startup (stale socket node) falls back to the
//!   Phase 1 headless rules unchanged: headless writer, lock taken,
//!   stale node cleaned up.
//!
//! The fixture mod is the pinned in-repo canary
//! (`testlab/fixtures/rimloc-wizard-mod`); the paid-instance seeding
//! copies `crates/rimloc-broker/tests/broker.rs` exactly (public upsert
//! seam + persisted-record flag flip — the keychain is off in test
//! builds).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rimloc_broker::ops::{BrokerOps, OpsImpl};
use rimloc_broker::server::{BrokerServer, SOCKET_FILE_NAME};
use rimloc_services::contract::ProviderInstanceUpsertRequest;
use rimloc_services::session::ProjectSessionManager;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The canary fixture mod (pinned in-repo: About + 8 English Keyed rows).
fn fixture_mod() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testlab/fixtures/rimloc-wizard-mod")
}

fn stderr_tail(stderr: &Mutex<Vec<String>>) -> String {
    let buf = stderr.lock().expect("stderr lock poisoned");
    let start = buf.len().saturating_sub(12);
    buf[start..].join("\n")
}

/// One live stdio session with the real `rimloc-mcp` binary. Strictly
/// sequential: every request line is followed by exactly one response
/// line (the stdio server never emits unsolicited lines).
struct Mcp {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<Vec<String>>>,
    _stderr_thread: std::thread::JoinHandle<()>,
    next_id: u64,
}

impl Mcp {
    /// Spawn with `RIMLOC_DATA_DIR = profile` (the profile dir; the
    /// managed root the broker serves is `profile/managed`).
    fn spawn(profile: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rimloc-mcp"))
            .env("RIMLOC_DATA_DIR", profile)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("rimloc-mcp binary spawns");
        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        let stderr_raw = child.stderr.take().expect("stderr piped");
        let stderr: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let drain = {
            let sink = Arc::clone(&stderr);
            let mut reader = BufReader::new(stderr_raw);
            std::thread::spawn(move || {
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
            })
        };
        Self {
            child,
            stdin,
            stdout,
            stderr,
            _stderr_thread: drain,
            next_id: 0,
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.stdin
            .write_all(format!("{line}\n").as_bytes())
            .and_then(|()| self.stdin.flush())
            .unwrap_or_else(|e| {
                panic!(
                    "write to server stdin failed: {e}; server stderr:\n{}",
                    stderr_tail(&self.stderr)
                )
            });
        let mut answer = String::new();
        self.stdout.read_line(&mut answer).unwrap_or_else(|e| {
            panic!(
                "read from server stdout failed: {e}; server stderr:\n{}",
                stderr_tail(&self.stderr)
            )
        });
        let value: Value = serde_json::from_str(answer.trim())
            .unwrap_or_else(|e| panic!("stdout line is not JSON ({e}): {answer:?}"));
        assert_eq!(value["id"], id, "the response must echo the request id");
        value
    }

    fn initialize(&mut self) {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2026-07-28",
                "clientInfo": { "name": "broker-client-test", "version": "0.0.0" },
            }),
        );
        assert!(response.get("error").is_none(), "{response}");
        // Notification: no response line follows (verified by the next
        // request's strict sequencing).
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        self.stdin
            .write_all(format!("{note}\n").as_bytes())
            .and_then(|()| self.stdin.flush())
            .expect("initialized notification");
    }

    /// tools/call → the `result` object (structuredContent inside).
    fn tool_ok(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        assert!(
            response.get("error").is_none(),
            "`{name}` must succeed, got: {response}"
        );
        response["result"].clone()
    }

    fn structured(&mut self, name: &str, arguments: Value) -> Value {
        self.tool_ok(name, arguments)["structuredContent"].clone()
    }

    /// tools/call → the typed JSON-RPC error object.
    fn tool_err(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        let error = response
            .get("error")
            .unwrap_or_else(|| panic!("`{name}` must refuse, got: {response}"));
        error.clone()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn create_fixture_project(managed: &Path) -> String {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    let snap = manager
        .create(&fixture_mod(), None)
        .expect("fixture mod scans into a managed project");
    snap.project_id
}

/// A LOCAL ollama provider instance whose base_url is the loopback
/// discard port (the broker.rs / writable_tools.rs pattern — form-valid,
/// connection-refused instantly, zero network beyond loopback).
fn seed_local_provider_instance(managed: &Path) -> String {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    let req: ProviderInstanceUpsertRequest = serde_json::from_value(json!({
        "preset": "ollama",
        "label": "test-local",
        "model": "test-model",
        "base_url": "http://127.0.0.1:9"
    }))
    .expect("upsert request shape");
    manager
        .provider_instance_upsert(&req)
        .expect("local instance upserts without any keychain")
        .instance
        .id
}

// ---------------------------------------------------------------------------
// The Phase 2 integration scenario
// ---------------------------------------------------------------------------

#[test]
fn broker_client_mode_proxies_the_surface_and_reports_broker_gui() {
    // -- the profile + the REAL broker server -----------------------------
    let dir = tempfile::tempdir().expect("tempdir");
    let profile = dir.path().join("profile");
    let managed = profile.join("managed");
    std::fs::create_dir_all(&managed).expect("managed root");
    let project_id = create_fixture_project(&managed);

    let ops: Arc<dyn BrokerOps> = Arc::new(OpsImpl::new(&managed));
    let server = BrokerServer::start(&managed, ops, Arc::new(|_| {})).expect("broker starts");
    assert_eq!(
        server.socket_path(),
        profile.join(SOCKET_FILE_NAME).as_path(),
        "the socket lives in the PROFILE"
    );

    // -- the REAL mcp binary connects at startup ---------------------------
    let mut mcp = Mcp::spawn(&profile);
    mcp.initialize();

    // status: the honest broker_gui shape.
    let sc = mcp.structured("rimloc.status", json!({}));
    assert_eq!(sc["app"], "broker_gui", "{sc}");
    assert_eq!(sc["mode"], "broker_client");
    assert_eq!(sc["writer"], "gui_process");
    assert_eq!(
        sc["writer_pid"],
        Value::Null,
        "the broker protocol v1 exposes no peer pid — an honest null"
    );
    assert_eq!(
        sc["broker_socket"],
        profile.join(SOCKET_FILE_NAME).display().to_string()
    );
    assert_eq!(sc["lock"]["held_by_us"], false);
    assert_eq!(sc["project_scoped_reads"], "available_via_gui_broker");
    assert_eq!(sc["tools"]["read_only_executable"], 10);
    assert_eq!(sc["tools"]["writes_executable"], 7);
    assert!(sc["gui_sharing_profile"]
        .as_str()
        .expect("sharing note")
        .contains("single-writer"));

    // Single-writer invariant: this process took NO writer lock.
    assert!(
        !managed.join("rimloc-mcp.lock").exists(),
        "broker mode must not take the writer lock"
    );

    // projects.list through the proxy (empty-params call).
    let list = mcp.structured("rimloc.projects.list", json!({}));
    let listed = list["projects"]
        .as_array()
        .expect("projects array")
        .iter()
        .any(|p| p["project_id"] == project_id.as_str());
    assert!(
        listed,
        "the seeded project must be listed through the proxy: {list}"
    );

    // project.open through the proxy → write coordinates.
    let opened = mcp.structured("rimloc.project.open", json!({ "project_id": project_id }));
    assert_eq!(opened["revision"], 1, "{opened}");
    let epoch = opened["session_epoch"].as_u64().expect("epoch");

    // translation.apply through the proxy — the single-writer broker
    // executes the mutation.
    let applied = mcp.structured(
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": epoch,
            "expected_revision": 1,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "Привет мир через брокер"
            }],
        }),
    );
    assert_eq!(applied["revision"], 2, "{applied}");
    assert_eq!(applied["applied"], 1, "{applied}");

    // The mutation REALLY landed in the store (an independent manager —
    // not the broker's, not the proxy's — sees it on disk).
    let verifier = ProjectSessionManager::new(&managed).expect("manager");
    let snap = verifier.open(&project_id).expect("stateless open");
    assert_eq!(
        snap.revision, 2,
        "the store is at the broker's new revision"
    );
    assert!(
        snap.project
            .translations
            .iter()
            .any(|t| t.text.as_deref() == Some("Привет мир через брокер")),
        "the translation applied THROUGH THE PROXY is in the store"
    );

    // project.snapshot through the proxy confirms the new coordinates.
    let snap_json = mcp.structured(
        "rimloc.project.snapshot",
        json!({ "project_id": project_id, "session_epoch": epoch }),
    );
    assert_eq!(snap_json["revision"], 2, "{snap_json}");
    assert_eq!(snap_json["counts"]["translated"], 1, "{snap_json}");

    // -- the broker-side PAID-GUARD answers through the proxy --------------
    let instance_id = seed_local_provider_instance(&managed);
    let settings_path = managed.join("settings").join("providers.json");
    let mut settings: Value =
        serde_json::from_slice(&std::fs::read(&settings_path).expect("settings file"))
            .expect("settings json");
    let instance = settings["instances"]
        .as_array_mut()
        .expect("instances")
        .iter_mut()
        .find(|i| i["id"] == instance_id.as_str())
        .expect("seeded instance present");
    instance["local"] = json!(false);
    std::fs::write(
        &settings_path,
        serde_json::to_vec(&settings).expect("serialize"),
    )
    .expect("write settings");

    let err = mcp.tool_err(
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": epoch,
            "locale": "Russian",
            "provider_instance_id": instance_id,
        }),
    );
    // CONTRACT_ERROR, NOT profile_shared — the broker's -32004 paid code
    // collides numerically with the MCP profile-shared code and must map
    // to the contract refusal, stable string preserved.
    assert_eq!(err["code"], -32002, "{err}");
    assert_eq!(err["data"]["code"], "paid_providers_disabled", "{err}");
    assert_eq!(
        err["data"]["broker_code"], -32004,
        "the lossy mapping keeps the raw broker code: {err}"
    );

    // -- kill the broker mid-session: typed, never a panic ------------------
    drop(server); // accept loop stops, socket file removed

    let err = mcp.tool_err("rimloc.projects.list", json!({}));
    assert_eq!(err["code"], -32005, "{err}");
    assert_eq!(err["data"]["code"], "broker_unavailable", "{err}");

    // The next proxied call fails typed the same way (stateless reconnect
    // per call), and the process still serves local surfaces — no panic.
    let err = mcp.tool_err(
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(err["code"], -32005, "{err}");
    let sc = mcp.structured("rimloc.status", json!({}));
    assert_eq!(
        sc["app"], "broker_gui",
        "status stays honest after the death: {sc}"
    );
}

#[test]
fn dead_broker_at_startup_falls_back_to_headless_rules() {
    let dir = tempfile::tempdir().expect("tempdir");
    let profile = dir.path().join("profile");
    let managed = profile.join("managed");
    std::fs::create_dir_all(&managed).expect("managed root");
    // Seed BEFORE the spawn (the spawned writer must not race another
    // writer process — the Phase 1 discipline the live tests follow).
    let project_id = create_fixture_project(&managed);
    let socket_path = profile.join(SOCKET_FILE_NAME);
    {
        // A broker that died: the socket node remains, connect refuses.
        let listener = std::os::unix::net::UnixListener::bind(&socket_path).expect("bind");
        drop(listener);
    }

    let mut mcp = Mcp::spawn(&profile);
    mcp.initialize();
    let sc = mcp.structured("rimloc.status", json!({}));
    assert_eq!(sc["app"], "self", "{sc}");
    assert_eq!(sc["mode"], "headless");
    assert_eq!(sc["lock"]["held_by_us"], true);
    assert!(
        !socket_path.exists(),
        "the stale node is cleaned up by the startup probe (design: клиент удаляет)"
    );
    assert!(
        managed.join("rimloc-mcp.lock").exists(),
        "the Phase 1 headless writer lock is taken as before"
    );

    // And a local write still works headless (Phase 1 rules untouched).
    let opened = mcp.structured("rimloc.project.open", json!({ "project_id": project_id }));
    assert_eq!(opened["revision"], 1, "{opened}");
}
