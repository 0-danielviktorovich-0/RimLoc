//! Project session layer (binding first slice, lead decision 033) — the ONE
//! executor behind the UI binding contract.
//!
//! Responsibilities:
//! - opaque durable [`ProjectId`]s minted here (never path-derived), the
//!   id + revision living in the SAME atomic managed-project record as the
//!   content ([`crate::project_store::ProjectEnvelopeMeta`]) — the registry
//!   is derived/recoverable from the managed directory, never a second
//!   authoritative half-transaction;
//! - `session_epoch` bumps on every open/refresh; stale UI sessions are
//!   dropped client-side by comparing epochs;
//! - `apply` (apply-intents): the service loads the TRUSTED canonical
//!   state, resolves each intent's identity/eligibility itself, sets
//!   provenance/status itself, and persists BEFORE the ack — the UI never
//!   sends a whole [`Project`];
//! - lost-update protection (`expected_revision`), external-change
//!   protection (content-hash of the managed file), per-project serialized
//!   save under the source-tree containment guard;
//! - `save_failed` keeps the applied (dirty) state in memory while the disk
//!   keeps the last good file; `refresh` is the recovery path (disk wins,
//!   epoch bumps).
//!
//! Job semantics in this synchronous slice: every mutating call carries an
//! observability job id; `cancel_next` marks the project so the NEXT apply
//! skips all intents (deterministic; a mid-batch cancel lands between
//! intents once application goes async in a later slice).

use crate::contract::{
    ApplyIntentsRequest, ApplyIntentsResponse, ContractError, ContractErrorCode, IntentAction,
    JobId, ProjectId, ProjectSnapshot, ProjectSummary, Revision, SessionEpoch, SkippedIntent,
    TranslationIntent, UI_CONTRACT_VERSION,
};
use crate::observability::{generate_operation_id, sha256_hex, OperationLog};
use crate::project::build_project;
use crate::project_store::{load_project_with_meta, save_project_with_meta, ProjectEnvelopeMeta};
use rimloc_domain::canonical::{
    Completeness, EntryKind, Lifecycle, Origin, Project, SourceEntry, SourceEntryId, Translation,
    ValidationState,
};
use rimloc_domain::eligibility::{Decision, Verdict};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Managed-project file extension (the same container format the store
/// owns; only the envelope metadata is additive).
const MANAGED_EXT: &str = "rimloc.json";

/// The per-project session state: the trusted in-memory canonical state,
/// its durable coordinates, and the dirty/recovery bookkeeping.
struct SessionState {
    /// Managed file (inside the manager's managed root).
    path: PathBuf,
    display_name: String,
    /// Read-only source root the project was built from (kept for
    /// diagnostics/rescan context; NEVER written).
    mod_root: PathBuf,
    target_version: Option<String>,
    epoch: SessionEpoch,
    /// Current in-memory revision (may be ahead of `acked_revision` while
    /// dirty).
    revision: Revision,
    /// Revision durably on disk (the last ACKED state).
    acked_revision: Revision,
    project: Project,
    /// sha256 of the managed file as last seen on disk.
    disk_hash: Option<String>,
    /// Applied-but-unpersisted edits exist in `project`/`revision`.
    dirty: bool,
    /// Cancel request for the next mutating operation (consumed by apply).
    cancel_requested: bool,
    /// The last FAILED operation (validate with errors, save failure) —
    /// the diagnostics bundle describes exactly this operation.
    last_failed_operation: Option<OperationLog>,
    /// Identities affected by the last failed operation (sanitized
    /// upstream of the bundle writer).
    last_failed_affected: Vec<String>,
    /// Source fingerprint RECORDED with the inventory (M3). `None` on
    /// legacy envelopes — drift cannot be evaluated for them.
    source_fingerprint: Option<String>,
    /// Drift verdict evaluated at session (re)start (create / open from
    /// disk / refresh) against the recorded fingerprint: `Some(false)` in
    /// sync, `Some(true)` the source changed under the project, `None`
    /// unknown (legacy envelope, or the source is unreadable at check
    /// time). Never recomputed per snapshot call — the verdict reflects
    /// the session's (re)start, which is exactly the restart cycle M3 is
    /// about.
    source_changed: Option<bool>,
}

/// The session manager. Clone-able (`Arc` inner); all methods take `&self`
/// and are serialized per project by construction.
#[derive(Clone)]
pub struct ProjectSessionManager {
    managed_root: PathBuf,
    inner: Arc<Mutex<HashMap<ProjectId, Arc<Mutex<SessionState>>>>>,
}

impl ProjectSessionManager {
    /// Create the manager over a managed-projects directory (created on
    /// demand). The directory is canonicalized once so every later guard
    /// check compares canonical views.
    pub fn new(managed_root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = managed_root.into();
        std::fs::create_dir_all(&root)?;
        let managed_root = std::fs::canonicalize(&root)?;
        Ok(Self {
            managed_root,
            inner: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// The managed root (for diagnostics surfaces).
    pub fn managed_root(&self) -> &Path {
        &self.managed_root
    }

    /// The managed file for a project id. FAIL-CLOSED: the id must match
    /// the mint form (`proj-<a-z0-9->`) — client-supplied strings with `/`,
    /// `..`, dots or absolute prefixes are rejected as a typed contract
    /// violation BEFORE any filesystem access — and the resulting path must
    /// stay within the managed root (defense in depth; `..` is impossible
    /// after the form check, the containment check survives future edits).
    pub fn managed_path(&self, project_id: &str) -> Result<PathBuf, ContractError> {
        static PROJECT_ID_FORM: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new(r"^proj-[a-z0-9-]+$").unwrap());
        if !PROJECT_ID_FORM.is_match(project_id) {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                "malformed project id: expected the minted `proj-<id>` form".to_string(),
            ));
        }
        let path = self
            .managed_root
            .join(format!("{project_id}.{MANAGED_EXT}"));
        // ALLOW-direction check (P2-7): this check GRANTS access to the
        // computed path, so containment must be proven — an unresolvable
        // path refuses, it is not treated as contained.
        if !crate::util::is_within_allow(&path, &self.managed_root) {
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "managed project path escapes the managed root".to_string(),
            ));
        }
        Ok(path)
    }

    fn mint_project_id(&self) -> ProjectId {
        loop {
            let id = generate_operation_id().replacen("op-", "proj-", 1);
            // Generated ids match the form by construction; only the
            // on-disk collision needs a retry.
            if !self
                .managed_root
                .join(format!("{id}.{MANAGED_EXT}"))
                .exists()
            {
                return id;
            }
        }
    }

    /// `project_create`: build the canonical project from a READ-ONLY
    /// source mod, mint the opaque id, persist-before-ack, return the
    /// snapshot.
    pub fn create(
        &self,
        mod_root: &Path,
        target_version: Option<&str>,
    ) -> Result<ProjectSnapshot, ContractError> {
        if !mod_root.is_dir() {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "source mod root `{}` is not a directory",
                    mod_root.display()
                ),
            ));
        }
        let project = build_project(mod_root, target_version)
            .map_err(|e| ContractError::new(ContractErrorCode::Internal, e.to_string()))?;
        // M3: the fingerprint is recorded at create so later sessions can
        // tell whether the source still matches the inventory. The view the
        // scanner just resolved successfully, so a fingerprint failure here
        // is an I/O race — a typed refusal, never a silently undetectable
        // project.
        let fingerprint =
            crate::scan::source_fingerprint(mod_root, target_version).map_err(|e| {
                ContractError::new(
                    ContractErrorCode::Internal,
                    format!("source fingerprint failed: {e}"),
                )
            })?;
        let project_id = self.mint_project_id();
        let display_name = mod_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| project_id.clone());
        let path = self
            .managed_root
            .join(format!("{project_id}.{MANAGED_EXT}"));

        // Persist-before-ack: the acked create is durably on disk.
        let revision: Revision = 1;
        let meta = ProjectEnvelopeMeta {
            project_id: Some(project_id.clone()),
            revision: Some(revision),
            display_name: Some(display_name.clone()),
            source_root: envelope_source_root(mod_root),
            source_fingerprint: Some(fingerprint.clone()),
        };
        save_project_with_meta(&project, &meta, &path).map_err(|e| {
            ContractError::new(ContractErrorCode::SaveFailed, format!("create failed: {e}"))
        })?;
        let disk_hash = disk_hash(&path);

        let state = SessionState {
            path,
            display_name,
            mod_root: mod_root.to_path_buf(),
            target_version: target_version.map(str::to_string),
            epoch: 1,
            revision,
            acked_revision: revision,
            project,
            disk_hash,
            dirty: false,
            cancel_requested: false,
            last_failed_operation: None,
            last_failed_affected: Vec::new(),
            source_fingerprint: Some(fingerprint),
            source_changed: Some(false),
        };
        let snapshot = snapshot_of(&project_id, &state);
        self.inner
            .lock()
            .expect("session registry poisoned")
            .insert(project_id, Arc::new(Mutex::new(state)));
        Ok(snapshot)
    }

    /// `project_open`: return the current snapshot and bump the session
    /// epoch. Recovers from disk when this manager instance has no live
    /// state for the id (restart / crash recovery).
    pub fn open(&self, project_id: &str) -> Result<ProjectSnapshot, ContractError> {
        let path = self.managed_path(project_id)?;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(project_id)
            .cloned();
        match arc {
            Some(arc) => {
                let mut st = arc.lock().expect("project session poisoned");
                st.epoch += 1;
                Ok(snapshot_of(project_id, &st))
            }
            None => {
                if !path.is_file() {
                    return Err(ContractError::project_not_found(project_id));
                }
                let loaded = load_project_with_meta(&path)
                    .map_err(|e| schema_or_internal(&e, project_id))?;
                let display_name = loaded
                    .meta
                    .display_name
                    .clone()
                    .unwrap_or_else(|| project_id.to_string());
                let target_version = loaded.project.context.target_version.clone();
                let revision = loaded.meta.revision.unwrap_or(1);
                // H5: the source root is durable envelope state — a
                // restart-recovered session restores it, keeping the
                // read-only source-tree guard functional. Legacy envelopes
                // (pre-H5) have none and stay fail-closed.
                let mod_root = loaded
                    .meta
                    .source_root
                    .clone()
                    .map(PathBuf::from)
                    .unwrap_or_default();
                // M3: at (re)start the current source content is compared
                // against the fingerprint recorded with the inventory —
                // drift (content rewrite, LoadFolders/version rollback)
                // becomes a visible verdict instead of an invisible trap.
                let fingerprint = loaded.meta.source_fingerprint.clone();
                let source_changed =
                    drift_verdict(fingerprint.as_deref(), &mod_root, target_version.as_deref());
                let state = SessionState {
                    path: path.clone(),
                    display_name,
                    mod_root,
                    target_version,
                    epoch: 1,
                    revision,
                    acked_revision: revision,
                    project: loaded.project,
                    disk_hash: disk_hash(&path),
                    dirty: false,
                    cancel_requested: false,
                    last_failed_operation: None,
                    last_failed_affected: Vec::new(),
                    source_fingerprint: fingerprint,
                    source_changed,
                };
                let snapshot = snapshot_of(project_id, &state);
                self.inner
                    .lock()
                    .expect("session registry poisoned")
                    .insert(project_id.to_string(), Arc::new(Mutex::new(state)));
                Ok(snapshot)
            }
        }
    }

    /// Read-only snapshot of the current state (no epoch bump).
    pub fn snapshot(&self, project_id: &str) -> Result<ProjectSnapshot, ContractError> {
        let path = self.managed_path(project_id)?;
        let _ = &path;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(project_id)
            .cloned()
            .ok_or_else(|| ContractError::project_not_found(project_id))?;
        let st = arc.lock().expect("project session poisoned");
        Ok(snapshot_of(project_id, &st))
    }

    /// `project_list`: every managed project — live states merged over the
    /// derived disk scan (the registry is recoverable, never authoritative).
    /// Unloadable files are skipped here; use [`Self::list_report`] when
    /// the caller can surface diagnostics (M2).
    pub fn list(&self) -> Vec<ProjectSummary> {
        self.list_report().projects
    }

    /// The full list view (M2): loadable projects PLUS explicit
    /// diagnostics for managed files that could not be loaded — corruption
    /// never looks like "the project is gone".
    pub fn list_report(&self) -> crate::contract::ProjectListReport {
        let mut out: BTreeMap<ProjectId, ProjectSummary> = BTreeMap::new();
        let mut unloadable: Vec<crate::contract::UnloadableProject> = Vec::new();
        let suffix = format!(".{MANAGED_EXT}");
        if let Ok(entries) = std::fs::read_dir(&self.managed_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Match by full file name: `Path::extension` of
                // `proj-x.rimloc.json` is "json", so an extension compare
                // against "rimloc.json" silently skipped EVERY managed
                // file (the disk half of the list was dead code).
                let Some(id) = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .and_then(|n| n.strip_suffix(&suffix))
                    .map(str::to_owned)
                else {
                    continue;
                };
                match load_project_with_meta(&path) {
                    Ok(loaded) => {
                        out.insert(
                            id.clone(),
                            ProjectSummary {
                                project_id: id.clone(),
                                name: loaded.meta.display_name.unwrap_or_else(|| id.clone()),
                                revision: loaded.meta.revision.unwrap_or(1),
                                target_version: loaded.project.context.target_version.clone(),
                            },
                        );
                    }
                    Err(e) => {
                        let reason = if e.to_string().contains("schema_version") {
                            "schema_version"
                        } else {
                            "corrupt_project"
                        };
                        unloadable.push(crate::contract::UnloadableProject {
                            project_id: id,
                            reason: reason.to_string(),
                        });
                    }
                }
            }
        }
        for (id, arc) in self.inner.lock().expect("session registry poisoned").iter() {
            let st = arc.lock().expect("project session poisoned");
            out.insert(
                id.clone(),
                ProjectSummary {
                    project_id: id.clone(),
                    name: st.display_name.clone(),
                    revision: st.revision,
                    target_version: st.target_version.clone(),
                },
            );
        }
        crate::contract::ProjectListReport {
            projects: out.into_values().collect(),
            unloadable,
        }
    }

    /// Recovery path: discard in-memory (possibly dirty) state and reload
    /// from disk; the session epoch bumps so stale sessions drop out.
    pub fn refresh(&self, project_id: &str) -> Result<ProjectSnapshot, ContractError> {
        let path = self.managed_path(project_id)?;
        if !path.is_file() {
            self.inner
                .lock()
                .expect("session registry poisoned")
                .remove(project_id);
            return Err(ContractError::project_not_found(project_id));
        }
        let loaded =
            load_project_with_meta(&path).map_err(|e| schema_or_internal(&e, project_id))?;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .entry(project_id.to_string())
            .or_insert_with(|| {
                Arc::new(Mutex::new(SessionState {
                    path: path.clone(),
                    display_name: project_id.to_string(),
                    mod_root: PathBuf::new(),
                    target_version: None,
                    epoch: 0,
                    revision: 1,
                    acked_revision: 1,
                    project: Project::default(),
                    disk_hash: None,
                    dirty: false,
                    cancel_requested: false,
                    last_failed_operation: None,
                    last_failed_affected: Vec::new(),
                    source_fingerprint: None,
                    source_changed: None,
                }))
            })
            .clone();
        {
            let target_version = loaded.project.context.target_version.clone();
            let mut st = arc.lock().expect("project session poisoned");
            st.epoch += 1;
            st.revision = loaded.meta.revision.unwrap_or(1);
            st.acked_revision = st.revision;
            st.project = loaded.project;
            st.disk_hash = disk_hash(&path);
            st.dirty = false;
            st.cancel_requested = false;
            if let Some(name) = loaded.meta.display_name {
                st.display_name = name;
            }
            // H5: refresh adopts the durable source root when the envelope
            // carries one (a legacy envelope leaves the current value —
            // never erase a known root because a field is absent).
            if let Some(src) = &loaded.meta.source_root {
                st.mod_root = PathBuf::from(src);
            }
            st.target_version = target_version.clone();
            // M3: refresh is a session (re)start — re-evaluate drift.
            st.source_fingerprint = loaded.meta.source_fingerprint.clone();
            st.source_changed = drift_verdict(
                st.source_fingerprint.as_deref(),
                &st.mod_root,
                target_version.as_deref(),
            );
        }
        let st = arc.lock().expect("project session poisoned");
        Ok(snapshot_of(project_id, &st))
    }

    /// Request cancellation of the project's NEXT mutating operation (and
    /// of a future async batch between intents). Deterministic in this
    /// synchronous slice: set before `apply` → the batch is skipped whole;
    /// set after → no-op.
    pub fn cancel_next(&self, project_id: &str) -> Result<bool, ContractError> {
        self.managed_path(project_id)?;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(project_id)
            .cloned()
            .ok_or_else(|| ContractError::project_not_found(project_id))?;
        let mut st = arc.lock().expect("project session poisoned");
        st.cancel_requested = true;
        Ok(true)
    }

    /// `project_apply` — THE mutating seam. Works on the trusted in-memory
    /// state, checks epoch/revision, resolves each intent's identity and
    /// eligibility itself, applies, sets provenance/status itself, bumps
    /// the revision and persists BEFORE the ack. The client never sends a
    /// whole [`Project`].
    pub fn apply(&self, req: &ApplyIntentsRequest) -> Result<ApplyIntentsResponse, ContractError> {
        let job_id: JobId = generate_operation_id();
        self.managed_path(&req.project_id)?;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(&req.project_id)
            .cloned()
            .ok_or_else(|| ContractError::project_not_found(&req.project_id))?;
        let mut st = arc.lock().expect("project session poisoned");

        // Stale-session guard.
        if st.epoch != req.session_epoch {
            return Err(ContractError::stale_epoch(req.session_epoch, st.epoch));
        }
        // Lost-update guard compares against the last ACKED revision: a
        // dirty failed save never published its revision, so a retry with
        // the acked base is legal and re-applies onto the dirty state.
        let base = if st.dirty {
            st.acked_revision
        } else {
            st.revision
        };
        if req.expected_revision != base {
            return Err(ContractError::stale_revision(req.expected_revision, base));
        }

        let mut log = OperationLog::new("project_apply");
        log.begin_stage("intents");
        let mut skipped: Vec<SkippedIntent> = Vec::new();
        let mut applied = 0usize;
        let cancelled = st.cancel_requested;
        st.cancel_requested = false;
        let engine = crate::eligibility_engine::EligibilityEngine::new();

        for (index, intent) in req.intents.iter().enumerate() {
            if cancelled {
                skipped.push(SkippedIntent {
                    index,
                    code: ContractErrorCode::UnsupportedCapability,
                    message: "cancelled before application".into(),
                });
                continue;
            }
            match apply_intent(&mut st.project, &engine, intent) {
                Ok(()) => applied += 1,
                Err((code, message)) => skipped.push(SkippedIntent {
                    index,
                    code,
                    message,
                }),
            }
        }
        log.end_stage("intents");

        // Nothing acked changes when every intent was refused: no revision
        // bump, no save, no dirty state.
        if applied == 0 {
            log.counter("intents", "applied", 0);
            log.counter("intents", "skipped", skipped.len() as u64);
            log.finish();
            return Ok(ApplyIntentsResponse {
                job_id,
                revision: st.revision,
                applied: 0,
                skipped,
                cancelled,
            });
        }

        // Revision bump + persist-before-ack.
        let new_revision = st.revision + 1;
        st.revision = new_revision;
        log.begin_stage("persist");
        let meta = ProjectEnvelopeMeta {
            project_id: Some(req.project_id.clone()),
            revision: Some(new_revision),
            display_name: Some(st.display_name.clone()),
            // The source root is durable state: every save re-asserts it so
            // a restart-recovered session keeps the export guard (H5).
            source_root: envelope_source_root(&st.mod_root),
            // The recorded fingerprint rides along (M3) — the DRIFT
            // verdict is computed at session (re)start, not rewritten by
            // edits.
            source_fingerprint: st.source_fingerprint.clone(),
        };
        // External-change check: the managed file must look exactly like the
        // last state this session saw on disk.
        if let Some(expected) = &st.disk_hash {
            let current = disk_hash(&st.path);
            if current.is_some_and(|c| &c != expected) {
                st.dirty = true;
                log.error_message(
                    "persist",
                    "managed file changed outside the session (content hash mismatch)",
                );
                log.finish();
                return Err(ContractError::new(
                    ContractErrorCode::ProjectChangedOnDisk,
                    "the managed project file changed outside this session; refresh to adopt                      the on-disk state (in-memory edits are kept until then)"
                        .to_string(),
                ));
            }
        }
        match save_project_with_meta(&st.project, &meta, &st.path) {
            Ok(()) => {
                st.acked_revision = new_revision;
                st.disk_hash = disk_hash(&st.path);
                st.dirty = false;
            }
            Err(e) => {
                // save_failed: the applied state stays DIRTY in memory (the
                // client keeps its draft); the disk keeps the last good
                // file; a retry re-applies onto the dirty state.
                st.dirty = true;
                log.error_message("persist", &e.to_string());
                log.finish();
                return Err(ContractError::new(
                    ContractErrorCode::SaveFailed,
                    format!("persist failed: {e}"),
                ));
            }
        }
        log.end_stage("persist");
        log.counter("intents", "applied", applied as u64);
        log.counter("intents", "skipped", skipped.len() as u64);
        log.finish();
        Ok(ApplyIntentsResponse {
            job_id,
            revision: new_revision,
            applied,
            skipped,
            cancelled,
        })
    }
    /// `project_validate` — run the EXISTING validator (026 severity) over
    /// the trusted session state. NEVER mutates the project: this is a
    /// read-only operation over the canonical inventory and its
    /// translations. Error-severity findings mark the operation failed
    /// (and become the diagnostics target); warnings/info are successful
    /// and reported.
    ///
    /// `locale` is a pure in-memory filter over stored translations — it
    /// never reaches the filesystem, so it needs no form guard; the strict
    /// language-folder form is enforced on the WRITE entries (`apply`,
    /// `export_project`) where client strings join output paths.
    pub fn validate_project(
        &self,
        project_id: &str,
        session_epoch: SessionEpoch,
        locale: Option<&str>,
    ) -> Result<crate::contract::ValidateProjectResponse, ContractError> {
        // Fail-closed id-form guard — the same entry discipline as every
        // other session operation (never probe the registry with a
        // traversal-shaped id).
        self.managed_path(project_id)?;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(project_id)
            .cloned()
            .ok_or_else(|| ContractError::project_not_found(project_id))?;
        let st = arc.lock().expect("project session poisoned");
        if st.epoch != session_epoch {
            return Err(ContractError::stale_epoch(session_epoch, st.epoch));
        }

        let job_id: JobId = generate_operation_id();
        let mut log = OperationLog::new("project_validate");
        log.begin_stage("validate");

        // Synthesize the validated surface from the TRUSTED state: one unit
        // per translation, with a path mirroring the REAL export layout
        // (`Languages/<locale>/...`). That path is what gives the units
        // validator scopes the existing duplicate logic understands
        // (per-language-folder): translations of one identity in two
        // locales compare as one Def (correct), while colliding
        // serialization keys of DIFFERENT def types stay distinct units.
        //
        // `identity_by_unit` carries the FULL structural id per unit —
        // findings are resolved back through (key, path), never through
        // the serialization key alone: the inventory legitimately holds
        // colliding keys across def types, and key-only matching attaches
        // the first entry's identity to someone else's finding.
        let mut units: Vec<rimloc_core::TransUnit> = Vec::new();
        let mut identity_by_unit: HashMap<(String, String), SourceEntryId> = HashMap::new();
        for tr in &st.project.translations {
            if let Some(filter) = locale {
                if tr.locale != filter {
                    continue;
                }
            }
            let Some(text) = tr.text.as_deref().filter(|t| !t.trim().is_empty()) else {
                continue;
            };
            let Some(entry) = st.project.entries.iter().find(|e| e.id == tr.source_id) else {
                continue;
            };
            let unit_path = synthesized_unit_path(entry, &tr.locale);
            identity_by_unit.insert((entry.id.key.clone(), unit_path.clone()), entry.id.clone());
            units.push(rimloc_core::TransUnit {
                key: entry.id.key.clone(),
                source: Some(text.to_string()),
                path: PathBuf::from(&unit_path),
                ..Default::default()
            });
        }

        let messages = rimloc_validate::validate(&units)
            .map_err(|e| ContractError::new(ContractErrorCode::Internal, e.to_string()))?;

        // Session-level classification (026 semantics, mine): a source
        // placeholder LOST by the translation is an Error even when the
        // translated text is well-formed on its own.
        let mut findings: Vec<crate::contract::ValidationFinding> = Vec::new();
        let mut error_count = 0usize;
        let mut warning_count = 0usize;
        let mut info_count = 0usize;
        let mut affected: Vec<String> = Vec::new();
        for m in messages {
            // Structural resolution: (key, path) is the synthesized unit's
            // own fingerprint; every validator emission echoes the unit's
            // fields verbatim. A miss (defensive — should not happen)
            // yields a finding WITHOUT an identity, never a wrong one.
            let id = identity_by_unit
                .get(&(m.key.clone(), m.path.clone()))
                .cloned();
            let severity = m.severity.as_str().to_string();
            match m.severity {
                rimloc_validate::ValidationSeverity::Error => {
                    error_count += 1;
                    if let Some(id) = &id {
                        affected.push(id.display_identity());
                    }
                }
                rimloc_validate::ValidationSeverity::Warning => warning_count += 1,
                rimloc_validate::ValidationSeverity::Info => info_count += 1,
            }
            findings.push(crate::contract::ValidationFinding {
                id,
                severity,
                kind: m.kind,
                key: m.key,
                path: m.path,
                line: m.line,
                message: m.message,
            });
        }
        // Lost-placeholder pass over the trusted pairs (source vs target).
        for tr in &st.project.translations {
            if let Some(filter) = locale {
                if tr.locale != filter {
                    continue;
                }
            }
            let Some(entry) = st.project.entries.iter().find(|e| e.id == tr.source_id) else {
                continue;
            };
            let src_tokens = placeholder_tokens(&entry.text);
            if src_tokens.is_empty() {
                continue;
            }
            let tr_tokens = tr
                .text
                .as_deref()
                .map(placeholder_tokens)
                .unwrap_or_default();
            if tr_tokens.is_empty() {
                error_count += 1;
                affected.push(entry.id.display_identity());
                findings.push(crate::contract::ValidationFinding {
                    id: Some(entry.id.clone()),
                    severity: "error".into(),
                    kind: "lost-placeholder".into(),
                    key: entry.id.key.clone(),
                    path: entry
                        .contexts
                        .first()
                        .map(|c| c.file.clone())
                        .unwrap_or_default(),
                    line: None,
                    message: format!(
                        "source has placeholder token(s) {src_tokens:?} but the translation dropped them all"
                    ),
                });
            }
        }

        // M4 early signal: case-colliding defNames are a WARNING here (the
        // project still validates against its own inventory) and a hard
        // refusal at export — the early finding lets the user fix the
        // source defName before building anything.
        for (def_type, name_a, name_b) in case_collision_pairs(&st.project, locale) {
            warning_count += 1;
            findings.push(crate::contract::ValidationFinding {
                id: None,
                severity: "warning".into(),
                kind: "case-collision".into(),
                key: format!("{name_a} / {name_b}"),
                path: format!("DefInjected/{def_type}"),
                line: None,
                message: format!(
                    "defNames `{name_a}` and `{name_b}` differ only in case: both would write into the same DefInjected file on Windows/macOS; rename one defName in the source mod"
                ),
            });
        }

        // M3 source-drift signal: the source content the inventory was
        // built from changed since this session's (re)start (edit, added/
        // removed file, LoadFolders version rollback). The project still
        // validates against its OWN inventory, so this stays a warning —
        // but the stale inventory must not pass silently.
        if st.source_changed == Some(true) {
            warning_count += 1;
            findings.push(crate::contract::ValidationFinding {
                id: None,
                severity: "warning".into(),
                kind: "source-drift".into(),
                key: "source-drift".into(),
                path: st.mod_root.to_string_lossy().into_owned(),
                line: None,
                message: "the source mod changed since this project was built (content edit or game-version rollback): the inventory is stale — rescan the source mod into a fresh project before building exports".into(),
            });
        }

        let status = if error_count > 0 {
            "failed"
        } else {
            "succeeded"
        };
        log.counter("validate", "findings", findings.len() as u64);
        log.counter("validate", "errors", error_count as u64);
        if status == "failed" {
            log.error_message("validate", "error-severity findings present");
        }
        log.finish();

        let response = crate::contract::ValidateProjectResponse {
            job_id: job_id.clone(),
            status: status.into(),
            findings,
            error_count,
            warning_count,
            info_count,
            locale: locale.map(str::to_string),
        };
        // Release the read view FIRST: guards drop lexically, so re-locking
        // the same session mutex below while `st` is still alive would
        // deadlock.
        drop(st);
        if status == "failed" {
            let mut st = arc.lock().expect("project session poisoned");
            st.last_failed_operation = Some(log);
            st.last_failed_affected = affected;
        }
        Ok(response)
    }

    /// `project_export` — build the native RimWorld translation output from
    /// the trusted session state into a CALLER-SPECIFIED out directory
    /// (isolated artifact, like the corpus harness). Guards: the out dir
    /// must NOT sit inside the read-only source tree nor inside the managed
    /// root (guard_output_denied). The result is REPARSED with the existing
    /// scanner before the ack; counters must match.
    pub fn export_project(
        &self,
        project_id: &str,
        session_epoch: SessionEpoch,
        out_dir: &Path,
        locale: &str,
    ) -> Result<crate::contract::ExportProjectResponse, ContractError> {
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(project_id)
            .cloned()
            .ok_or_else(|| ContractError::project_not_found(project_id))?;
        let st = arc.lock().expect("project session poisoned");
        if st.epoch != session_epoch {
            return Err(ContractError::stale_epoch(session_epoch, st.epoch));
        }
        // P1-2: the locale is joined into output paths
        // (`Languages/<locale>/...`) by the export writer. Validate the
        // strict folder form BEFORE any guard, path operation or write —
        // a traversal-shaped locale must never reach the writer.
        ensure_locale_form(locale)
            .map_err(|m| ContractError::new(ContractErrorCode::ContractViolation, m))?;
        let job_id: JobId = generate_operation_id();
        let mut log = OperationLog::new("project_export");
        log.begin_stage("guard");

        // Read-only source-tree guard. Fail-closed (P2-2): a session
        // recovered from disk after a restart carries no source root (the
        // envelope does not persist it) — a guard that cannot be
        // established must refuse the write, never silently disable
        // itself.
        if st.mod_root.as_os_str().is_empty() {
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "the session has no source root recorded; the read-only source-tree guard cannot be established — re-create the project from its source mod".to_string(),
            ));
        }
        // The out dir must not be inside/equal the read-only source root
        // (uses the fail-closed canonical view).
        if crate::is_within(out_dir, &st.mod_root) {
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                format!(
                    "output directory `{}` is inside the read-only source tree `{}`",
                    out_dir.display(),
                    st.mod_root.display()
                ),
            ));
        }
        // Managed-root guard: artifacts never overwrite managed records.
        if crate::is_within(out_dir, &self.managed_root) {
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                format!(
                    "output directory `{}` is inside the managed projects root `{}`",
                    out_dir.display(),
                    self.managed_root.display()
                ),
            ));
        }
        log.end_stage("guard");

        // H3 pre-write content check: texts that are about to be written
        // (target-locale translations + the display name landing in
        // About.xml) must be XML 1.0-clean — control characters cannot be
        // escaped, so they are a typed refusal, never a raw write.
        let mut poisoned: Vec<String> = Vec::new();
        for t in &st.project.translations {
            if t.locale != locale {
                continue;
            }
            let Some(text) = t.text.as_deref() else {
                continue;
            };
            let Some(bad) = rimloc_core::xml_chars::find_invalid_xml_char(text) else {
                continue;
            };
            let identity = st
                .project
                .entries
                .iter()
                .find(|e| e.id == t.source_id)
                .map(|e| e.id.display_identity())
                .unwrap_or_else(|| t.source_id.key.clone());
            if poisoned.len() < 5 {
                poisoned.push(format!("{identity} (U+{:04X})", bad as u32));
            } else {
                poisoned.push("…".to_string());
                break;
            }
        }
        if !poisoned.is_empty() {
            log.error_message("write", "XML-invalid control characters in export content");
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "cannot export: translation text contains characters that are invalid in XML 1.0 — the game would drop the whole file. Fix the entries: {}",
                    poisoned.join(", ")
                ),
            ));
        }
        if let Some(bad) = rimloc_core::xml_chars::find_invalid_xml_char(&st.display_name) {
            log.error_message("write", "XML-invalid control character in display name");
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "cannot export: the project display name contains a character that is invalid in XML 1.0 (U+{:04X})",
                    bad as u32
                ),
            ));
        }

        // M4 pre-write content check: defNames differing only by case within
        // one def type materialize into the SAME output file on a
        // case-insensitive filesystem (macOS/Windows) — the reparse guard
        // would only catch that on such filesystems, and on a case-sensitive
        // one the collision would ship to players. Refuse deterministically,
        // on every filesystem, with the colliding names.
        let collisions = case_collision_pairs(&st.project, Some(locale));
        if !collisions.is_empty() {
            let shown: Vec<String> = collisions
                .iter()
                .take(5)
                .map(|(dt, a, b)| format!("{dt}: {a} / {b}"))
                .collect();
            log.error_message("write", "case-colliding defNames in export content");
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                format!(
                    "cannot export: defNames that differ only in case would write into the same file on Windows/macOS — rename the defName in the source mod: {}",
                    shown.join(", ")
                ),
            ));
        }

        let rw_version = st.target_version.clone().unwrap_or_else(|| "1.6".into());
        log.begin_stage("write");
        let report = crate::project::write_rimworld_translation(
            &st.project,
            out_dir,
            locale,
            &st.display_name,
            // H4: the folder name is NOT a valid RimWorld packageId — a
            // restricted-charset slug with the fixed `rimloc.` author
            // segment is.
            &crate::util::package_id_slug(&st.display_name),
            &rw_version,
        )
        .map_err(|e| {
            log.error_message("write", &e.to_string());
            log.finish();
            ContractError::new(
                ContractErrorCode::Internal,
                format!("native write failed: {e}"),
            )
        })?;
        log.end_stage("write");

        // Reparse check BEFORE the ack: the written output must come back
        // through the EXISTING scanner with exactly the keys the writer
        // reports writing (the writer's own acceptance accounting — never
        // a re-derived filter, which silently drifts).
        log.begin_stage("reparse");
        let units = rimloc_parsers_xml::scan_keyed_xml(&report.out_mod).map_err(|e| {
            log.error_message("reparse", &e.to_string());
            log.finish();
            ContractError::new(
                ContractErrorCode::Internal,
                format!("reparse of the written output failed: {e}"),
            )
        })?;
        if units.len() != report.keys_written {
            log.error_message(
                "reparse",
                &format!(
                    "reparse count mismatch: wrote {}, reparsed {}",
                    report.keys_written,
                    units.len()
                ),
            );
            log.finish();
            return Err(ContractError::new(
                ContractErrorCode::Internal,
                format!(
                    "reparse count mismatch: wrote {} keys, reparsed {}",
                    report.keys_written,
                    units.len()
                ),
            ));
        }
        // H3 strict tripwire: the lenient scanner above can silently
        // accept XML a strict parser (and the game) would reject. Every
        // written file is byte-verified against the XML 1.0 char set; a
        // failure here is a writer bug and fails the export.
        crate::util::verify_xml_char_validity(&report.out_mod).map_err(|e| {
            log.error_message("reparse", &e.to_string());
            log.finish();
            ContractError::new(
                ContractErrorCode::Internal,
                format!("strict XML verification of the written output failed: {e}"),
            )
        })?;
        log.counter("reparse", "keys", units.len() as u64);
        log.end_stage("reparse");
        log.finish();

        let files_written = count_files(&report.out_mod);
        Ok(crate::contract::ExportProjectResponse {
            job_id,
            out_dir: crate::contract::PathBufDto::new(report.out_mod.display().to_string()),
            files_written,
            reparsed_keys: units.len(),
            skipped_unknown_type: report.skipped_unknown_type,
        })
    }

    /// `project_diagnose` — sanitized support bundle over the project's
    /// LAST FAILED operation (validate errors, save failure). The out dir
    /// must stay outside the read-only source tree (the collector enforces
    /// this itself); the bundle is sanitized by the L-owned pipeline.
    pub fn diagnose(
        &self,
        project_id: &str,
        out_dir: &Path,
    ) -> Result<crate::contract::DiagnoseResponse, ContractError> {
        // Fail-closed id-form guard — the same entry discipline as every
        // other session operation.
        self.managed_path(project_id)?;
        let arc = self
            .inner
            .lock()
            .expect("session registry poisoned")
            .get(project_id)
            .cloned()
            .ok_or_else(|| ContractError::project_not_found(project_id))?;
        let st = arc.lock().expect("project session poisoned");
        // Fail-closed (P2-2): without the source root the bundle's
        // out-of-source-tree guard is vacuous — refuse instead of writing
        // an unverifiable bundle.
        if st.mod_root.as_os_str().is_empty() {
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "the session has no source root recorded; the read-only source-tree guard cannot be established — re-create the project from its source mod".to_string(),
            ));
        }
        let Some(operation) = st.last_failed_operation.clone() else {
            return Err(ContractError::new(
                ContractErrorCode::ContractViolation,
                "the project has no failed operation to diagnose; run validate or apply first"
                    .to_string(),
            ));
        };
        let job_id: JobId = generate_operation_id();
        let inputs = crate::observability::SupportBundleInputs {
            scan_root: st.mod_root.clone(),
            project_meta: crate::observability::ProjectMeta {
                name: Some(st.display_name.clone()),
                target_lang: None,
                rw_version: st.target_version.clone(),
                rimloc_version: Some(format!("ui-contract/{UI_CONTRACT_VERSION}")),
                extra: serde_json::json!({
                    "project_id": project_id,
                    "revision": st.revision,
                }),
            },
            operation: Some(operation),
            affected: st.last_failed_affected.clone(),
        };
        let bundle =
            crate::observability::collect_support_bundle_for(&inputs, out_dir).map_err(|e| {
                ContractError::new(
                    ContractErrorCode::Internal,
                    format!("support bundle collection failed: {e}"),
                )
            })?;
        Ok(crate::contract::DiagnoseResponse {
            job_id,
            bundle_dir: crate::contract::PathBufDto::new(bundle.dir.display().to_string()),
            operation_id: bundle.operation_id,
            files: bundle.files.iter().map(|f| f.path.clone()).collect(),
            redacted_count: bundle.redacted.len(),
            excluded_count: bundle.excluded.len(),
        })
    }
}

/// Strict language-folder form for client locale strings (P1-2). A locale
/// is joined into `Languages/<locale>/...` output paths by the export
/// writer — anything but a plain folder name (letters, digits, `_`, `-`)
/// is rejected BEFORE any path is built, mirroring the project-id form
/// guard (`managed_path`). One form source for session and CLI: the
/// predicate lives in [`crate::util::lang_dir_form_ok`].
fn ensure_locale_form(locale: &str) -> Result<(), String> {
    if crate::util::lang_dir_form_ok(locale) {
        Ok(())
    } else {
        Err(format!(
            "malformed locale `{locale}`: expected the language-folder form (letters, digits, `_`, `-`)"
        ))
    }
}

/// Virtual validation path for one entry translation, mirroring the REAL
/// export layout (`Languages/<locale>/Keyed|DefInjected/...`). Every unit
/// gets a stable location the validator scopes correctly (per language
/// folder), and units of different def types stay structurally distinct
/// even when their serialization keys collide. An entry whose def type is
/// not resolvable from its identity lands in `_unresolved` — the label is
/// a scope, never a guessed classification (no context-path heuristics
/// here; the export writer owns that resolution).
fn synthesized_unit_path(entry: &SourceEntry, locale: &str) -> String {
    match entry.id.kind {
        EntryKind::Keyed => format!("Languages/{locale}/Keyed/Translation.xml"),
        EntryKind::TKey | EntryKind::DefInjected => {
            let def_type = entry
                .id
                .def_type
                .clone()
                .or_else(|| entry.tkey.as_ref().map(|m| m.def_type.clone()))
                .unwrap_or_else(|| "_unresolved".into());
            let def_name = entry
                .id
                .key
                .split('.')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("unnamed");
            format!("Languages/{locale}/DefInjected/{def_type}/{def_name}.xml")
        }
        // Kinds the export writer does not emit yet (Strings/Backstories/
        // PatchDerived): still validated, in a distinct stable bucket.
        _ => format!("Languages/{locale}/_other/{}.xml", entry.id.key),
    }
}

/// DefInjected defNames that collide case-insensitively within one def
/// type (M4): the export writer materializes ONE file per defName
/// (`{defName}.xml`), so `Dup` and `dup` are the same file on a
/// case-insensitive filesystem (macOS/Windows) — one translation would
/// silently overwrite the other. Detected per (locale, def type) —
/// colliding names across different export locales never share a file.
/// Returns `(def_type, name_a, name_b)` with the ORIGINAL spellings.
fn case_collision_pairs(project: &Project, locale: Option<&str>) -> Vec<(String, String, String)> {
    use std::collections::BTreeMap;
    // (locale, def_type, lowercase defName) -> original defName.
    let mut seen: BTreeMap<(String, String, String), String> = BTreeMap::new();
    // (def_type, name_a) -> name_b (first collision wins, deterministic).
    let mut collisions: BTreeMap<(String, String), String> = BTreeMap::new();
    for t in &project.translations {
        if let Some(filter) = locale {
            if t.locale != filter {
                continue;
            }
        }
        if t.lifecycle == Lifecycle::Obsolete {
            continue;
        }
        let Some(text) = t.text.as_deref().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let _ = text;
        let Some(entry) = project.entries.iter().find(|e| e.id == t.source_id) else {
            continue;
        };
        if !matches!(entry.id.kind, EntryKind::TKey | EntryKind::DefInjected) {
            continue;
        }
        let Some(def_type) = entry
            .id
            .def_type
            .clone()
            .or_else(|| entry.tkey.as_ref().map(|m| m.def_type.clone()))
            .or_else(|| crate::project::def_type_from_contexts(entry))
        else {
            continue;
        };
        let Some(def_name) = entry.id.key.split('.').next().filter(|s| !s.is_empty()) else {
            continue;
        };
        let slot = seen.entry((t.locale.clone(), def_type.clone(), def_name.to_lowercase()));
        match slot {
            std::collections::btree_map::Entry::Vacant(v) => {
                v.insert(def_name.to_string());
            }
            std::collections::btree_map::Entry::Occupied(o) => {
                let first = o.get();
                if *first != def_name {
                    collisions
                        .entry((def_type.clone(), first.clone()))
                        .or_insert_with(|| def_name.to_string());
                }
            }
        }
    }
    collisions
        .into_iter()
        .map(|((def_type, a), b)| (def_type, a, b))
        .collect()
}

/// Placeholder tokens of a text ('%s'-style and {name}/{0}).
fn placeholder_tokens(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j].is_ascii_alphanumeric() {
                    j += 1;
                }
                if j > i + 1 {
                    out.push(text[i..j].to_string());
                    i = j;
                    continue;
                }
                i += 1;
            }
            b'{' => {
                if let Some(end) = text[i..].find('}') {
                    out.push(text[i..=i + end].to_string());
                    i = i + end + 1;
                    continue;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count files under a directory recursively (export artifact report).
fn count_files(root: &Path) -> usize {
    std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| {
                    if e.path().is_dir() {
                        count_files(&e.path())
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// sha256 of the managed file (None when absent).
fn disk_hash(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| sha256_hex(&b))
}

/// Durable envelope form of the session's source root (H5): `None` when the
/// session has no root (recovered legacy state) so the envelope keeps
/// honestly saying "no source context", never an empty-string root.
fn envelope_source_root(mod_root: &Path) -> Option<String> {
    if mod_root.as_os_str().is_empty() {
        None
    } else {
        Some(mod_root.to_string_lossy().into_owned())
    }
}

fn schema_or_internal(err: &color_eyre::Report, project_id: &str) -> ContractError {
    let text = err.to_string();
    if text.contains("schema_version") {
        ContractError::new(
            ContractErrorCode::SchemaVersion,
            format!("project `{project_id}` uses an unsupported container version"),
        )
    } else {
        ContractError::new(
            ContractErrorCode::Internal,
            format!("failed to load project `{project_id}`: {text}"),
        )
    }
}

fn snapshot_of(project_id: &str, st: &SessionState) -> ProjectSnapshot {
    ProjectSnapshot {
        project_id: project_id.to_string(),
        revision: st.revision,
        session_epoch: st.epoch,
        dirty: st.dirty,
        acked_revision: st.acked_revision,
        project: st.project.clone(),
        source_changed: st.source_changed,
    }
}

/// M3 drift verdict: `Some(cur != recorded)` when a recorded fingerprint
/// can be compared against the readable source; `None` when there is
/// nothing recorded (legacy envelope) or the source cannot be fingerprinted
/// right now — unknown stays unknown, never a false "in sync".
fn drift_verdict(
    recorded: Option<&str>,
    mod_root: &Path,
    target_version: Option<&str>,
) -> Option<bool> {
    let recorded = recorded?;
    if mod_root.as_os_str().is_empty() {
        return None;
    }
    let current = crate::scan::source_fingerprint(mod_root, target_version).ok()?;
    Some(current != recorded)
}

/// Apply ONE intent to the trusted canonical state. The service owns
/// provenance/status: origin is Human (a user edit), completeness follows
/// the action, validation runs the on-emission severity classification
/// (placeholder scan per 026), and eligibility is enforced — a
/// Deterministic/Verified NON_TRANSLATABLE entry cannot be edited.
fn apply_intent(
    project: &mut Project,
    engine: &crate::eligibility_engine::EligibilityEngine,
    intent: &TranslationIntent,
) -> Result<(), (ContractErrorCode, String)> {
    // Locale form first (P1-2): intent locales persist into the durable
    // record, and a malformed one would poison every later export long
    // before the writer materializes a path. Rejected per intent, as data.
    ensure_locale_form(&intent.locale).map_err(|m| (ContractErrorCode::ContractViolation, m))?;

    // Identity resolution: FULL structural match against the trusted
    // inventory. No key-shape fallback, no "Misc" scope.
    let Some(pos) = project.entries.iter().position(|e| e.id == intent.entry) else {
        return Err((
            ContractErrorCode::ContractViolation,
            format!(
                "identity {} not found in the inventory; rescan the source mod",
                intent.entry.display_identity()
            ),
        ));
    };
    let entry = &project.entries[pos];

    // Eligibility gate (the single engine validate uses).
    let Verdict { decision, .. } = engine.evaluate(entry, None);
    if decision == Decision::NonTranslatable {
        return Err((
            ContractErrorCode::ContractViolation,
            format!(
                "identity {} is classified non-translatable; editing it is not a permitted action",
                intent.entry.display_identity()
            ),
        ));
    }

    let locale = intent.locale.clone();
    let text = match intent.action {
        IntentAction::SetTranslation => {
            let Some(text) = intent
                .text
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
            else {
                return Err((
                    ContractErrorCode::ContractViolation,
                    "set_translation requires non-empty text".into(),
                ));
            };
            Some(text.to_string())
        }
        IntentAction::MarkTodo => Some("TODO".to_string()),
        IntentAction::ClearTranslation => None,
    };
    // H3: control characters cannot be escaped in XML 1.0 (even numeric
    // references to them are invalid) — a text carrying one would be
    // written raw and RimWorld would drop the whole file. Refused at
    // emission, per intent.
    if let Some(text) = &text {
        if let Some(bad) = rimloc_core::xml_chars::find_invalid_xml_char(text) {
            return Err((
                ContractErrorCode::ContractViolation,
                format!(
                    "text contains a character that is invalid in XML 1.0 (U+{:04X}); remove the control character",
                    bad as u32
                ),
            ));
        }
    }
    let completeness = match intent.action {
        IntentAction::SetTranslation => Completeness::Translated,
        IntentAction::MarkTodo => Completeness::Todo,
        IntentAction::ClearTranslation => Completeness::Untranslated,
    };

    // Severity classification at emission (026): suspicious placeholders
    // are Issues, everything else Ok for this slice.
    let validation = match &text {
        Some(text) if rimloc_core::placeholders::is_bad_percent(text) => {
            ValidationState::Issues(vec![
                "suspicious placeholder (single % not part of a known token)".into(),
            ])
        }
        _ => ValidationState::Ok,
    };

    let existing = project
        .translations
        .iter_mut()
        .find(|t| t.source_id == intent.entry && t.locale == locale);
    match existing {
        Some(t) => {
            t.text = text;
            t.completeness = completeness;
            t.validation = validation;
            t.origin = Origin::Human;
        }
        None => {
            project.translations.push(Translation {
                source_id: intent.entry.clone(),
                locale,
                text,
                completeness,
                review: rimloc_domain::canonical::Review::None,
                validation,
                lifecycle: rimloc_domain::canonical::Lifecycle::Active,
                origin: Origin::Human,
                notes: String::new(),
                source_changed: None,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ApplyIntentsRequest, ContractErrorCode, UI_CONTRACT_VERSION};
    use rimloc_domain::canonical::{EntryKind, SourceEntryId};
    use std::fs;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    /// Two ThingDefs whose defNames differ ONLY in case (`Dup` / `dup`) —
    /// from two source files (defName collisions live in XML content, so
    /// the fixture works on every filesystem).
    fn case_collision_mod(root: &Path) {
        write(
            &root.join("Defs/A1.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing A</label></ThingDef></Defs>"#,
        );
        write(
            &root.join("Defs/B1.xml"),
            r#"<Defs><ThingDef><defName>dup</defName><label>thing B</label></ThingDef></Defs>"#,
        );
    }

    fn two_types_mod(root: &Path) {
        write(
            &root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing label</label></ThingDef></Defs>"#,
        );
        write(
            &root.join("Defs/B_Ability.xml"),
            r#"<Defs><AbilityDef><defName>Dup</defName><label>ability label</label></AbilityDef></Defs>"#,
        );
    }

    fn intent(
        key: &str,
        def_type: &str,
        text: Option<&str>,
        action: IntentAction,
    ) -> TranslationIntent {
        TranslationIntent {
            entry: SourceEntryId {
                kind: EntryKind::DefInjected,
                key: key.into(),
                def_type: Some(def_type.into()),
            },
            locale: "Russian".into(),
            action,
            text: text.map(str::to_string),
        }
    }

    fn set_text(key: &str, def_type: &str, text: &str) -> TranslationIntent {
        intent(key, def_type, Some(text), IntentAction::SetTranslation)
    }

    fn req(
        pid: &str,
        epoch: SessionEpoch,
        rev: Revision,
        intents: Vec<TranslationIntent>,
    ) -> ApplyIntentsRequest {
        ApplyIntentsRequest {
            project_id: pid.into(),
            expected_revision: rev,
            session_epoch: epoch,
            intents,
        }
    }

    /// Create → snapshot(rev 1) → apply(ack) → rev 2 durable → reopen keeps
    /// everything, bumps the epoch.
    #[test]
    fn create_apply_reopen_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();

        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        assert_eq!(snap.revision, 1);
        assert_eq!(snap.session_epoch, 1);
        assert!(snap.project.entries.len() >= 2);

        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap();
        assert_eq!(res.applied, 1);
        assert_eq!(res.revision, 2);
        assert!(!res.job_id.is_empty());

        // Durable: the file carries the new revision + the translation.
        let loaded = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        assert_eq!(loaded.meta.revision, Some(2));
        assert_eq!(
            loaded.meta.project_id.as_deref(),
            Some(snap.project_id.as_str())
        );

        // Reopen: epoch bumps, content persists.
        let reopened = mgr.open(&snap.project_id).unwrap();
        assert_eq!(reopened.session_epoch, 2);
        assert_eq!(reopened.revision, 2);
    }

    /// Stale epoch and stale revision are distinct typed errors, and a
    /// refused request changes nothing.
    #[test]
    fn stale_epoch_and_revision_are_typed_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let err = mgr
            .apply(&req(&snap.project_id, 99, 1, vec![]))
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::StaleEpoch);
        let err = mgr
            .apply(&req(&snap.project_id, 1, 99, vec![]))
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::StaleRevision);
        assert_eq!(mgr.snapshot(&snap.project_id).unwrap().revision, 1);
    }

    /// Full structural identity: two def types sharing a key get their own
    /// translations; the wire never sends a whole Project.
    #[test]
    fn intents_hit_typed_identities_only() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![
                    set_text("Dup.label", "ThingDef", "вещь"),
                    set_text("Dup.label", "AbilityDef", "способность"),
                ],
            ))
            .unwrap();
        assert_eq!(res.applied, 2);

        let loaded = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        let get = |dt: &str| {
            loaded
                .project
                .translations
                .iter()
                .find(|t| {
                    t.source_id.key == "Dup.label"
                        && t.source_id.def_type.as_deref() == Some(dt)
                        && t.locale == "Russian"
                })
                .unwrap()
        };
        assert_eq!(get("ThingDef").text.as_deref(), Some("вещь"));
        assert_eq!(get("AbilityDef").text.as_deref(), Some("способность"));
        assert_eq!(get("ThingDef").origin, Origin::Human);
        assert_eq!(get("ThingDef").completeness, Completeness::Translated);
    }

    /// save_failed keeps the dirty state; a retry after the fs problem is
    /// cleared persists it (persist-before-ack: nothing was acked).
    #[test]
    fn save_failed_keeps_dirty_and_retry_persists() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // Break the durable write: a DIRECTORY sits at the managed path.
        let file_path = mgr.managed_path(&snap.project_id).unwrap();
        fs::remove_file(&file_path).unwrap();
        fs::create_dir(&file_path).unwrap();

        let err = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::SaveFailed);

        // The dirty in-memory state exists (revision applied ahead of disk).
        assert_eq!(mgr.snapshot(&snap.project_id).unwrap().revision, 2);

        // Clear the fs problem; the retried batch (same acked base 1)
        // applies onto the dirty state and persists.
        fs::remove_dir(&file_path).unwrap();
        // The retry re-applies onto the dirty state: the in-memory revision
        // is monotonic (2 -> 3); the retried base is the last ACKED 1.
        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap();
        assert_eq!(res.revision, 3);
        assert_eq!(res.applied, 1);
        let loaded = load_project_with_meta(&file_path).unwrap();
        assert_eq!(loaded.meta.revision, Some(3));
        assert!(!loaded.project.translations.is_empty());
    }

    /// External change of the managed file between load and save is a
    /// typed error; refresh adopts the on-disk state and bumps the epoch.
    #[test]
    fn external_change_is_detected_and_refresh_adopts_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // External writer (another process) bumps the file to revision 9.
        let path = mgr.managed_path(&snap.project_id).unwrap();
        let external = load_project_with_meta(&path).unwrap();
        let meta = ProjectEnvelopeMeta {
            project_id: Some(snap.project_id.clone()),
            revision: Some(9),
            display_name: Some("external".into()),
            source_root: Some(mod_root.to_string_lossy().into_owned()),
            // The recorded fingerprint is durable state an outside writer
            // bumps only alongside the content it describes.
            source_fingerprint: external.meta.source_fingerprint.clone(),
        };
        save_project_with_meta(&external.project, &meta, &path).unwrap();

        let err = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ProjectChangedOnDisk);

        // Refresh adopts disk: revision 9, epoch bumps.
        let refreshed = mgr.refresh(&snap.project_id).unwrap();
        assert_eq!(refreshed.revision, 9);
        assert_eq!(refreshed.session_epoch, 2);
    }

    /// Restart/crash recovery: a fresh manager (no live state) reopens the
    /// durable record with its id/revision intact.
    #[test]
    fn reopen_after_manager_loss_recovers_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();
        assert_eq!(reopened.revision, 1);
        assert_eq!(reopened.session_epoch, 1);
        assert!(mgr2.list().iter().any(|s| s.project_id == snap.project_id));
    }

    /// Unknown identity and eligibility (NoTranslate family) are per-intent
    /// contract violations; a fully-refused batch bumps nothing.
    #[test]
    fn refused_intents_are_data_not_failures() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        // A DefInjected sidecar with a NoTranslate-family field: the builtin
        // engine classifies `texPath` as deterministic NON_TRANSLATABLE.
        write(
            &mod_root.join("Languages/English/DefInjected/ThingDef/W.xml"),
            "<LanguageData>\n  <Widget.texPath>Some/Texture</Widget.texPath>\n  <Widget.label>a label</Widget.label>\n</LanguageData>\n",
        );
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![
                    set_text("Ghost.label", "ThingDef", "x"),
                    set_text("Widget.texPath", "ThingDef", "Texture/Path"),
                    set_text("Widget.label", "ThingDef", "метка"),
                ],
            ))
            .unwrap();
        assert_eq!(res.applied, 1);
        assert_eq!(res.skipped.len(), 2);
        assert_eq!(res.skipped[0].code, ContractErrorCode::ContractViolation);
        assert_eq!(res.skipped[1].code, ContractErrorCode::ContractViolation);
        assert_eq!(res.revision, 2);

        // Zero-applied batch: no revision bump, no save, no dirty. (apply
        // never bumps the epoch, so the live session is still epoch 1.)
        let before = mgr.snapshot(&snap.project_id).unwrap().revision;
        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                2,
                vec![set_text("Ghost.label", "ThingDef", "x")],
            ))
            .unwrap();
        assert_eq!(res.applied, 0);
        assert_eq!(res.revision, before);
    }

    /// Cancel requested before apply: the batch is skipped whole, nothing
    /// persists; the flag is consumed by the operation.
    #[test]
    fn cancel_skips_the_next_batch_whole() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        mgr.cancel_next(&snap.project_id).unwrap();
        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap();
        assert!(res.cancelled);
        assert_eq!(res.applied, 0);
        assert_eq!(res.revision, 1, "nothing acked, nothing bumped");

        // The flag was consumed: the next batch applies normally.
        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap();
        assert!(!res.cancelled);
        assert_eq!(res.applied, 1);
    }

    /// list() merges live and disk states; unknown project id is a typed
    /// not-found across open/snapshot/refresh.
    #[test]
    fn list_merges_and_unknown_id_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let listed = mgr.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].project_id, snap.project_id);
        assert_eq!(listed[0].name, "mod");

        let err = mgr.open("proj-does-not-exist").unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ProjectNotFound);
        assert_eq!(
            mgr.snapshot("proj-does-not-exist").unwrap_err().code,
            ContractErrorCode::ProjectNotFound
        );
        assert_eq!(
            mgr.refresh("proj-does-not-exist").unwrap_err().code,
            ContractErrorCode::ProjectNotFound
        );
    }

    /// The contract version constant reaches the session surface (one
    /// handshake constant, no drift).
    #[test]
    fn contract_version_reaches_sessions() {
        assert_eq!(UI_CONTRACT_VERSION, 1);
    }
    /// P1 security regression (L cross-review): client-supplied project ids
    /// with traversal/absolute shapes are a typed rejection on EVERY entry
    /// point — never an Ok, never a ProjectNotFound that depends on what
    /// happens to exist at the escaped path.
    #[test]
    fn path_traversal_ids_are_rejected_on_every_entry_point() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        mgr.create(&mod_root, Some("1.6")).unwrap();

        let evil = [
            "../../etc/passwd",
            "/tmp/evil",
            "proj-../../etc/x",
            "proj-a/../../x",
            "proj-a\\x",
            "PROJ-UPPER",
            "proj-a.",
            ".",
            "..",
            "",
        ];
        for id in evil {
            let open = mgr.open(id).unwrap_err();
            assert_eq!(
                open.code,
                ContractErrorCode::ContractViolation,
                "[{id}] {open}"
            );
            let snap = mgr.snapshot(id).unwrap_err();
            assert_eq!(snap.code, ContractErrorCode::ContractViolation, "[{id}]");
            let refresh = mgr.refresh(id).unwrap_err();
            assert_eq!(refresh.code, ContractErrorCode::ContractViolation, "[{id}]");
            let cancel = mgr.cancel_next(id).unwrap_err();
            assert_eq!(cancel.code, ContractErrorCode::ContractViolation, "[{id}]");
            let apply = mgr
                .apply(&req(id, 1, 1, vec![set_text("Dup.label", "ThingDef", "x")]))
                .unwrap_err();
            assert_eq!(apply.code, ContractErrorCode::ContractViolation, "[{id}]");
        }
        // Nothing was written outside the managed root by any of the
        // rejected calls (the form check fires BEFORE fs access).
        let leaked = dir.path().join("etc");
        assert!(!leaked.exists(), "no traversal side effects");
        assert!(!std::env::temp_dir().join("evil.rimloc.json").exists());
    }

    /// A VALID minted id with no managed file still yields the honest
    /// ProjectNotFound — the form guard never shadows real lookups.
    #[test]
    fn valid_form_without_file_is_project_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let id = format!("proj-{:x}", 1234567);
        assert!(id.starts_with("proj-"));
        let err = mgr.open(&id).unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ProjectNotFound);
    }
    /// Validate wave: a well-formed brace placeholder is a SUCCESS with an
    /// Info finding (026: info is successful and reported).
    #[test]
    fn validate_success_with_info_finding() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "Урон {0} единиц")],
        ))
        .unwrap();

        let res = mgr
            .validate_project(&snap.project_id, 1, Some("Russian"))
            .unwrap();
        assert_eq!(res.status, "succeeded", "{res:?}");
        assert_eq!(res.error_count, 0);
        assert!(
            res.findings
                .iter()
                .any(|f| f.severity == "info" && f.kind == "placeholder-check"),
            "{res:?}"
        );
        // The finding carries the FULL structural identity.
        let info = res
            .findings
            .iter()
            .find(|f| f.kind == "placeholder-check")
            .unwrap();
        assert_eq!(
            info.id.as_ref().unwrap().def_type.as_deref(),
            Some("ThingDef")
        );
    }

    /// Validate wave: a LOST source placeholder (broken %) is an
    /// Error-severity finding → the operation FAILS and becomes the
    /// diagnostics target.
    #[test]
    fn validate_lost_placeholder_fails_operation() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        write(
            &mod_root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>Damage {0} done</label></ThingDef></Defs>"#,
        );
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // "Translation" that drops the source placeholder and adds a broken %.
        let res = mgr
            .validate_project(&snap.project_id, 1, Some("Russian"))
            .unwrap();
        // No translations yet → success; apply a broken one first.
        assert_eq!(res.status, "succeeded");
        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "Урон 50% единиц")],
        ))
        .unwrap();

        let res = mgr
            .validate_project(&snap.project_id, 1, Some("Russian"))
            .unwrap();
        assert_eq!(res.status, "failed", "{res:?}");
        assert!(res.error_count >= 1);
        assert!(res
            .findings
            .iter()
            .any(|f| f.severity == "error" && f.key == "Dup.label"));
        // The failed operation is now the diagnostics target.
        let diag_dir = dir.path().join("bundle");
        let diag = mgr.diagnose(&snap.project_id, &diag_dir).unwrap();
        assert!(diag.operation_id.starts_with("op-"));
        assert!(!diag.files.is_empty());
        assert!(diag_dir.exists());
    }

    /// P2-3 regression: colliding serialization keys across def types —
    /// findings carry the identity of the entry the finding is actually
    /// ABOUT, not the first entry that happens to share the key.
    #[test]
    fn findings_carry_structural_identity_across_def_types() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // Well-formed placeholder on ThingDef, broken % on AbilityDef —
        // both share the serialization key `Dup.label`.
        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![
                set_text("Dup.label", "ThingDef", "Урон {0} единиц"),
                set_text("Dup.label", "AbilityDef", "Сломано 50% единиц"),
            ],
        ))
        .unwrap();

        let res = mgr
            .validate_project(&snap.project_id, 1, Some("Russian"))
            .unwrap();
        assert_eq!(res.status, "failed", "{res:?}");
        assert!(
            res.findings.iter().any(|f| f.severity == "error"),
            "{res:?}"
        );
        let error = res.findings.iter().find(|f| f.severity == "error").unwrap();
        assert_eq!(
            error.id.as_ref().map(|i| i.def_type.as_deref()),
            Some(Some("AbilityDef")),
            "the broken % belongs to the AbilityDef entry, {res:?}"
        );
        assert!(res.findings.iter().any(|f| f.severity == "info"), "{res:?}");
        let info = res.findings.iter().find(|f| f.severity == "info").unwrap();
        assert_eq!(
            info.id.as_ref().map(|i| i.def_type.as_deref()),
            Some(Some("ThingDef")),
            "the well-formed brace placeholder belongs to the ThingDef entry, {res:?}"
        );
    }

    /// Locale-scoped validation surface: one identity translated into two
    /// locales is ONE def in two language folders — never a duplicate.
    #[test]
    fn same_identity_in_two_locales_is_not_a_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let mut ru = set_text("Dup.label", "ThingDef", "вещь");
        ru.locale = "Russian".into();
        let mut pl = set_text("Dup.label", "ThingDef", "rzecz");
        pl.locale = "Polish".into();
        mgr.apply(&req(&snap.project_id, 1, 1, vec![ru, pl]))
            .unwrap();

        let res = mgr.validate_project(&snap.project_id, 1, None).unwrap();
        assert_eq!(res.status, "succeeded", "{res:?}");
        assert!(
            !res.findings
                .iter()
                .any(|f| f.kind == "duplicate" || f.kind == "duplicate-global"),
            "{res:?}"
        );
    }

    /// Build wave: successful export into an ISOLATED dir, reparse-verified
    /// (counters match), skipped-unknown surfaced.
    #[test]
    fn export_build_is_reparse_verified() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "вещь")],
        ))
        .unwrap();

        let out = dir.path().join("export-out");
        let res = mgr
            .export_project(&snap.project_id, 1, &out, "Russian")
            .unwrap();
        assert!(res.files_written >= 1);
        assert_eq!(res.reparsed_keys, 1, "one translated key reparsed");
        assert!(res.skipped_unknown_type.is_empty());
        assert!(out
            .join("Languages/Russian/DefInjected/ThingDef/Dup.xml")
            .exists());
    }

    /// Build wave guard: an out dir inside the read-only source tree is a
    /// typed guard_output_denied rejection.
    #[test]
    fn export_into_source_tree_is_denied() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        mgr.create(&mod_root, Some("1.6")).unwrap();

        let out_inside = mod_root.join("Translation");
        let err = mgr
            .export_project(&snap_of_create(&mgr, &mod_root), 1, &out_inside, "Russian")
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::GuardOutputDenied);
    }

    /// P1-2 security regression: client locales are joined into
    /// `Languages/<locale>/...` output paths — traversal, absolute,
    /// backslash and unicode-slash shapes are a typed rejection on EVERY
    /// entry that takes a locale for writing (export) or persists one
    /// (apply), with no filesystem side effects and no poison at rest.
    #[test]
    fn path_traversal_locales_are_rejected_on_every_write_entry() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let evil = [
            "../../evil",
            "/abs/evil",
            "C:\\evil",
            "ru/../../evil",
            "ru\\..\\evil",
            "ru\u{2044}evil", // FRACTION SLASH
            "ru\u{2215}evil", // DIVISION SLASH
            "..",
            ".",
            "",
        ];
        for (idx, loc) in evil.iter().enumerate() {
            // Export entry: refused before the out dir is even created.
            let out = dir.path().join(format!("never-{idx}"));
            let err = mgr
                .export_project(&snap.project_id, 1, &out, loc)
                .unwrap_err();
            assert_eq!(
                err.code,
                ContractErrorCode::ContractViolation,
                "[{loc}] {err}"
            );
            assert!(!out.exists(), "[{loc}] no write side effects");

            // Apply entry: the intent is skipped as data, nothing persists
            // (the poisoned locale never reaches the durable record).
            let mut intent = set_text("Dup.label", "ThingDef", "x");
            intent.locale = (*loc).to_string();
            let res = mgr
                .apply(&req(&snap.project_id, 1, 1, vec![intent]))
                .unwrap();
            assert_eq!(res.applied, 0, "[{loc}]");
            assert_eq!(res.skipped.len(), 1, "[{loc}]");
            assert_eq!(
                res.skipped[0].code,
                ContractErrorCode::ContractViolation,
                "[{loc}]"
            );
        }
        // No translation was persisted under any poisoned locale.
        let loaded = load_project_with_meta(&mgr.managed_path(&snap.project_id).unwrap()).unwrap();
        assert!(loaded.project.translations.is_empty());
    }

    /// P2-2/H5 fail-closed: a LEGACY envelope (pre-source_root field) has
    /// no durable source root — export and diagnose refuse instead of
    /// running with the read-only source-tree guard disabled.
    #[test]
    fn legacy_envelope_without_source_root_refuses_export_and_diagnose() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // Rewrite the record as a LEGACY envelope: no source_root field.
        let path = mgr.managed_path(&snap.project_id).unwrap();
        let loaded = load_project_with_meta(&path).unwrap();
        let legacy = ProjectEnvelopeMeta {
            project_id: loaded.meta.project_id.clone(),
            revision: loaded.meta.revision,
            display_name: loaded.meta.display_name.clone(),
            source_root: None,
            source_fingerprint: None,
        };
        save_project_with_meta(&loaded.project, &legacy, &path).unwrap();

        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();

        let out = dir.path().join("export-out");
        let err = mgr2
            .export_project(
                &reopened.project_id,
                reopened.session_epoch,
                &out,
                "Russian",
            )
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::GuardOutputDenied, "{err}");
        assert!(!out.exists());
        let err = mgr2
            .diagnose(&reopened.project_id, &dir.path().join("bundle"))
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::GuardOutputDenied, "{err}");
        assert!(!dir.path().join("bundle").exists());
    }

    /// H5: the source root is durable envelope state — create → save →
    /// reopen (fresh manager = app restart) restores the full context, so
    /// export works and the close-open-build cycle completes. refresh
    /// keeps the root as well.
    #[test]
    fn reopen_restores_source_root_and_export_works() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "вещь")],
        ))
        .unwrap();

        // "App restart": a fresh manager recovers from the envelope.
        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();

        let out = dir.path().join("export-out");
        let res = mgr2
            .export_project(
                &reopened.project_id,
                reopened.session_epoch,
                &out,
                "Russian",
            )
            .unwrap();
        assert_eq!(res.reparsed_keys, 1);
        assert!(out
            .join("Languages/Russian/DefInjected/ThingDef/Dup.xml")
            .exists());

        // refresh adopts the durable root: export still works afterwards.
        let refreshed = mgr2.refresh(&reopened.project_id).unwrap();
        let out2 = dir.path().join("export-out-2");
        mgr2.export_project(
            &refreshed.project_id,
            refreshed.session_epoch,
            &out2,
            "Russian",
        )
        .unwrap();
    }

    // ------------------------------------------------------------------
    // M3: source-drift detection (fingerprint recorded in the envelope,
    // verdict evaluated at session (re)start, surfaced on snapshot and
    // validate).
    // ------------------------------------------------------------------

    /// M3: the source the inventory was built from changed between
    /// sessions (edit + added file, harness "sourcedrift") — a fresh
    /// manager's open honestly marks the drift, and validate carries the
    /// source-drift finding (a warning: the project still validates
    /// against its own inventory).
    #[test]
    fn source_drift_detected_on_reopen_and_validate() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        assert_eq!(snap.source_changed, Some(false));

        // Source rewrite: edit one Def's text, add another.
        write(
            &mod_root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>CHANGED label</label></ThingDef></Defs>"#,
        );
        write(
            &mod_root.join("Defs/C_New.xml"),
            r#"<Defs><ThingDef><defName>NewThing</defName><label>new</label></ThingDef></Defs>"#,
        );

        // "App restart": a fresh manager reopens and re-evaluates.
        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();
        assert_eq!(reopened.source_changed, Some(true));

        let v = mgr2
            .validate_project(&reopened.project_id, reopened.session_epoch, None)
            .unwrap();
        assert_eq!(v.status, "succeeded", "drift is a warning, not a failure");
        assert!(v.findings.iter().any(|f| f.kind == "source-drift"), "{v:?}");
        assert_eq!(v.warning_count, 1);
    }

    /// M3: unchanged source stays in sync across restart and refresh —
    /// no drift finding on validate.
    #[test]
    fn no_drift_when_source_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "метка")],
        ))
        .unwrap();

        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();
        assert_eq!(reopened.source_changed, Some(false));
        let v = mgr2
            .validate_project(&reopened.project_id, reopened.session_epoch, None)
            .unwrap();
        assert!(
            !v.findings.iter().any(|f| f.kind == "source-drift"),
            "{v:?}"
        );

        // refresh is a session (re)start too — still in sync.
        let refreshed = mgr2.refresh(&reopened.project_id).unwrap();
        assert_eq!(refreshed.source_changed, Some(false));
    }

    /// M3: refresh re-evaluates the verdict without a fresh manager — a
    /// source edit then refresh flips it to drifted.
    #[test]
    fn refresh_reevaluates_drift() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        assert_eq!(snap.source_changed, Some(false));

        write(
            &mod_root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>REWRITTEN</label></ThingDef></Defs>"#,
        );
        let refreshed = mgr.refresh(&snap.project_id).unwrap();
        assert_eq!(refreshed.source_changed, Some(true));
    }

    /// M3: an applied edit does NOT rewrite the recorded fingerprint —
    /// the envelope keeps describing the SOURCE, not the session.
    #[test]
    fn apply_keeps_recorded_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let path = mgr.managed_path(&snap.project_id).unwrap();
        let recorded = load_project_with_meta(&path)
            .unwrap()
            .meta
            .source_fingerprint;
        assert!(recorded.is_some(), "create records the fingerprint");

        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "метка")],
        ))
        .unwrap();
        let after = load_project_with_meta(&path)
            .unwrap()
            .meta
            .source_fingerprint;
        assert_eq!(after, recorded);
    }

    /// M3: a legacy envelope (no recorded fingerprint) stays UNKNOWN —
    /// `None`, never a false "in sync" — and validate carries no drift
    /// finding for it.
    #[test]
    fn legacy_envelope_drift_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let path = mgr.managed_path(&snap.project_id).unwrap();
        let loaded = load_project_with_meta(&path).unwrap();
        let legacy = ProjectEnvelopeMeta {
            project_id: loaded.meta.project_id.clone(),
            revision: loaded.meta.revision,
            display_name: loaded.meta.display_name.clone(),
            source_root: Some(mod_root.to_string_lossy().into_owned()),
            source_fingerprint: None,
        };
        save_project_with_meta(&loaded.project, &legacy, &path).unwrap();

        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();
        assert_eq!(reopened.source_changed, None);
        let v = mgr2
            .validate_project(&reopened.project_id, reopened.session_epoch, None)
            .unwrap();
        assert!(
            !v.findings.iter().any(|f| f.kind == "source-drift"),
            "{v:?}"
        );
    }

    /// M3: LoadFolders rollback (1.6 → 1.5-only manifest) between
    /// sessions is visible drift at reopen (harness "cases" shape).
    #[test]
    fn loadfolders_rollback_is_drift_on_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        write(
            &mod_root.join("LoadFolders.xml"),
            "<loadFolders><v1.6><li>/</li><li>Common16</li></v1.6>\
             <v1.5><li>/</li><li>Common15</li></v1.5></loadFolders>",
        );
        write(
            &mod_root.join("Defs/A.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>label</label></ThingDef></Defs>"#,
        );
        write(
            &mod_root.join("Common16/Languages/Russian/Keyed/K.xml"),
            "<LanguageData><K1>16</K1></LanguageData>",
        );
        write(
            &mod_root.join("Common15/Languages/Russian/Keyed/K.xml"),
            "<LanguageData><K1>15</K1></LanguageData>",
        );
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // Roll the manifest back to 1.5-only.
        write(
            &mod_root.join("LoadFolders.xml"),
            "<loadFolders><v1.5><li>/</li><li>Common15</li></v1.5></loadFolders>",
        );

        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();
        assert_eq!(reopened.source_changed, Some(true));
    }

    /// The managed-projects root is a protected write target too: export
    /// artifacts never land next to the durable records.
    #[test]
    fn export_into_managed_root_is_denied() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let err = mgr
            .export_project(&snap.project_id, 1, &managed.join("out"), "Russian")
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::GuardOutputDenied);
    }

    /// P2-4 (backend half): the snapshot exposes the dirty flag and the
    /// last acked revision, so the client can GATE refresh (which discards
    /// dirty edits) and retry applies against a legal base instead of a
    /// revision the next apply would reject.
    #[test]
    fn snapshot_exposes_dirty_state_and_acked_revision() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        assert!(!snap.dirty);
        assert_eq!(snap.acked_revision, snap.revision);

        // Capture a pristine copy for the later refresh, then break the
        // durable write → the applied state is dirty in memory only.
        let file_path = mgr.managed_path(&snap.project_id).unwrap();
        let pristine = load_project_with_meta(&file_path).unwrap();
        fs::remove_file(&file_path).unwrap();
        fs::create_dir(&file_path).unwrap();
        let err = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![set_text("Dup.label", "ThingDef", "метка")],
            ))
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::SaveFailed);

        let dirty = mgr.snapshot(&snap.project_id).unwrap();
        assert!(dirty.dirty, "applied-but-unacked edits must be visible");
        assert_eq!(dirty.revision, 2);
        assert_eq!(dirty.acked_revision, 1, "nothing was acked");

        // A good file returns to disk; refresh adopts it: dirty clears and
        // both revisions agree again.
        fs::remove_dir(&file_path).unwrap();
        save_project_with_meta(&pristine.project, &pristine.meta, &file_path).unwrap();
        let refreshed = mgr.refresh(&snap.project_id).unwrap();
        assert!(!refreshed.dirty);
        assert_eq!(refreshed.revision, refreshed.acked_revision);
    }

    /// H3: control characters are invalid in XML 1.0 and cannot be
    /// escaped — a text carrying one is refused at emission (per-intent
    /// skip, nothing persists).
    #[test]
    fn control_char_text_is_refused_at_emission() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let mut intent = set_text("Dup.label", "ThingDef", "плохой\u{7}текст\u{c}конец");
        let res = mgr
            .apply(&req(&snap.project_id, 1, 1, vec![intent.clone()]))
            .unwrap();
        assert_eq!(res.applied, 0);
        assert_eq!(res.skipped.len(), 1);
        assert_eq!(res.skipped[0].code, ContractErrorCode::ContractViolation);
        assert_eq!(res.revision, 1, "nothing acked");

        // The clean twin of the same batch applies normally (the guard is
        // character-scoped, not batch-scoped).
        intent.text = Some("чистый текст".into());
        let res = mgr
            .apply(&req(&snap.project_id, 1, 1, vec![intent]))
            .unwrap();
        assert_eq!(res.applied, 1);
    }

    /// H3 export refusal: a poisoned durable record (legacy state written
    /// before the emission guard) cannot export — the failure names the
    /// identity and no directory is written.
    #[test]
    fn export_refuses_control_char_content_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // Poison the durable record directly (simulating pre-guard state):
        // a translation carrying raw 0x07.
        let path = mgr.managed_path(&snap.project_id).unwrap();
        let mut loaded = load_project_with_meta(&path).unwrap();
        let entry_id = loaded.project.entries[0].id.clone();
        loaded.project.translations.push(Translation {
            source_id: entry_id,
            locale: "Russian".into(),
            text: Some("плохой\u{7}текст".into()),
            completeness: Completeness::Translated,
            review: rimloc_domain::canonical::Review::None,
            validation: ValidationState::Ok,
            lifecycle: rimloc_domain::canonical::Lifecycle::Active,
            origin: Origin::Imported,
            notes: String::new(),
            source_changed: None,
        });
        save_project_with_meta(&loaded.project, &loaded.meta, &path).unwrap();

        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let reopened = mgr2.open(&snap.project_id).unwrap();
        let out = dir.path().join("never-export");
        let err = mgr2
            .export_project(
                &reopened.project_id,
                reopened.session_epoch,
                &out,
                "Russian",
            )
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation, "{err}");
        assert!(err.message.contains("invalid in XML 1.0"), "{err}");
        assert!(!out.exists(), "nothing was written");
    }

    fn snap_of_create(mgr: &ProjectSessionManager, mod_root: &Path) -> ProjectId {
        mgr.create(mod_root, Some("1.6")).unwrap().project_id
    }
    /// H4: About.xml is valid XML even for hostile folder names, and the
    /// packageId is a RimWorld-shaped slug, never the raw folder name.
    #[test]
    fn export_about_xml_is_escaped_and_package_id_is_a_slug() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("My Mod & <Test>");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        mgr.apply(&req(
            &snap.project_id,
            1,
            1,
            vec![set_text("Dup.label", "ThingDef", "вещь")],
        ))
        .unwrap();

        let out = dir.path().join("export-out");
        let res = mgr
            .export_project(&snap.project_id, 1, &out, "Russian")
            .unwrap();
        assert_eq!(res.reparsed_keys, 1);

        let about = std::fs::read_to_string(out.join("About/About.xml")).unwrap();
        assert!(about.contains("My Mod &amp; &lt;Test&gt;"), "{about}");
        assert!(
            !about.contains("My Mod & <Test>"),
            "raw special chars must not survive: {about}"
        );
        let id_start = about.find("<packageId>").unwrap() + "<packageId>".len();
        let id_end = about[id_start..].find("</packageId>").unwrap() + id_start;
        let package_id = &about[id_start..id_end];
        assert_eq!(package_id, "rimloc.my-mod-test", "{about}");
    }

    /// M2: a corrupt managed file is reported by list_report (typed
    /// reason) instead of silently disappearing; list() still returns
    /// only loadable projects; reopen of the corrupt file stays a typed
    /// failure with intact bytes.
    #[test]
    fn corrupt_project_file_is_surfaced_in_list_report() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let managed = dir.path().join("managed");
        let mgr = ProjectSessionManager::new(&managed).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        // Corrupt the durable record on disk (simulated disk fault).
        let path = mgr.managed_path(&snap.project_id).unwrap();
        let bytes = fs::read(&path).unwrap();
        let cut = bytes.len() * 6 / 10;
        fs::write(&path, &bytes[..cut]).unwrap();

        // A FRESH manager (restart) scans only disk — the corrupt file is
        // now explicitly reported. (The live session legitimately shadows
        // the disk with its in-memory state.)
        let mgr2 = ProjectSessionManager::new(&managed).unwrap();
        let report = mgr2.list_report();
        assert!(report.projects.is_empty(), "{report:?}");
        assert_eq!(report.unloadable.len(), 1, "{report:?}");
        assert_eq!(report.unloadable[0].project_id, snap.project_id);
        assert_eq!(report.unloadable[0].reason, "corrupt_project");

        // list() keeps its shape (only loadable projects)...
        assert!(mgr2.list().is_empty());
        // ...and a raw bytes view remains possible: the file still exists.
        assert!(path.is_file());
    }

    /// M4: defNames differing only by case (`Dup`/`dup`, same def type)
    /// collide into ONE DefInjected output file on case-insensitive
    /// filesystems. Export refuses deterministically on EVERY filesystem
    /// (the old reparse guard only caught this on such a filesystem, and on
    /// a case-sensitive one the collision shipped to players); validate
    /// surfaces the pair early as a warning finding.
    #[test]
    fn export_refuses_defname_case_collisions_and_validate_warns() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        case_collision_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![
                    set_text("Dup.label", "ThingDef", "вещь А"),
                    set_text("dup.label", "ThingDef", "вещь Б"),
                ],
            ))
            .unwrap();
        assert_eq!(res.applied, 2, "both case-distinct identities must exist");

        // Validate: honest early signal — warning, does not fail the run.
        let resp = mgr.validate_project(&snap.project_id, 1, None).unwrap();
        let hit = resp
            .findings
            .iter()
            .find(|f| f.kind == "case-collision")
            .expect("case-collision finding expected");
        assert_eq!(hit.severity, "warning");
        assert_eq!(hit.path, "DefInjected/ThingDef");
        assert!(
            hit.message.contains("Dup") && hit.message.contains("dup"),
            "{}",
            hit.message
        );

        // Export: typed refusal naming both spellings — never a silent
        // one-file overwrite, never a shipped collision.
        let out = dir.path().join("out");
        let err = mgr
            .export_project(&snap.project_id, 1, &out, "Russian")
            .unwrap_err();
        assert_eq!(err.code, ContractErrorCode::ContractViolation);
        let msg = err.to_string();
        assert!(msg.contains("case"), "{msg}");
        assert!(msg.contains("Dup") && msg.contains("dup"), "{msg}");
        assert!(
            !out.join("Languages").exists(),
            "nothing written on refusal"
        );

        // A cross-locale pair (same spelling pair, different locales) never
        // shares one export file — no finding when the locales differ.
    }

    /// M4 negative: the SAME defName in DIFFERENT def types is legitimate
    /// (separate output directories) — no collision, export proceeds.
    #[test]
    fn same_defname_across_def_types_is_not_a_collision() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        two_types_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();

        let res = mgr
            .apply(&req(
                &snap.project_id,
                1,
                1,
                vec![
                    set_text("Dup.label", "ThingDef", "вещь"),
                    set_text("Dup.label", "AbilityDef", "способность"),
                ],
            ))
            .unwrap();
        assert_eq!(res.applied, 2);

        let resp = mgr.validate_project(&snap.project_id, 1, None).unwrap();
        assert!(
            !resp.findings.iter().any(|f| f.kind == "case-collision"),
            "cross-def-type same-case names are not collisions"
        );

        let out = dir.path().join("out");
        let export = mgr
            .export_project(&snap.project_id, 1, &out, "Russian")
            .unwrap();
        assert_eq!(export.files_written >= 3, true);
    }
}
