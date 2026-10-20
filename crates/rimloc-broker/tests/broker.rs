//! Real-server coverage for the broker crate (tempdir profiles, real
//! Unix sockets, no window): every scenario this phase's ask pins, plus
//! the bounds the design names.
//!
//! - protocol round-trip → `crates/rimloc-broker/src/protocol.rs` tests
//!   (the frames are pinned at the unit level);
//! - a second server on a LIVE socket refuses (`BrokerStartError::
//!   AlreadyRunning`), a STALE socket file is unlinked and rebound;
//! - a malformed line answers a typed parse error AND the server keeps
//!   serving the same connection;
//! - a paid (non-local) provider instance refuses
//!   `paid_providers_disabled` — the instance is seeded through the
//!   manager exactly like `crates/rimloc-mcp/tests/writable_tools.rs`
//!   seeds its local one, then the persisted record's `local` flag is
//!   flipped on disk (the keychain is off in test builds, so a genuinely
//!   cloud instance cannot be created through the public upsert seam);
//! - an apply fans `revision_changed` to ALL connected clients and the
//!   host notify callback;
//! - a stale revision is a typed `stale_revision` refusal.
//!
//! Every fixture is the pinned in-repo canary mod (About + 8 English
//! Keyed rows) under `testlab/fixtures/rimloc-wizard-mod`.

use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rimloc_broker::ops::{BrokerOps, OpsImpl};
use rimloc_broker::protocol::{codes, BrokerError};
use rimloc_broker::server::{BrokerServer, BrokerStartError, SOCKET_FILE_NAME};
use rimloc_services::session::ProjectSessionManager;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The canary fixture mod (pinned in-repo).
fn fixture_mod() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testlab/fixtures/rimloc-wizard-mod")
}

struct TestEnv {
    /// Keeps the tempdir alive for the whole test.
    _dir: tempfile::TempDir,
    /// `<profile>/managed` — the authoritative root the server is started
    /// against; the socket lands in the PARENT (the profile).
    managed: PathBuf,
    socket_path: PathBuf,
    /// Events observed by the HOST notify callback (the GUI would forward
    /// these to Tauri emit).
    host_events: Arc<Mutex<Vec<Value>>>,
    /// Taken by tests that need the server stopped; otherwise dropped at
    /// test end.
    server: Option<BrokerServer>,
}

fn start_env() -> TestEnv {
    let dir = tempfile::tempdir().expect("tempdir");
    let profile = dir.path().join("profile");
    let managed = profile.join("managed");
    std::fs::create_dir_all(&managed).expect("managed root");
    start_server_on(managed, dir)
}

fn start_server_on(managed: PathBuf, dir: tempfile::TempDir) -> TestEnv {
    let profile = managed.parent().expect("profile dir").to_path_buf();
    let socket_path = profile.join(SOCKET_FILE_NAME);
    let host_events: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = host_events.clone();
    let ops: Arc<dyn BrokerOps> = Arc::new(OpsImpl::new(&managed));
    let server = BrokerServer::start(
        &managed,
        ops,
        Arc::new(move |event| {
            // Record the event exactly as the wire carries it.
            if let Ok(line) = serde_json::to_value(&event) {
                sink.lock().expect("host events").push(line);
            }
        }),
    )
    .expect("broker starts");
    // The socket contract: inside the profile, 0600.
    assert_eq!(server.socket_path(), socket_path.as_path());
    let mode = std::fs::metadata(&socket_path)
        .expect("socket file exists")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "the broker socket must be 0600");
    TestEnv {
        _dir: dir,
        managed,
        socket_path,
        host_events,
        server: Some(server),
    }
}

fn create_fixture_project(managed: &Path) -> String {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    let snap = manager
        .create(&fixture_mod(), None)
        .expect("fixture mod scans into a managed project");
    snap.project_id
}

/// A bare client over the real socket with a PERSISTENT receive buffer —
/// frames may coalesce in one TCP-style read, so buffering must live in
/// the client, not in a per-call BufReader.
struct Client {
    stream: UnixStream,
    buf: Vec<u8>,
}

impl Client {
    fn connect(socket_path: &Path) -> Self {
        let stream = UnixStream::connect(socket_path).expect("connect");
        Self {
            stream,
            buf: Vec::new(),
        }
    }

    fn send(&mut self, frame: &Value) {
        let line = rimloc_broker::protocol::encode_line(frame).expect("encode");
        self.stream.write_all(&line).expect("write");
        self.stream.flush().expect("flush");
    }

    fn send_raw(&mut self, bytes: &[u8]) {
        self.stream.write_all(bytes).expect("write raw");
        self.stream.flush().expect("flush");
    }

    /// Next frame from the socket (event or response — callers decide by
    /// shape: a response carries `id`, an event carries `event`).
    fn read_frame(&mut self) -> Value {
        loop {
            if let Some(pos) = self.buf.iter().position(|&b| b == b'\n') {
                let frame: Value =
                    serde_json::from_slice(&self.buf[..pos]).expect("frame is valid JSON");
                self.buf.drain(..=pos);
                return frame;
            }
            let mut chunk = [0u8; 8192];
            let n = self.stream.read(&mut chunk).expect("read");
            assert!(n > 0, "socket closed before a frame arrived");
            self.buf.extend_from_slice(&chunk[..n]);
        }
    }

    /// Next RESPONSE frame (skips unsolicited events in front of it).
    fn read_response(&mut self, id: &str) -> Value {
        loop {
            let frame = self.read_frame();
            if frame.get("id").is_some() {
                assert_eq!(frame["id"], id, "response id must echo the request");
                return frame;
            }
        }
    }

    fn call(&mut self, id: &str, method: &str, params: Value) -> Value {
        self.send(&json!({ "id": id, "method": method, "params": params }));
        self.read_response(id)
    }

    /// Next EVENT frame (refuses to mistake a response for one).
    fn read_event(&mut self) -> Value {
        let frame = self.read_frame();
        assert!(
            frame.get("event").is_some(),
            "expected an event frame, got: {frame}"
        );
        frame
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Close the socket so the server-side reader thread winds down.
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

fn expect_broker_code(frame: &Value, numeric: i64, snake: &str) {
    let error = frame
        .get("error")
        .unwrap_or_else(|| panic!("expected an error frame, got: {frame}"));
    assert_eq!(error["code"], numeric);
    assert_eq!(error["data"]["code"], snake, "{frame}");
}

/// A LOCAL ollama provider instance whose base_url is the loopback
/// discard port (the exact writable_tools.rs:148 pattern — form-valid,
/// connection-refused instantly, zero network beyond loopback).
fn seed_local_provider_instance(managed: &Path) -> String {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    let req: rimloc_services::contract::ProviderInstanceUpsertRequest =
        serde_json::from_value(json!({
            "preset": "ollama",
            "label": "test-local",
            "model": "test-model",
            "base_url": "http://127.0.0.1:9"
        }))
        .expect("upsert request shape");
    let resp = manager
        .provider_instance_upsert(&req)
        .expect("local instance upserts without any keychain");
    resp.instance.id
}

// ---------------------------------------------------------------------------
// Second server on a live socket refuses; a stale socket is taken over
// ---------------------------------------------------------------------------

#[test]
fn second_server_on_a_live_socket_refuses_and_stale_socket_is_recovered() {
    let mut env = start_env();
    let managed = env.managed.clone();

    // LIVE: the probe connect succeeds → typed refusal.
    let ops: Arc<dyn BrokerOps> = Arc::new(OpsImpl::new(&managed));
    let second = BrokerServer::start(&managed, ops, Arc::new(|_| {}));
    match second {
        Err(BrokerStartError::AlreadyRunning { socket_path }) => {
            assert_eq!(socket_path, env.socket_path);
        }
        Ok(_) => panic!("a live socket must refuse startup, not start a second server"),
        Err(other) => panic!("a live socket must refuse with AlreadyRunning, got: {other}"),
    }

    // The live server is genuinely serving.
    let mut client = Client::connect(&env.socket_path);
    client
        .call("probe", "projects.list", json!({}))
        .get("result")
        .expect("live server answers");
    drop(client);
    env.server.take(); // Drop → accept loop stops, socket file removed

    // STALE: a dead node where the socket was (the crash leftover). A
    // plain file defeats connect → the startup path must unlink and bind.
    std::fs::write(&env.socket_path, b"stale").expect("plant stale file");
    let ops: Arc<dyn BrokerOps> = Arc::new(OpsImpl::new(&managed));
    let recovered = BrokerServer::start(&managed, ops, Arc::new(|_| {}))
        .expect("a stale socket must be unlinked and rebound");
    assert_eq!(recovered.socket_path(), env.socket_path.as_path());

    // And the recovered server actually serves.
    let mut client2 = Client::connect(&env.socket_path);
    let answer = client2.call("after-recovery", "projects.list", json!({}));
    assert!(answer.get("result").is_some(), "{answer}");
}

// ---------------------------------------------------------------------------
// Malformed line: typed error, server (and connection) live on
// ---------------------------------------------------------------------------

#[test]
fn malformed_line_answers_typed_and_the_server_lives() {
    let env = start_env();
    let project_id = create_fixture_project(&env.managed);
    let mut client = Client::connect(&env.socket_path);

    // A broken line: typed parse error with a null id (nothing to echo).
    client.send_raw(b"{ this is not json \n");
    let frame = client.read_frame();
    expect_broker_code(&frame, codes::PARSE_ERROR, "not_a_broker_request");
    assert_eq!(frame["id"], Value::Null);

    // Valid JSON that is not a request object: still a typed PARSE error
    // (nothing request-shaped arrived), with a null id.
    client.send_raw(b"[1,2,3]\n");
    let frame = client.read_frame();
    expect_broker_code(&frame, codes::PARSE_ERROR, "not_a_broker_request");
    assert_eq!(frame["id"], Value::Null);

    // A request object with an EMPTY method: typed invalid_request.
    client.send_raw(b"{\"id\":\"e\",\"method\":\"\",\"params\":{}}\n");
    let frame = client.read_frame();
    expect_broker_code(&frame, codes::INVALID_REQUEST, "invalid_request");

    // Unknown method: typed method_not_found.
    let frame = client.call("m1", "definitely.not.a.method", json!({}));
    expect_broker_code(&frame, codes::METHOD_NOT_FOUND, "method_not_found");

    // THE SERVER LIVES: the SAME connection still executes real work.
    let answer = client.call("m2", "projects.list", json!({}));
    let projects = answer["result"]["projects"]
        .as_array()
        .expect("projects array");
    assert_eq!(
        projects
            .iter()
            .filter(|p| p["project_id"] == project_id.as_str())
            .count(),
        1
    );
}

// ---------------------------------------------------------------------------
// Paid guard: a non-local provider instance refuses typed
// ---------------------------------------------------------------------------

#[test]
fn paid_provider_instance_refuses_typed_and_local_one_starts() {
    let env = start_env();
    let project_id = create_fixture_project(&env.managed);
    let instance_id = seed_local_provider_instance(&env.managed);

    // Sanity positive: a LOCAL instance starts (the guard is scoped, not
    // a blanket refusal). The worker fails instantly on the loopback
    // discard port; the synchronous start ack is what is asserted.
    let mut client = Client::connect(&env.socket_path);
    let answer = client.call(
        "local-start",
        "ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "provider_instance_id": instance_id,
        }),
    );
    let started = answer
        .get("result")
        .expect("local instance must start")
        .clone();
    assert_eq!(started["state"], "running", "{started}");
    let started_job = started["job_id"].as_str().expect("job id").to_string();

    // Flip the persisted record to NON-local (cloud/paid). The keychain
    // is off in test builds, so a real cloud instance cannot pass the
    // public upsert seam — the flag flip edits exactly the field the
    // guard reads (`local`, providers.rs:146).
    let settings_path = env.managed.join("settings").join("providers.json");
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

    // The refusal: typed, before any launch, with the instance id and no
    // secret material anywhere.
    let answer = client.call(
        "paid-start",
        "ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "provider_instance_id": instance_id,
        }),
    );
    expect_broker_code(
        &answer,
        codes::PAID_PROVIDERS_DISABLED,
        "paid_providers_disabled",
    );
    assert_eq!(
        answer["error"]["data"]["provider_instance_id"],
        instance_id.as_str()
    );

    // No phantom job for the refused call: only the local start is
    // visible via jobs.status.
    let status = client.call("status", "jobs.status", json!({ "job_id": started_job }));
    assert!(status.get("result").is_some(), "{status}");
}

// ---------------------------------------------------------------------------
// Apply → revision_changed to every connected client + the host callback
// ---------------------------------------------------------------------------

#[test]
fn apply_emits_revision_changed_to_all_clients_and_the_host() {
    let env = start_env();
    let project_id = create_fixture_project(&env.managed);

    // Two clients: the caller and a bystander — the fan-out must reach
    // BOTH (design: «всем подключённым клиентам»).
    let mut caller = Client::connect(&env.socket_path);
    let mut bystander = Client::connect(&env.socket_path);

    // project.open mints the write coordinates and already emits its own
    // event (a new epoch invalidates other clients' epochs — worth a
    // refetch). The event frame is written during dispatch, so it lands
    // BEFORE the response frame on every client.
    caller.send(&json!({ "id": "open", "method": "project.open",
                         "params": { "project_id": project_id } }));
    let open_event_caller = caller.read_event();
    assert_eq!(open_event_caller["event"], "revision_changed");
    assert_eq!(open_event_caller["payload"]["kind"], "project_open");
    let opened = caller.read_response("open");
    assert_eq!(opened["result"]["revision"], 1);
    assert_eq!(opened["result"]["session_epoch"], 1);
    let open_event_bystander = bystander.read_event();
    assert_eq!(open_event_bystander["payload"]["kind"], "project_open");

    // apply: one valid intent → revision bumps 1 → 2.
    caller.send(&json!({
        "id": "apply",
        "method": "translation.apply",
        "params": {
            "project_id": project_id,
            "session_epoch": 1,
            "expected_revision": 1,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "Привет мир"
            }]
        }
    }));

    // The caller: event first, then the response.
    let apply_event = caller.read_event();
    assert_eq!(apply_event["event"], "revision_changed");
    assert_eq!(apply_event["payload"]["project_id"], project_id.as_str());
    assert_eq!(apply_event["payload"]["revision"], 2);
    assert_eq!(apply_event["payload"]["session_epoch"], 1);
    assert_eq!(apply_event["payload"]["kind"], "apply");
    let applied = caller.read_response("apply");
    assert_eq!(applied["result"]["revision"], 2, "{applied}");

    // The bystander received the same fan-out.
    let bystander_event = bystander.read_event();
    assert_eq!(bystander_event["payload"]["revision"], 2);
    assert_eq!(bystander_event["payload"]["kind"], "apply");

    // And the HOST callback got both events (the Tauri emit seam).
    let host = env.host_events.lock().expect("host events");
    let kinds: Vec<&str> = host
        .iter()
        .map(|e| e["payload"]["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(
        kinds,
        vec!["project_open", "apply"],
        "host events: {host:?}"
    );
}

// ---------------------------------------------------------------------------
// Stale revision: typed refusal, never a blind overwrite
// ---------------------------------------------------------------------------

#[test]
fn stale_revision_refuses_typed() {
    let env = start_env();
    let project_id = create_fixture_project(&env.managed);
    let mut client = Client::connect(&env.socket_path);

    let opened = client.call("open", "project.open", json!({ "project_id": project_id }));
    // (the project_open event was consumed by the skip loop above)

    let refused = client.call(
        "apply-stale",
        "translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": opened["result"]["session_epoch"],
            "expected_revision": 99,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "не должен примениться"
            }]
        }),
    );
    expect_broker_code(&refused, codes::CONTRACT_ERROR, "stale_revision");

    // Nothing was applied: the project stays at revision 1 with zero
    // translations.
    let snap = client.call(
        "snap",
        "project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap["result"]["revision"], 1);
    assert_eq!(snap["result"]["counts"]["translations"], 0);
}

// ---------------------------------------------------------------------------
// Draft keys stay GUI-compatible (the propose/apply key contract)
// ---------------------------------------------------------------------------

#[test]
fn propose_writes_gui_compatible_draft_keys() {
    let env = start_env();
    let project_id = create_fixture_project(&env.managed);
    let mut client = Client::connect(&env.socket_path);

    client.send(&json!({
        "id": "propose",
        "method": "translation.propose",
        "params": {
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "entries": [
                { "identity": { "kind": "keyed", "key": "Fixture_Greet" },
                  "text": "Черновик привет" },
                { "identity": { "kind": "def_injected", "key": "Some_Label",
                                "def_type": "ThingDef" },
                  "text": "Черновик два" }
            ]
        }
    }));
    let propose_event = client.read_event();
    assert_eq!(propose_event["payload"]["kind"], "propose");
    assert_eq!(
        propose_event["payload"]["revision"], 1,
        "drafts do not bump the revision"
    );
    let answered = client.read_response("propose");
    assert_eq!(answered["result"]["proposed"], 2, "{answered}");

    // The durable store holds EXACTLY the keys the GUI convention spells
    // (the same string the rimloc-mcp suite pins at
    // writable_tools.rs:357) — including the def_type middle segment.
    let drafts_path = rimloc_services::drafts_file_path(
        &env.managed.join(format!("{project_id}.rimloc.json")),
        &project_id,
    );
    let loaded = rimloc_services::load_drafts(&drafts_path).expect("drafts store loads");
    let rimloc_services::DraftsLoadOutcome::Loaded { drafts, .. } = loaded else {
        panic!("drafts store must exist after a propose");
    };
    assert!(drafts.entries.contains_key("Russian::keyed·Fixture_Greet"));
    assert!(drafts
        .entries
        .contains_key("Russian::def_injected·ThingDef·Some_Label"));
}

// ---------------------------------------------------------------------------
// Panic containment: a panicking handler costs one typed error, the server
// (and the connection) live on
// ---------------------------------------------------------------------------

/// Wraps [`OpsImpl`] and panics on one named method — the driver for the
/// `catch_unwind` guarantee (the ask: «каждая строка-обработка в отдельном
/// потоке с catch_unwind → typed internal error (сервер живёт)»).
struct PanicOn {
    method: &'static str,
    inner: OpsImpl,
}

impl BrokerOps for PanicOn {
    fn projects_list(&self) -> Result<Value, BrokerError> {
        self.inner.projects_list()
    }
    fn project_snapshot(
        &self,
        p: rimloc_broker::ops::ProjectRefParams,
    ) -> Result<Value, BrokerError> {
        if self.method == "project.snapshot" {
            panic!("synthetic handler panic: project.snapshot");
        }
        self.inner.project_snapshot(p)
    }
    fn entries_search(&self, p: rimloc_broker::ops::SearchParams) -> Result<Value, BrokerError> {
        self.inner.entries_search(p)
    }
    fn entry_context(&self, p: rimloc_broker::ops::ContextParams) -> Result<Value, BrokerError> {
        self.inner.entry_context(p)
    }
    fn validation_run(&self, p: rimloc_broker::ops::ValidateParams) -> Result<Value, BrokerError> {
        self.inner.validation_run(p)
    }
    fn export_preview(&self, p: rimloc_broker::ops::PreviewParams) -> Result<Value, BrokerError> {
        self.inner.export_preview(p)
    }
    fn jobs_status(&self, p: rimloc_broker::ops::JobIdParams) -> Result<Value, BrokerError> {
        self.inner.jobs_status(p)
    }
    fn project_open(&self, p: rimloc_broker::ops::ProjectOpenParams) -> Result<Value, BrokerError> {
        self.inner.project_open(p)
    }
    fn translation_propose(
        &self,
        p: rimloc_broker::ops::ProposeParams,
    ) -> Result<Value, BrokerError> {
        self.inner.translation_propose(p)
    }
    fn translation_apply(&self, p: rimloc_broker::ops::ApplyParams) -> Result<Value, BrokerError> {
        self.inner.translation_apply(p)
    }
    fn ai_job_start(&self, p: rimloc_broker::ops::AiJobStartParams) -> Result<Value, BrokerError> {
        self.inner.ai_job_start(p)
    }
    fn ai_job_status(&self, p: rimloc_broker::ops::JobIdParams) -> Result<Value, BrokerError> {
        self.inner.ai_job_status(p)
    }
    fn ai_job_cancel(&self, p: rimloc_broker::ops::JobIdParams) -> Result<Value, BrokerError> {
        self.inner.ai_job_cancel(p)
    }
    fn ai_job_accept_selected(
        &self,
        p: rimloc_broker::ops::AcceptSelectedParams,
    ) -> Result<Value, BrokerError> {
        self.inner.ai_job_accept_selected(p)
    }
    fn export_build(&self, p: rimloc_broker::ops::ExportBuildParams) -> Result<Value, BrokerError> {
        self.inner.export_build(p)
    }
}

#[test]
fn handler_panic_is_a_typed_internal_error_and_the_server_lives() {
    let dir = tempfile::tempdir().expect("tempdir");
    let profile = dir.path().join("profile");
    let managed = profile.join("managed");
    std::fs::create_dir_all(&managed).expect("managed root");
    let project_id = create_fixture_project(&managed);

    let ops: Arc<dyn BrokerOps> = Arc::new(PanicOn {
        method: "project.snapshot",
        inner: OpsImpl::new(&managed),
    });
    let server = BrokerServer::start(&managed, ops, Arc::new(|_| {})).expect("broker starts");
    let socket_path = server.socket_path().to_path_buf();
    // Keep the server alive for the whole test (no `env` wrapper here).
    let _server = server;

    let mut client = Client::connect(&socket_path);

    // The panicking method: one typed internal error, id echoed.
    let frame = client.call(
        "boom",
        "project.snapshot",
        json!({ "project_id": project_id }),
    );
    expect_broker_code(&frame, codes::INTERNAL_ERROR, "handler_panicked");

    // The SAME connection keeps working, and non-panicking methods are
    // untouched.
    let answer = client.call("alive", "projects.list", json!({}));
    assert!(answer.get("result").is_some(), "{answer}");
}

// ---------------------------------------------------------------------------
// Bounds: the 5th concurrent client and the oversized line
// ---------------------------------------------------------------------------

#[test]
fn client_and_line_bounds_are_typed() {
    let env = start_env();
    create_fixture_project(&env.managed);

    // Four live connections fill the profile bound.
    let mut clients: Vec<Client> = (0..4).map(|_| Client::connect(&env.socket_path)).collect();
    for (i, client) in clients.iter_mut().enumerate() {
        let answer = client.call(&format!("warm-{i}"), "projects.list", json!({}));
        assert!(answer.get("result").is_some(), "client {i} must be served");
    }

    // The 5th connect: one typed busy frame, then close.
    let mut fifth = Client::connect(&env.socket_path);
    let frame = fifth.read_frame();
    expect_broker_code(&frame, codes::BUSY, "too_many_clients");
    assert_eq!(frame["id"], Value::Null);
    drop(fifth);

    // Free a slot, then push an oversized line (> 8 MiB before the
    // newline): typed refusal and the connection closes; the SERVER and
    // the other clients live on.
    clients.pop();
    let mut over = Client::connect(&env.socket_path);
    let limit = rimloc_broker::protocol::MAX_LINE_BYTES;
    let mut giant = vec![b'x'; limit + 1];
    giant.push(b'\n');
    over.send_raw(&giant);
    let frame = over.read_frame();
    expect_broker_code(&frame, codes::LINE_TOO_LONG, "line_too_long");
    drop(over);

    // A healthy client still works after both violations.
    let answer = clients[0].call("after-abuse", "projects.list", json!({}));
    assert!(answer.get("result").is_some(), "{answer}");
}
