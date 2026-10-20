//! Integration coverage for the CONTROLLED-WRITE MCP tool layer
//! (`docs/development/MCP_SERVER_PHASE1.md`, DoD A2/A5 shape, driven at the
//! `tools::call` seam — the same harness discipline as
//! `tests/readonly_tools.rs`).
//!
//! The four behaviors this ask pins, each with a dedicated test:
//! - stale epoch / stale revision → TYPED refusal, never a blind overwrite
//!   (`apply_refuses_stale_epoch_and_revision_first`);
//! - apply/accept touch ONLY the explicitly named identities
//!   (`apply_applies_only_the_explicit_identities`,
//!   `accept_selected_applies_only_the_selected_rows`);
//! - propose is a durable DRAFT store with MERGE semantics — an earlier
//!   draft is never clobbered by a later propose, and no draft ever becomes
//!   a committed translation (`propose_drafts_merge_and_never_become_committed`);
//! - export.build writes ONLY the explicit guarded out dir — the source
//!   mod tree is byte-compared before/after and the managed root is
//!   byte-compared across the export itself
//!   (`export_build_writes_only_the_explicit_guarded_dir`).
//!
//! AI-job tests never touch the network: the start-path test drives a LOCAL
//! `ollama` provider instance whose base_url is a loopback discard port
//! (connection refused instantly — the worker fails, the synchronous
//! contract is what is asserted), and the accept-path test drives the
//! services' demo `mock` provider behind its documented
//! `RIMLOC_ALLOW_MOCK_JOBS=1` opt-in (the exact opt-in the services' own
//! test-suite uses), started through the SAME public `start_job` seam the
//! MCP tool calls. Paid providers stay out of tests entirely: in this
//! build a secret-bearing provider upsert refuses (`keychain` feature off),
//! which its own test pins.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use rimloc_mcp::error::{codes, RpcError};
use rimloc_mcp::runtime::Runtime;
use rimloc_mcp::tools;
use rimloc_services::session::ProjectSessionManager;

// ---------------------------------------------------------------------------
// Harness (mirrors tests/readonly_tools.rs)
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

fn create_fixture_project(managed: &Path) -> String {
    let manager = ProjectSessionManager::new(managed).expect("manager");
    let snap = manager
        .create(&fixture_mod(), None)
        .expect("fixture mod scans into a managed project");
    snap.project_id
}

/// Byte-exact snapshot of every file under `root` (relative path → bytes).
/// The fixture is tiny, so full-content comparison is the honest form of
/// "the source mod tree is byte-for-byte untouched".
fn snapshot_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, rel: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let entries = std::fs::read_dir(root).unwrap_or_else(|e| {
            panic!("read_dir {}: {e}", root.display());
        });
        for entry in entries.flatten() {
            let path = entry.path();
            let child_rel = rel.join(entry.file_name());
            if path.is_dir() {
                walk(&path, &child_rel, out);
            } else {
                let bytes =
                    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
                // Unix-style keys on every OS: the tree is compared
                // before/after (separator-agnostic either way) and matched
                // with slash-bearing substrings below.
                out.insert(child_rel.to_string_lossy().replace('\\', "/"), bytes);
            }
        }
    }
    walk(root, Path::new(""), &mut out);
    out
}

fn call(rt: &Runtime, name: &str, args: Value) -> Value {
    tools::call(name, args, rt).unwrap_or_else(|err| panic!("`{name}` must succeed: {err:?}"))
}

fn call_err(rt: &Runtime, name: &str, args: Value) -> RpcError {
    tools::call(name, args, rt)
        .err()
        .unwrap_or_else(|| panic!("`{name}` must refuse"))
}

fn expect_contract_code(err: &RpcError, code: &str) {
    assert_eq!(err.code, codes::CONTRACT_ERROR, "{err:?}");
    assert_eq!(
        err.data.as_ref().expect("typed data")["code"],
        code,
        "{err:?}"
    );
}

/// Poll the job registry until the job leaves `running` (the mock worker
/// finishes in milliseconds; the bound only guards a wedged CI machine).
fn wait_until_not_running(rt: &Runtime, job_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let status = call(rt, "rimloc.ai_job.status", json!({ "job_id": job_id }));
        let state = status["structuredContent"]["state"]
            .as_str()
            .expect("state is a string")
            .to_string();
        if state != "running" {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "job {job_id} never left the running state"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

/// A LOCAL ollama provider instance whose base_url is the loopback discard
/// port: form-valid for the service guards, connection-refused instantly —
/// the only network a test ever touches.
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
// rimloc.project.open
// ---------------------------------------------------------------------------

#[test]
fn project_open_returns_the_write_coordinates() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let out = call(
        &env.rt,
        "rimloc.project.open",
        json!({ "project_id": project_id }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["project_id"], project_id.as_str());
    assert_eq!(sc["revision"], 1);
    assert_eq!(sc["session_epoch"], 1);
    assert_eq!(sc["dirty"], false);
    assert_eq!(sc["acked_revision"], 1);
    assert_eq!(sc["source_changed"], false);
    assert_eq!(sc["counts"]["entries"], 8);
    assert_eq!(sc["counts"]["translations"], 0);
    assert!(out["content"][0]["text"]
        .as_str()
        .expect("summary")
        .contains("epoch 1"));
}

// ---------------------------------------------------------------------------
// rimloc.translation.apply: typed stale guards + explicit identities only
// ---------------------------------------------------------------------------

#[test]
fn apply_refuses_stale_epoch_and_revision_first() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let good_intent = json!({
        "entry": { "kind": "keyed", "key": "Fixture_Greet" },
        "locale": "Russian",
        "action": "set_translation",
        "text": "Привет мир"
    });

    // Stale epoch (a newer open superseded the caller's): typed refusal,
    // and the project state is untouched afterwards.
    let err = call_err(
        &env.rt,
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 99,
            "expected_revision": 1,
            "intents": [good_intent]
        }),
    );
    expect_contract_code(&err, "stale_epoch");

    // Stale revision (the caller based its edit on a revision that never
    // was): typed refusal.
    let err = call_err(
        &env.rt,
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "expected_revision": 7,
            "intents": [good_intent]
        }),
    );
    expect_contract_code(&err, "stale_revision");

    // Neither refusal applied anything.
    let snap = call(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap["structuredContent"]["counts"]["translations"], 0);
    assert_eq!(snap["structuredContent"]["revision"], 1);
}

#[test]
fn apply_applies_only_the_explicit_identities() {
    let env = env_self();
    let fixture_before = snapshot_tree(&fixture_mod());
    let project_id = create_fixture_project(&env.managed);

    // TWO intents: one resolvable identity, one identity that is NOT in
    // the inventory. The service resolves each explicitly — the bogus one
    // is skipped AS DATA, the valid one applies.
    let out = call(
        &env.rt,
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "expected_revision": 1,
            "intents": [
                {
                    "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                    "locale": "Russian",
                    "action": "set_translation",
                    "text": "Привет мир"
                },
                {
                    "entry": { "kind": "keyed", "key": "NoSuch_Key_Anywhere" },
                    "locale": "Russian",
                    "action": "set_translation",
                    "text": "не должен примениться"
                }
            ]
        }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(
        sc["applied"], 1,
        "only the explicit valid identity lands: {sc}"
    );
    let skipped = sc["skipped"].as_array().expect("skipped array");
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0]["index"], 1);
    assert_eq!(skipped[0]["code"], "contract_violation");
    assert_eq!(sc["revision"], 2, "the acked apply bumps the revision");

    // Exactly ONE translation exists afterwards, and it is the named one.
    let snap = call(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap["structuredContent"]["counts"]["translations"], 1);
    assert_eq!(snap["structuredContent"]["counts"]["translated"], 1);
    let preview = call(
        &env.rt,
        "rimloc.export.preview",
        json!({ "project_id": project_id, "locale": "Russian" }),
    );
    let xml = preview["structuredContent"]["files"]
        .as_array()
        .expect("preview files")
        .iter()
        .map(|f| f["xml"].as_str().expect("xml"))
        .collect::<String>();
    assert!(xml.contains("Привет мир"));
    assert!(!xml.contains("не должен примениться"));

    // The source mod tree is byte-for-byte untouched by the whole flow.
    assert_eq!(
        snapshot_tree(&fixture_mod()),
        fixture_before,
        "the source mod tree must be byte-identical after an apply"
    );
}

// ---------------------------------------------------------------------------
// rimloc.translation.propose: durable DRAFT, merge, never committed
// ---------------------------------------------------------------------------

#[test]
fn propose_drafts_merge_and_never_become_committed() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let drafts_path = rimloc_services::drafts_file_path(
        &env.managed.join(format!("{project_id}.rimloc.json")),
        &project_id,
    );

    // First proposal: one draft row.
    let out = call(
        &env.rt,
        "rimloc.translation.propose",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "entries": [
                {
                    "identity": { "kind": "keyed", "key": "Fixture_Greet" },
                    "text": "Черновик привет"
                }
            ]
        }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(sc["proposed"], 1);
    assert_eq!(sc["committed"], false);
    assert_eq!(sc["project_revision"], 1);

    // The draft is durable, keyed `<locale>::<display_identity()>`.
    let loaded = rimloc_services::load_drafts(&drafts_path).expect("drafts store loads");
    let rimloc_services::DraftsLoadOutcome::Loaded { drafts, .. } = loaded else {
        panic!("drafts store must exist after a propose");
    };
    assert!(drafts.entries.contains_key("Russian::keyed·Fixture_Greet"));

    // Second proposal for a DIFFERENT identity: the first draft survives —
    // merge semantics, never a clobber.
    call(
        &env.rt,
        "rimloc.translation.propose",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "entries": [
                {
                    "identity": { "kind": "keyed", "key": "Fixture_Farewell" },
                    "text": "Черновик прощай"
                }
            ]
        }),
    );
    let loaded = rimloc_services::load_drafts(&drafts_path).expect("drafts store loads");
    let rimloc_services::DraftsLoadOutcome::Loaded { drafts, .. } = loaded else {
        panic!("drafts store must exist");
    };
    assert!(
        drafts.entries.contains_key("Russian::keyed·Fixture_Greet"),
        "an earlier draft must NEVER be clobbered by a later propose: {drafts:?}"
    );
    assert!(drafts
        .entries
        .contains_key("Russian::keyed·Fixture_Farewell"));

    // A draft is NOT a translation: revision and committed state unmoved.
    let snap = call(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap["structuredContent"]["revision"], 1);
    assert_eq!(snap["structuredContent"]["counts"]["translations"], 0);
    assert_eq!(snap["structuredContent"]["counts"]["translated"], 0);

    // The stale-epoch discipline holds for drafts too.
    let err = call_err(
        &env.rt,
        "rimloc.translation.propose",
        json!({
            "project_id": project_id,
            "session_epoch": 42,
            "locale": "Russian",
            "entries": [
                { "identity": { "kind": "keyed", "key": "Fixture_Greet" }, "text": "x" }
            ]
        }),
    );
    expect_contract_code(&err, "stale_epoch");

    // An empty proposal is a client bug, refused typed.
    let err = call_err(
        &env.rt,
        "rimloc.translation.propose",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "entries": []
        }),
    );
    expect_contract_code(&err, "contract_violation");
}

// ---------------------------------------------------------------------------
// rimloc.export.build: explicit guarded out dir, nothing else is written
// ---------------------------------------------------------------------------

#[test]
fn export_build_writes_only_the_explicit_guarded_dir() {
    let env = env_self();
    let fixture_before = snapshot_tree(&fixture_mod());
    let project_id = create_fixture_project(&env.managed);

    // One committed translation via the MCP apply tool itself.
    call(
        &env.rt,
        "rimloc.translation.apply",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "expected_revision": 1,
            "intents": [{
                "entry": { "kind": "keyed", "key": "Fixture_Greet" },
                "locale": "Russian",
                "action": "set_translation",
                "text": "Привет мир"
            }]
        }),
    );
    let managed_before = snapshot_tree(&env.managed);
    let out_root = env._dir.path().join("export-out");
    let out_root_str = out_root.to_string_lossy().into_owned();

    let out = call(
        &env.rt,
        "rimloc.export.build",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "out_dir": out_root_str,
            "locale": "Russian"
        }),
    );
    let sc = &out["structuredContent"];
    assert!(
        sc["files_written"].as_u64().expect("files_written") >= 1,
        "{sc}"
    );
    // The reparse check is per KEY (one committed keyed row here), while
    // files_written counts the written FILES — distinct metrics, both must
    // be positive.
    assert_eq!(
        sc["reparsed_keys"], 1,
        "exactly the one committed key re-verifies: {sc}"
    );
    let written_root = sc["out_dir"]["path"].as_str().expect("out dir");
    assert!(
        Path::new(written_root).starts_with(env._dir.path()),
        "the writer reports the caller's explicit dir: {sc}"
    );

    // The written tree exists ONLY under the explicit out dir and carries
    // the committed translation.
    let exported = snapshot_tree(&out_root);
    assert!(
        exported.keys().any(|k| k.contains("Languages/Russian")),
        "the Languages/<locale> tree is inside the explicit dir: {:?}",
        exported.keys().collect::<Vec<_>>()
    );
    let joined = exported
        .values()
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .collect::<String>();
    assert!(joined.contains("Привет мир"));

    // NOTHING else changed: the source mod tree is byte-identical to the
    // pre-test snapshot, and the managed root is byte-identical to its
    // post-apply snapshot (the export added no managed artifact).
    assert_eq!(
        snapshot_tree(&fixture_mod()),
        fixture_before,
        "export.build must never touch the source mod tree"
    );
    assert_eq!(
        snapshot_tree(&env.managed),
        managed_before,
        "export.build must never touch the managed root"
    );

    // Form guard: a relative out dir refuses BEFORE any filesystem access.
    let err = call_err(
        &env.rt,
        "rimloc.export.build",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "out_dir": "relative/out",
            "locale": "Russian"
        }),
    );
    expect_contract_code(&err, "invalid_output_path");

    // Source-tree guard: an out dir inside the read-only mod tree denies.
    let err = call_err(
        &env.rt,
        "rimloc.export.build",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "out_dir": fixture_mod().join("Languages").to_string_lossy().into_owned(),
            "locale": "Russian"
        }),
    );
    expect_contract_code(&err, "guard_output_denied");

    // Managed-root guard: managed records are never an export target.
    let err = call_err(
        &env.rt,
        "rimloc.export.build",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "out_dir": env.managed.to_string_lossy().into_owned(),
            "locale": "Russian"
        }),
    );
    expect_contract_code(&err, "guard_output_denied");

    // Stale epoch refuses before any of the guards above can pass.
    let err = call_err(
        &env.rt,
        "rimloc.export.build",
        json!({
            "project_id": project_id,
            "session_epoch": 99,
            "out_dir": out_root_str,
            "locale": "Russian"
        }),
    );
    expect_contract_code(&err, "stale_epoch");
}

// ---------------------------------------------------------------------------
// rimloc.ai_job.start: only an existing provider instance, never a secret
// ---------------------------------------------------------------------------

#[test]
fn ai_job_start_requires_an_existing_provider_instance() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);

    // The form guard: no provider_instance_id → invalid params, BEFORE any
    // service call. The instance is the only launch channel.
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian"
        }),
    );
    assert_eq!(err.code, codes::INVALID_PARAMS, "{err:?}");

    // The stale-epoch guard runs FIRST — stronger than any provider
    // problem (the services' documented ordering).
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 99,
            "locale": "Russian",
            "provider_instance_id": "prov-missing"
        }),
    );
    expect_contract_code(&err, "stale_epoch");

    // An unknown instance refuses typed (project_not_found per the
    // services' resolve vocabulary).
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "provider_instance_id": "prov-missing"
        }),
    );
    expect_contract_code(&err, "project_not_found");

    // Happy path through a LOCAL instance (no keychain in this build): the
    // job starts, and its launch target comes from the instance.
    let instance_id = seed_local_provider_instance(&env.managed);
    let out = call(
        &env.rt,
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "provider_instance_id": instance_id
        }),
    );
    let sc = &out["structuredContent"];
    let job_id = sc["job_id"].as_str().expect("job id").to_string();
    assert!(job_id.starts_with("aijob-"), "{sc}");
    assert_eq!(
        sc["state"], "running",
        "start_job answers the initial state"
    );
    assert_eq!(
        sc["provider_id"], "ollama",
        "the instance defines the launch target"
    );
    assert_eq!(sc["model"], "test-model");
    assert_eq!(
        sc["total"], 8,
        "the untranslated fixture rows scope the job"
    );
    assert_eq!(sc["project_id"], project_id.as_str());
    let checkpoint = sc["checkpoint_path"].as_str().expect("checkpoint path");
    assert!(
        Path::new(checkpoint).starts_with(env.rt.managed_root()),
        "the checkpoint lives in the managed root: {checkpoint}"
    );
    // The durable record exists the moment start returns (compare through
    // the runtime's canonical root: macOS tempdirs sit behind /var →
    // /private/var symlinks).
    assert!(env
        .rt
        .managed_root()
        .join(format!("{job_id}.job.json"))
        .is_file());

    // No secret ever rides the response: no key/secret/token field exists
    // anywhere in the payload.
    fn assert_no_secret_fields(value: &Value, path: &str) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    let lower = k.to_lowercase();
                    assert!(
                        !lower.contains("secret") && !lower.contains("api_key"),
                        "secret-shaped field `{k}` leaked at {path}"
                    );
                    assert_no_secret_fields(v, &format!("{path}.{k}"));
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    assert_no_secret_fields(v, &format!("{path}[{i}]"));
                }
            }
            _ => {}
        }
    }
    assert_no_secret_fields(&out, "$");
}

// ---------------------------------------------------------------------------
// rimloc.ai_job.status / cancel
// ---------------------------------------------------------------------------

#[test]
fn ai_job_status_and_cancel_flow_through_the_process_registry() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);

    // Unknown job → typed refusal (jobs live in the app process).
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.status",
        json!({ "job_id": "aijob-nope" }),
    );
    expect_contract_code(&err, "project_not_found");
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.cancel",
        json!({ "job_id": "aijob-nope" }),
    );
    expect_contract_code(&err, "project_not_found");

    // Malformed job ids refuse before any filesystem access.
    for bad in ["../evil", "a/b", ".hidden", ""] {
        let err = call_err(&env.rt, "rimloc.ai_job.status", json!({ "job_id": bad }));
        expect_contract_code(&err, "contract_violation");
        let err = call_err(&env.rt, "rimloc.ai_job.cancel", json!({ "job_id": bad }));
        expect_contract_code(&err, "contract_violation");
    }

    // A started job is visible through the alias tool and cancellable.
    let instance_id = seed_local_provider_instance(&env.managed);
    let started = call(
        &env.rt,
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 1,
            "locale": "Russian",
            "provider_instance_id": instance_id
        }),
    );
    let job_id = started["structuredContent"]["job_id"]
        .as_str()
        .expect("job id")
        .to_string();

    let status = call(&env.rt, "rimloc.ai_job.status", json!({ "job_id": job_id }));
    assert_eq!(
        status["structuredContent"]["project_id"],
        project_id.as_str()
    );
    assert!(
        status["structuredContent"]["rows"]
            .as_array()
            .expect("rows")
            .len()
            <= 400,
        "the preview ledger is bounded (PREVIEW_ROW_LIMIT)"
    );

    // The cancel itself must succeed; the resulting state is racy by
    // design (the loopback worker may have already failed the job), and
    // the Cancelled transition is services-tested — here the wiring is.
    let cancelled = call(&env.rt, "rimloc.ai_job.cancel", json!({ "job_id": job_id }));
    assert_eq!(
        cancelled["structuredContent"]["job_id"],
        job_id.as_str(),
        "{cancelled}"
    );
}

// ---------------------------------------------------------------------------
// rimloc.ai_job.accept_selected: explicit selection + explicit rejected
// ---------------------------------------------------------------------------

/// Starts a demo `mock` job through the SAME public `start_job` seam the
/// MCP start tool calls (behind the services' documented
/// `RIMLOC_ALLOW_MOCK_JOBS=1` opt-in) and waits for the preview.
fn start_mock_job_to_preview(env: &TestEnv, project_id: &str) -> String {
    std::env::set_var("RIMLOC_ALLOW_MOCK_JOBS", "1");
    let manager = ProjectSessionManager::new(&env.managed).expect("manager");
    manager.open(project_id).expect("open for the job");
    let params: rimloc_services::ai_job::AiJobParams = serde_json::from_value(json!({
        "project_id": project_id,
        "session_epoch": 1,
        "locale": "Russian",
        "provider_id": "mock"
    }))
    .expect("params shape");
    let status =
        rimloc_services::ai_job::start_job(env.rt.ai_jobs(), &manager, &env.managed, params)
            .expect("mock job starts");
    let job_id = status.job_id.clone();
    let finished = wait_until_not_running(&env.rt, &job_id);
    assert_eq!(
        finished["structuredContent"]["state"], "preview_ready",
        "the mock worker completes deterministically offline: {finished}"
    );
    job_id
}

#[test]
fn accept_selected_applies_only_the_selected_rows() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let job_id = start_mock_job_to_preview(&env, &project_id);

    // Row identities come from the preview ledger, never hardcoded.
    let preview = call(&env.rt, "rimloc.ai_job.status", json!({ "job_id": job_id }));
    let rows = preview["structuredContent"]["rows"]
        .as_array()
        .expect("preview rows")
        .clone();
    let identity_of = |key: &str| -> String {
        rows.iter()
            .find(|r| r["key"] == key)
            .map(|r| r["identity"].as_str().expect("identity").to_string())
            .unwrap_or_else(|| panic!("preview row for {key}"))
    };
    let greet = identity_of("Fixture_Greet");
    let farewell = identity_of("Fixture_Farewell");
    assert_eq!(rows.len(), 8);
    assert_eq!(
        preview["structuredContent"]["translated"], 8,
        "the mock translated every pending row"
    );

    // Accept ONE row, explicitly reject ONE row. The rest must stay
    // reviewable — never silently rejected, never applied.
    let out = call(
        &env.rt,
        "rimloc.ai_job.accept_selected",
        json!({
            "job_id": job_id,
            "session_epoch": 1,
            "selected": [greet],
            "rejected": [farewell]
        }),
    );
    let sc = &out["structuredContent"];
    assert_eq!(
        sc["state"], "preview_ready",
        "a partial selection leaves the job reviewable: {sc}"
    );
    let note = sc["error"].as_str().expect("partial-apply note");
    assert!(note.contains("partial apply"), "{note}");

    // The ledger: selected → applied, explicitly rejected → rejected,
    // everything else still `translated`.
    let after = call(&env.rt, "rimloc.ai_job.status", json!({ "job_id": job_id }));
    let rows_after = after["structuredContent"]["rows"]
        .as_array()
        .expect("rows")
        .clone();
    let status_of = |key: &str| -> String {
        rows_after
            .iter()
            .find(|r| r["key"] == key)
            .map(|r| r["status"].as_str().expect("status").to_string())
            .unwrap_or_else(|| panic!("row for {key}"))
    };
    assert_eq!(status_of("Fixture_Greet"), "applied");
    assert_eq!(
        status_of("Fixture_Farewell"),
        "rejected",
        "only the EXPLICITLY rejected row lands in the rejected ledger"
    );
    assert_eq!(
        status_of("Fixture_Settings_Header"),
        "translated",
        "unselected rows stay reviewable"
    );

    // EXACTLY the selected identity became a committed translation.
    let snap = call(
        &env.rt,
        "rimloc.project.snapshot",
        json!({ "project_id": project_id }),
    );
    assert_eq!(snap["structuredContent"]["counts"]["translated"], 1);
    assert_eq!(
        snap["structuredContent"]["revision"], 2,
        "the accept acked an apply"
    );

    let preview_xml = call(
        &env.rt,
        "rimloc.export.preview",
        json!({ "project_id": project_id, "locale": "Russian" }),
    );
    let xml = preview_xml["structuredContent"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| f["xml"].as_str().expect("xml"))
        .collect::<String>();
    assert!(
        xml.contains("Russian: Hello world"),
        "the mock value landed: {xml}"
    );
    assert!(
        !xml.contains("Russian: Goodbye friend"),
        "the rejected row must NOT be applied: {xml}"
    );

    // An EMPTY selection is a typed refusal (pick rows or accept the full
    // scope explicitly — never an accidental full apply).
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.accept_selected",
        json!({ "job_id": job_id, "session_epoch": 1, "selected": [] }),
    );
    expect_contract_code(&err, "validation_failed");

    // And a stale epoch refuses before any ledger mutation.
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.accept_selected",
        json!({
            "job_id": job_id,
            "session_epoch": 77,
            "selected": [identity_of("Fixture_Action_Open")]
        }),
    );
    expect_contract_code(&err, "stale_epoch");

    std::env::remove_var("RIMLOC_ALLOW_MOCK_JOBS");
}

// ---------------------------------------------------------------------------
// The whole write layer in other_writer mode
// ---------------------------------------------------------------------------

#[test]
fn all_write_tools_refuse_in_other_writer_mode() {
    let dir = tempfile::tempdir().expect("tempdir");
    let managed = dir.path().join("managed");
    let owner = Runtime::build(managed.clone()).expect("first writer");
    let project_id = create_fixture_project(&managed);
    let busy = Runtime::build(managed.clone()).expect("busy runtime");
    assert!(busy.is_other_writer());

    let out_dir = dir.path().join("elsewhere");
    let cases: Vec<(&str, Value)> = vec![
        ("rimloc.project.open", json!({ "project_id": project_id })),
        (
            "rimloc.translation.propose",
            json!({
                "project_id": project_id,
                "session_epoch": 1,
                "locale": "Russian",
                "entries": [{ "identity": { "kind": "keyed", "key": "Fixture_Greet" }, "text": "x" }]
            }),
        ),
        (
            "rimloc.translation.apply",
            json!({
                "project_id": project_id,
                "session_epoch": 1,
                "expected_revision": 1,
                "intents": []
            }),
        ),
        (
            "rimloc.ai_job.start",
            json!({
                "project_id": project_id,
                "session_epoch": 1,
                "locale": "Russian",
                "provider_instance_id": "prov-x"
            }),
        ),
        ("rimloc.ai_job.cancel", json!({ "job_id": "aijob-x" })),
        (
            "rimloc.ai_job.accept_selected",
            json!({ "job_id": "aijob-x", "session_epoch": 1, "selected": ["k"] }),
        ),
        (
            "rimloc.export.build",
            json!({
                "project_id": project_id,
                "session_epoch": 1,
                "out_dir": out_dir.to_string_lossy(),
                "locale": "Russian"
            }),
        ),
    ];
    for (tool, args) in cases {
        let err = call_err(&busy, tool, args);
        assert_eq!(err.code, codes::OTHER_WRITER, "{tool}");
        assert_eq!(
            err.data.expect("data")["code"],
            "other_writer_active",
            "{tool}"
        );
    }
    drop(owner);
}

// ---------------------------------------------------------------------------
// Secrets never enter this build's write path at all
// ---------------------------------------------------------------------------

#[test]
fn secret_bearing_provider_state_is_refused_without_the_keychain_feature() {
    // The MCP build has no keychain sink (rimloc-services `keychain` is
    // opt-in and off): a CLOUD instance with a secret refuses honestly —
    // there is no path where the server would store or echo a key.
    let env = env_self();
    let manager = ProjectSessionManager::new(&env.managed).expect("manager");
    let req: rimloc_services::contract::ProviderInstanceUpsertRequest =
        serde_json::from_value(json!({
            "preset": "openai",
            "label": "cloud",
            "model": "gpt-test",
            "secret": "sk-never-stored-anywhere"
        }))
        .expect("upsert shape");
    let err = manager
        .provider_instance_upsert(&req)
        .expect_err("secret-bearing upsert must refuse without a keychain sink");
    assert_eq!(err.code.as_str(), "unsupported_capability", "{err}");

    // A keyless CLOUD instance is not a launch target either: the upsert
    // gate refuses it BEFORE any instance exists, so `ai_job.start` can
    // never resolve a paid provider without a stored key. (The only
    // launchable instances in this build are LOCAL ones — exactly what the
    // happy-path test drives.)
    let local_req: rimloc_services::contract::ProviderInstanceUpsertRequest =
        serde_json::from_value(json!({
            "preset": "openai",
            "label": "keyless-cloud",
            "model": "gpt-test"
        }))
        .expect("upsert shape");
    let err = manager
        .provider_instance_upsert(&local_req)
        .expect_err("a keyless cloud instance must refuse");
    assert_eq!(err.code.as_str(), "invalid_config", "{err}");
    assert!(
        !err.message.contains("sk-"),
        "error text never quotes secret material: {err}"
    );
}
