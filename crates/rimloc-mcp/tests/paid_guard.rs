//! Integration coverage for the MCP paid-provider guard (audit 2026-10,
//! fail-closed consent requirement): `rimloc.ai_job.start` refuses a
//! NON-LOCAL provider instance with the typed
//! `paid_providers_disabled_via_mcp` code BEFORE any secret resolution or
//! worker spawn, while LOCAL (ollama/loopback) instances keep launching.
//!
//! Paid API calls are never performed. The negative test drives an
//! `anthropic` instance that exists ONLY as persisted settings metadata —
//! the exact cross-process shape production has: the GUI app (keychain
//! feature on) stores a paid instance; the external MCP sees it in the
//! shared profile and must refuse. No key material is needed for the
//! refusal and none exists in the test — one seeded record even carries
//! `has_key_cache: true` (a presence marker only) to pin that key
//! presence is irrelevant to the guard.
//!
//! Harness discipline mirrors `tests/writable_tools.rs`: the LOCAL happy
//! path drives an ollama instance whose base_url is the loopback discard
//! port (connection refused instantly — the worker fails honestly, the
//! absence of the paid refusal is what is asserted).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use rimloc_mcp::error::{codes, RpcError};
use rimloc_mcp::runtime::Runtime;
use rimloc_mcp::tools;
use rimloc_services::session::ProjectSessionManager;

// ---------------------------------------------------------------------------
// Harness (mirrors tests/writable_tools.rs)
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

fn call(rt: &Runtime, name: &str, args: Value) -> Value {
    tools::call(name, args, rt).unwrap_or_else(|err| panic!("`{name}` must succeed: {err:?}"))
}

fn call_err(rt: &Runtime, name: &str, args: Value) -> RpcError {
    tools::call(name, args, rt)
        .err()
        .unwrap_or_else(|| panic!("`{name}` must refuse"))
}

/// The persisted provider-settings shape (`rimloc-services` writes the
/// same container: kind `rimloc-provider-settings`, version 1, metadata
/// ONLY — no secret field exists on the records). Seeding the file
/// directly is the honest stand-in for "a keychain-capable GUI process
/// created a paid instance": the sink-less MCP build refuses any
/// secret-bearing upsert by design, so the manager API cannot produce
/// this state.
fn seed_paid_instances(managed: &Path) {
    let dir = managed.join("settings");
    std::fs::create_dir_all(&dir).expect("settings dir");
    std::fs::write(
        dir.join("providers.json"),
        serde_json::to_vec_pretty(&json!({
            "kind": "rimloc-provider-settings",
            "version": 1,
            "revision": 2,
            "instances": [
                {
                    "id": "prov-paid-anthropic",
                    "preset": "anthropic",
                    "label": "Claude (cloud)",
                    "model": "claude-sonnet-4-5",
                    "local": false,
                    "has_key_cache": false,
                    "created_at_ms": 1,
                    "updated_at_ms": 1
                },
                {
                    "id": "prov-paid-keyed",
                    "preset": "openai",
                    "label": "GPT (cloud, key present)",
                    "model": "gpt-test",
                    "local": false,
                    "has_key_cache": true,
                    "created_at_ms": 1,
                    "updated_at_ms": 1
                }
            ]
        }))
        .expect("settings serialize"),
    )
    .expect("settings write");
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

/// Poll the job registry until the job leaves `running` OR the deadline
/// passes. The loopback worker fails fast on unix (connection refused),
/// but windows TCP retries against a closed loopback port can hold the
/// connect open past any honest test bound — a still-`running` job is an
/// acceptable outcome for THIS guard (the paid-refusal substring is
/// checked on whatever state we observe). Returns the last status.
fn wait_until_not_running_or_deadline(rt: &Runtime, job_id: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut status = json!({});
    loop {
        status = call(rt, "rimloc.ai_job.status", json!({ "job_id": job_id }));
        let state = status["structuredContent"]["state"]
            .as_str()
            .expect("state is a string")
            .to_string();
        if state != "running" || Instant::now() >= deadline {
            return status;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

// ---------------------------------------------------------------------------
// The paid guard
// ---------------------------------------------------------------------------

#[test]
fn paid_instance_refuses_with_typed_code_before_any_secret() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    seed_paid_instances(&env.managed);

    for instance_id in ["prov-paid-anthropic", "prov-paid-keyed"] {
        let err = call_err(
            &env.rt,
            "rimloc.ai_job.start",
            json!({
                "project_id": project_id,
                "session_epoch": 1,
                "locale": "Russian",
                "provider_instance_id": instance_id
            }),
        );
        assert_eq!(err.code, codes::CONTRACT_ERROR, "{err:?}");
        let data = err.data.as_ref().expect("typed data");
        assert_eq!(
            data["code"], "paid_providers_disabled_via_mcp",
            "the stable policy code rides in data.code: {err:?}"
        );
        assert_eq!(data["tool"], "rimloc.ai_job.start");
        assert_eq!(
            data["details"]["provider_instance_id"], instance_id,
            "the refusing instance is named"
        );
        assert_eq!(data["details"]["local"], false);
        assert!(
            err.message.contains("per-run consent"),
            "the message names the missing consent mechanism: {err:?}"
        );
        // NOT the services' resolve-time key refusal: the guard fires on
        // metadata alone, before any secret resolution (the resolve-time
        // wording would be "has no stored API key" under
        // validation_failed).
        assert!(
            !err.message.contains("no stored API key"),
            "the refusal must not be the keyless-resolve error: {err:?}"
        );
    }

    // Ordering stays honest: a stale epoch outranks the provider problem
    // (the services' documented order, pinned by writable_tools for the
    // unknown instance — the same must hold against the paid guard).
    let err = call_err(
        &env.rt,
        "rimloc.ai_job.start",
        json!({
            "project_id": project_id,
            "session_epoch": 99,
            "locale": "Russian",
            "provider_instance_id": "prov-paid-anthropic"
        }),
    );
    assert_eq!(err.code, codes::CONTRACT_ERROR, "{err:?}");
    assert_eq!(
        err.data.as_ref().expect("typed data")["code"],
        "stale_epoch",
        "epoch problem is reported, not the provider problem: {err:?}"
    );
}

#[test]
fn local_instance_is_not_paid_refused() {
    let env = env_self();
    let project_id = create_fixture_project(&env.managed);
    let instance_id = seed_local_provider_instance(&env.managed);

    // The start call SUCCEEDS — a paid refusal would have been an Err.
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
    assert_eq!(
        sc["provider_id"], "ollama",
        "the local instance launches: {sc}"
    );
    let job_id = sc["job_id"].as_str().expect("job id").to_string();

    // The job may honestly FAIL (the loopback discard port refuses the
    // connection) — that is fine. What must never happen is the paid
    // refusal: no part of the terminal status carries the guard's code.
    let status = wait_until_not_running_or_deadline(&env.rt, &job_id);
    let rendered = serde_json::to_string(&status).expect("status renders");
    assert!(
        !rendered.contains("paid_providers_disabled_via_mcp"),
        "the local job must never carry the paid-refusal code: {rendered}"
    );
}
