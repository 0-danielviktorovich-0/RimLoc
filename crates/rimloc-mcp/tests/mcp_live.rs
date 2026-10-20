//! LIVE external MCP client over the real `rimloc-mcp` binary — the DoD
//! scenarios A1, A2-minimum and A5 of issue #10
//! (`docs/development/MCP_SERVER_PHASE1.md`), one continuous stdio session.
//!
//! No mocks anywhere: the spawned process is the real binary
//! (`CARGO_BIN_EXE_rimloc-mcp`) with an isolated `RIMLOC_DATA_DIR=tempdir`
//! profile; the fixture mod is `testlab/fixtures/rimloc-wizard-mod`
//! (About + 8 English Keyed rows); stdout purity (protocol bytes ONLY —
//! «логи — stderr, stdout — только протокол») is enforced on EVERY line
//! the client ever reads, plus a trailing-bytes check at EOF.
//!
//! Project creation is the one step that cannot ride the wire: the Phase 1
//! v1 surface has no project-create tool BY DESIGN (creation is the GUI's
//! job; `rimloc.project.open` only opens existing managed projects). The
//! harness therefore seeds the project through the SAME
//! `ProjectSessionManager` seam the server itself calls — a real service
//! writing real managed envelopes, not a fixture double. The seed also
//! commits one translation (Fixture_Greet → «Привет мир»): the A1 finale
//! `rimloc.export.preview` honestly refuses a locale whose serializer
//! renders no XML (`rimloc-services/src/session.rs` — "the serializer
//! produced no XML"), so the success-flow preview needs content on disk.
//!
//! Scenario (in this order, one server process):
//! - A1: initialize → notifications/initialized → tools/list (the full ТЗ
//!   surface + honest readOnly annotations) → rimloc.status →
//!   rimloc.capabilities → projects.list → project.snapshot →
//!   entries.search paginated (offset/max_results) → validation.run →
//!   export.preview;
//! - A2-minimum: translation.propose → translation.apply of ONE selected
//!   row with epoch+revision → repeated snapshot confirms the change;
//!   the source mod tree is hash-compared before/after (byte-for-byte
//!   untouched);
//! - A5-negatives: broken JSON → typed parse error; stale revision (and
//!   stale epoch) → typed refusal, store unmoved; tools/call on an unknown
//!   tool → typed error.

use std::collections::{hash_map::DefaultHasher, BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use rimloc_mcp::error::codes;
use rimloc_services::contract::ApplyIntentsRequest;
use rimloc_services::session::ProjectSessionManager;

// ---------------------------------------------------------------------------
// The Phase 1 v1 surface exactly as declared by the ТЗ
// (MCP_SERVER_PHASE1.md, «Поверхность инструментов v1»): nine read-only
// tools, the read-only ai_job.status alias, seven controlled writes.
// ---------------------------------------------------------------------------

const SPEC_READ_ONLY: &[&str] = &[
    "rimloc.status",
    "rimloc.capabilities",
    "rimloc.projects.list",
    "rimloc.project.snapshot",
    "rimloc.entries.search",
    "rimloc.entry.context",
    "rimloc.validation.run",
    "rimloc.export.preview",
    "rimloc.jobs.status",
    "rimloc.ai_job.status",
];

const SPEC_WRITES: &[&str] = &[
    "rimloc.project.open",
    "rimloc.translation.propose",
    "rimloc.translation.apply",
    "rimloc.ai_job.start",
    "rimloc.ai_job.cancel",
    "rimloc.ai_job.accept_selected",
    "rimloc.export.build",
];

fn spec_tools_sorted() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = SPEC_READ_ONLY.to_vec();
    names.extend_from_slice(SPEC_WRITES);
    names.sort_unstable();
    names
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The canary fixture mod (pinned in-repo: About + 8 English Keyed rows).
fn fixture_mod() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testlab/fixtures/rimloc-wizard-mod")
}

/// Per-file content hashes of the whole tree (relative path → hash).
/// The fixture is tiny, so hashing every file's full bytes is the honest
/// form of «исходники мода байт-в-байт не тронуты».
fn hash_tree(root: &Path) -> BTreeMap<String, u64> {
    fn walk(root: &Path, rel: &Path, out: &mut BTreeMap<String, u64>) {
        let entries =
            std::fs::read_dir(root).unwrap_or_else(|e| panic!("read_dir {}: {e}", root.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            let child_rel = rel.join(entry.file_name());
            if path.is_dir() {
                walk(&path, &child_rel, out);
            } else {
                let bytes =
                    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                let mut hasher = DefaultHasher::new();
                bytes.hash(&mut hasher);
                out.insert(child_rel.to_string_lossy().into_owned(), hasher.finish());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, Path::new(""), &mut out);
    out
}

/// Setup seam: create the fixture project through the SAME
/// `ProjectSessionManager` the server calls, then commit ONE translation so
/// the A1 `export.preview` finale has content (see module docs). Fresh
/// manager instances, stateless like every server handler: `create` builds
/// the envelope (revision 1), the second manager's open recovers it from
/// disk (epoch 1) and the apply bumps the on-disk revision to 2.
fn seed_fixture_project(managed: &Path) -> String {
    let creator = ProjectSessionManager::new(managed).expect("manager");
    let snap = creator
        .create(&fixture_mod(), None)
        .expect("fixture mod scans into a managed project");
    drop(creator);

    let project_id = snap.project_id;
    let manager = ProjectSessionManager::new(managed).expect("manager");
    manager
        .open(&project_id)
        .expect("stateless open recovers the project from disk");
    let req: ApplyIntentsRequest = serde_json::from_value(json!({
        "project_id": project_id,
        "expected_revision": 1,
        "session_epoch": 1,
        "intents": [{
            "entry": { "kind": "keyed", "key": "Fixture_Greet" },
            "locale": "Russian",
            "action": "set_translation",
            "text": "Привет мир"
        }]
    }))
    .expect("intent request shape");
    manager.apply(&req).expect("seed translation lands");
    project_id
}

/// The stderr tail, for panic messages: a live-protocol test that dies
/// without the server's stderr is undiagnosable.
fn stderr_tail(stderr: &Mutex<Vec<String>>) -> String {
    let buf = stderr.lock().expect("stderr lock poisoned");
    let start = buf.len().saturating_sub(12);
    buf[start..].join("\n")
}

/// One live stdio session with the real binary. Strictly sequential
/// request → response: every stdout line must be exactly one JSON object,
/// and the very next line after a request must be ITS answer (this is what
/// makes «notifications produce no stdout bytes» observable).
struct LiveServer {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<Vec<String>>>,
    _stderr_thread: Option<std::thread::JoinHandle<()>>,
    next_id: u64,
    /// stdout lines ever read — for the final purity audit.
    all_lines: Vec<String>,
    /// responses the client still expects (requests sent, notifications
    /// excluded).
    expected_responses: usize,
}

impl LiveServer {
    fn spawn(data_dir: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_rimloc-mcp"))
            .env("RIMLOC_DATA_DIR", data_dir)
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
            stdin: Some(stdin),
            stdout,
            stderr,
            _stderr_thread: Some(drain),
            next_id: 0,
            all_lines: Vec::new(),
            expected_responses: 0,
        }
    }

    fn send_raw_line(&mut self, raw: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin
            .write_all(format!("{raw}\n").as_bytes())
            .and_then(|()| stdin.flush())
            .unwrap_or_else(|e| {
                panic!(
                    "write to server stdin failed: {e}; server stderr:\n{}",
                    stderr_tail(&self.stderr)
                )
            });
    }

    fn read_line(&mut self) -> String {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line).unwrap_or_else(|e| {
            panic!(
                "read from server stdout failed: {e}; server stderr:\n{}",
                stderr_tail(&self.stderr)
            )
        });
        assert!(
            bytes > 0,
            "server closed stdout before the session finished; server stderr:\n{}",
            stderr_tail(&self.stderr)
        );
        self.all_lines.push(line.clone());
        line
    }

    /// Read the NEXT stdout line and return it parsed. STDOUT PURITY lives
    /// here: every line that ever leaves the server must be exactly one
    /// complete JSON object — any stray byte, log line or partial frame
    /// fails the test right here.
    fn read_message(&mut self) -> Value {
        let line = self.read_line();
        let msg: Value = serde_json::from_str(line.trim()).unwrap_or_else(|err| {
            panic!(
                "STDOUT PURITY: non-JSON bytes on stdout (line {}): {err}\nline: {line:?}\nserver stderr:\n{}",
                self.all_lines.len(),
                stderr_tail(&self.stderr)
            )
        });
        assert!(
            msg.is_object(),
            "STDOUT PURITY: stdout line is valid JSON but not an object: {line:?}"
        );
        msg
    }

    /// Send a request with a fresh id and the gabs_mcp-style stateless
    /// `_meta.progressToken` the reference testlab client attaches to every
    /// request, then read ITS response (the next stdout line — nothing may
    /// precede it).
    fn request(&mut self, method: &str, mut params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        if let Some(obj) = params.as_object_mut() {
            obj.insert("_meta".into(), json!({ "progressToken": id }));
        }
        self.send_raw_line(
            &serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }))
            .expect("request serializes"),
        );
        self.expected_responses += 1;
        let msg = self.read_message();
        assert_eq!(
            msg["id"], id,
            "the line after request {method} (id {id}) must be ITS response, not: {msg}"
        );
        msg
    }

    /// A notification never produces a response — the NEXT line the client
    /// reads must be the answer to the NEXT request (enforced by the id
    /// check inside `request`).
    fn notification(&mut self, method: &str) {
        self.send_raw_line(
            &serde_json::to_string(&json!({ "jsonrpc": "2.0", "method": method }))
                .expect("notification serializes"),
        );
    }

    /// `tools/call` that MUST succeed; returns the tool's
    /// `structuredContent`.
    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        let resp = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        assert!(
            resp.get("error").is_none(),
            "`{name}` must succeed, got: {resp}"
        );
        resp["result"]["structuredContent"].clone()
    }

    /// `tools/call` that MUST be refused; returns the JSON-RPC error object
    /// (code / message / data).
    fn call_tool_err(&mut self, name: &str, arguments: Value) -> Value {
        let resp = self.request(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        );
        let err = resp
            .get("error")
            .unwrap_or_else(|| panic!("`{name}` must be refused, got: {resp}"));
        err.clone()
    }

    /// A raw stdin line that is not valid JSON must come back as a typed
    /// parse error with a null id (JSON-RPC 2.0 §5.1) — and still exactly
    /// one stdout line.
    fn request_broken_json(&mut self, raw: &str) -> Value {
        self.send_raw_line(raw);
        self.expected_responses += 1;
        self.read_message()
    }

    /// Close stdin (EOF), verify nothing but the counted protocol lines
    /// ever appeared on stdout, verify the clean exit.
    fn shutdown(mut self) {
        drop(self.stdin.take()); // EOF: the client closed the pipe

        // No trailing bytes after the last counted response.
        let mut extra = String::new();
        let bytes = self
            .stdout
            .read_line(&mut extra)
            .expect("final stdout read to EOF");
        assert_eq!(
            bytes, 0,
            "STDOUT PURITY: stdout carried trailing bytes after the last response: {extra:?}"
        );
        assert_eq!(
            self.lines_expected_and_seen(),
            0,
            "every request must produce exactly one stdout line (no more, no fewer)"
        );
        // Final audit over EVERY line the session ever carried.
        for line in &self.all_lines {
            let msg: Value = serde_json::from_str(line.trim())
                .unwrap_or_else(|err| panic!("STDOUT PURITY: {line:?}: {err}"));
            assert!(msg.is_object(), "STDOUT PURITY: not an object: {line:?}");
        }

        let status = self.child.wait().expect("wait on rimloc-mcp");
        if let Some(handle) = self._stderr_thread.take() {
            let _ = handle.join();
        }
        assert!(
            status.success(),
            "server exited with {status}; server stderr:\n{}",
            stderr_tail(&self.stderr)
        );
    }

    /// 0 when the counted lines line up; positive/negative otherwise.
    fn lines_expected_and_seen(&self) -> i64 {
        self.all_lines.len() as i64 - self.expected_responses as i64
    }
}

// ---------------------------------------------------------------------------
// The live scenario: A1 → A2 → A5, one server process
// ---------------------------------------------------------------------------

#[test]
fn live_stdio_session_a1_then_a2_then_a5() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_dir = dir.path().join("profile"); // → <data_dir>/managed (same rule as the GUI)
    let managed = data_dir.join("managed");

    // Hash the source mod BEFORE anything runs — the byte-for-byte oracle
    // for the whole session (A2's «исходники мода не тронуты»).
    let fixture_before = hash_tree(&fixture_mod());

    let project_id = seed_fixture_project(&managed);
    let mut srv = LiveServer::spawn(&data_dir);

    // ------------------------------------------------------------------
    // A1 — the read-only surface, live
    // ------------------------------------------------------------------

    // initialize (protocol-version negotiation, server identity).
    let init = srv.request(
        "initialize",
        json!({
            "protocolVersion": "2026-07-28",
            "capabilities": {},
            "clientInfo": { "name": "rimloc-mcp-live-test", "version": "0" }
        }),
    );
    assert!(
        init.get("error").is_none(),
        "initialize must succeed: {init}"
    );
    assert_eq!(init["result"]["protocolVersion"], "2026-07-28");
    assert_eq!(init["result"]["serverInfo"]["name"], "rimloc-mcp");
    assert_eq!(
        init["result"]["capabilities"]["tools"]["listChanged"], false,
        "the server never adds tools mid-session"
    );

    // notifications/initialized — no stdout bytes may answer it; the NEXT
    // line on stdout must be the tools/list response (id check below).
    srv.notification("notifications/initialized");

    // tools/list: the FULL ТЗ surface, each tool with a JSON Schema and
    // honest annotations; read-only hints partition exactly the read layer.
    let list = srv.request("tools/list", json!({}));
    assert!(
        list.get("error").is_none(),
        "tools/list must succeed: {list}"
    );
    let tools = list["result"]["tools"].as_array().expect("tools array");
    let mut names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name is a string"))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        spec_tools_sorted(),
        "tools/list must advertise exactly the ТЗ v1 surface"
    );
    assert!(
        list["result"].get("nextCursor").is_none(),
        "no fake pagination in Phase 1"
    );
    for tool in tools {
        let name = tool["name"].as_str().expect("name");
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "{name}: JSON Schema on input"
        );
        let annotations = &tool["annotations"];
        for hint in [
            "readOnlyHint",
            "destructiveHint",
            "idempotentHint",
            "openWorldHint",
        ] {
            assert!(
                annotations[hint].is_boolean(),
                "{name}: annotation {hint} must be present"
            );
        }
        assert_eq!(
            annotations["destructiveHint"], false,
            "{name}: the v1 surface is never destructive"
        );
        let expected_read_only = SPEC_READ_ONLY.contains(&name);
        assert_eq!(
            annotations["readOnlyHint"], expected_read_only,
            "{name}: readOnlyHint must be honest"
        );
    }

    // rimloc.status: this spawned process IS the headless writer.
    let status = srv.call_tool("rimloc.status", json!({}));
    assert_eq!(status["app"], "self");
    assert_eq!(status["mode"], "headless");
    assert_eq!(status["lock"]["held_by_us"], true);
    assert_eq!(status["profile_source"], "RIMLOC_DATA_DIR");

    // rimloc.capabilities.
    let caps = srv.call_tool("rimloc.capabilities", json!({}));
    assert_eq!(caps["contract_version"], 1);
    let supported = caps["supported"].as_array().expect("supported array");
    assert!(
        !supported.is_empty(),
        "capabilities must name supported ops"
    );
    let unsupported = caps["unsupported"].as_array().expect("unsupported array");
    assert!(
        unsupported
            .iter()
            .all(|u| u["reason"].as_str().is_some_and(|r| !r.is_empty())),
        "every unsupported entry carries a reason"
    );

    // rimloc.projects.list: the seeded fixture project is visible, live.
    let projects = srv.call_tool("rimloc.projects.list", json!({}));
    let listed = projects["projects"]
        .as_array()
        .expect("projects array")
        .iter()
        .find(|p| p["project_id"] == project_id.as_str())
        .expect("the seeded fixture project is listed")
        .clone();
    assert_eq!(listed["name"], "rimloc-wizard-mod");
    assert_eq!(
        listed["revision"], 2,
        "the seed apply bumped revision 1 → 2"
    );
    assert!(
        projects["unloadable"]
            .as_array()
            .expect("unloadable")
            .is_empty(),
        "the fresh profile has no unloadable projects"
    );

    // rimloc.project.snapshot: bounded mirror of the fixture.
    let snap = srv.call_tool(
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap["project_id"], project_id.as_str());
    assert_eq!(snap["revision"], 2);
    assert_eq!(snap["session_epoch"], 1, "stateless fresh session per call");
    assert_eq!(
        snap["counts"]["entries"], 8,
        "the canary fixture has 8 rows"
    );
    assert_eq!(snap["counts"]["translations"], 1);
    assert_eq!(snap["counts"]["translated"], 1);
    assert!(
        snap["source_root"]["path"]
            .as_str()
            .expect("source root")
            .ends_with("rimloc-wizard-mod"),
        "the snapshot mirrors the fixture mod: {snap}"
    );
    let sample = snap["entries"].as_array().expect("entries sample");
    assert_eq!(sample.len(), 8);
    assert_eq!(snap["entries_truncated"], false);
    assert!(sample.iter().any(|e| e["key"] == "Fixture_Greet"));

    // rimloc.entries.search WITH PAGINATION: 8 matches, walked in two pages.
    let page1 = srv.call_tool(
        "rimloc.entries.search",
        json!({ "project_id": project_id, "query": "fixture", "max_results": 3 }),
    );
    assert_eq!(page1["total_matches"], 8);
    assert_eq!(page1["offset"], 0);
    assert_eq!(page1["matches"].as_array().expect("page 1").len(), 3);
    assert_eq!(page1["truncated"], true, "the cut is flagged");

    let page2 = srv.call_tool(
        "rimloc.entries.search",
        json!({ "project_id": project_id, "query": "fixture", "offset": 3 }),
    );
    assert_eq!(page2["total_matches"], 8);
    assert_eq!(page2["offset"], 3);
    assert_eq!(page2["matches"].as_array().expect("page 2").len(), 5);
    assert_eq!(page2["truncated"], false, "the last page is not truncated");

    let mut seen_rows: BTreeSet<String> = BTreeSet::new();
    for page in [page1, page2] {
        for m in page["matches"].as_array().expect("matches") {
            seen_rows.insert(format!(
                "{}:{}",
                m["file"].as_str().expect("file"),
                m["line"].as_u64().expect("line")
            ));
        }
    }
    assert_eq!(
        seen_rows.len(),
        8,
        "the two pages must tile the 8 fixture rows without overlap: {seen_rows:?}"
    );

    // rimloc.validation.run over the clean fixture.
    let validation = srv.call_tool("rimloc.validation.run", json!({ "project_id": project_id }));
    assert_eq!(validation["status"], "succeeded");
    assert_eq!(validation["error_count"], 0);

    // rimloc.export.preview: the seeded translation renders real XML.
    let preview = srv.call_tool(
        "rimloc.export.preview",
        json!({ "project_id": project_id, "locale": "Russian" }),
    );
    assert_eq!(preview["locale"], "Russian");
    assert_eq!(preview["truncated"], false);
    let files = preview["files"].as_array().expect("rendered files");
    assert!(
        !files.is_empty(),
        "the seeded translation renders: {preview}"
    );
    let xml: String = files
        .iter()
        .map(|f| f["xml"].as_str().expect("xml"))
        .collect();
    assert!(
        xml.contains("Привет мир"),
        "the seed translation is in the preview XML"
    );

    // ------------------------------------------------------------------
    // A2-minimum — propose → apply ONE selected row → snapshot confirms
    // ------------------------------------------------------------------

    // translation.propose: a durable DRAFT, never an apply — no revision
    // bump, nothing committed.
    let proposed = srv.call_tool(
        "rimloc.translation.propose",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "entries": [{
                "identity": { "kind": "keyed", "key": "Fixture_Farewell" },
                "text": "Черновик: прощай"
            }]
        }),
    );
    assert_eq!(proposed["proposed"], 1);
    assert_eq!(proposed["committed"], false);
    assert_eq!(
        proposed["project_revision"], 2,
        "a draft never bumps the revision"
    );

    // translation.apply of exactly ONE selected row, with the mandatory
    // epoch + expected_revision guards.
    let applied = srv.call_tool(
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "expected_revision": 2,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Farewell" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "Прощай, друг"
            }]
        }),
    );
    assert_eq!(applied["applied"], 1, "exactly the selected row lands");
    assert_eq!(
        applied["skipped"].as_array().expect("skipped array").len(),
        0
    );
    assert_eq!(applied["revision"], 3, "the acked apply bumps 2 → 3");

    // The repeated snapshot confirms the change end-to-end.
    let snap_after = srv.call_tool(
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap_after["revision"], 3);
    assert_eq!(snap_after["counts"]["translations"], 2);
    assert_eq!(snap_after["counts"]["translated"], 2);
    let preview_after = srv.call_tool(
        "rimloc.export.preview",
        json!({ "project_id": project_id, "locale": "Russian" }),
    );
    let xml_after: String = preview_after["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| f["xml"].as_str().expect("xml"))
        .collect();
    assert!(
        xml_after.contains("Прощай, друг"),
        "the applied row is in the rendered output: {xml_after}"
    );

    // ------------------------------------------------------------------
    // A5-negatives — typed failures, store unmoved, stdout pure
    // ------------------------------------------------------------------

    // Битый JSON → typed parse error (-32700) with a null id.
    let broken = srv.request_broken_json("this is definitely not json {");
    assert_eq!(
        broken["error"]["code"],
        codes::PARSE_ERROR,
        "broken JSON must be a typed parse error: {broken}"
    );
    assert!(
        broken["id"].is_null(),
        "an unparseable line has no detectable id: {broken}"
    );

    // Stale revision → typed refusal (stale_revision in data.code).
    let stale_rev = srv.call_tool_err(
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "expected_revision": 999,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "не должен примениться"
            }]
        }),
    );
    assert_eq!(
        stale_rev["code"],
        codes::CONTRACT_ERROR,
        "stale revision is a typed contract refusal: {stale_rev}"
    );
    assert_eq!(stale_rev["data"]["code"], "stale_revision");

    // Stale epoch → typed refusal too (the A5 pair of the same guard).
    let stale_epoch = srv.call_tool_err(
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 42,
            "expected_revision": 3,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "не должен примениться"
            }]
        }),
    );
    assert_eq!(stale_epoch["code"], codes::CONTRACT_ERROR);
    assert_eq!(stale_epoch["data"]["code"], "stale_epoch");

    // The refusals must have left the store untouched.
    let snap_untouched = srv.call_tool(
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap_untouched["revision"], 3, "refusals never mutate");
    assert_eq!(snap_untouched["counts"]["translations"], 2);

    // tools/call on an unknown tool → typed error.
    let unknown = srv.call_tool_err("rimloc.no_such_tool", json!({}));
    assert_eq!(
        unknown["code"],
        codes::INVALID_PARAMS,
        "an unknown tool is invalid params: {unknown}"
    );
    assert!(
        unknown["message"]
            .as_str()
            .expect("message is a string")
            .contains("rimloc.no_such_tool"),
        "the refusal names the unknown tool: {unknown}"
    );

    // ------------------------------------------------------------------
    // Shutdown: stdout stayed pure, the server exits cleanly, and the
    // source mod tree is byte-for-byte what it was (hash comparison).
    // ------------------------------------------------------------------

    srv.shutdown();

    let fixture_after = hash_tree(&fixture_mod());
    assert_eq!(
        fixture_after, fixture_before,
        "the source mod tree must be byte-for-byte untouched by the whole live session"
    );
}
