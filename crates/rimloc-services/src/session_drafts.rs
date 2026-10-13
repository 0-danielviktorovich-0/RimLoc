//! Durable draft-autosave store (Data Safety mandate) — unsaved editor work
//! survives a crash, a kill or a lost session, and NEVER leaks into the
//! confirmed translation state.
//!
//! Separation of concerns (the core of the mandate):
//! - confirmed translations live ONLY in the managed project envelope
//!   (`<id>.rimloc.json`, revisioned, origin/completeness stamped by the
//!   session) — autosave drafts never touch it: no `Project` mutation, no
//!   revision bump, no provenance write happens on any path below;
//! - drafts live in a SEPARATE sidecar `<id>.drafts.json` next to the
//!   managed file in the managed root:
//!   `{ schema_version, project_revision, entries: {key: {text, updated_at}}, updated_at }`;
//! - writes are atomic (`write_atomic`, tmp-rename) with a bounded rotation
//!   of up to [`DRAFTS_MAX_BACKUPS`] backups (`.drafts.1.json` .. `.N.json`);
//!   rotation happens AFTER a successful new write, so old data is only ever
//!   removed once the new data is durably in place;
//! - restore is honest: a store whose `project_revision` no longer matches
//!   the open session is quarantined as `.drafts.orphan.json` and REPORTED,
//!   never silently applied or silently dropped; a corrupt live file is
//!   recovered from the newest intact backup and the recovery is reported.
//!
//! The manager ops attach to [`ProjectSessionManager`] from this module (the
//! `chat_batch` precedent): every op is epoch-guarded against the LIVE
//! session and stamps the live revision — the service, not the client, owns
//! the durable coordinates.

use crate::contract::{ContractError, ContractErrorCode, PathBufDto, Revision, SessionEpoch};
use crate::session::ProjectSessionManager;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Current sidecar schema. A file with a different version is treated as
/// unreadable (recovery from backups, honest unavailability report) — never
/// parsed with wrong-shape assumptions.
pub const DRAFTS_SCHEMA_VERSION: u32 = 1;

/// Backups kept next to the live file: `.drafts.1.json` (the previous save)
/// … `.drafts.{N}.json` (the oldest). The chain is capped — rotation shifts
/// it on every save, so the store can never grow without bound.
pub const DRAFTS_MAX_BACKUPS: usize = 3;

/// Sidecar file suffix: the managed file `<id>.rimloc.json` gains its
/// sibling `<id>.drafts.json` in the same managed root.
pub const DRAFTS_SUFFIX: &str = "drafts.json";

/// One draft entry: the editor text plus the wall-clock stamp of the last
/// autosave (unix ms). No status, no provenance — a draft is ONLY text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftEntry {
    pub text: String,
    pub updated_at: u64,
}

impl DraftEntry {
    pub fn new(text: impl Into<String>, updated_at: u64) -> Self {
        Self {
            text: text.into(),
            updated_at,
        }
    }
}

/// The durable drafts sidecar document. Store keys are FRONTEND-owned and
/// opaque to this service: the current UI convention scopes every draft to
/// its target locale and the FULL structural identity —
/// `<registry-locale>::<display_identity()>` (e.g.
/// `ru::def_injected·ThingDef·Dup.label`) — so parallel target locales keep
/// parallel unsaved slices and a locale switch cannot erase another slice.
/// Legacy v1-era files hold bare display keys; the UI re-anchors them on
/// restore (never destroys). Save/restore/clear treat key syntax as opaque
/// strings — the service never parses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftsFile {
    pub schema_version: u32,
    /// Project revision the drafts were based on (stamped by the service
    /// from the LIVE session at save time). Restore compares it against the
    /// session's current revision; a mismatch quarantines the file.
    pub project_revision: Revision,
    pub entries: BTreeMap<String, DraftEntry>,
    pub updated_at: u64,
}

impl DraftsFile {
    pub fn new(project_revision: Revision, entries: BTreeMap<String, DraftEntry>) -> Self {
        Self {
            schema_version: DRAFTS_SCHEMA_VERSION,
            project_revision,
            entries,
            updated_at: crate::providers::now_ms(),
        }
    }
}

/// Typed store failure (parse/IO). Converted into a [`ContractError`] only
/// where a refusal is the honest outcome (save); the restore path carries
/// the detail in its REPORT instead — drafts are auxiliary state and must
/// never block opening a project.
#[derive(Debug)]
pub enum DraftsStoreError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Corrupt {
        path: PathBuf,
        detail: String,
    },
    SchemaVersion {
        path: PathBuf,
        found: u32,
        expected: u32,
    },
}

impl fmt::Display for DraftsStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DraftsStoreError::Io { path, source } => {
                write!(f, "drafts store IO error at `{}`: {source}", path.display())
            }
            DraftsStoreError::Corrupt { path, detail } => {
                write!(
                    f,
                    "drafts store file is corrupt at `{}`: {detail}",
                    path.display()
                )
            }
            DraftsStoreError::SchemaVersion {
                path,
                found,
                expected,
            } => write!(
                f,
                "drafts store schema version {found} at `{}` is unsupported (expected {expected})",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DraftsStoreError {}

impl From<DraftsStoreError> for ContractError {
    fn from(e: DraftsStoreError) -> Self {
        let code = match &e {
            DraftsStoreError::Io { .. } => ContractErrorCode::SaveFailed,
            DraftsStoreError::Corrupt { .. } | DraftsStoreError::SchemaVersion { .. } => {
                ContractErrorCode::Internal
            }
        };
        ContractError::new(code, e.to_string())
    }
}

/// Outcome of loading the drafts store (with backup recovery).
#[derive(Debug, Clone)]
pub enum DraftsLoadOutcome {
    /// No live file and nothing recoverable.
    Absent,
    /// Loaded from the live file, or — after the live file proved unreadable
    /// — from the newest intact backup (`recovered_from` names it).
    Loaded {
        drafts: DraftsFile,
        recovered_from: Option<PathBuf>,
    },
}

/// Sibling path of the managed file: `<id>.drafts.json`.
pub fn drafts_file_path(managed_project_file: &Path, project_id: &str) -> PathBuf {
    managed_project_file.with_file_name(format!("{project_id}.{DRAFTS_SUFFIX}"))
}

/// `.drafts.<marker>.json` next to the live store (`1`/`2`/`3` backups,
/// `orphan` quarantine).
fn suffixed(main: &Path, marker: &str) -> PathBuf {
    let file = main
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| DRAFTS_SUFFIX.to_string());
    let stemmed = file.strip_suffix(".json").unwrap_or(file.as_str());
    main.with_file_name(format!("{stemmed}.{marker}.json"))
}

fn parse_drafts(path: &Path, bytes: &[u8]) -> Result<DraftsFile, DraftsStoreError> {
    let file: DraftsFile =
        serde_json::from_slice(bytes).map_err(|e| DraftsStoreError::Corrupt {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })?;
    if file.schema_version != DRAFTS_SCHEMA_VERSION {
        return Err(DraftsStoreError::SchemaVersion {
            path: path.to_path_buf(),
            found: file.schema_version,
            expected: DRAFTS_SCHEMA_VERSION,
        });
    }
    Ok(file)
}

/// Load the store with backup recovery: the live file first, then the backup
/// chain newest-first (`.1` is the previous save). A corrupt live file never
/// wins over an intact backup — the first readable source returns, and
/// `recovered_from` reports the deviation honestly.
pub fn load_drafts(main: &Path) -> Result<DraftsLoadOutcome, DraftsStoreError> {
    let mut candidates = vec![main.to_path_buf()];
    for n in 1..=DRAFTS_MAX_BACKUPS {
        candidates.push(suffixed(main, &n.to_string()));
    }
    let mut first_error: Option<DraftsStoreError> = None;
    for candidate in candidates {
        let bytes = match std::fs::read(&candidate) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(DraftsStoreError::Io {
                    path: candidate,
                    source: e,
                });
            }
        };
        match parse_drafts(&candidate, &bytes) {
            Ok(drafts) => {
                return Ok(DraftsLoadOutcome::Loaded {
                    drafts,
                    recovered_from: (candidate != main).then_some(candidate),
                });
            }
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }
    match first_error {
        Some(e) => Err(e),
        None => Ok(DraftsLoadOutcome::Absent),
    }
}

/// Atomic save with bounded rotation. Order guarantees:
/// 1. the NEW content lands atomically first (`write_atomic`, tmp-rename) —
///    on failure nothing else is touched and the error propagates;
/// 2. only then the previous live content becomes `.1` and the backup chain
///    shifts down — the oldest backup (`.3`) is removed at this point, i.e.
///    strictly AFTER the new write succeeded.
///
/// Individual rotation steps are best-effort: the durable truth is the new
/// live file; a mid-shift failure can at worst duplicate a backup, which the
/// newest-first recovery tolerates.
pub fn save_drafts(main: &Path, file: &DraftsFile) -> Result<(), DraftsStoreError> {
    // Never overwrite bytes we could not read (review iteration 6 P1): a
    // PermissionDenied read followed by a directory-writable rename would
    // destroy the previous store WITHOUT a backup. NotFound is the normal
    // fresh-store case; any other read error refuses the save.
    let previous = match std::fs::read(main) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(DraftsStoreError::Io {
                path: main.to_path_buf(),
                source,
            })
        }
    };
    let bytes = serde_json::to_vec_pretty(file).map_err(|e| DraftsStoreError::Corrupt {
        path: main.to_path_buf(),
        detail: format!("serialize failed: {e}"),
    })?;
    crate::util::write_atomic(main, &bytes).map_err(|source| DraftsStoreError::Io {
        path: main.to_path_buf(),
        source,
    })?;
    if let Some(previous) = previous {
        let _ = std::fs::remove_file(suffixed(main, &DRAFTS_MAX_BACKUPS.to_string()));
        for n in (1..DRAFTS_MAX_BACKUPS).rev() {
            let _ = std::fs::rename(
                suffixed(main, &n.to_string()),
                suffixed(main, &(n + 1).to_string()),
            );
        }
        let _ = crate::util::write_atomic(&suffixed(main, "1"), &previous);
    }
    Ok(())
}

/// Quarantine a store whose revision no longer matches the open session:
/// the file is preserved verbatim as `.drafts.orphan.json` (atomic write)
/// and the live file is removed, so the next session starts clean and the
/// data stays inspectable. Idempotent: re-quarantining rewrites the same
/// orphan. Returns the orphan path, or `None` when there was nothing to
/// quarantine.
pub fn quarantine_orphan(main: &Path) -> Result<Option<PathBuf>, DraftsStoreError> {
    let bytes = match std::fs::read(main) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(DraftsStoreError::Io {
                path: main.to_path_buf(),
                source,
            });
        }
    };
    let orphan = suffixed(main, "orphan");
    crate::util::write_atomic(&orphan, &bytes).map_err(|source| DraftsStoreError::Io {
        path: orphan.clone(),
        source,
    })?;
    let _ = std::fs::remove_file(main);
    Ok(Some(orphan))
}

// ---------------------------------------------------------------------------
// Session-manager ops (additive attachment; epoch-guarded, revision-stamped)
// ---------------------------------------------------------------------------

/// Ack of `save_drafts` / `clear_drafts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftsSaveAck {
    /// Entries now durable in the store.
    pub saved: usize,
    /// The LIVE revision the store was stamped with (the service's, not the
    /// caller's).
    pub project_revision: Revision,
    pub updated_at: u64,
}

/// Status of `restore_drafts` — an honest report, never a silent apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftsRestoreStatus {
    /// No drafts store existed (or nothing was left in it).
    None,
    /// Drafts restored into the workspace (revision matched).
    Restored,
    /// The store's revision no longer matches the session — the file was
    /// preserved as `.drafts.orphan.json` and NOT applied.
    OrphanQuarantined,
    /// The store is unreadable beyond backups (or quarantining failed) —
    /// `detail` carries the typed store error verbatim.
    Unavailable,
}

/// What `restore_drafts` decided, with everything the UI needs to report it
/// honestly (restore banner, orphan notice, backup-recovery note).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftsRestoreReport {
    pub status: DraftsRestoreStatus,
    /// Restored entries (empty unless status is `restored`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub drafts: BTreeMap<String, DraftEntry>,
    pub restored_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_revision: Option<Revision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orphan_path: Option<PathBufDto>,
    /// Set when the live file was corrupt and a backup supplied the data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_from_backup: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

fn none_report() -> DraftsRestoreReport {
    DraftsRestoreReport {
        status: DraftsRestoreStatus::None,
        drafts: BTreeMap::new(),
        restored_count: 0,
        project_revision: None,
        orphan_path: None,
        recovered_from_backup: None,
        detail: None,
    }
}

impl ProjectSessionManager {
    /// The drafts sidecar path for a project: the same fail-closed id-form
    /// and containment discipline as [`ProjectSessionManager::managed_path`]
    /// (the sidecar is derived from the managed file, then re-checked).
    fn drafts_path(&self, project_id: &str) -> Result<PathBuf, ContractError> {
        let managed = self.managed_path(project_id)?;
        let path = drafts_file_path(&managed, project_id);
        if !crate::util::is_within_allow(&path, self.managed_root()) {
            return Err(ContractError::new(
                ContractErrorCode::GuardOutputDenied,
                "drafts store path escapes the managed root".to_string(),
            ));
        }
        Ok(path)
    }

    /// Shared epoch guard: the project must be OPEN in this manager and the
    /// caller must carry the live epoch. Returns the live revision — the
    /// only stamp authority for the store.
    fn drafts_guard(
        &self,
        project_id: &str,
        session_epoch: SessionEpoch,
    ) -> Result<(PathBuf, Revision), ContractError> {
        let path = self.drafts_path(project_id)?;
        let revision = self.with_live_project(project_id, |epoch, revision, _project| {
            if epoch != session_epoch {
                return Err(ContractError::stale_epoch(session_epoch, epoch));
            }
            Ok(revision)
        })?;
        Ok((path, revision))
    }

    /// Persist the workspace's unsaved editor texts (autosave). The store is
    /// stamped with the LIVE revision; the canonical project state is NOT
    /// touched — no revision bump, no origin/completeness write, no envelope
    /// save. A stale epoch is a typed refusal (`stale_epoch`).
    pub fn save_drafts_session(
        &self,
        project_id: &str,
        session_epoch: SessionEpoch,
        entries: BTreeMap<String, DraftEntry>,
    ) -> Result<DraftsSaveAck, ContractError> {
        let (path, revision) = self.drafts_guard(project_id, session_epoch)?;
        // MERGE semantics (review iteration 4 P0-B): the incoming map is
        // upserted INTO the existing store — a partial or raced write can
        // never drop keys it does not mention. Removals are EXPLICIT
        // (`clear_drafts_session` with a key, or the whole store). An
        // unreadable store starts fresh here; its bytes survive in the
        // rotation backup this save creates.
        let mut merged = match load_drafts(&path) {
            Ok(DraftsLoadOutcome::Loaded { drafts, .. }) => drafts.entries,
            _ => BTreeMap::new(),
        };
        for (key, entry) in entries {
            merged.insert(key, entry);
        }
        let saved = merged.len();
        let file = DraftsFile::new(revision, merged);
        let updated_at = file.updated_at;
        save_drafts(&path, &file)?;
        Ok(DraftsSaveAck {
            saved,
            project_revision: revision,
            updated_at,
        })
    }

    /// Restore unsaved work after (re)open. Decision table:
    /// - no store → `none` (a fresh session, honestly empty);
    /// - revision matches the open session → `restored` with the entries for
    ///   the workspace to raise (the UI shows "restored N unsaved edits");
    /// - revision mismatch → the file is quarantined as `.drafts.orphan.json`
    ///   and `orphan_quarantined` is reported — never silently applied;
    /// - unreadable beyond backups (or a failed quarantine) → `unavailable`
    ///   with the typed detail. Opening the project itself never fails
    ///   because of drafts.
    pub fn restore_drafts_session(
        &self,
        project_id: &str,
        session_epoch: SessionEpoch,
    ) -> Result<DraftsRestoreReport, ContractError> {
        let (path, revision) = self.drafts_guard(project_id, session_epoch)?;
        let loaded = match load_drafts(&path) {
            Ok(outcome) => outcome,
            Err(e) => {
                return Ok(DraftsRestoreReport {
                    status: DraftsRestoreStatus::Unavailable,
                    detail: Some(e.to_string()),
                    ..none_report()
                });
            }
        };
        let (drafts, recovered_from) = match loaded {
            DraftsLoadOutcome::Absent => return Ok(none_report()),
            DraftsLoadOutcome::Loaded {
                drafts,
                recovered_from,
            } => (drafts, recovered_from),
        };
        if drafts.entries.is_empty() {
            return Ok(none_report());
        }
        if drafts.project_revision != revision {
            let detail = format!(
                "drafts revision {} does not match the open session revision {revision}",
                drafts.project_revision
            );
            return match quarantine_orphan(&path) {
                Ok(Some(orphan)) => Ok(DraftsRestoreReport {
                    status: DraftsRestoreStatus::OrphanQuarantined,
                    project_revision: Some(drafts.project_revision),
                    orphan_path: Some(PathBufDto::new(orphan.to_string_lossy().into_owned())),
                    detail: Some(detail),
                    ..none_report()
                }),
                Ok(None) => Ok(none_report()),
                Err(e) => Ok(DraftsRestoreReport {
                    status: DraftsRestoreStatus::Unavailable,
                    project_revision: Some(drafts.project_revision),
                    detail: Some(format!("{detail}; quarantine failed: {e}")),
                    ..none_report()
                }),
            };
        }
        // Self-heal a recovered store: the corrupt live file is replaced by
        // the intact backup content. Best-effort — the report stands either
        // way, the workspace already holds the entries.
        if recovered_from.is_some() {
            let _ = save_drafts(&path, &drafts);
        }
        Ok(DraftsRestoreReport {
            status: DraftsRestoreStatus::Restored,
            restored_count: drafts.entries.len(),
            project_revision: Some(drafts.project_revision),
            recovered_from_backup: recovered_from.map(|p| p.to_string_lossy().into_owned()),
            drafts: drafts.entries,
            ..none_report()
        })
    }

    /// Remove one key (revert / commit ack) or the whole store (`None`) from
    /// the durable drafts. Removing an absent key/file is an honest no-op —
    /// the end state the caller asked for already holds.
    pub fn clear_drafts_session(
        &self,
        project_id: &str,
        session_epoch: SessionEpoch,
        key: Option<&str>,
    ) -> Result<DraftsSaveAck, ContractError> {
        let (path, revision) = self.drafts_guard(project_id, session_epoch)?;
        let loaded = match load_drafts(&path)? {
            DraftsLoadOutcome::Absent => {
                return Ok(DraftsSaveAck {
                    saved: 0,
                    project_revision: revision,
                    updated_at: crate::providers::now_ms(),
                });
            }
            DraftsLoadOutcome::Loaded { drafts, .. } => drafts,
        };
        let mut entries = loaded.entries;
        match key {
            Some(key) => {
                if entries.remove(key).is_none() {
                    return Ok(DraftsSaveAck {
                        saved: entries.len(),
                        project_revision: revision,
                        updated_at: crate::providers::now_ms(),
                    });
                }
            }
            None => entries.clear(),
        }
        let saved = entries.len();
        let file = DraftsFile::new(revision, entries);
        let updated_at = file.updated_at;
        save_drafts(&path, &file)?;
        Ok(DraftsSaveAck {
            saved,
            project_revision: revision,
            updated_at,
        })
    }
}

// ---------------------------------------------------------------------------
// Adversarial tests (Data Safety mandate): every destructive scenario runs
// against its OWN temp RIMLOC-equivalent managed root — real projects are
// never touched.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{ApplyIntentsRequest, IntentAction};
    use rimloc_domain::canonical::{EntryKind, SourceEntryId};

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    /// Minimal source mod fixture (same shape the session tests use).
    fn draft_mod(root: &Path) {
        write(
            &root.join("Defs/A_Thing.xml"),
            r#"<Defs><ThingDef><defName>Dup</defName><label>thing label</label></ThingDef></Defs>"#,
        );
    }

    fn entries(pairs: &[(&str, &str)]) -> BTreeMap<String, DraftEntry> {
        pairs
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    DraftEntry::new(*v, crate::providers::now_ms()),
                )
            })
            .collect()
    }

    fn drafts_files_in(dir: &Path, project_id: &str) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(&format!("{project_id}.drafts")))
            .collect();
        names.sort();
        names
    }

    // --- store primitives ---

    #[test]
    fn save_then_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        let file = DraftsFile::new(7, entries(&[("K.a", "черновик A"), ("K.b", "draft B")]));
        save_drafts(&main, &file).unwrap();
        match load_drafts(&main).unwrap() {
            DraftsLoadOutcome::Loaded {
                drafts,
                recovered_from,
            } => {
                assert_eq!(drafts, file);
                assert_eq!(recovered_from, None);
            }
            other => assert!(
                matches!(other, DraftsLoadOutcome::Loaded { .. }),
                "expected Loaded, got {other:?}"
            ),
        }
    }

    #[test]
    fn load_absent_is_honest_none() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        assert!(matches!(
            load_drafts(&main).unwrap(),
            DraftsLoadOutcome::Absent
        ));
    }

    /// An UNREADABLE sidecar refuses the save (review iteration 6 P1):
    /// PermissionDenied on read followed by a directory-writable rename
    /// would destroy the previous store without a backup. The typed IO
    /// error surfaces instead; the store bytes stay untouched.
    #[test]
    fn save_refuses_to_overwrite_unreadable_store() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        write(&main, "{\"schema_version\":1,\"project_revision\":1,\"entries\":{},\"updated_at\":0}");
        let mut perms = std::fs::metadata(&main).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
        std::fs::set_permissions(&main, perms.clone()).unwrap();
        let result = save_drafts(
            &main,
            &DraftsFile::new(2, entries(&[("K", "v")])),
        );
        perms.set_mode(0o644);
        std::fs::set_permissions(&main, perms).unwrap();
        match result {
            Err(DraftsStoreError::Io { .. }) => {}
            other => assert!(
                matches!(other, Err(DraftsStoreError::Io { .. })),
                "expected a typed IO refusal, got {other:?}"
            ),
        }
        // The store bytes survived the refused save.
        let body = std::fs::read_to_string(&main).unwrap();
        assert!(body.contains("proj"), "store content replaced: {body}");
    }

    /// Locale-scoped composite store keys
    /// Locale-scoped composite store keys (`<locale>::<display_identity()>`,
    /// two Def types sharing one display key included) and legacy v1 bare
    /// display keys coexist in ONE store: the service treats key syntax as
    /// opaque, so a save/restore roundtrip preserves BOTH generations
    /// byte-faithfully. The v1→v2 re-anchoring itself lives in the UI,
    /// which owns the key convention — the backend must never reinterpret
    /// a key it does not understand.
    #[test]
    fn locale_scoped_and_legacy_keys_coexist() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        let file = DraftsFile::new(
            4,
            entries(&[
                ("ru::def_injected·ThingDef·Shared.label", "вещь"),
                ("ru::def_injected·HediffDef·Shared.label", "эффект"),
                ("uk::keyed·Settings", "украинский черновик"),
                ("Shared.legacy", "ключ первого поколения"),
            ]),
        );
        save_drafts(&main, &file).unwrap();
        let loaded = match load_drafts(&main).unwrap() {
            DraftsLoadOutcome::Loaded { drafts, .. } => drafts,
            other => {
                assert!(
                    matches!(other, DraftsLoadOutcome::Loaded { .. }),
                    "expected Loaded, got {other:?}"
                );
                return;
            }
        };
        assert_eq!(loaded, file, "all key generations roundtrip verbatim");
        assert_eq!(
            loaded.entries["ru::def_injected·ThingDef·Shared.label"].text,
            "вещь"
        );
        assert_eq!(
            loaded.entries["ru::def_injected·HediffDef·Shared.label"].text, "эффект",
            "same display key, different def types — two independent drafts"
        );
    }

    /// Rotation: `.drafts.1..3.json` exactly — the chain never grows past
    /// the cap, the oldest is removed only after the new write succeeded,
    /// and `.1` is always the PREVIOUS save (newest-first recovery order).
    #[test]
    fn rotation_is_bounded_and_ordered() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        for i in 0..10u64 {
            save_drafts(
                &main,
                &DraftsFile::new(i, entries(&[("K", &format!("v{i}"))])),
            )
            .unwrap();
        }
        let names = drafts_files_in(dir.path(), "proj-x");
        assert_eq!(
            names,
            vec![
                "proj-x.drafts.1.json",
                "proj-x.drafts.2.json",
                "proj-x.drafts.3.json",
                "proj-x.drafts.json",
            ],
            "exactly the live file + 3 backups, nothing older"
        );
        // .1 holds the previous save (rev 8), .2 the one before (rev 7).
        let prev = match load_drafts(&suffixed(&main, "1")).unwrap() {
            DraftsLoadOutcome::Loaded { drafts, .. } => drafts,
            other => {
                assert!(
                    matches!(other, DraftsLoadOutcome::Loaded { .. }),
                    "expected Loaded, got {other:?}"
                );
                return;
            }
        };
        assert_eq!(prev.project_revision, 8);
        assert_eq!(prev.entries["K"].text, "v8");
        // The live file holds the newest content.
        let live = match load_drafts(&main).unwrap() {
            DraftsLoadOutcome::Loaded { drafts, .. } => drafts,
            other => {
                assert!(
                    matches!(other, DraftsLoadOutcome::Loaded { .. }),
                    "expected Loaded, got {other:?}"
                );
                return;
            }
        };
        assert_eq!(live.project_revision, 9);
        assert_eq!(live.entries["K"].text, "v9");
    }

    /// MERGE semantics (review iteration 4 P0-B): a save that mentions only
    /// SOME keys upserts exactly those — unmentioned keys survive, so a
    /// raced or partial write can never erase durable work. Removal is
    /// explicit only (`clear_drafts_session`).
    #[test]
    fn save_merges_and_never_drops_unmentioned_keys() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        draft_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let pid = snap.project_id.clone();
        let epoch = snap.session_epoch;

        mgr.save_drafts_session(
            &pid,
            epoch,
            entries(&[("ru::keyed·A", "первый"), ("ru::keyed·B", "второй")]),
        )
        .unwrap();
        // A partial save mentioning ONLY A: B must survive.
        let ack = mgr
            .save_drafts_session(&pid, epoch, entries(&[("ru::keyed·A", "обновлённый")]))
            .unwrap();
        assert_eq!(ack.saved, 2, "merged store holds both keys");
        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::Restored);
        assert_eq!(report.drafts["ru::keyed·A"].text, "обновлённый");
        assert_eq!(report.drafts["ru::keyed·B"].text, "второй");
        // Explicit removal is the only way a key leaves the store.
        mgr.clear_drafts_session(&pid, epoch, Some("ru::keyed·B"))
            .unwrap();
        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert!(report.drafts.contains_key("ru::keyed·A"));
        assert!(!report.drafts.contains_key("ru::keyed·B"));
    }

    /// A corrupt live file never wins: recovery falls back to the newest
    /// intact backup and REPORTS the deviation.
    /// A corrupt live file never wins: recovery falls back to the newest
    /// intact backup and REPORTS the deviation.
    #[test]
    fn corrupt_main_recovers_from_backup() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        save_drafts(&main, &DraftsFile::new(1, entries(&[("K", "v1")]))).unwrap();
        save_drafts(&main, &DraftsFile::new(2, entries(&[("K", "v2")]))).unwrap();
        // Kill-style corruption of the live file (partial/truncated write).
        std::fs::write(&main, "{\"schema_ver").unwrap();
        match load_drafts(&main).unwrap() {
            DraftsLoadOutcome::Loaded {
                drafts,
                recovered_from,
            } => {
                assert_eq!(drafts.entries["K"].text, "v1", "previous save survives");
                assert_eq!(
                    recovered_from.as_deref(),
                    Some(suffixed(&main, "1").as_path())
                );
            }
            other => assert!(
                matches!(other, DraftsLoadOutcome::Loaded { .. }),
                "expected Loaded, got {other:?}"
            ),
        }
        // A schema-version mismatch is the same typed failure class. Tested
        // on a LONE file (no recoverable backup): when any backup parses,
        // recovery wins and the typed error is reported, not raised.
        let lone = dir.path().join("proj-y.drafts.json");
        std::fs::write(
            &lone,
            r#"{"schema_version": 99, "project_revision": 3, "entries": {}, "updated_at": 0}"#,
        )
        .unwrap();
        match load_drafts(&lone) {
            Err(DraftsStoreError::SchemaVersion {
                found, expected, ..
            }) => {
                assert_eq!(found, 99);
                assert_eq!(expected, DRAFTS_SCHEMA_VERSION);
            }
            other => assert!(
                matches!(other, Err(DraftsStoreError::SchemaVersion { .. })),
                "expected SchemaVersion error, got {other:?}"
            ),
        }
        // The schema-mismatched file WITH backups still recovers newest-first
        // (.1 holds the previous save) — the typed deviation is reported via
        // `recovered_from`, never silently dropped.
        match load_drafts(&main).unwrap() {
            DraftsLoadOutcome::Loaded {
                drafts,
                recovered_from,
            } => {
                assert_eq!(drafts.entries["K"].text, "v1");
                assert_eq!(
                    recovered_from.as_deref(),
                    Some(suffixed(&main, "1").as_path()),
                    "recovery reported honestly"
                );
            }
            other => assert!(
                matches!(other, DraftsLoadOutcome::Loaded { .. }),
                "expected Loaded, got {other:?}"
            ),
        }
    }

    /// Quarantine preserves the mismatched store verbatim and clears the
    /// live path; re-quarantine is idempotent.
    #[test]
    fn orphan_quarantine_preserves_and_clears() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("proj-x.drafts.json");
        let file = DraftsFile::new(4, entries(&[("K", "orphaned work")]));
        save_drafts(&main, &file).unwrap();
        let orphan = quarantine_orphan(&main).unwrap().expect("quarantined");
        assert_eq!(orphan, suffixed(&main, "orphan"));
        assert!(!main.exists(), "live store cleared after quarantine");
        let parsed = match load_drafts(&orphan).unwrap() {
            DraftsLoadOutcome::Loaded { drafts, .. } => drafts,
            other => {
                assert!(
                    matches!(other, DraftsLoadOutcome::Loaded { .. }),
                    "expected Loaded, got {other:?}"
                );
                return;
            }
        };
        assert_eq!(parsed, file, "orphan is byte-faithful");
        assert!(
            quarantine_orphan(&main).unwrap().is_none(),
            "nothing left to quarantine"
        );
    }

    // --- manager ops (epoch guard, revision stamp, semantics separation) ---

    /// Kill-after-autosave: one manager saves drafts and is dropped; a NEW
    /// manager (the restarted app) opens the same project and restores the
    /// exact entries. The canonical project state (revision, translations,
    /// completeness) is untouched by both operations — drafts never become
    /// confirmed translations.
    #[test]
    fn kill_after_autosave_new_manager_restores() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        draft_mod(&mod_root);
        let managed = dir.path().join("managed");
        let project_id;
        let epoch;
        {
            let mgr = ProjectSessionManager::new(&managed).unwrap();
            let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
            project_id = snap.project_id.clone();
            epoch = snap.session_epoch;
            let ack = mgr
                .save_drafts_session(
                    &project_id,
                    epoch,
                    entries(&[("Dup.label", "несохранённая метка")]),
                )
                .unwrap();
            assert_eq!(ack.saved, 1);
            assert_eq!(ack.project_revision, 1);
        } // the manager (and its session state) is gone — the "kill"

        let restarted = ProjectSessionManager::new(&managed).unwrap();
        let snap = restarted.open(&project_id).unwrap();
        assert_eq!(snap.revision, 1, "drafts never bump the project revision");
        assert!(
            snap.project.translations.is_empty(),
            "no draft leaked into translations"
        );
        let report = restarted
            .restore_drafts_session(&project_id, snap.session_epoch)
            .unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::Restored);
        assert_eq!(report.restored_count, 1);
        assert_eq!(report.drafts["Dup.label"].text, "несохранённая метка");
        assert_eq!(report.recovered_from_backup, None);
        // The canonical state STILL untouched after the restore.
        let after = restarted.snapshot(&project_id).unwrap();
        assert_eq!(after.revision, 1);
        assert!(after.project.translations.is_empty());
    }

    /// A stale epoch is a typed refusal — a UI session that lost its
    /// session (reopen bumped the epoch) cannot write drafts past the live
    /// session; concurrent writers through two clones of the same session
    /// stay atomic (the file is always a fully-parsed document).
    #[test]
    fn stale_epoch_refused_and_concurrent_writes_stay_atomic() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        draft_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let pid = snap.project_id.clone();
        let live_epoch = snap.session_epoch;

        let refused = mgr.save_drafts_session(&pid, live_epoch + 1, entries(&[("K", "late")]));
        assert_eq!(refused.unwrap_err().code, ContractErrorCode::StaleEpoch);
        let refused = mgr.restore_drafts_session(&pid, live_epoch.wrapping_sub(1));
        assert_eq!(refused.unwrap_err().code, ContractErrorCode::StaleEpoch);

        let a = mgr.clone();
        let b = mgr.clone();
        let handles: Vec<_> = [(0u32, a), (1u32, b)]
            .into_iter()
            .map(|(i, m)| {
                let pid = pid.clone();
                std::thread::spawn(move || {
                    for round in 0..25u32 {
                        let mut map = BTreeMap::new();
                        for j in 0..5u32 {
                            map.insert(
                                format!("K.{j}"),
                                DraftEntry::new(format!("w{i}-r{round}-{j}"), 0),
                            );
                        }
                        m.save_drafts_session(&pid, live_epoch, map).unwrap();
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        // 50 concurrent saves: the live file parses and carries ONE coherent
        // revision-stamped document (atomic rename, no torn write).
        let path = drafts_file_path(&mgr.managed_path(&pid).unwrap(), &pid);
        match load_drafts(&path).unwrap() {
            DraftsLoadOutcome::Loaded {
                drafts,
                recovered_from,
            } => {
                assert_eq!(recovered_from, None);
                assert_eq!(drafts.entries.len(), 5);
                let text = &drafts.entries["K.0"].text;
                let writer = text.split('-').next().unwrap();
                assert!(writer == "w0" || writer == "w1");
                for (k, v) in &drafts.entries {
                    assert_eq!(
                        v.text.split('-').next().unwrap(),
                        writer,
                        "{k} from the same write"
                    );
                }
            }
            other => assert!(
                matches!(other, DraftsLoadOutcome::Loaded { .. }),
                "expected Loaded, got {other:?}"
            ),
        }
    }

    /// Restore decision table at the manager level: match → restored;
    /// mismatch (the project moved on) → orphan quarantine + honest report;
    /// unreadable beyond backups → `unavailable` with the typed detail —
    /// and opening/working with the project never fails because of drafts.
    #[test]
    fn restore_matches_quarantines_and_reports_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        draft_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let pid = snap.project_id.clone();
        let epoch = snap.session_epoch;

        // No store yet → none.
        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::None);

        // Save at revision 1, then move the project forward (a confirmed
        // translation, revision 2). The old store must NOT silently apply.
        mgr.save_drafts_session(&pid, epoch, entries(&[("Dup.label", "черновик")]))
            .unwrap();
        let ack = mgr
            .apply(&ApplyIntentsRequest {
                project_id: pid.clone(),
                expected_revision: 1,
                session_epoch: epoch,
                origin: None,
                intents: vec![crate::contract::TranslationIntent {
                    entry: SourceEntryId {
                        kind: EntryKind::DefInjected,
                        key: "Dup.label".into(),
                        def_type: Some("ThingDef".into()),
                    },
                    locale: "Russian".into(),
                    action: IntentAction::SetTranslation,
                    text: Some("подтверждённая метка".into()),
                }],
            })
            .unwrap();
        assert_eq!(ack.revision, 2);

        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::OrphanQuarantined);
        assert_eq!(report.project_revision, Some(1));
        let orphan = report.orphan_path.expect("orphan path reported");
        assert!(
            Path::new(&orphan.path).is_file(),
            "orphan preserved for inspection"
        );
        assert!(
            report.drafts.is_empty(),
            "mismatched drafts are NOT applied"
        );
        // Quarantine cleared the live path — the next restore is clean.
        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::None);

        // Unreadable beyond backups → honest `unavailable`, not a refusal.
        mgr.save_drafts_session(&pid, epoch, entries(&[("Dup.label", "черновик 2")]))
            .unwrap();
        let path = drafts_file_path(&mgr.managed_path(&pid).unwrap(), &pid);
        std::fs::write(&path, "not json at all{{{").unwrap();
        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::Unavailable);
        assert!(report.detail.unwrap().contains("corrupt"));
        // The project itself keeps working.
        assert_eq!(mgr.snapshot(&pid).unwrap().revision, 2);
    }

    /// Backup recovery at the manager level: a killed save corrupts the
    /// live file, restore raises the previous save's entries AND reports
    /// the recovery, then self-heals the live file.
    #[test]
    fn restore_recovers_from_backup_and_self_heals() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        draft_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let pid = snap.project_id.clone();
        let epoch = snap.session_epoch;
        mgr.save_drafts_session(&pid, epoch, entries(&[("Dup.label", "первый черновик")]))
            .unwrap();
        mgr.save_drafts_session(&pid, epoch, entries(&[("Dup.label", "второй черновик")]))
            .unwrap();
        let path = drafts_file_path(&mgr.managed_path(&pid).unwrap(), &pid);
        std::fs::write(&path, "{\"entries\":").unwrap(); // corrupted live file

        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::Restored);
        assert_eq!(report.restored_count, 1);
        assert_eq!(
            report.drafts["Dup.label"].text, "первый черновик",
            "previous save survives"
        );
        assert!(
            report.recovered_from_backup.is_some(),
            "recovery from a backup is reported, never silent"
        );
        // Self-healed: the live file parses again and carries the raised data.
        match load_drafts(&path).unwrap() {
            DraftsLoadOutcome::Loaded {
                drafts,
                recovered_from,
            } => {
                assert_eq!(recovered_from, None);
                assert_eq!(drafts.entries["Dup.label"].text, "первый черновик");
            }
            other => assert!(
                matches!(other, DraftsLoadOutcome::Loaded { .. }),
                "expected Loaded, got {other:?}"
            ),
        }
    }

    /// clear: one key on commit/revert, the whole store on demand; absent
    /// keys and files are honest no-ops.
    #[test]
    fn clear_removes_keys_and_stays_honest_on_absent() {
        let dir = tempfile::tempdir().unwrap();
        let mod_root = dir.path().join("mod");
        draft_mod(&mod_root);
        let mgr = ProjectSessionManager::new(dir.path().join("managed")).unwrap();
        let snap = mgr.create(&mod_root, Some("1.6")).unwrap();
        let pid = snap.project_id.clone();
        let epoch = snap.session_epoch;

        // Absent store → no-op ack.
        let ack = mgr
            .clear_drafts_session(&pid, epoch, Some("Dup.label"))
            .unwrap();
        assert_eq!(ack.saved, 0);

        mgr.save_drafts_session(
            &pid,
            epoch,
            entries(&[("Dup.label", "a"), ("Dup.description", "b")]),
        )
        .unwrap();
        let ack = mgr
            .clear_drafts_session(&pid, epoch, Some("Dup.label"))
            .unwrap();
        assert_eq!(ack.saved, 1);
        let path = drafts_file_path(&mgr.managed_path(&pid).unwrap(), &pid);
        match load_drafts(&path).unwrap() {
            DraftsLoadOutcome::Loaded { drafts, .. } => {
                assert!(!drafts.entries.contains_key("Dup.label"));
                assert!(drafts.entries.contains_key("Dup.description"));
            }
            other => assert!(
                matches!(other, DraftsLoadOutcome::Loaded { .. }),
                "expected Loaded, got {other:?}"
            ),
        }
        // Unknown key → no-op (the asked-for end state already holds).
        let ack = mgr
            .clear_drafts_session(&pid, epoch, Some("Missing.key"))
            .unwrap();
        assert_eq!(ack.saved, 1);
        // Whole store.
        let ack = mgr.clear_drafts_session(&pid, epoch, None).unwrap();
        assert_eq!(ack.saved, 0);
        let report = mgr.restore_drafts_session(&pid, epoch).unwrap();
        assert_eq!(report.status, DraftsRestoreStatus::None);
    }
}
