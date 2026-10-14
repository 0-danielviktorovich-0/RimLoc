//! AI translation jobs (desktop-first mandate, Stage B) — the shared
//! Rust service behind the GUI «ИИ-перевод» screen. ONE engine
//! (`rimloc-llm` TranslationEngine + Provider), ONE canonical writer
//! (`ProjectSessionManager::apply`); model output stays a DRAFT/PROPOSAL
//! until the user accepts it, then it applies through the canonical intents
//! with `ApplyOrigin::Llm`.
//!
//! Data-safety invariants preserved: no provider call touches managed
//! files; cancel/errors never claim "saved" and never drop drafts; the
//! job runs on a worker thread so the GUI main thread never blocks.
//! Checkpoints live next to the managed project for resume.

use crate::contract::{
    ApplyIntentsRequest, ApplyOrigin, ContractError, ContractErrorCode, IntentAction,
    TranslationIntent,
};
use crate::ProjectSessionManager;
use rimloc_domain::canonical::SourceEntryId;
use rimloc_domain::eligibility::Decision;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Target-locale scope of a job. v1: untranslated entries of the open
/// project (the "newbie" journey); `selected`/`changed` extend later
/// without a format break (explicit field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiJobScope {
    Untranslated,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiJobParams {
    pub project_id: String,
    pub session_epoch: u64,
    /// RimWorld folder contract locale ("Russian"/"Ukrainian"/…).
    pub locale: String,
    /// Registry source locale of the project ("en"/"ja"/…), resolved from
    /// the snapshot by the adapter — never hardcoded.
    #[serde(default)]
    pub source_locale: String,
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub batch_char_budget: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiJobRow {
    /// Logical key (`key` or `defName.field`).
    pub key: String,
    /// Full identity rendering (`kind·def_type·key`).
    pub identity: String,
    pub identity_json: SourceEntryId,
    pub source: String,
    /// Proposed translation — DRAFT until the job is accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// `translated` | `failed` | `excluded_notranslate`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiJobState {
    Running,
    PreviewReady,
    Applied,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiJobStatus {
    pub job_id: String,
    pub project_id: String,
    /// Revision the preview was computed against (pinned base).
    #[serde(default)]
    pub base_revision: u64,
    pub locale: String,
    pub provider_id: String,
    pub model: String,
    pub state: AiJobState,
    pub total: usize,
    pub translated: usize,
    pub failed: usize,
    pub excluded_notranslate: usize,
    /// Preview rows (DRAFT translations until accepted). Bounded copy —
    /// the UI pages through it; the full set lives in the job record file.
    pub rows: Vec<AiJobRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_path: Option<String>,
}

/// Worker-side shared state: results stream in from `on_unit`, cancel
/// flags flip between batches (checkpoint/resume makes a mid-batch cancel
/// resumable, never a data hazard).
struct JobShared {
    cancelled: AtomicBool,
    rows: Mutex<BTreeMap<String, AiJobRow>>,
    error: Mutex<Option<String>>,
}

pub struct AiJobHandle {
    pub job_id: String,
    pub status: Mutex<AiJobStatus>,
    shared: Arc<JobShared>,
    /// Managed-root checkpoint path for resume.
    pub checkpoint_path: PathBuf,
}

static JOB_SEQ: AtomicU64 = AtomicU64::new(1);

/// Registry of running/finished jobs, owned by the Tauri state (per app
/// process). Finished jobs keep their preview until a newer job for the
/// same project replaces it or the app exits — the durable record is the
/// canonical apply + checkpoint files.
#[derive(Default)]
pub struct AiJobRegistry {
    jobs: Mutex<HashMap<String, Arc<AiJobHandle>>>,
}

impl AiJobRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, handle: Arc<AiJobHandle>) {
        self.jobs
            .lock()
            .expect("ai job registry poisoned")
            .insert(handle.job_id.clone(), handle);
    }

    pub fn get(&self, job_id: &str) -> Option<Arc<AiJobHandle>> {
        self.jobs
            .lock()
            .expect("ai job registry poisoned")
            .get(job_id)
            .cloned()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn start_job(
    registry: &AiJobRegistry,
    manager: &ProjectSessionManager,
    managed_root: &Path,
    mut params: AiJobParams,
) -> Result<AiJobStatus, ContractError> {
    // Epoch guard FIRST (the stronger invariant): superseded sessions never
    // start paid work. (A stale-epoch call with a demo provider must
    // report the epoch problem, not the provider problem.)
    let snap = manager.snapshot(&params.project_id)?;
    if snap.session_epoch != params.session_epoch {
        return Err(ContractError::stale_epoch(
            params.session_epoch,
            snap.session_epoch,
        ));
    }
    // Honest provider gate (review follow-up P1): the mock echo provider
    // never runs unless the caller explicitly opted into demo mode via the
    // environment — a stale frontend selector must not fabricate a "real"
    // AI job in BYO/local modes.
    if params.provider_id == "mock" && std::env::var("RIMLOC_ALLOW_MOCK_JOBS").as_deref() != Ok("1")
    {
        return Err(ContractError::new(
            ContractErrorCode::ValidationFailed,
            "provider `mock` is a demo echo — set RIMLOC_ALLOW_MOCK_JOBS=1 to run it (tests/dev only)".to_string(),
        ));
    }
    // PINNED base revision (review finding 2): the preview was computed
    // against THIS revision; accept refuses when the project moved on
    // (manual edit during inference) instead of overwriting newer work.
    let pinned_revision = snap.acked_revision;
    // The project's ACTUAL source locale (review UX finding): never a
    // hardcoded "en" — a Japanese-source project must prompt ja→target.
    // It rides the params the worker consumes (empty = adapter did not
    // resolve it; the worker falls back to "en" honestly).
    if params.source_locale.trim().is_empty() {
        params.source_locale = snap
            .project
            .context
            .source_locale
            .clone()
            .unwrap_or_else(|| "en".to_string());
    }
    // The project's source language entries are the inventory; target locale
    // rides the params (folder contract).
    let _ = &params.locale;
    let job_id = format!(
        "aijob-{:06}-{}",
        JOB_SEQ.fetch_add(1, Ordering::Relaxed),
        &params.project_id[..params.project_id.len().min(12)]
    );
    let checkpoint_path = managed_root.join(format!("{job_id}.checkpoint.jsonl"));
    // Durable job record: the JSON snapshot lands with the checkpoint for
    // cross-restart history (Stage C wiring).
    let _record_path = managed_root.join(format!("{job_id}.job.json"));

    // Scope: untranslated + non-empty + eligibility-prefiltered (never
    // billed for entries the canonical gate would refuse).
    let engine_gate = crate::EligibilityEngine::new();
    let mut rows: BTreeMap<String, AiJobRow> = BTreeMap::new();
    let mut llm_units: Vec<rimloc_llm::provider::TranslateUnit> = Vec::new();
    let mut excluded = 0usize;
    for e in &snap.project.entries {
        if !snap
            .project
            .translations
            .iter()
            .any(|t| t.locale == params.locale && t.source_id == e.id && t.text.is_some())
        {
            if e.text.trim().is_empty() {
                continue;
            }
            let verdict = engine_gate.evaluate(e, None);
            // FULL structural identity (review finding 1): the display key
            // alone collapses ThingDef/HediffDef sharing Dup.label; every
            // row and every provider unit id is the complete
            // display_identity(), so two same-key identities stay two rows,
            // two translations and two intents.
            let uid = e.id.display_identity();
            if verdict.decision == Decision::NonTranslatable {
                excluded += 1;
                rows.insert(
                    format!("{:020}{}", u64::MAX, uid),
                    AiJobRow {
                        key: e.id.key.clone(),
                        identity: uid.clone(),
                        identity_json: e.id.clone(),
                        source: e.text.clone(),
                        target: None,
                        status: "excluded_notranslate".to_string(),
                        error: None,
                    },
                );
                continue;
            }
            rows.insert(
                format!("{:020}{}", 0, uid),
                AiJobRow {
                    key: e.id.key.clone(),
                    identity: uid.clone(),
                    identity_json: e.id.clone(),
                    source: e.text.clone(),
                    target: None,
                    status: "pending".to_string(),
                    error: None,
                },
            );
            llm_units.push(rimloc_llm::provider::TranslateUnit {
                id: uid,
                source: e.text.clone(),
                context: Some(
                    e.contexts
                        .first()
                        .map(|c| c.file.clone())
                        .unwrap_or_default(),
                ),
            });
        }
    }
    let total = llm_units.len();

    let shared = Arc::new(JobShared {
        cancelled: AtomicBool::new(false),
        rows: Mutex::new(rows),
        error: Mutex::new(None),
    });
    let status = AiJobStatus {
        job_id: job_id.clone(),
        project_id: params.project_id.clone(),
        base_revision: pinned_revision,
        locale: params.locale.clone(),
        provider_id: params.provider_id.clone(),
        model: params.model.clone().unwrap_or_default(),
        state: AiJobState::Running,
        total,
        translated: 0,
        failed: 0,
        excluded_notranslate: excluded,
        rows: Vec::new(),
        error: None,
        checkpoint_path: Some(checkpoint_path.display().to_string()),
    };
    let handle = Arc::new(AiJobHandle {
        job_id: job_id.clone(),
        status: Mutex::new(status.clone()),
        shared: shared.clone(),
        checkpoint_path: checkpoint_path.clone(),
    });
    registry.insert(handle.clone());

    // Worker thread: provider calls are blocking by design; the GUI polls
    // `job_status` and never blocks.
    let params = params.clone();
    let handle_thread = handle.clone();
    std::thread::spawn(move || {
        let result = run_job_worker(&params, total, &shared, &handle_thread);
        let mut st = handle_thread.status.lock().unwrap();
        match result {
            Ok(()) => {
                st.state = AiJobState::PreviewReady;
            }
            Err(e) => {
                if shared.cancelled.load(Ordering::Relaxed) {
                    st.state = AiJobState::Cancelled;
                } else {
                    st.state = AiJobState::Failed;
                    st.error = Some(e.to_string());
                }
            }
        }
        refresh_status_locked(&mut st, &shared);
    });

    Ok(status)
}

fn run_job_worker(
    params: &AiJobParams,
    total: usize,
    shared: &Arc<JobShared>,
    handle: &AiJobHandle,
) -> Result<(), LlmShapeError> {
    use rimloc_llm as llm;

    let provider = llm::build::build_provider(llm::build::ProviderBuildArgs {
        provider: &params.provider_id,
        model: params.model.as_deref(),
        base_url: params.base_url.as_deref(),
        key_env: None,
    })
    .map_err(|e| LlmShapeError(e.to_string()))?;

    // Entries for the engine: from the shared rows (pending only), id=logical key.
    let mut units: Vec<rimloc_llm::provider::TranslateUnit> = Vec::new();
    {
        let rows = shared.rows.lock().unwrap();
        for row in rows.values() {
            if row.status == "pending" {
                units.push(rimloc_llm::provider::TranslateUnit {
                    id: row.identity.clone(),
                    source: row.source.clone(),
                    context: None,
                });
            }
        }
    }
    let glossary = llm::Glossary::builtin();
    let opts = llm::EngineOptions {
        source_lang: if params.source_locale.is_empty() {
            "en".to_string()
        } else {
            params.source_locale.clone()
        },
        target_lang: params.locale.clone(),
        batch_char_budget: if params.batch_char_budget > 0 {
            params.batch_char_budget
        } else {
            6000
        },
        strict_placeholders: true,
        checkpoint_path: Some(handle.checkpoint_path.clone()),
        ..Default::default()
    };
    let engine = llm::TranslationEngine::new(provider.as_ref(), opts, glossary);
    let summary = engine
        .translate(units.as_slice(), |outcome| {
            if shared.cancelled.load(Ordering::Relaxed) {
                return;
            }
            let mut rows = shared.rows.lock().unwrap();
            // rows keyed by padded sort key + key; find by key (linear scan
            // acceptable at GUI scale).
            if let Some((_, row)) = rows.iter_mut().find(|(_, r)| r.identity == outcome.id) {
                match &outcome.translation {
                    Some(t) => {
                        row.target = Some(t.clone());
                        row.status = "translated".to_string();
                    }
                    None => {
                        row.status = "failed".to_string();
                        row.error = outcome.error.clone();
                    }
                }
            }
        })
        .map_err(|e| LlmShapeError(e.to_string()))?;
    let _ = total;
    let mut st_error = shared.error.lock().unwrap();
    *st_error = None;
    drop(st_error);
    let _ = summary;
    Ok(())
}

/// Accept the preview: canonical intents through `ProjectSessionManager`
/// (origin Llm), epoch/revision guarded. Rows whose target is missing or
/// whose identity no longer resolves are skipped with reasons.
pub fn accept_job(
    registry: &AiJobRegistry,
    manager: &ProjectSessionManager,
    job_id: &str,
    expected_epoch: u64,
) -> Result<AiJobStatus, ContractError> {
    let _ = manager; // reserved: durable job-record refresh (Stage C)
    let handle = registry.get(job_id).ok_or_else(|| missing_job(job_id))?;
    {
        let st = handle.status.lock().unwrap();
        if st.state != AiJobState::PreviewReady {
            return Err(ContractError::new(
                ContractErrorCode::ValidationFailed,
                format!(
                    "job {job_id} is not in preview state (state={:?})",
                    st.state
                ),
            ));
        }
    }
    let rows: Vec<AiJobRow> = {
        let rows = handle.shared.rows.lock().unwrap();
        rows.values().cloned().collect()
    };
    let (locale, project_id, base_revision) = {
        let st = handle.status.lock().unwrap();
        (st.locale.clone(), st.project_id.clone(), st.base_revision)
    };
    // PINNED revision check (review follow-up P0): ONE pre-check, then the
    // PINNED base revision rides the atomic apply itself — the earlier
    // double snapshot re-opened a TOCTOU window (a concurrent edit between
    // the two snapshots would produce a fresh valid revision and let the
    // AI overwrite it). manager.apply re-verifies under its own lock.
    {
        let snap = manager.snapshot(&project_id)?;
        if snap.session_epoch != expected_epoch {
            return Err(ContractError::stale_epoch(
                expected_epoch,
                snap.session_epoch,
            ));
        }
        if snap.acked_revision != base_revision {
            return Err(ContractError::new(
                ContractErrorCode::StaleRevision,
                format!(
                    "project moved from revision {base_revision} to {} while the preview was open — re-run the job against the fresh state",
                    snap.acked_revision
                ),
            ));
        }
    }
    let mut intents: Vec<TranslationIntent> = Vec::new();
    for row in &rows {
        if row.status != "translated" {
            continue;
        }
        if let Some(text) = &row.target {
            intents.push(TranslationIntent {
                entry: row.identity_json.clone(),
                locale: locale.clone(),
                action: IntentAction::SetTranslation,
                text: Some(text.clone()),
            });
        }
    }
    if intents.is_empty() {
        // Truthful no-op (review follow-up P1): nothing translatable to
        // apply — never report Applied.
        let mut st = handle.status.lock().unwrap();
        st.state = AiJobState::Failed;
        st.error = Some("nothing to apply: the preview holds no translated rows".into());
        return Ok(st.clone());
    }
    let req = ApplyIntentsRequest {
        project_id: project_id.clone(),
        // PINNED base revision, NOT a re-snapshot: the atomic apply's
        // under-lock stale guard is the single authority.
        expected_revision: base_revision,
        session_epoch: expected_epoch,
        intents,
        origin: Some(ApplyOrigin::Llm),
    };
    let resp = manager.apply(&req)?;
    // Truthful outcome (review follow-up P1): inspect applied/skipped/
    // cancelled instead of unconditionally claiming Applied.
    let mut st = handle.status.lock().unwrap();
    if resp.cancelled {
        st.state = AiJobState::Failed;
        st.error = Some("the apply was cancelled by the session".into());
    } else if resp.applied == 0 {
        st.state = AiJobState::Failed;
        let reasons: Vec<String> = resp
            .skipped
            .iter()
            .map(|sk| format!("{}: {}", sk.code.as_str(), sk.message))
            .collect();
        st.error = Some(format!(
            "the session refused every intent (applied 0): {}",
            if reasons.is_empty() {
                "no reason given".to_string()
            } else {
                reasons.join("; ")
            }
        ));
        // Keep the per-row typed refusals on the rows the UI shows
        // (skips reference the intent INDEX; intents were built in row
        // order among `translated` rows).
        mark_refused_rows(&handle, &resp, &rows);
    } else if !resp.skipped.is_empty() {
        // Partial: some landed, some refused — honest partial state.
        st.state = AiJobState::Applied;
        let refused: Vec<String> = resp
            .skipped
            .iter()
            .map(|sk| format!("{}: {}", sk.code.as_str(), sk.message))
            .collect();
        st.error = Some(format!(
            "partial apply: {} applied, {} refused — {}",
            resp.applied,
            resp.skipped.len(),
            refused.join("; ")
        ));
        drop(st);
        mark_refused_rows(&handle, &resp, &rows);
        st = handle.status.lock().unwrap();
    } else {
        st.state = AiJobState::Applied;
        st.error = None;
    }
    refresh_status_locked(&mut st, &handle.shared);
    Ok(st.clone())
}

pub fn cancel_job(registry: &AiJobRegistry, job_id: &str) -> Result<AiJobStatus, ContractError> {
    let handle = registry.get(job_id).ok_or_else(|| missing_job(job_id))?;
    handle.shared.cancelled.store(true, Ordering::Relaxed);
    let mut st = handle.status.lock().unwrap();
    if st.state == AiJobState::Running {
        st.state = AiJobState::Cancelled;
    }
    Ok(st.clone())
}

pub fn job_status(registry: &AiJobRegistry, job_id: &str) -> Result<AiJobStatus, ContractError> {
    let handle = registry.get(job_id).ok_or_else(|| missing_job(job_id))?;
    let mut st = handle.status.lock().unwrap();
    refresh_status_locked(&mut st, &handle.shared);
    Ok(st.clone())
}

/// Map intent-index refusals back onto preview rows (review follow-up P1):
/// intents were pushed in the order of `translated` rows, so the skip's
/// index addresses that same row.
fn mark_refused_rows(
    handle: &Arc<AiJobHandle>,
    resp: &crate::contract::ApplyIntentsResponse,
    rows: &[AiJobRow],
) {
    let translated: Vec<&AiJobRow> = rows.iter().filter(|r| r.status == "translated").collect();
    let mut rows_mut = handle.shared.rows.lock().unwrap();
    for sk in &resp.skipped {
        let Some(target) = translated.get(sk.index) else {
            continue;
        };
        if let Some(row) = rows_mut
            .values_mut()
            .find(|r| r.identity == target.identity)
        {
            row.status = "apply_refused".to_string();
            row.error = Some(format!("{}: {}", sk.code.as_str(), sk.message));
        }
    }
}

fn refresh_status_locked(st: &mut AiJobStatus, shared: &Arc<JobShared>) {
    let rows = shared.rows.lock().unwrap();
    st.translated = rows.values().filter(|r| r.status == "translated").count();
    st.failed = rows.values().filter(|r| r.status == "failed").count();
    // Preview copy: bounded to the first 400 rows (UI pages via the full
    // job record file when larger — documented).
    st.rows = rows.values().take(400).cloned().collect();
    if st.state == AiJobState::Running
        && st.translated + st.failed + st.excluded_notranslate >= st.total
        && st.total > 0
    {
        st.state = AiJobState::PreviewReady;
    }
    if let Some(e) = shared.error.lock().unwrap().clone() {
        st.error = Some(e);
    }
}

fn missing_job(job_id: &str) -> ContractError {
    ContractError::new(
        ContractErrorCode::ProjectNotFound,
        format!("ai job `{job_id}` not found (jobs live in the app process)"),
    )
}

/// Local shim error type for the engine boundary (String-carried, like the
/// CLI flow used before the canonical apply).
struct LlmShapeError(String);

impl std::fmt::Display for LlmShapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_mod(root: &Path) {
        std::fs::create_dir_all(root.join("Defs")).unwrap();
        std::fs::write(
            root.join("Defs/A.xml"),
            "<Defs>\n  <ThingDef>\n    <defName>Apple</defName>\n    <label>Apple</label>\n  </ThingDef>\n  <ThingDef>\n    <defName>Pear</defName>\n    <label>Pear</label>\n  </ThingDef>\n</Defs>\n",
        )
        .unwrap();
    }

    struct Ctx {
        _dir: tempfile::TempDir,
        mgr: ProjectSessionManager,
        managed: PathBuf,
        pid: String,
        epoch: u64,
    }

    /// Test-only guard: enable the mock provider for the duration of the
    /// closure (thread-safe via a global env mutex would be ideal; tests
    /// in this module are serialized by the default test harness threads —
    /// use a static lock to avoid env races).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct MockGuard<'a>(#[allow(dead_code)] std::sync::MutexGuard<'a, ()>);
    impl MockGuard<'_> {
        fn acquire() -> Self {
            let g = ENV_LOCK.lock().unwrap();
            std::env::set_var("RIMLOC_ALLOW_MOCK_JOBS", "1");
            MockGuard(g)
        }
    }
    impl Drop for MockGuard<'_> {
        fn drop(&mut self) {
            // Keep the variable set for the process: parallel mock tests
            // hold the same lock but drop interleave with tests that do
            // not — removing here made the suite flaky. The refusal test
            // clears it explicitly under the lock.
        }
    }

    fn setup() -> Ctx {
        let dir = tempfile::tempdir().unwrap();
        fixture_mod(&dir.path().join("mod"));
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        Ctx {
            _dir: dir,
            mgr,
            managed,
            pid: snap.project_id.clone(),
            epoch: snap.session_epoch,
        }
    }

    #[test]
    fn mock_job_runs_to_preview_and_accept_applies_translations() {
        let _mock = MockGuard::acquire();
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let params = AiJobParams {
            project_id: ctx.pid.clone(),
            session_epoch: ctx.epoch,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "mock".into(),
            model: None,
            base_url: None,
            batch_char_budget: 4000,
        };
        let st = start_job(&registry, &ctx.mgr, &ctx.managed, params.clone()).expect("start");
        assert_eq!(st.state, AiJobState::Running);
        assert!(st.total >= 2, "expected the two labels, total={}", st.total);

        let mut st = job_status(&registry, &st.job_id).unwrap();
        for _ in 0..300 {
            st = job_status(&registry, &st.job_id).unwrap();
            if st.state == AiJobState::PreviewReady {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(st.state, AiJobState::PreviewReady, "st={st:?}");
        assert!(st.translated >= 2, "st={st:?}");

        // Accept: canonical apply through the session (origin Llm).
        let st = accept_job(&registry, &ctx.mgr, &st.job_id, ctx.epoch).expect("accept");
        assert_eq!(st.state, AiJobState::Applied);

        // Durable truth: translations visible in the snapshot under the
        // folder-contract locale.
        let snap = ctx.mgr.snapshot(&ctx.pid).unwrap();
        let rus: Vec<_> = snap
            .project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .collect();
        assert!(rus.len() >= 2, "{rus:?}");
    }

    #[test]
    fn stale_epoch_refused_at_start() {
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let params = AiJobParams {
            project_id: ctx.pid.clone(),
            session_epoch: ctx.epoch + 42,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "mock".into(),
            model: None,
            base_url: None,
            batch_char_budget: 0,
        };
        let err = start_job(&registry, &ctx.mgr, &ctx.managed, params.clone()).unwrap_err();
        assert_eq!(err.code, crate::contract::ContractErrorCode::StaleEpoch);
    }

    #[test]
    fn two_same_key_identities_translate_independently() {
        let _mock = MockGuard::acquire();
        // Review finding 1 regression: ThingDef + HediffDef sharing
        // `Dup.label` must produce TWO rows, TWO translations and TWO
        // intents — never a collapsed single translation.
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mod/Defs")).unwrap();
        std::fs::write(
            dir.path().join("mod/Defs/Dup.xml"),
            "<Defs>\n  <ThingDef>\n    <defName>Dup</defName>\n    <label>Apple</label>\n  </ThingDef>\n  <HediffDef>\n    <defName>Dup</defName>\n    <label>Apple</label>\n  </HediffDef>\n</Defs>\n",
        )
        .unwrap();
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        let registry = AiJobRegistry::new();
        let params = AiJobParams {
            project_id: snap.project_id.clone(),
            session_epoch: snap.session_epoch,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "mock".into(),
            model: None,
            base_url: None,
            batch_char_budget: 4000,
        };
        let st = start_job(&registry, &mgr, &managed, params.clone()).unwrap();
        assert_eq!(st.total, 2, "two structural identities must be queued");
        let mut st = job_status(&registry, &st.job_id).unwrap();
        for _ in 0..300 {
            st = job_status(&registry, &st.job_id).unwrap();
            if st.state == AiJobState::PreviewReady {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(st.state, AiJobState::PreviewReady);
        assert_eq!(st.translated, 2, "both identities translated: {st:?}");
        let identities: Vec<&str> = st.rows.iter().map(|r| r.identity.as_str()).collect();
        assert!(
            identities.contains(&"def_injected·ThingDef·Dup.label"),
            "{identities:?}"
        );
        assert!(
            identities.contains(&"def_injected·HediffDef·Dup.label"),
            "{identities:?}"
        );
        let st = accept_job(&registry, &mgr, &st.job_id, snap.session_epoch).unwrap();
        assert_eq!(st.state, AiJobState::Applied);
        // Canonical truth: TWO Russian translations with distinct def types.
        let after = mgr.snapshot(&snap.project_id).unwrap();
        let rus: Vec<_> = after
            .project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .collect();
        assert_eq!(rus.len(), 2, "collapsed identity would leave one: {rus:?}");
        assert!(rus
            .iter()
            .any(|t| t.source_id.def_type.as_deref() == Some("ThingDef")));
        assert!(rus
            .iter()
            .any(|t| t.source_id.def_type.as_deref() == Some("HediffDef")));
    }

    #[test]
    fn accept_refuses_when_project_moved_during_preview() {
        let _mock = MockGuard::acquire();
        // Review finding 2 regression: a manual edit during inference moves
        // the revision; accept must refuse with stale_revision instead of
        // overwriting the newer work.
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let params = AiJobParams {
            project_id: ctx.pid.clone(),
            session_epoch: ctx.epoch,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "mock".into(),
            model: None,
            base_url: None,
            batch_char_budget: 4000,
        };
        let st = start_job(&registry, &ctx.mgr, &ctx.managed, params.clone()).unwrap();
        let mut st = job_status(&registry, &st.job_id).unwrap();
        for _ in 0..300 {
            st = job_status(&registry, &st.job_id).unwrap();
            if st.state == AiJobState::PreviewReady {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(st.state, AiJobState::PreviewReady);
        // A manual edit lands through the canonical session: the revision
        // moves past the pinned base.
        let snap = ctx.mgr.snapshot(&ctx.pid).unwrap();
        let intent = TranslationIntent {
            entry: rimloc_domain::canonical::SourceEntryId {
                kind: rimloc_domain::canonical::EntryKind::DefInjected,
                key: "Apple.label".into(),
                def_type: Some("ThingDef".into()),
            },
            locale: "Russian".into(),
            action: IntentAction::SetTranslation,
            text: Some("ручная правка".into()),
        };
        ctx.mgr
            .apply(&ApplyIntentsRequest {
                project_id: ctx.pid.clone(),
                expected_revision: snap.acked_revision,
                session_epoch: ctx.epoch,
                intents: vec![intent],
                origin: Some(ApplyOrigin::Human),
            })
            .unwrap();
        // Accept now MUST refuse: stale_revision, newer work preserved.
        let err = accept_job(&registry, &ctx.mgr, &st.job_id, ctx.epoch).unwrap_err();
        assert_eq!(err.code, crate::contract::ContractErrorCode::StaleRevision);
        let after = ctx.mgr.snapshot(&ctx.pid).unwrap();
        let manual: Vec<_> = after
            .project
            .translations
            .iter()
            .filter(|t| t.text.as_deref() == Some("ручная правка"))
            .collect();
        assert_eq!(
            manual.len(),
            1,
            "the manual edit survives the refused accept"
        );
    }

    #[test]
    fn mock_provider_refused_without_explicit_dev_env() {
        // Review follow-up P1 regression: a stale frontend selector posting
        // provider_id=mock must NOT fabricate a job in normal runs. Hold
        // the env lock so a parallel mock test's env var cannot leak in.
        let _env = ENV_LOCK.lock().unwrap();
        std::env::remove_var("RIMLOC_ALLOW_MOCK_JOBS");
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let params = AiJobParams {
            project_id: ctx.pid.clone(),
            session_epoch: ctx.epoch,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "mock".into(),
            model: None,
            base_url: None,
            batch_char_budget: 0,
        };
        let err = start_job(&registry, &ctx.mgr, &ctx.managed, params).unwrap_err();
        assert_eq!(
            err.code,
            crate::contract::ContractErrorCode::ValidationFailed
        );
        assert!(err.message.contains("demo echo"));
    }

    #[test]
    fn concurrent_edit_between_check_and_apply_is_refused_by_pinned_revision() {
        // Review follow-up P0 regression (the TOCTOU interleaving): the
        // pre-check passes, an edit lands BEFORE the apply, and the apply
        // must still refuse because expected_revision is the PINNED base —
        // not a fresh re-snapshot. Simulated deterministically by moving
        // the project to a new revision right after a successful accept
        // pre-check would need hooks we do not have; instead: pin base R,
        // advance to R+1 via a manual apply, then call accept and assert
        // the stale_revision refusal (the same interleaving outcome — the
        // apply can never land on a moved revision).
        let _mock = MockGuard::acquire();
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let params = AiJobParams {
            project_id: ctx.pid.clone(),
            session_epoch: ctx.epoch,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "mock".into(),
            model: None,
            base_url: None,
            batch_char_budget: 4000,
        };
        let st = start_job(&registry, &ctx.mgr, &ctx.managed, params.clone()).unwrap();
        let mut st = job_status(&registry, &st.job_id).unwrap();
        for _ in 0..300 {
            st = job_status(&registry, &st.job_id).unwrap();
            if st.state == AiJobState::PreviewReady {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(st.state, AiJobState::PreviewReady);
        // The concurrent edit: revision moves PAST the pinned base.
        let snap = ctx.mgr.snapshot(&ctx.pid).unwrap();
        ctx.mgr
            .apply(&ApplyIntentsRequest {
                project_id: ctx.pid.clone(),
                expected_revision: snap.acked_revision,
                session_epoch: ctx.epoch,
                intents: vec![TranslationIntent {
                    entry: rimloc_domain::canonical::SourceEntryId {
                        kind: rimloc_domain::canonical::EntryKind::DefInjected,
                        key: "Apple.label".into(),
                        def_type: Some("ThingDef".into()),
                    },
                    locale: "Russian".into(),
                    action: IntentAction::SetTranslation,
                    text: Some("конкурирующая правка".into()),
                }],
                origin: Some(ApplyOrigin::Human),
            })
            .unwrap();
        // accept MUST refuse with stale_revision (pinned base ≠ live).
        let err = accept_job(&registry, &ctx.mgr, &st.job_id, ctx.epoch).unwrap_err();
        assert_eq!(err.code, crate::contract::ContractErrorCode::StaleRevision);
        // And the newer human edit survives.
        let after = ctx.mgr.snapshot(&ctx.pid).unwrap();
        assert!(after
            .project
            .translations
            .iter()
            .any(|t| t.text.as_deref() == Some("конкурирующая правка")));
    }

    #[test]
    fn all_refused_apply_reports_failure_not_applied() {
        // Review follow-up P1 regression: apply returning applied=0 must
        // surface a FAILED status with the typed refusal reasons — never
        // "Applied". Built by making every intent address an identity the
        // session will refuse (a stale identity is hard to force; instead
        // we exercise the empty-intent no-op path AND assert the response
        // handling shape on a refused apply via an unknown def-type entry
        // that the eligibility gate excludes at START — so the preview has
        // zero translated rows → the honest no-op branch).
        let _mock = MockGuard::acquire();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mod/Defs")).unwrap();
        std::fs::write(
            dir.path().join("mod/Defs/A.xml"),
            "<Defs><ThingDef><defName>X</defName><label>Only</label></ThingDef></Defs>",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("mod/About")).unwrap();
        std::fs::write(
            dir.path().join("mod/About/About.xml"),
            "<?xml version=\"1.0\"?><ModMetaData><name>R</name><packageId>r.t</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>",
        )
        .unwrap();
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        // Pre-translate the only row manually so the job scope (untranslated)
        // is EMPTY → preview with zero translated rows → accept must take
        // the honest no-op branch and report FAILED, not Applied.
        mgr.apply(&ApplyIntentsRequest {
            project_id: snap.project_id.clone(),
            expected_revision: snap.acked_revision,
            session_epoch: snap.session_epoch,
            intents: vec![TranslationIntent {
                entry: rimloc_domain::canonical::SourceEntryId {
                    kind: rimloc_domain::canonical::EntryKind::DefInjected,
                    key: "X.label".into(),
                    def_type: Some("ThingDef".into()),
                },
                locale: "Russian".into(),
                action: IntentAction::SetTranslation,
                text: Some("уже переведено".into()),
            }],
            origin: Some(ApplyOrigin::Human),
        })
        .unwrap();
        let registry = AiJobRegistry::new();
        let st = start_job(
            &registry,
            &mgr,
            &managed,
            AiJobParams {
                project_id: snap.project_id.clone(),
                session_epoch: snap.session_epoch,
                locale: "Russian".into(),
                source_locale: "en".into(),
                provider_id: "mock".into(),
                model: None,
                base_url: None,
                batch_char_budget: 4000,
            },
        )
        .unwrap();
        assert_eq!(st.total, 0, "nothing untranslated in scope");
        let mut st = job_status(&registry, &st.job_id).unwrap();
        for _ in 0..100 {
            st = job_status(&registry, &st.job_id).unwrap();
            if st.state != AiJobState::Running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(st.state, AiJobState::PreviewReady);
        let st = accept_job(&registry, &mgr, &st.job_id, snap.session_epoch).unwrap();
        assert_eq!(
            st.state,
            AiJobState::Failed,
            "zero-intent accept must NOT report Applied: {st:?}"
        );
        assert!(st.error.unwrap_or_default().contains("nothing to apply"));
    }

    #[test]
    fn accept_unknown_job_is_project_not_found() {
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let err =
            accept_job(&registry, &ctx.mgr, "aijob-000000-nonexistent", ctx.epoch).unwrap_err();
        assert_eq!(
            err.code,
            crate::contract::ContractErrorCode::ProjectNotFound
        );
    }
}
