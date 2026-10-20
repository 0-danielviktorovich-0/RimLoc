//! Integration coverage for the live READ-ONLY MCP tool layer
//! (`docs/development/MCP_SERVER_PHASE1.md`, DoD A1 shape, driven at the
//! `tools::call` seam).
//!
//! Every test runs against a TEMPDIR profile (`Runtime::build` with an
//! explicit root — no `RIMLOC_DATA_DIR` process-global mutation, tests
//! stay parallel-safe) and uses the canary fixture mod
//! `testlab/fixtures/rimloc-wizard-mod` (About + 8 English Keyed entries)
//! as the project source. Project creation for setup goes through the
//! SAME `ProjectSessionManager` the server itself calls — no parallel
//! fixture machinery exists.
//!
//! The last two tests spawn the real binary: `doctor` verifies the
//! `RIMLOC_DATA_DIR` profile resolution end-to-end, `serve` verifies the
//! full stdio protocol loop (initialize → tools/call) including stdout
//! purity — every byte on stdout is one complete JSON-RPC response line.

use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

use rimloc_mcp::error::{codes, RpcError};
use rimloc_mcp::runtime::{AppMode, Runtime, LOCK_FILE_NAME};
use rimloc_mcp::tools;
use rimloc_services::session::ProjectSessionManager;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The canary fixture mod (pinned in-repo: About + 8 English Keyed rows).
fn fixture_mod() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testlab/fixtures/rimloc-wizard-mod")
}

struct TestEnv {
    /// Keeps the tempdir alive for the whole test.
    _dir: tempfile::TempDir,
    managed: PathBuf,
    rt: Runtime,
}

/// A tempdir profile with THIS process holding the exclusive writer lock.
fn env_self() -> TestEnv {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("profile").join("managed");
    let rt = Runtime::build(managed.clone()).expect("runtime builds");
    TestEnv {
        _dir: dir,
        managed,
        rt,
    }
}

/// Setup seam: create the fixture project through the SAME service the
/// server calls, then drop the manager (stateless, like every handler).
fn create_fixture_project(managed: &Path) -> String {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    let snap = manager
        .create(&fixture_mod(), None)
        .expect("fixture mod scans into a managed project");
    snap.project_id
}

/// Setup seam: one durable translation so export.preview has content. The
/// project must be OPENED on this fresh manager instance first — `apply`
/// runs against live session state, and the stateless open recovers it
/// from disk (epoch 1).
fn apply_one_translation(managed: &Path, project_id: &str) {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    manager
        .open(project_id)
        .expect("stateless open recovers the project from disk");
    let req: rimloc_services::contract::ApplyIntentsRequest = serde_json::from_value(json!({
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
    manager.apply(&req).expect("apply lands on the fixture");
}

fn call(rt: &Runtime, name: &str, args: Value) -> Value {
    tools::call(name, args, rt).unwrap_or_else(|err| panic!("`{name}` must succeed: {err:?}"))
}

fn call_err(rt: &Runtime, name: &str, args: Value) -> RpcError {
    tools::call(name, args, rt)
        .err()
        .unwrap_or_else(|| panic!("`{name}` must refuse"))
}

// ---------------------------------------------------------------------------
// Lock + status
// ---------------------------------------------------------------------------

#[test]
fn lock_is_exclusive_and_raii_released() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("profile").join("managed");

    let first = Runtime::build(managed.clone()).expect("first writer wins");
    assert_eq!(*first.mode(), AppMode::HeadlessWriter);
    assert!(
        managed.join(LOCK_FILE_NAME).is_file(),
        "the lock file lives in the managed root"
    );

    // Same process, same pid, same boot token → busy, never a self-steal.
    let second = Runtime::build(managed.clone()).expect("busy is not an error");
    assert!(second.is_other_writer());
    match second.mode() {
        AppMode::OtherWriter { owner } => {
            assert_eq!(owner.pid, Some(std::process::id()), "owner pid is honest");
        }
        other => panic!("expected other_writer, got {other:?}"),
    }

    drop(first);
    assert!(
        !managed.join(LOCK_FILE_NAME).exists(),
        "RAII removes the lock file"
    );
    let third = Runtime::build(managed).expect("lock is free again");
    assert_eq!(*third.mode(), AppMode::HeadlessWriter);
}

#[test]
fn recycled_pid_stale_lock_is_stolen_foreign_lock_is_not() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("managed");
    std::fs::create_dir_all(&managed).expect("managed root");

    // A dead previous process whose pid the OS handed to US: recorded boot
    // token differs → provably stale → stolen.
    let stale = managed.join(LOCK_FILE_NAME);
    std::fs::write(
        &stale,
        format!(
            "pid={}\nboot=dead-process-of-a-recycled-pid\n",
            std::process::id()
        ),
    )
    .expect("stale lock seed");
    let rt = Runtime::build(managed.clone()).expect("stale same-pid lock is stolen");
    assert_eq!(*rt.mode(), AppMode::HeadlessWriter);
    drop(rt);

    // A foreign pid is never guessed away — even if that pid is long gone,
    // std has no portable liveness check and busy is the honest answer.
    std::fs::write(&stale, "pid=999999999\nboot=someone-else\n").expect("foreign lock seed");
    let rt = Runtime::build(managed).expect("busy is not an error");
    assert!(rt.is_other_writer(), "a foreign lock is never stolen");
}

#[test]
fn status_reports_self_mode_versions_and_lock() {
    let env = env_self();
    let out = call(&env.rt, "rimloc.status", json!({}));
    let sc = &out["structuredContent"];
    assert_eq!(sc["app"], "self");
    assert_eq!(sc["mode"], "headless");
    assert_eq!(sc["protocol_version"], "2026-07-28");
    assert_eq!(sc["ui_contract_version"], 1);
    assert_eq!(sc["lock"]["held_by_us"], true);
    assert_eq!(sc["lock"]["owner_pid"], std::process::id());
    assert_eq!(sc["project_scoped_reads"], "available");
    assert_eq!(sc["tools"]["declared"], tools::REGISTRY.len());
    assert_eq!(sc["tools"]["read_only_executable"], 10);
    assert_eq!(sc["tools"]["writes_executable"], 7);
    assert!(
        sc["managed_root"]
            .as_str()
            .expect("root")
            .contains("managed"),
        "the resolved profile is reported: {sc}"
    );
    assert!(out["content"][0]["text"]
        .as_str()
        .expect("text summary")
        .contains("headless writer"));
}

#[test]
fn status_reports_other_writer_honestly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("managed");
    let owner = Runtime::build(managed.clone()).expect("first writer");
    let busy = Runtime::build(managed).expect("busy runtime");

    let out = call(&busy, "rimloc.status", json!({}));
    let sc = &out["structuredContent"];
    assert_eq!(sc["app"], "other_writer");
    assert_eq!(sc["lock"]["held_by_us"], false);
    assert_eq!(sc["lock"]["owner_pid"], std::process::id());
    assert_eq!(sc["project_scoped_reads"], "refused_other_writer");
    assert_eq!(sc["tools"]["read_only_executable"], 5);
    assert_eq!(sc["tools"]["writes_executable"], 0);
    drop(owner);
}

// ---------------------------------------------------------------------------
// Capabilities + projects.list
// ---------------------------------------------------------------------------

#[test]
fn capabilities_lists_the_slice_boundary() {
    let env = env_self();
    let out = call(&env.rt, "rimloc.capabilities", json!({}));
    let sc = &out["structuredContent"];
    assert_eq!(sc["contract_version"], 1);
    let supported: Vec<&str> = sc["supported"]
        .as_array()
        .expect("supported array")
        .iter()
        .map(|v| v.as_str().expect("snake_case capability"))
        .collect();
    for capability in ["project_list", "project_snapshot", "project_validate"] {
        assert!(
            supported.contains(&capability),
            "{capability} must be reported supported"
        );
    }
    let unsupported = sc["unsupported"].as_array().expect("unsupported array");
    assert!(
        unsupported
            .iter()
            .all(|u| u["reason"].as_str().is_some_and(|r| !r.is_empty())),
        "every unsupported entry carries a reason"
    );
}

#[test]
fn projects_list_reports_fixture_project_and_unloadable_diagnostics() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);

    // Corruption must surface as unloadable diagnostics, never as silence.
    std::fs::write(
        env.managed.join("proj-bogus.rimloc.json"),
        "{ not valid json",
    )
    .expect("corrupt managed file seed");

    let out = call(&env.rt, "rimloc.projects.list", json!({}));
    let sc = &out["structuredContent"];
    let projects = sc["projects"].as_array().expect("projects array");
    let mine = projects
        .iter()
        .find(|p| p["project_id"] == project_id.as_str())
        .expect("created project is listed");
    assert_eq!(
        mine["name"], "rimloc-wizard-mod",
        "display name is the mod folder"
    );
    assert_eq!(mine["revision"], 1);
    let unloadable = sc["unloadable"].as_array().expect("unloadable array");
    let bogus = unloadable
        .iter()
        .find(|u| u["project_id"] == "proj-bogus")
        .expect("corrupt file is diagnosed");
    assert_eq!(bogus["reason"], "corrupt_project");
}

// ---------------------------------------------------------------------------
// Snapshot (bounded mirror + stateless epochs)
// ---------------------------------------------------------------------------

#[test]
fn snapshot_is_a_bounded_mirror_of_the_fixture() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let out = call(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["project_id"], project_id.as_str());
    assert_eq!(sc["revision"], 1);
    assert_eq!(sc["session_epoch"], 1);
    assert_eq!(sc["dirty"], false);
    assert_eq!(sc["acked_revision"], 1);
    assert_eq!(sc["source_changed"], false);
    assert!(sc["source_root"]["path"]
        .as_str()
        .expect("source root")
        .ends_with("rimloc-wizard-mod"));
    let counts = &sc["counts"];
    assert_eq!(
        counts["entries"], 8,
        "the canary fixture carries exactly 8 English Keyed rows"
    );
    assert_eq!(counts["translations"], 0);
    assert_eq!(counts["translated"], 0);
    let entries = sc["entries"].as_array().expect("entries sample");
    assert_eq!(entries.len(), 8);
    assert_eq!(sc["entries_truncated"], false);
    assert_eq!(
        entries[0]["kind"], "keyed",
        "sample entries carry structural identity"
    );
    assert!(entries.iter().any(|e| e["key"] == "Fixture_Greet"));
}

#[test]
fn snapshot_epoch_mismatch_is_a_typed_stale_epoch_refusal() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let err = call_err(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": project_id, "session_epoch": 99 }),
    );
    assert_eq!(err.code, codes::CONTRACT_ERROR);
    assert_eq!(err.data.expect("data")["code"], "stale_epoch");
}

#[test]
fn snapshot_session_is_fresh_on_every_call_stateless() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    for _ in 0..3 {
        let out = call(
            &env.rt,
            "rimloc.project.snapshot",
            json!({ "project_id": project_id }),
        );
        // A cached live session would bump its epoch on every open; the
        // stateless server drops the manager per call, so the epoch stays
        // at the fresh-session value 1 forever.
        assert_eq!(out["structuredContent"]["session_epoch"], 1);
    }
}

#[test]
fn snapshot_unknown_project_is_typed_refusal() {
    let env = env_self();
    let err = call_err(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": "proj-missing" }),
    );
    assert_eq!(err.code, codes::CONTRACT_ERROR);
    assert_eq!(err.data.expect("data")["code"], "project_not_found");
    // Form guard: non-minted ids refuse as contract violations BEFORE any
    // filesystem access (the services managed_path discipline).
    let err = call_err(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": "../../etc" }),
    );
    assert_eq!(err.data.expect("data")["code"], "contract_violation");
}

// ---------------------------------------------------------------------------
// entries.search (pagination + bounds)
// ---------------------------------------------------------------------------

#[test]
fn entries_search_paginates_and_flags_truncation() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);

    let page1 = call(
        &env.rt,
        "rimloc.entries.search",
        json!({ "project_id": project_id, "query": "FIXTURE", "max_results": 2 }),
    );
    let sc = &page1["structuredContent"];
    assert_eq!(
        sc["total_matches"], 8,
        "case-insensitive: every keyed row line matches"
    );
    assert_eq!(sc["matches"].as_array().expect("page").len(), 2);
    assert_eq!(sc["offset"], 0);
    assert_eq!(sc["truncated"], true, "the cut is flagged");
    assert!(
        page1["content"][0]["text"]
            .as_str()
            .expect("summary")
            .contains("truncated: true"),
        "the text summary mentions truncation"
    );

    let page2 = call(
        &env.rt,
        "rimloc.entries.search",
        json!({ "project_id": project_id, "query": "fixture", "offset": 2 }),
    );
    let sc2 = &page2["structuredContent"];
    assert_eq!(sc2["total_matches"], 8);
    assert_eq!(sc2["matches"].as_array().expect("page").len(), 6);
    assert_eq!(sc2["truncated"], false, "last page is not truncated");
}

#[test]
fn entries_search_short_query_is_a_typed_refusal() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let err = call_err(
        &env.rt,
        "rimloc.entries.search",
        json!({ "project_id": project_id, "query": "f" }),
    );
    assert_eq!(err.code, codes::CONTRACT_ERROR);
    assert_eq!(err.data.expect("data")["code"], "validation_failed");
}

#[test]
fn entries_search_project_without_source_root_refuses_typed() {
    // A project whose envelope carries no source root — search refuses
    // typed instead of guessing a root. The seed is a REAL fixture project
    // with `meta.source_root` stripped from the envelope, so the container
    // itself loads (honest unknown, exactly the legacy-envelope state the
    // contract describes).
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let path = env.managed.join(format!("{project_id}.rimloc.json"));
    let mut envelope: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("envelope"))
            .expect("envelope parses");
    // The envelope FLATTENS meta: source_root sits at the top level of the
    // managed file.
    envelope["source_root"] = Value::Null;
    std::fs::write(&path, serde_json::to_vec(&envelope).expect("envelope"))
        .expect("stripped envelope saved");

    let out = call(&env.rt, "rimloc.projects.list", json!({}));
    let listed = out["structuredContent"]["projects"]
        .as_array()
        .expect("projects")
        .iter()
        .any(|p| p["project_id"] == project_id.as_str());
    assert!(listed, "stripped envelope still loads: {out}");
    let err = call_err(
        &env.rt,
        "rimloc.entries.search",
        json!({ "project_id": project_id, "query": "anything" }),
    );
    assert_eq!(err.data.expect("data")["code"], "invalid_config");
}

// ---------------------------------------------------------------------------
// entry.context (containment + bounds)
// ---------------------------------------------------------------------------

#[test]
fn entry_context_returns_the_bounded_window() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let out = call(
        &env.rt,
        "rimloc.entry.context",
        json!({
            "project_id": project_id,
            "file": "Languages/English/Keyed/Fixture.xml",
            "line": 3,
            "before": 1,
            "after": 1
        }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["file"], "Languages/English/Keyed/Fixture.xml");
    assert_eq!(sc["target_line"], 3);
    let lines = sc["lines"].as_array().expect("window");
    let numbers: Vec<u64> = lines.iter().map(|l| l["n"].as_u64().expect("n")).collect();
    assert_eq!(numbers, vec![2, 3, 4], "one line of context on each side");
    assert!(
        lines.iter().any(|l| l["text"]
            .as_str()
            .expect("text")
            .contains("Fixture_Farewell")),
        "the real fixture text is in the window: {lines:?}"
    );
}

#[test]
fn entry_context_refuses_paths_that_escape_the_source_root() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    for bad in [
        "../About/About.xml",
        "/etc/hostname",
        "Languages/../../../../../etc/passwd",
    ] {
        let err = call_err(
            &env.rt,
            "rimloc.entry.context",
            json!({ "project_id": project_id, "file": bad }),
        );
        assert_eq!(err.code, codes::CONTRACT_ERROR, "{bad}");
        assert_eq!(
            err.data.expect("data")["code"],
            "guard_output_denied",
            "{bad}"
        );
    }
    let err = call_err(
        &env.rt,
        "rimloc.entry.context",
        json!({ "project_id": project_id, "file": "Languages/English/Keyed/Missing.xml" }),
    );
    assert_eq!(err.data.expect("data")["code"], "invalid_config");
}

// ---------------------------------------------------------------------------
// validation.run + export.preview
// ---------------------------------------------------------------------------

#[test]
fn validation_run_succeeds_on_the_clean_fixture() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let out = call(
        &env.rt,
        "rimloc.validation.run",
        json!({ "project_id": project_id }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["status"], "succeeded");
    assert_eq!(sc["error_count"], 0);
    assert_eq!(sc["warning_count"], 0);
    assert!(
        sc["job_id"].as_str().expect("job id").starts_with("op-"),
        "the validation op id rides the response"
    );
}

#[test]
fn export_preview_lists_rendered_files_after_a_translation() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    apply_one_translation(&env.managed, &project_id);

    let out = call(
        &env.rt,
        "rimloc.export.preview",
        json!({ "project_id": project_id, "locale": "Russian" }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["locale"], "Russian");
    assert_eq!(sc["truncated"], false);
    let files = sc["files"].as_array().expect("rendered files");
    assert!(!files.is_empty(), "the translated keyed row renders: {sc}");
    assert!(files
        .iter()
        .any(|f| f["xml"].as_str().expect("xml").contains("Привет мир")));
    assert!(out["content"][0]["text"]
        .as_str()
        .expect("summary")
        .contains("Russian"));
}

// ---------------------------------------------------------------------------
// jobs.status (durable record)
// ---------------------------------------------------------------------------

fn seed_job_record(managed: &Path) {
    let record = json!({
        "job_id": "op-test-abc",
        "project_id": "proj-x",
        "base_revision": 1,
        "locale": "Russian",
        "provider_id": "test-provider",
        "model": "test-model",
        "state": "interrupted",
        "total": 3,
        "translated": 1,
        "failed": 0,
        "excluded_notranslate": 0,
        "rows": [],
        "rows_truncated": false
    });
    std::fs::write(
        managed.join("op-test-abc.job.json"),
        serde_json::to_vec_pretty(&record).expect("record"),
    )
    .expect("job record seed");
}

#[test]
fn jobs_status_reads_the_durable_record_and_refuses_unknown_typed() {
    let env = env_self();
    seed_job_record(&env.managed);
    let out = call(
        &env.rt,
        "rimloc.jobs.status",
        json!({ "job_id": "op-test-abc" }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["job_id"], "op-test-abc");
    assert_eq!(
        sc["state"], "interrupted",
        "restored Interrupted jobs are visible"
    );
    assert_eq!(sc["translated"], 1);
    assert_eq!(sc["total"], 3);

    let err = call_err(
        &env.rt,
        "rimloc.jobs.status",
        json!({ "job_id": "op-missing" }),
    );
    assert_eq!(err.code, codes::CONTRACT_ERROR);
    assert_eq!(err.data.expect("data")["code"], "project_not_found");
}

// ---------------------------------------------------------------------------
// other_writer mode: the honest degradation matrix
// ---------------------------------------------------------------------------

#[test]
fn other_writer_mode_serves_disk_diagnostics_and_refuses_the_rest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("managed");
    let owner = Runtime::build(managed.clone()).expect("first writer");
    let project_id = create_fixture_project(&managed);
    seed_job_record(&managed);

    let busy = Runtime::build(managed.clone()).expect("busy runtime");
    assert!(busy.is_other_writer());

    // Disk diagnostics stay up (atomic envelopes + records).
    let list = call(&busy, "rimloc.projects.list", json!({}));
    assert!(
        list["structuredContent"]["projects"]
            .as_array()
            .expect("projects")
            .iter()
            .any(|p| p["project_id"] == project_id.as_str()),
        "disk-only list still sees the project"
    );
    let job = call(
        &busy,
        "rimloc.jobs.status",
        json!({ "job_id": "op-test-abc" }),
    );
    assert_eq!(job["structuredContent"]["state"], "interrupted");
    assert_eq!(
        call(&busy, "rimloc.capabilities", json!({}))["structuredContent"]["contract_version"],
        1
    );

    // Project-scoped reads refuse: session open would run the manager
    // constructor's recovery write against a live writer's store.
    for (tool, args) in [
        (
            "rimloc.project.snapshot",
            json!({ "project_id": project_id }),
        ),
        (
            "rimloc.entries.search",
            json!({ "project_id": project_id, "query": "fixture" }),
        ),
        (
            "rimloc.entry.context",
            json!({ "project_id": project_id, "file": "Languages/English/Keyed/Fixture.xml" }),
        ),
        ("rimloc.validation.run", json!({ "project_id": project_id })),
        (
            "rimloc.export.preview",
            json!({ "project_id": project_id, "locale": "Russian" }),
        ),
    ] {
        let err = call_err(&busy, tool, args);
        assert_eq!(err.code, codes::OTHER_WRITER, "{tool}");
        assert_eq!(
            err.data.expect("data")["code"],
            "other_writer_active",
            "{tool}"
        );
    }

    // The write layer refuses with the honest lock verdict now (it is
    // wired: other_writer_active, not not_implemented).
    let err = call_err(
        &busy,
        "rimloc.project.open",
        json!({ "project_id": project_id }),
    );
    assert_eq!(err.code, codes::OTHER_WRITER);
    assert_eq!(err.data.expect("data")["code"], "other_writer_active");

    // The read-only job-status alias stays up in other_writer mode too
    // (durable-record read, lock-free) — same as rimloc.jobs.status.
    let alias = call(
        &busy,
        "rimloc.ai_job.status",
        json!({ "job_id": "op-test-abc" }),
    );
    assert_eq!(alias["structuredContent"]["state"], "interrupted");
    drop(owner);
}

// ---------------------------------------------------------------------------
// The real binary: doctor (profile resolution) + serve (stdio protocol)
// ---------------------------------------------------------------------------

#[test]
fn doctor_resolves_the_rimloc_data_dir_profile_in_a_child_process() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_rimloc-mcp"))
        .arg("doctor")
        .env("RIMLOC_DATA_DIR", dir.path())
        .output()
        .expect("doctor runs");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains(&format!(
            "RIMLOC_DATA_DIR: {} (isolated profile)",
            dir.path().display()
        )),
        "doctor reports the isolated profile: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "resolved managed root: {}",
            dir.path().join("managed").display()
        )),
        "doctor reports the resolved root: {stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "tools executable: {} ({} read-only + {} controlled writes; all wired)",
            tools::REGISTRY.len(),
            tools::WIRED_READ_ONLY.len() + tools::WIRED_READ_ONLY_ALIASES.len(),
            tools::WIRED_WRITES.len()
        )),
        "{stdout}"
    );
}

#[test]
fn serve_answers_initialize_and_status_over_real_stdio() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rimloc-mcp"))
        .env("RIMLOC_DATA_DIR", dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("server spawns");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    let send = |stdin: &mut std::process::ChildStdin, line: Value| {
        stdin
            .write_all(format!("{line}\n").as_bytes())
            .and_then(|()| stdin.flush())
            .expect("request written");
    };

    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2026-07-28",
                "capabilities": {},
                "clientInfo": { "name": "rimloc-mcp-test", "version": "0" },
                "_meta": { "progressToken": 1 }
            }
        }),
    );
    send(
        &mut stdin,
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    );
    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": "rimloc.status", "arguments": {} }
        }),
    );
    send(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "rimloc.projects.list", "arguments": {} }
        }),
    );

    // read_id: every stdout line must be ONE complete JSON object — the
    // stdout-purity property of the transport is enforced by construction.
    let read_id = |stdout: &mut BufReader<std::process::ChildStdout>, want: u64| -> Value {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = stdout.read_line(&mut line).expect("response line");
            assert!(bytes > 0, "server closed stdout before responding {want}");
            let msg: Value = serde_json::from_str(line.trim())
                .unwrap_or_else(|err| panic!("stdout carried non-JSON bytes: {err}: {line}"));
            if msg["id"] == want {
                return msg;
            }
        }
    };

    let init = read_id(&mut stdout, 1);
    assert_eq!(init["result"]["protocolVersion"], "2026-07-28");
    assert_eq!(init["result"]["serverInfo"]["name"], "rimloc-mcp");

    let status = read_id(&mut stdout, 2);
    assert!(status.get("error").is_none(), "{status}");
    assert_eq!(status["result"]["structuredContent"]["app"], "self");

    let list = read_id(&mut stdout, 3);
    assert_eq!(
        list["result"]["structuredContent"]["projects"]
            .as_array()
            .expect("empty projects page still answers")
            .len(),
        0
    );

    let _ = child.kill();
    let _ = child.wait();
}
