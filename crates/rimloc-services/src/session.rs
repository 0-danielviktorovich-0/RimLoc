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
    TranslationIntent,
};
use crate::observability::{generate_operation_id, sha256_hex, OperationLog};
use crate::project::build_project;
use crate::project_store::{load_project_with_meta, save_project_with_meta, ProjectEnvelopeMeta};
use rimloc_domain::canonical::{Completeness, Origin, Project, Translation, ValidationState};
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
    /// diagnostics/rescan context; NEVER written). Unused in this slice —
    /// the rescan flow of a later slice consumes it.
    #[allow(dead_code)]
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
        if !crate::util::is_within(&path, &self.managed_root) {
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
                let state = SessionState {
                    path: path.clone(),
                    display_name,
                    mod_root: PathBuf::new(),
                    target_version,
                    epoch: 1,
                    revision,
                    acked_revision: revision,
                    project: loaded.project,
                    disk_hash: disk_hash(&path),
                    dirty: false,
                    cancel_requested: false,
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
    pub fn list(&self) -> Vec<ProjectSummary> {
        let mut out: BTreeMap<ProjectId, ProjectSummary> = BTreeMap::new();
        if let Ok(entries) = std::fs::read_dir(&self.managed_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some(MANAGED_EXT) {
                    continue;
                }
                let Some(id) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                    continue;
                };
                if let Ok(loaded) = load_project_with_meta(&path) {
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
        out.into_values().collect()
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
            st.target_version = target_version;
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
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// sha256 of the managed file (None when absent).
fn disk_hash(path: &Path) -> Option<String> {
    std::fs::read(path).ok().map(|b| sha256_hex(&b))
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
        project: st.project.clone(),
    }
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
        let external = load_project_with_meta(&path).unwrap().project;
        let meta = ProjectEnvelopeMeta {
            project_id: Some(snap.project_id.clone()),
            revision: Some(9),
            display_name: Some("external".into()),
        };
        save_project_with_meta(&external, &meta, &path).unwrap();

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
}
