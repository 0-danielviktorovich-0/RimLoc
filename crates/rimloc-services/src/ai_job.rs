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
    /// ProvidersScreen instance (`prov-*`, review 2026-10-14 factory
    /// wiring): when set, the instance's preset/model/base_url AND stored
    /// key define the launch target — the direct `provider_id`/`model`/
    /// `base_url` fields are ignored. Resolution happens in `start_job`
    /// BEFORE any worker spawn, so an unknown or keyless instance is a
    /// typed immediate refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_instance_id: Option<String>,
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
    /// A durable record survived an app restart mid-run. The worker is
    /// gone by definition; drafts were never touched, so the honest UX is
    /// "re-run the job", never silent auto-resume (billing safety).
    Interrupted,
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
/// resumable, never a data hazard). `cancelled` is an Arc because the SAME
/// flag instance rides `EngineOptions::cancel` into the translation
/// engine (review 2026-10-14 P1) — one flip stops the provider loop.
/// `finished` flips after the worker published its FINAL status, so tests
/// (and only tests) can await true worker exit before asserting the
/// terminal state survived it.
struct JobShared {
    cancelled: Arc<AtomicBool>,
    finished: AtomicBool,
    rows: Mutex<BTreeMap<String, AiJobRow>>,
    error: Mutex<Option<String>>,
}

/// Test seam (review 2026-10-14 P1): provider construction is injectable
/// so the job service can be driven by blocking/scripted providers in
/// deterministic regressions. Production uses [`default_provider_factory`].
/// The second argument is the resolved instance key (prov-* wiring): the
/// factory passes it to `build_provider` as `key_inline` — the secret
/// crosses this boundary in-process only, never on the wire or in logs.
pub(crate) type AiProviderFactory = Arc<
    dyn Fn(
            &AiJobParams,
            Option<&crate::providers::ProviderSecret>,
        ) -> Result<Arc<dyn rimloc_llm::provider::Provider + Send + Sync>, String>
        + Send
        + Sync,
>;

fn default_provider_factory() -> AiProviderFactory {
    Arc::new(
        |params: &AiJobParams, key: Option<&crate::providers::ProviderSecret>| {
            rimloc_llm::build::build_provider(rimloc_llm::build::ProviderBuildArgs {
                provider: &params.provider_id,
                model: params.model.as_deref(),
                base_url: params.base_url.as_deref(),
                key_env: None,
                key_inline: key.map(crate::providers::ProviderSecret::expose),
            })
            .map(|p| {
                let boxed: Box<dyn rimloc_llm::provider::Provider + Send + Sync> = p;
                Arc::from(boxed)
            })
            .map_err(|e| e.to_string())
        },
    )
}

pub struct AiJobHandle {
    pub job_id: String,
    pub status: Mutex<AiJobStatus>,
    shared: Arc<JobShared>,
    /// Managed-root checkpoint path for resume.
    pub checkpoint_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Durable job records (restart honesty): every state transition lands in
// `<managed_root>/<job_id>.job.json` (atomic tmp+rename). After a restart
// `recover_interrupted_jobs` re-labels stale Running records as
// Interrupted; `job_status_from_record` lets the UI ask about a job whose
// in-memory registry is gone and get the durable truth instead of a fake
// "unknown project".
// ---------------------------------------------------------------------------

fn job_record_path(managed_root: &Path, job_id: &str) -> PathBuf {
    managed_root.join(format!("{job_id}.job.json"))
}

/// Persist the status snapshot atomically. Best-effort by design: a record
/// write failure must never fail the translation itself — it surfaces in
/// stderr only.
fn write_job_record(managed_root: &Path, status: &AiJobStatus) {
    let path = job_record_path(managed_root, &status.job_id);
    let tmp = path.with_extension("job.json.tmp");
    let payload = match serde_json::to_vec_pretty(status) {
        Ok(v) => v,
        Err(_) => return,
    };
    if std::fs::write(&tmp, payload)
        .and_then(|_| std::fs::rename(&tmp, &path))
        .is_err()
    {
        eprintln!("rimloc: ai job record write failed for {}", status.job_id);
    }
}

/// Read one durable record (None when absent or unreadable).
pub fn job_status_from_record(managed_root: &Path, job_id: &str) -> Option<AiJobStatus> {
    let bytes = std::fs::read(job_record_path(managed_root, job_id)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Re-label stale `running` records as `interrupted`. Call once when the
/// app (re)opens a managed root — before any new job starts. Returns how
/// many records were re-labelled (0 on a clean start).
pub fn recover_interrupted_jobs(managed_root: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(managed_root) else {
        return 0;
    };
    let mut recovered = 0usize;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".job.json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(entry.path()) else {
            continue;
        };
        let Ok(mut status) = serde_json::from_slice::<AiJobStatus>(&bytes) else {
            continue;
        };
        if status.state == AiJobState::Running {
            status.state = AiJobState::Interrupted;
            write_job_record(managed_root, &status);
            recovered += 1;
        }
    }
    recovered
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
    params: AiJobParams,
) -> Result<AiJobStatus, ContractError> {
    start_job_with_factory(
        registry,
        manager,
        managed_root,
        params,
        default_provider_factory(),
    )
}

/// Provider-injectable core of [`start_job`] — the ONLY difference is where
/// the provider comes from; engine, worker, status transitions and data
/// safety are byte-identical between production and test drivers.
pub(crate) fn start_job_with_factory(
    registry: &AiJobRegistry,
    manager: &ProjectSessionManager,
    managed_root: &Path,
    mut params: AiJobParams,
    factory: AiProviderFactory,
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
    // Instance resolution (review 2026-10-14 factory wiring) BEFORE any
    // worker spawn: an unknown or keyless instance is a typed immediate
    // refusal. When set, the instance FULLY defines the launch target —
    // preset mapped to the llm provider id, its model/base_url and its
    // stored key — and the direct provider fields are ignored. The secret
    // rides in-process only (Debug-masked), never on the wire.
    let resolved_key: Option<crate::providers::ProviderSecret> = match &params.provider_instance_id
    {
        Some(instance_id) => {
            let target = manager.provider_instance_resolve(instance_id)?;
            params.provider_id = target.provider_id;
            params.model = Some(target.model);
            params.base_url = target.base_url;
            target.key
        }
        None => None,
    };
    // Honest provider gate (review follow-up P1): the mock echo provider
    // never runs unless the caller explicitly opted into demo mode via the
    // environment — a stale frontend selector must not fabricate a "real"
    // AI job in BYO/local modes. (Instances cannot resolve to `mock` —
    // the preset vocabulary has no such value.)
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
    // Durable job record: written at creation and at every terminal
    // transition (see write_job_record / recover_interrupted_jobs).

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
        cancelled: Arc::new(AtomicBool::new(false)),
        finished: AtomicBool::new(false),
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
    // The record outlives the process: restart honesty starts here.
    let record_root = managed_root.to_path_buf();
    write_job_record(&record_root, &status);

    // Worker thread: provider calls are blocking by design; the GUI polls
    // `job_status` and never blocks.
    let params = params.clone();
    let handle_thread = handle.clone();
    let checkpoint_thread = checkpoint_path.clone();
    let key_thread = resolved_key;
    let record_root_thread = record_root.clone();
    std::thread::spawn(move || {
        let result = match factory(&params, key_thread.as_ref()) {
            Ok(provider) => run_job_worker(provider.as_ref(), &params, &shared, &checkpoint_thread),
            Err(e) => Err(LlmShapeError(e)),
        };
        // TERMINAL INVARIANT (review 2026-10-14 P1): Cancelled is terminal
        // for this job execution. A late worker completion — engine loop
        // finishing right after the user pressed Cancel — must NEVER
        // resurrect the job into PreviewReady/Applied/Running. Every
        // transition below is guarded by the live state under the mutex.
        let mut st = handle_thread.status.lock().unwrap();
        match result {
            Ok(()) => {
                if !shared.cancelled.load(Ordering::Relaxed) && st.state == AiJobState::Running {
                    st.state = AiJobState::PreviewReady;
                }
            }
            Err(e) => {
                if shared.cancelled.load(Ordering::Relaxed) {
                    // The engine returns LlmError::Cancelled at its next
                    // boundary; the state cancel_job already wrote stands.
                    // No error text: a cancel is a user decision, not a
                    // failure.
                    if st.state == AiJobState::Running {
                        st.state = AiJobState::Cancelled;
                    }
                } else if st.state == AiJobState::Running {
                    st.state = AiJobState::Failed;
                    st.error = Some(e.to_string());
                }
            }
        }
        refresh_status_locked(&mut st, &shared);
        if st.state != AiJobState::Interrupted {
            write_job_record(&record_root_thread, &st);
        }
        drop(st);
        shared.finished.store(true, Ordering::Release);
    });

    Ok(status)
}

fn run_job_worker(
    provider: &dyn rimloc_llm::provider::Provider,
    params: &AiJobParams,
    shared: &Arc<JobShared>,
    checkpoint_path: &Path,
) -> Result<(), LlmShapeError> {
    use rimloc_llm as llm;

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
        checkpoint_path: Some(checkpoint_path.to_path_buf()),
        // THE cancellation wire (review 2026-10-14 P1): the SAME flag
        // cancel_job flips now rides into the engine loop, so a cancel
        // stops provider dispatch at the next batch/request/retry boundary
        // instead of merely muting UI updates while paid calls continue.
        cancel: Some(shared.cancelled.clone()),
        ..Default::default()
    };
    let engine = llm::TranslationEngine::new(provider, opts, glossary);
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
    // Deterministic inter-thread interleaving point (review 2026-10-14
    // P1e): tests install a closure here — AFTER the accept pre-check,
    // BEFORE the atomic apply — so a competing edit can land on another
    // thread and the under-lock stale guard can be proven against a REAL
    // race. Production never installs the hook.
    #[cfg(test)]
    if let Some(hook) = ACCEPT_PRE_APPLY_HOOK.lock().unwrap().as_ref() {
        hook();
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
    if let Some(root) = handle.checkpoint_path.parent() {
        write_job_record(root, &st);
    }
    Ok(st.clone())
}

pub fn cancel_job(registry: &AiJobRegistry, job_id: &str) -> Result<AiJobStatus, ContractError> {
    let handle = registry.get(job_id).ok_or_else(|| missing_job(job_id))?;
    // Flag FIRST (the engine observes it at its next boundary), state
    // second, under the status mutex (review round 2): the worker
    // completion handler evaluates the flag while holding the same mutex,
    // so the pair {flag store → state check} is atomic against any
    // terminal transition — a cancel can never be half-applied.
    handle.shared.cancelled.store(true, Ordering::Relaxed);
    let mut st = handle.status.lock().unwrap();
    if st.state == AiJobState::Running {
        st.state = AiJobState::Cancelled;
        if let Some(root) = handle.checkpoint_path.parent() {
            write_job_record(root, &st);
        }
    }
    Ok(st.clone())
}

/// Test-only interleaving point between the accept pre-check and the
/// atomic apply (see accept_job). Never set outside tests.
#[cfg(test)]
static ACCEPT_PRE_APPLY_HOOK: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>> =
    std::sync::Mutex::new(None);

pub fn job_status(registry: &AiJobRegistry, job_id: &str) -> Result<AiJobStatus, ContractError> {
    let handle = registry.get(job_id).ok_or_else(|| missing_job(job_id))?;
    let mut st = handle.status.lock().unwrap();
    refresh_status_locked(&mut st, &handle.shared);
    Ok(st.clone())
}

/// Restart-honest status: the in-memory registry first, then the durable
/// record. After an app restart the registry is empty by construction —
/// the record answers with its last persisted truth (terminal state or
/// `interrupted` once `recover_interrupted_jobs` ran) instead of a fake
/// "job not found".
pub fn job_status_with_record(
    registry: &AiJobRegistry,
    managed_root: &Path,
    job_id: &str,
) -> Result<AiJobStatus, ContractError> {
    match job_status(registry, job_id) {
        Ok(st) => Ok(st),
        Err(registry_err) => {
            if let Some(st) = job_status_from_record(managed_root, job_id) {
                Ok(st)
            } else {
                Err(registry_err)
            }
        }
    }
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
    // NO automatic terminal transition here (review round 2, P1-A): the
    // old `translated + failed + excluded >= total` condition mixed two
    // populations — `total` counts only translatable units while
    // `excluded_notranslate` counts excluded entries — so a poll could
    // flip a RUNNING job to PreviewReady while units were still in flight
    // (0+0+1 >= 1), which in turn made cancel_job a state no-op and let
    // accept apply an incomplete preview. Terminal states are written by
    // exactly ONE authority: the worker completion handler in
    // `start_job_with_factory`.
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

    // --- durable records + restart honesty -------------------------------

    #[test]
    fn durable_record_survives_restart_and_interrupted_recovery() {
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
            provider_instance_id: None,
            batch_char_budget: 4000,
        };
        let st = start_job(&registry, &ctx.mgr, &ctx.managed, params).expect("start");
        // The RUNNING record exists on disk the moment the job starts.
        let on_disk = job_status_from_record(&ctx.managed, &st.job_id).expect("record at start");
        assert_eq!(on_disk.state, AiJobState::Running);

        let mut final_st = on_disk;
        for _ in 0..300 {
            final_st = job_status(&registry, &st.job_id).unwrap();
            if final_st.state == AiJobState::PreviewReady {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(final_st.state, AiJobState::PreviewReady);
        // The terminal transition landed in the record too.
        let persisted = job_status_from_record(&ctx.managed, &st.job_id).expect("record after");
        assert_eq!(persisted.state, AiJobState::PreviewReady);

        // RESTART SIMULATION: a fresh registry (new process) — the status
        // answers from the durable record, never "job not found".
        let fresh_registry = AiJobRegistry::new();
        let via_record = job_status_with_record(&fresh_registry, &ctx.managed, &st.job_id)
            .expect("status via record");
        assert_eq!(via_record.state, AiJobState::PreviewReady);
        // And with no record at all the typed refusal still holds.
        assert!(job_status_with_record(&fresh_registry, &ctx.managed, "aijob-nope").is_err());

        // A stale RUNNING record (crash mid-run) is re-labelled Interrupted
        // the next time a manager opens the root.
        let mut stale = persisted.clone();
        stale.state = AiJobState::Running;
        write_job_record(&ctx.managed, &stale);
        let _relabeled = ProjectSessionManager::new(&ctx.managed).unwrap();
        let recovered = job_status_from_record(&ctx.managed, &st.job_id).unwrap();
        assert_eq!(recovered.state, AiJobState::Interrupted);
        assert_eq!(recovered.job_id, stale.job_id);
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
            provider_instance_id: None,
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
            provider_instance_id: None,
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
            provider_instance_id: None,
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
            provider_instance_id: None,
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
            provider_instance_id: None,
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
    fn sequential_stale_edit_is_refused_by_pinned_revision() {
        // Review follow-up P0 regression (SEQUENTIAL variant): base R is
        // pinned at start, the project advances to R+1, accept must refuse
        // with stale_revision. This does NOT interleave threads — the
        // concurrent variant with a real inter-thread barrier is
        // `concurrent_edit_between_precheck_and_apply_barriers_and_is_refused`.
        // (Honest relabeling per review 2026-10-14: the earlier name
        // claimed concurrency this deterministic test did not have.)
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
            provider_instance_id: None,
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
                provider_instance_id: None,
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

    // --- deterministic negative regressions (review 2026-10-14 P1) ---
    // These run through start_job_with_factory: blocking/scripted providers
    // replace build_provider, so no paid API and no mock env gate.

    use std::collections::VecDeque;
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;
    use std::time::Duration;

    enum ScriptedAction {
        /// Echo "ru: {source}" for every unit.
        Reply,
        /// Exact per-id translations; unknown ids fall back to the echo.
        ReplyMap(BTreeMap<String, String>),
        RateLimited(Duration),
        /// Block until the paired sender drops (or 10s guard), then echo.
        Block(mpsc::Receiver<()>),
    }

    struct ScriptedProvider {
        calls: AtomicUsize,
        started_tx: mpsc::Sender<()>,
        script: Mutex<VecDeque<ScriptedAction>>,
    }

    impl ScriptedProvider {
        fn echo(
            req: &rimloc_llm::provider::TranslateRequest,
        ) -> rimloc_llm::provider::TranslateResponse {
            rimloc_llm::provider::TranslateResponse {
                results: req
                    .units
                    .iter()
                    .map(|u| rimloc_llm::provider::TranslateUnitResult {
                        id: u.id.clone(),
                        translation: format!("ru: {}", u.source),
                    })
                    .collect(),
                usage: None,
            }
        }
    }

    impl rimloc_llm::provider::Provider for ScriptedProvider {
        fn id(&self) -> &str {
            "scripted"
        }
        fn model(&self) -> &str {
            "scripted-1"
        }
        fn info(&self) -> rimloc_llm::provider::ProviderInfo {
            rimloc_llm::provider::ProviderInfo {
                id: self.id().into(),
                model: self.model().into(),
                endpoint: "x".into(),
            }
        }
        fn translate_batch(
            &self,
            req: &rimloc_llm::provider::TranslateRequest,
        ) -> Result<rimloc_llm::provider::TranslateResponse, rimloc_llm::LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let _ = self.started_tx.send(());
            let action = self
                .script
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(ScriptedAction::Reply);
            match action {
                ScriptedAction::Reply => Ok(Self::echo(req)),
                ScriptedAction::ReplyMap(map) => Ok(rimloc_llm::provider::TranslateResponse {
                    results: req
                        .units
                        .iter()
                        .map(|u| rimloc_llm::provider::TranslateUnitResult {
                            id: u.id.clone(),
                            translation: map
                                .get(&u.id)
                                .cloned()
                                .unwrap_or_else(|| format!("ru: {}", u.source)),
                        })
                        .collect(),
                    usage: None,
                }),
                ScriptedAction::RateLimited(after) => {
                    Err(rimloc_llm::LlmError::RateLimited(self.id().into(), after))
                }
                ScriptedAction::Block(rx) => {
                    let _ = rx.recv_timeout(Duration::from_secs(10));
                    Ok(Self::echo(req))
                }
            }
        }
        fn test_connection(
            &self,
        ) -> Result<rimloc_llm::provider::ProviderInfo, rimloc_llm::LlmError> {
            Ok(self.info())
        }
    }

    fn scripted_factory(p: Arc<ScriptedProvider>) -> AiProviderFactory {
        Arc::new(move |_, _| Ok(p.clone() as Arc<dyn rimloc_llm::provider::Provider + Send + Sync>))
    }

    /// Worker exit is observable: `finished` flips after the FINAL status
    /// is published, so assertions below test the state AS THE WORKER LEFT
    /// it, not a transient mid-exit value.
    fn wait_worker_finished(handle: &AiJobHandle, secs: u64) {
        let deadline = std::time::Instant::now() + Duration::from_secs(secs);
        while !handle.shared.finished.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not exit within {secs}s"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_translated_count(registry: &AiJobRegistry, job_id: &str, n: usize, secs: u64) {
        let deadline = std::time::Instant::now() + Duration::from_secs(secs);
        loop {
            let st = job_status(registry, job_id).unwrap();
            if st.translated >= n {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "translated count never reached {n}: {st:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Six long entries → six single-unit batches at budget 700
    /// (unit weight ≈ 660+16; two units would exceed the budget).
    fn setup_long(
        n: usize,
    ) -> (
        tempfile::TempDir,
        ProjectSessionManager,
        PathBuf,
        String,
        u64,
    ) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mod/Defs")).unwrap();
        let mut xml = String::from("<Defs>\n");
        for i in 0..n {
            let label = "A very long description sentence about steel walls. ".repeat(10);
            xml.push_str(&format!(
                "  <ThingDef>\n    <defName>Long{i}</defName>\n    <label>{label}</label>\n  </ThingDef>\n"
            ));
        }
        xml.push_str("</Defs>\n");
        std::fs::write(dir.path().join("mod/Defs/Long.xml"), xml).unwrap();
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        (
            dir,
            mgr,
            managed,
            snap.project_id.clone(),
            snap.session_epoch,
        )
    }

    fn long_job_params(pid: &str, epoch: u64) -> AiJobParams {
        AiJobParams {
            project_id: pid.to_string(),
            session_epoch: epoch,
            locale: "Russian".into(),
            source_locale: "en".into(),
            provider_id: "scripted".into(),
            model: None,
            base_url: None,
            provider_instance_id: None,
            batch_char_budget: 700,
        }
    }

    /// P1(a), the reviewer's literal scenario: multi-batch job, the FIRST
    /// provider request blocked, cancel flips while it is in flight, then
    /// release. The worker must exit having dispatched EXACTLY ONE
    /// request; the job stays Cancelled after worker exit; no PreviewReady;
    /// canonical translations untouched; checkpoint carries no half records.
    #[test]
    fn cancel_between_batches_blocks_further_provider_calls() {
        let (_dir, mgr, managed, pid, epoch) = setup_long(6);
        let registry = AiJobRegistry::new();
        let (started_tx, started_rx) = mpsc::channel();
        let (rel_tx, rel_rx) = mpsc::channel();
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx,
            script: Mutex::new(
                vec![
                    ScriptedAction::Block(rel_rx),
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                ]
                .into(),
            ),
        });
        let st = start_job_with_factory(
            &registry,
            &mgr,
            &managed,
            long_job_params(&pid, epoch),
            scripted_factory(provider.clone()),
        )
        .unwrap();
        let handle = registry.get(&st.job_id).unwrap();
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("first provider call must start");
        let st = cancel_job(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::Cancelled);
        drop(rel_tx); // release the blocked request
        wait_worker_finished(&handle, 5);
        assert_eq!(
            provider.calls.load(Ordering::SeqCst),
            1,
            "no second batch, no retry after cancel"
        );
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(
            st.state,
            AiJobState::Cancelled,
            "terminal Cancelled must survive worker exit, got {st:?}"
        );
        assert!(!st.error.as_deref().unwrap_or("").contains("panic"));
        // Canonical truth untouched: nothing accepted, nothing applied.
        let snap = mgr.snapshot(&pid).unwrap();
        assert!(
            !snap
                .project
                .translations
                .iter()
                .any(|t| t.locale == "Russian"),
            "cancelled job must not write translations: {:?}",
            snap.project.translations
        );
        // Checkpoint valid: the in-flight batch was discarded, so zero records.
        if let Some(cp) = &st.checkpoint_path {
            if std::path::Path::new(cp).exists() {
                let lines = std::fs::read_to_string(cp).unwrap();
                assert_eq!(
                    lines.lines().count(),
                    0,
                    "no completed batch before the flip → no records"
                );
            }
        }
    }

    /// P1(a) preservation variant: batch 1 completes BEFORE the cancel, so
    /// its checkpoint record must survive; the in-flight batch 2 is
    /// discarded; batch 3 never dispatches; state stays Cancelled.
    #[test]
    fn cancel_preserves_completed_batch_and_discards_inflight() {
        let (_dir, mgr, managed, pid, epoch) = setup_long(6);
        let registry = AiJobRegistry::new();
        let (started_tx, _started_rx) = mpsc::channel();
        let (rel_tx, rel_rx) = mpsc::channel();
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx,
            script: Mutex::new(
                vec![
                    ScriptedAction::Reply,
                    ScriptedAction::Block(rel_rx),
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                    ScriptedAction::Reply,
                ]
                .into(),
            ),
        });
        let st = start_job_with_factory(
            &registry,
            &mgr,
            &managed,
            long_job_params(&pid, epoch),
            scripted_factory(provider.clone()),
        )
        .unwrap();
        let handle = registry.get(&st.job_id).unwrap();
        // Batch 1 fully processed before we flip anything.
        wait_translated_count(&registry, &st.job_id, 1, 5);
        cancel_job(&registry, &st.job_id).unwrap();
        drop(rel_tx);
        wait_worker_finished(&handle, 5);
        assert_eq!(
            provider.calls.load(Ordering::SeqCst),
            2,
            "batch 2 was in flight pre-flip; batch 3 must never dispatch"
        );
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::Cancelled, "{st:?}");
        assert_eq!(st.translated, 1, "only batch 1's row stays translated");
        // Completed checkpoint record preserved and parseable.
        let cp = st.checkpoint_path.clone().expect("checkpoint path");
        let lines = std::fs::read_to_string(&cp).unwrap();
        let records: Vec<&str> = lines.lines().collect();
        assert_eq!(records.len(), 1, "exactly batch 1's record: {records:?}");
        assert!(records[0].contains("Long0"), "{records:?}");
        // Resume sanity: the record parses as a checkpoint entry.
        let entry: rimloc_llm::CheckpointEntry =
            serde_json::from_str(records[0].trim()).expect("valid checkpoint JSON");
        assert_eq!(entry.id, "def_injected·ThingDef·Long0.label");
    }

    /// P1(b): cancel during the rate-limit wait — no retry attempt after
    /// the flip, bounded cancellation latency, and the state the worker
    /// leaves behind is Cancelled (never Failed-with-error, never
    /// PreviewReady).
    #[test]
    fn cancel_during_rate_limit_wait_stops_retries_and_stays_cancelled() {
        let (_dir, mgr, managed, pid, epoch) = setup_long(2);
        let registry = AiJobRegistry::new();
        let (started_tx, _started_rx) = mpsc::channel();
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx,
            script: Mutex::new(vec![ScriptedAction::RateLimited(Duration::from_secs(30))].into()),
        });
        let st = start_job_with_factory(
            &registry,
            &mgr,
            &managed,
            long_job_params(&pid, epoch),
            scripted_factory(provider.clone()),
        )
        .unwrap();
        let handle = registry.get(&st.job_id).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while provider.calls.load(Ordering::SeqCst) == 0 {
            assert!(std::time::Instant::now() < deadline, "call never started");
            std::thread::sleep(Duration::from_millis(2));
        }
        let started = std::time::Instant::now();
        cancel_job(&registry, &st.job_id).unwrap();
        wait_worker_finished(&handle, 5);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "cancel must cut the 5s-capped wait promptly, took {:?}",
            started.elapsed()
        );
        assert_eq!(
            provider.calls.load(Ordering::SeqCst),
            1,
            "no retry attempt after the flip"
        );
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::Cancelled, "{st:?}");
        assert!(
            st.error.is_none(),
            "a cancel is a user decision, not a failure: {st:?}"
        );
    }

    /// P1(c): TRUE all-refused apply through a REAL session — every intent
    /// the session itself refuses (control character invalid in XML 1.0),
    /// so manager.apply returns applied=0 with a NON-EMPTY skip list. This
    /// is the branch the previous "all-refused" test did not reach (it
    /// exercised the empty-intent no-op instead).
    #[test]
    fn all_refused_real_apply_reports_failure_with_reasons() {
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let bad = "ru:\u{0007}apple"; // U+0007 BEL is invalid in XML 1.0
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx: {
                let (tx, _rx) = mpsc::channel();
                tx
            },
            script: Mutex::new(
                vec![ScriptedAction::ReplyMap(BTreeMap::from([
                    (
                        "def_injected·ThingDef·Apple.label".to_string(),
                        bad.to_string(),
                    ),
                    (
                        "def_injected·ThingDef·Pear.label".to_string(),
                        format!("{bad}2"),
                    ),
                ]))]
                .into(),
            ),
        });
        let st = start_job_with_factory(
            &registry,
            &ctx.mgr,
            &ctx.managed,
            AiJobParams {
                batch_char_budget: 4000,
                ..long_job_params(&ctx.pid, ctx.epoch)
            },
            scripted_factory(provider.clone()),
        )
        .unwrap();
        wait_worker_finished(&registry.get(&st.job_id).unwrap(), 5);
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::PreviewReady, "{st:?}");
        assert_eq!(st.translated, 2);
        let st = accept_job(&registry, &ctx.mgr, &st.job_id, ctx.epoch).unwrap();
        assert_eq!(
            st.state,
            AiJobState::Failed,
            "applied=0 with refusals must be Failed, got {st:?}"
        );
        let err = st.error.clone().unwrap_or_default();
        assert!(err.contains("applied 0"), "{err}");
        assert!(
            err.contains("invalid in XML 1.0"),
            "the exact refusal reason must surface: {err}"
        );
        // Per-row typed refusals visible to the UI.
        assert!(
            st.rows
                .iter()
                .all(|r| r.status == "apply_refused" && r.error.is_some()),
            "{st:?}"
        );
        // Canonical truth untouched.
        let snap = ctx.mgr.snapshot(&ctx.pid).unwrap();
        assert!(
            !snap
                .project
                .translations
                .iter()
                .any(|t| t.locale == "Russian"),
            "refused apply must not write translations"
        );
    }

    /// P1(d): partial apply — one intent lands, one is refused; the
    /// outcome is Applied with an HONEST "N applied, M refused" summary
    /// and the refused row carrying its reason.
    #[test]
    fn partial_apply_reports_honest_counts_and_refusal_reasons() {
        let ctx = setup();
        let registry = AiJobRegistry::new();
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx: {
                let (tx, _rx) = mpsc::channel();
                tx
            },
            script: Mutex::new(
                vec![ScriptedAction::ReplyMap(BTreeMap::from([
                    (
                        "def_injected·ThingDef·Apple.label".to_string(),
                        "Яблоко".to_string(),
                    ),
                    (
                        "def_injected·ThingDef·Pear.label".to_string(),
                        "ru:\u{0007}pear".to_string(),
                    ),
                ]))]
                .into(),
            ),
        });
        let st = start_job_with_factory(
            &registry,
            &ctx.mgr,
            &ctx.managed,
            AiJobParams {
                batch_char_budget: 4000,
                ..long_job_params(&ctx.pid, ctx.epoch)
            },
            scripted_factory(provider.clone()),
        )
        .unwrap();
        wait_worker_finished(&registry.get(&st.job_id).unwrap(), 5);
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::PreviewReady);
        let st = accept_job(&registry, &ctx.mgr, &st.job_id, ctx.epoch).unwrap();
        assert_eq!(
            st.state,
            AiJobState::Applied,
            "1 of 2 landed is an honest partial Applied, got {st:?}"
        );
        let err = st.error.clone().unwrap_or_default();
        assert!(err.contains("partial apply: 1 applied, 1 refused"), "{err}");
        let refused: Vec<_> = st
            .rows
            .iter()
            .filter(|r| r.status == "apply_refused")
            .collect();
        assert_eq!(refused.len(), 1, "{st:?}");
        assert!(refused[0].identity.contains("Pear"), "{refused:?}");
        // Canonical truth: exactly the one accepted translation.
        let snap = ctx.mgr.snapshot(&ctx.pid).unwrap();
        let rus: Vec<_> = snap
            .project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .collect();
        assert_eq!(rus.len(), 1, "{rus:?}");
        assert_eq!(rus[0].text.as_deref(), Some("Яблоко"));
    }

    /// Review round 2, P1-A: the poll path must NEVER flip a Running job
    /// to PreviewReady. Fixture: one translatable label + one excluded
    /// texPath entry — the old `translated+failed+excluded >= total`
    /// arithmetic was satisfied (0+0+1 >= 1) while the only translatable
    /// unit was still blocked in flight, letting job_status publish
    /// PreviewReady, neutralizing cancel_job and letting accept apply an
    /// incomplete preview. Only the worker sets terminal states now.
    #[test]
    fn poll_never_flips_running_job_to_preview_cancel_stays_effective() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("mod/Defs")).unwrap();
        std::fs::create_dir_all(
            dir.path()
                .join("mod/Languages/English/DefInjected/ThingDef"),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mod/Defs/Mixed.xml"),
            "<Defs><ThingDef><defName>Mixed</defName><label>Mixed label</label></ThingDef></Defs>",
        )
        .unwrap();
        std::fs::write(
            dir.path()
                .join("mod/Languages/English/DefInjected/ThingDef/M.xml"),
            "<LanguageData>\n  <Mixed.texPath>Things/Mixed</Mixed.texPath>\n</LanguageData>\n",
        )
        .unwrap();
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        let registry = AiJobRegistry::new();
        let (rel_tx, rel_rx) = mpsc::channel();
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx: {
                let (tx, _rx) = mpsc::channel();
                tx
            },
            script: Mutex::new(vec![ScriptedAction::Block(rel_rx)].into()),
        });
        let st = start_job_with_factory(
            &registry,
            &mgr,
            &managed,
            long_job_params(&snap.project_id, snap.session_epoch),
            scripted_factory(provider.clone()),
        )
        .unwrap();
        let handle = registry.get(&st.job_id).unwrap();
        assert_eq!(st.excluded_notranslate, 1, "texPath must be excluded");
        assert_eq!(st.total, 1, "only the label is translatable");
        // The completion arithmetic IS satisfied while the unit is blocked:
        // the state must stay Running through every poll.
        for _ in 0..20 {
            let polled = job_status(&registry, &st.job_id).unwrap();
            assert_eq!(
                polled.state,
                AiJobState::Running,
                "premature PreviewReady while a unit is in flight: {polled:?}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        // Cancel stays effective: Running → Cancelled, worker exits, state
        // survives, accept refuses.
        let st = cancel_job(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::Cancelled);
        drop(rel_tx);
        wait_worker_finished(&handle, 5);
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::Cancelled, "{st:?}");
        let err = accept_job(&registry, &mgr, &st.job_id, snap.session_epoch).unwrap_err();
        assert!(err.message.contains("not in preview state"), "{err:?}");
    }

    /// prov-* → job factory vertical (review 2026-10-14 priority): an
    /// instance stored through ProviderOps (preset openai, explicit model,
    /// key in the sink) resolves in start_job into the factory target —
    /// provider id, model, base_url AND the key as key_inline — the job
    /// runs to preview and the full accept vertical works. The captured
    /// key is asserted, then dropped; it never prints.
    #[test]
    fn provider_instance_resolves_into_job_factory_target() {
        use crate::providers::MemorySecretSink;
        let dir = tempfile::tempdir().unwrap();
        fixture_mod(&dir.path().join("mod"));
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new_with_secret_sink(
            &managed,
            Arc::new(MemorySecretSink::default()),
        )
        .unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        let upserted = mgr
            .provider_instance_upsert(&crate::contract::ProviderInstanceUpsertRequest {
                instance_id: None,
                preset: "openai".into(),
                label: "Test instance".into(),
                model: "gpt-test".into(),
                base_url: None,
                secret: Some("sk-test-123".into()),
                local: None,
                expected_revision: None,
            })
            .unwrap();
        let instance_id = upserted.instance.id.clone();

        let registry = AiJobRegistry::new();
        type CapturedTarget = (String, Option<String>, Option<String>, Option<String>);
        let captured: Arc<Mutex<Vec<CapturedTarget>>> = Arc::new(Mutex::new(Vec::new()));
        let provider = Arc::new(ScriptedProvider {
            calls: AtomicUsize::new(0),
            started_tx: {
                let (tx, _rx) = mpsc::channel();
                tx
            },
            script: Mutex::new(VecDeque::new()),
        });
        let factory: AiProviderFactory = {
            let provider = provider.clone();
            let captured = captured.clone();
            Arc::new(
                move |params: &AiJobParams, key: Option<&crate::providers::ProviderSecret>| {
                    captured.lock().unwrap().push((
                        params.provider_id.clone(),
                        params.model.clone(),
                        params.base_url.clone(),
                        key.map(|k| k.expose().to_string()),
                    ));
                    Ok(provider.clone() as Arc<dyn rimloc_llm::provider::Provider + Send + Sync>)
                },
            )
        };
        let st = start_job_with_factory(
            &registry,
            &mgr,
            &managed,
            AiJobParams {
                project_id: snap.project_id.clone(),
                session_epoch: snap.session_epoch,
                locale: "Russian".into(),
                source_locale: "en".into(),
                provider_id: "direct-value-must-be-ignored".into(),
                model: None,
                base_url: None,
                provider_instance_id: Some(instance_id.clone()),
                batch_char_budget: 4000,
            },
            factory,
        )
        .unwrap();
        // Status reflects the RESOLVED target, not the ignored direct value.
        // No Debug formatting of status/manager values here: the test holds
        // a sink-carrying manager and CodeQL flags any log path from it
        // (cleartext-logging); the two plain asserts carry the failure info.
        assert_eq!(st.provider_id, "openai");
        assert_eq!(st.model, "gpt-test");
        let handle = registry.get(&st.job_id).unwrap();
        wait_worker_finished(&handle, 5);
        let st = job_status(&registry, &st.job_id).unwrap();
        assert_eq!(st.state, AiJobState::PreviewReady);
        let st = accept_job(&registry, &mgr, &st.job_id, snap.session_epoch).unwrap();
        assert_eq!(st.state, AiJobState::Applied);
        let seen = captured.lock().unwrap();
        assert_eq!(seen.len(), 1, "factory called exactly once: {seen:?}");
        let (pid, model, base_url, key) = &seen[0];
        assert_eq!(pid, "openai");
        assert_eq!(model.as_deref(), Some("gpt-test"));
        assert!(
            key.as_deref() == Some("sk-test-123"),
            "key must ride key_inline"
        );
        let _ = base_url; // preset default, provider-specific
        drop(seen);
    }

    /// Unknown and unresolvable instances are typed immediate refusals —
    /// BEFORE any worker spawn, so no paid work can start.
    #[test]
    fn provider_instance_refusals_are_typed_and_pre_spawn() {
        use crate::providers::MemorySecretSink;
        let dir = tempfile::tempdir().unwrap();
        fixture_mod(&dir.path().join("mod"));
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new_with_secret_sink(
            &managed,
            Arc::new(MemorySecretSink::default()),
        )
        .unwrap();
        let snap = mgr
            .create_with_source(&dir.path().join("mod"), Some("1.6"), None, None)
            .unwrap();
        let registry = AiJobRegistry::new();
        let err = start_job_with_factory(
            &registry,
            &mgr,
            &managed,
            AiJobParams {
                provider_instance_id: Some("prov-absent".into()),
                ..long_job_params(&snap.project_id, snap.session_epoch)
            },
            scripted_factory(Arc::new(ScriptedProvider {
                calls: AtomicUsize::new(0),
                started_tx: {
                    let (tx, _rx) = mpsc::channel();
                    tx
                },
                script: Mutex::new(VecDeque::new()),
            })),
        )
        .unwrap_err();
        assert_eq!(
            err.code,
            crate::contract::ContractErrorCode::ProjectNotFound
        );
        assert!(err.message.contains("unknown provider instance"), "{err:?}");
    }
}
