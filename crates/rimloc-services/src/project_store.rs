//! Project persistence (Gate I3): canonical `Project` ⇄ disk.
//!
//! Evidence-based choice for the POC: a single versioned JSON file with
//! deterministic serialization — portable, human-readable, zero new
//! dependencies, trivially diffable. SQLite stays DEFERRED until real
//! query/scale needs appear (documented in CANONICAL_PROJECT_MODEL.md);
//! the store interface below is the seam that would change.
//!
//! Crash safety: writes go through `write_atomic` (tmp → rename), so a
//! crash never leaves a half-written project file.
//!
//! Container v2 (identity fix): `SourceEntryId` carries an optional def-type
//! discriminator. v1 files (kind+key ids) are MIGRATED on load: each entry's
//! def type is derived ONLY from unambiguous evidence (TKey metadata, or
//! fully consistent context def types); a v1 entry whose contexts show two
//! different def types (a collision the old model collapsed) is REJECTED
//! with an actionable typed diagnostic — never guessed, never split by
//! copying one translation into both identities. v2 files are validated for
//! identity uniqueness and translation-reference integrity. Loading never
//! writes: on any failure the original file bytes stay untouched.

use crate::Result;
use rimloc_domain::canonical::Project;
use std::path::Path;

/// v2: identity discriminator in `SourceEntryId`. A semantic container
/// change — independent of crate/package versions and of
/// `RIMLOC_SCHEMA_VERSION` (the CLI transport schema, which is unchanged).
pub const PROJECT_FILE_VERSION: u32 = 2;

/// Version the v1 files were written with.
pub const PROJECT_FILE_VERSION_V1: u32 = 1;

/// Typed load/migration diagnostics. Every variant is actionable and the
/// original file is never modified when one is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectLoadDiagnostic {
    /// A v1 entry's context evidence carries several def types — the old
    /// kind+key model collapsed two distinct Defs into one entry. Rescan
    /// the source mod with the identity-aware build and review the affected
    /// translations.
    AmbiguousDefType { key: String, types: Vec<String> },
    /// Two entries claim one identity (kind + key + def type).
    DuplicateIdentity { identity: String },
    /// A translation references an identity no entry carries.
    UnmappedTranslation { locale: String, identity: String },
    /// Two translations claim one (identity, locale) slot.
    DuplicateTranslationSlot { identity: String, locale: String },
    /// A v1 translation's (kind, key) matches entries of several def types,
    /// so it cannot be re-pointed uniquely.
    AmbiguousReference {
        locale: String,
        key: String,
        types: Vec<String>,
    },
    /// The entry's own evidence contradicts itself (id discriminator vs
    /// TKey metadata vs contexts) — the file is internally inconsistent.
    ConflictingEvidence { key: String, types: Vec<String> },
    /// A def-type discriminator is present but empty — never a valid
    /// identity (an unknown type is `None`, not `Some("")`).
    InvalidDiscriminator { identity: String },
    /// The project was written by an adapter this build does not know (§F9):
    /// refuse cleanly instead of interpreting foreign semantics as RimWorld.
    UnknownAdapter { adapter_id: String, known: String },
}

impl ProjectLoadDiagnostic {
    /// The action the user should take, phrased for the UI/CLI surface.
    pub fn action(&self) -> &'static str {
        match self {
            Self::AmbiguousDefType { .. } => {
                "rescan the source mod and review affected translations"
            }
            Self::DuplicateIdentity { .. } => "rescan and rebuild the project",
            Self::UnmappedTranslation { .. } => {
                "reimport the translation pack or rescan the source"
            }
            Self::DuplicateTranslationSlot { .. } => "review duplicate translation rows",
            Self::AmbiguousReference { .. } => {
                "rescan the source mod; assign the translation manually"
            }
            Self::ConflictingEvidence { .. } => {
                "rescan the source mod and review the affected entry"
            }
            Self::InvalidDiscriminator { .. } => "rescan and rebuild the project",
            Self::UnknownAdapter { .. } => {
                "open this project with the application that owns its adapter"
            }
        }
    }
}

impl std::fmt::Display for ProjectLoadDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AmbiguousDefType { key, types } => write!(
                f,
                "ambiguous v1 entry `{key}`: contexts show several def types ({types:?}); \
                 cannot split one translation across two identities"
            ),
            Self::DuplicateIdentity { identity } => {
                write!(f, "duplicate entry identity `{identity}`")
            }
            Self::UnmappedTranslation { locale, identity } => write!(
                f,
                "translation [{locale}] references missing entry `{identity}`"
            ),
            Self::DuplicateTranslationSlot { identity, locale } => {
                write!(f, "duplicate translations for `{identity}` [{locale}]")
            }
            Self::AmbiguousReference { locale, key, types } => write!(
                f,
                "v1 translation [{locale}] for `{key}` maps to several def types ({types:?})"
            ),
            Self::ConflictingEvidence { key, types } => write!(
                f,
                "entry `{key}` carries contradictory def-type evidence ({types:?})"
            ),
            Self::InvalidDiscriminator { identity } => write!(
                f,
                "entry identity `{identity}` has an empty def-type discriminator"
            ),
            Self::UnknownAdapter { adapter_id, known } => write!(
                f,
                "unknown localization adapter `{adapter_id}` (this build speaks: {known}); \
                 the project file is intact but its source semantics are foreign"
            ),
        }
    }
}

impl std::error::Error for ProjectLoadDiagnostic {}

/// Additive binding-envelope metadata (binding first slice, 033): the
/// durable opaque project id and content revision live in the SAME atomic
/// managed-project record as the content — there is no second authoritative
/// index. All fields optional: legacy v2 files (and bare `save_project`
/// writes) load unchanged, and bare saves keep writing without the fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectEnvelopeMeta {
    /// Opaque durable id minted by the session layer (never path-derived).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Durable monotonic content revision; bumps on every acked change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
    /// Human display name captured at create (mod folder name).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Read-only source mod root the project was built from (H5). Durable
    /// state: a session reopened after an app restart restores it, so the
    /// export/diagnose source-tree guard stays functional and the
    /// close-open-build cycle completes. `None` on legacy envelopes and
    /// bare saves — those sessions stay fail-closed for export (the guard
    /// refuses instead of silently disabling itself).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_root: Option<String>,
    /// Fingerprint of the source content the inventory was built from (M3):
    /// the resolved effective view (including the resolved game version)
    /// plus hashes of the scanned trees. Recorded at create, re-asserted on
    /// every save; `None` on legacy envelopes - drift detection stays
    /// honestly unavailable for them (never reported as "in sync").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_fingerprint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectFile {
    /// Persistence container version (transport), independent of the domain
    /// model evolution handled by serde defaults.
    schema_version: u32,
    #[serde(flatten)]
    project: Project,
    /// Binding envelope metadata (additive within v2; omitted when bare).
    #[serde(flatten)]
    meta: ProjectEnvelopeMeta,
}

/// A loaded project together with its binding-envelope metadata.
#[derive(Debug, Clone)]
pub struct LoadedProject {
    pub project: Project,
    pub meta: ProjectEnvelopeMeta,
}

/// Serialize deterministically (BTree/Vec field order is stable; maps sorted
/// via BTreeMap inside the model). Pretty-printed for diffability.
pub fn serialize_project(project: &Project) -> std::io::Result<Vec<u8>> {
    let file = ProjectFile {
        schema_version: PROJECT_FILE_VERSION,
        project: project.clone(),
        meta: ProjectEnvelopeMeta::default(),
    };
    let s = serde_json::to_string_pretty(&file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut out = s.into_bytes();
    out.push(b'\n');
    Ok(out)
}

/// Serialize with binding-envelope metadata (same container, same
/// validation path).
pub fn serialize_project_with_meta(
    project: &Project,
    meta: &ProjectEnvelopeMeta,
) -> std::io::Result<Vec<u8>> {
    let file = ProjectFile {
        schema_version: PROJECT_FILE_VERSION,
        project: project.clone(),
        meta: meta.clone(),
    };
    let s = serde_json::to_string_pretty(&file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut out = s.into_bytes();
    out.push(b'\n');
    Ok(out)
}

/// Save atomically with binding-envelope metadata. Same validating-save
/// contract as [`save_project`]: the proposed state must pass the same
/// checker the loader uses BEFORE any byte is written.
pub fn save_project_with_meta(
    project: &Project,
    meta: &ProjectEnvelopeMeta,
    path: &Path,
) -> std::io::Result<()> {
    validate_v2(project.clone()).map_err(|d| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{d}; action: {}", d.action()),
        )
    })?;
    let bytes = serialize_project_with_meta(project, meta)?;
    crate::write_atomic(path, &bytes)
}

/// Save atomically: tmp file → rename, never a torn project file. The
/// proposed state is validated with the SAME typed invariant checker the
/// loader uses — a project the store itself could not reopen is never
/// written, so a bad save cannot destroy the last good file.
pub fn save_project(project: &Project, path: &Path) -> std::io::Result<()> {
    validate_v2(project.clone()).map_err(|d| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{d}; action: {}", d.action()),
        )
    })?;
    let bytes = serialize_project(project)?;
    crate::write_atomic(path, &bytes)
}

/// Load a project file WITH its binding-envelope metadata (dispatches v1/v2
/// like [`load_project`]; v1 files carry no envelope — defaults apply).
pub fn load_project_with_meta(path: &Path) -> Result<LoadedProject> {
    let text = std::fs::read_to_string(path)?;
    load_project_str_with_meta(&text)
}

/// String form of [`load_project_with_meta`].
pub fn load_project_str_with_meta(text: &str) -> Result<LoadedProject> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| color_eyre::eyre::eyre!("project file missing schema_version"))?;
    match u32::try_from(version) {
        Ok(v) if v == PROJECT_FILE_VERSION => {
            let file: ProjectFile = serde_json::from_value(value)?;
            let project = validate_v2(file.project)
                .map_err::<color_eyre::Report, _>(std::convert::Into::into)?;
            Ok(LoadedProject {
                project,
                meta: file.meta,
            })
        }
        Ok(v) if v == PROJECT_FILE_VERSION_V1 => {
            let file: ProjectFile = serde_json::from_value(value)?;
            let project: Project = migrate_v1(file.project)
                .map_err::<color_eyre::Report, _>(std::convert::Into::into)?;
            let project =
                validate_v2(project).map_err::<color_eyre::Report, _>(std::convert::Into::into)?;
            // v1 predates the envelope: no durable id/revision existed.
            Ok(LoadedProject {
                project,
                meta: ProjectEnvelopeMeta::default(),
            })
        }
        _ => Err(color_eyre::eyre::eyre!(
            "project file schema_version {version} not supported (expected {PROJECT_FILE_VERSION})"
        )),
    }
}

/// Load a project file, dispatching on the container version. Loading never
/// writes: on any failure the original file bytes stay untouched.
pub fn load_project(path: &Path) -> Result<Project> {
    let text = std::fs::read_to_string(path)?;
    load_project_str(&text)
}

/// Parse a project file from a string (same container rules as
/// [`load_project`]); factored out so callers/tests can exercise migrations
/// without touching the filesystem.
pub fn load_project_str(text: &str) -> Result<Project> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| color_eyre::eyre::eyre!("project file missing schema_version"))?;
    match u32::try_from(version) {
        Ok(v) if v == PROJECT_FILE_VERSION => {
            let file: ProjectFile = serde_json::from_value(value)?;
            validate_v2(file.project).map_err::<color_eyre::Report, _>(std::convert::Into::into)
        }
        Ok(v) if v == PROJECT_FILE_VERSION_V1 => {
            let file: ProjectFile = serde_json::from_value(value)?;
            let project: Project = migrate_v1(file.project)
                .map_err::<color_eyre::Report, _>(std::convert::Into::into)?;
            validate_v2(project).map_err::<color_eyre::Report, _>(std::convert::Into::into)
        }
        _ => Err(color_eyre::eyre::eyre!(
            "project file schema_version {version} not supported (expected {PROJECT_FILE_VERSION})"
        )),
    }
}

/// Collect ALL def-type evidence of one entry (exact case — two types that
/// differ only in case are DIFFERENT types, never folded) and require a
/// single unanimous answer. Evidence sources: the id discriminator itself,
/// TKey metadata, context def types, and — for authentic pre-identity v1
/// entries whose contexts carry no def type — the DefInjected segment of the
/// effective context's canonical path. Contradicting evidence is REJECTED,
/// never guessed; missing evidence stays explicitly unknown (None).
fn entry_def_type_evidence(
    entry: &rimloc_domain::canonical::SourceEntry,
) -> std::result::Result<Option<String>, ProjectLoadDiagnostic> {
    let mut sources: Vec<String> = Vec::new();
    let push = |sources: &mut Vec<String>, dt: Option<&String>| {
        if let Some(dt) = dt {
            if !dt.trim().is_empty() && !sources.iter().any(|known| known == dt) {
                sources.push(dt.clone());
            }
        }
    };
    push(&mut sources, entry.id.def_type.as_ref());
    if let Some(meta) = &entry.tkey {
        push(&mut sources, Some(&meta.def_type));
    }
    let mut context_distinct: Vec<String> = Vec::new();
    for c in &entry.contexts {
        if let Some(dt) = &c.def_type {
            if !context_distinct.iter().any(|known| known == dt) {
                context_distinct.push(dt.clone());
            }
        }
    }
    match context_distinct.len() {
        0 => {}
        1 => push(&mut sources, Some(&context_distinct[0])),
        _ => {
            return Err(ProjectLoadDiagnostic::AmbiguousDefType {
                key: entry.id.key.clone(),
                types: context_distinct,
            });
        }
    }
    // Legacy v1 bridge left context def_type empty but recorded the canonical
    // virtual output path (`.../DefInjected/<DefType>/...`) — unambiguous
    // path evidence that keeps normal old translations attached to their
    // fresh typed identities. ALL contexts are considered: an authentic v1
    // collapsed collision carries TWO virtual paths (two def types), which
    // must FAIL like any other collapsed evidence — the first path is never
    // silently taken as the whole truth.
    let mut path_distinct: Vec<String> = Vec::new();
    for c in &entry.contexts {
        if let Some(dt) = crate::canonical_bridge::definjected_def_type_str(&c.file) {
            if !path_distinct.iter().any(|known| known == &dt) {
                path_distinct.push(dt);
            }
        }
    }
    match path_distinct.len() {
        0 => {}
        1 => push(&mut sources, Some(&path_distinct[0])),
        _ => {
            return Err(ProjectLoadDiagnostic::AmbiguousDefType {
                key: entry.id.key.clone(),
                types: path_distinct,
            });
        }
    }
    match sources.len() {
        0 => Ok(None),
        1 => Ok(sources.pop()),
        _ => Err(ProjectLoadDiagnostic::ConflictingEvidence {
            key: entry.id.key.clone(),
            types: sources,
        }),
    }
}

/// v1 → v2 migration: derive each entry's def-type discriminator from its
/// unanimous evidence in ONE pass (no nested evidence × entries re-scan),
/// then re-point every translation reference together with its entry. A
/// (kind, key) reference is unique in honestly-produced v1 files; anything
/// else is rejected, never guessed.
fn migrate_v1(mut project: Project) -> std::result::Result<Project, ProjectLoadDiagnostic> {
    use rimloc_domain::canonical::EntryKind;
    use std::collections::HashMap;

    let mut derived: HashMap<(EntryKind, String), Vec<Option<String>>> = HashMap::new();
    let mut derived_ordered: Vec<Option<String>> = Vec::with_capacity(project.entries.len());
    for e in &project.entries {
        let def_type = entry_def_type_evidence(e)?;
        let bucket = derived.entry((e.id.kind, e.id.key.clone())).or_default();
        if !bucket.iter().any(|known| known == &def_type) {
            bucket.push(def_type.clone());
        }
        derived_ordered.push(def_type);
    }
    for (e, def_type) in project.entries.iter_mut().zip(&derived_ordered) {
        e.id.def_type = def_type.clone();
    }
    for t in &mut project.translations {
        match derived.get(&(t.source_id.kind, t.source_id.key.clone())) {
            None => {
                return Err(ProjectLoadDiagnostic::UnmappedTranslation {
                    locale: t.locale.clone(),
                    identity: t.source_id.display_identity(),
                });
            }
            Some(slice) if slice.len() == 1 => t.source_id.def_type = slice[0].clone(),
            Some(many) => {
                return Err(ProjectLoadDiagnostic::AmbiguousReference {
                    locale: t.locale.clone(),
                    key: t.source_id.key.clone(),
                    types: many.iter().filter_map(|dt| dt.clone()).collect(),
                });
            }
        }
    }
    Ok(project)
}

/// v2 integrity, on STRUCTURAL identities (the enum+key+type struct itself,
/// never a concatenated string): unique entry identities, translations
/// pointing at existing entries, one translation per (identity, locale)
/// slot, and per-entry evidence consistency (an id discriminator must not
/// contradict TKey metadata or contexts). A present-but-empty discriminator
/// is invalid — unknown is `None`, not `Some("")`.
fn validate_v2(project: Project) -> std::result::Result<Project, ProjectLoadDiagnostic> {
    use rimloc_domain::canonical::{adapter_ids, SourceEntryId};

    // §F9 adapter identity gate: an unknown adapter's project is FOREIGN —
    // its EntryKind/context semantics cannot be interpreted here. Fail with
    // a useful diagnostic; the file on disk stays untouched.
    let known = format!(
        "{}, {}",
        adapter_ids::RIMWORLD,
        adapter_ids::RIMLOC_APPLICATION
    );
    match project.adapter.adapter_id.as_str() {
        adapter_ids::RIMWORLD | adapter_ids::RIMLOC_APPLICATION => {}
        other => {
            return Err(ProjectLoadDiagnostic::UnknownAdapter {
                adapter_id: other.to_string(),
                known,
            });
        }
    }

    let mut seen: std::collections::BTreeSet<SourceEntryId> = std::collections::BTreeSet::new();
    for e in &project.entries {
        if e.id
            .def_type
            .as_deref()
            .is_some_and(|dt| dt.trim().is_empty())
        {
            return Err(ProjectLoadDiagnostic::InvalidDiscriminator {
                identity: e.id.display_identity(),
            });
        }
        entry_def_type_evidence(e)?;
        if !seen.insert(e.id.clone()) {
            return Err(ProjectLoadDiagnostic::DuplicateIdentity {
                identity: e.id.display_identity(),
            });
        }
    }
    let ids: std::collections::BTreeSet<SourceEntryId> = seen;
    let mut slots: std::collections::BTreeSet<(SourceEntryId, String)> =
        std::collections::BTreeSet::new();
    for t in &project.translations {
        if !ids.contains(&t.source_id) {
            return Err(ProjectLoadDiagnostic::UnmappedTranslation {
                locale: t.locale.clone(),
                identity: t.source_id.display_identity(),
            });
        }
        if !slots.insert((t.source_id.clone(), t.locale.clone())) {
            return Err(ProjectLoadDiagnostic::DuplicateTranslationSlot {
                identity: t.source_id.display_identity(),
                locale: t.locale.clone(),
            });
        }
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rimloc_domain::canonical::{
        ContextRole, EntryKind, Origin, PatchStage, Project, SourceContext, SourceEntry,
        SourceEntryId, SourceProvenance, Translation,
    };

    fn sample() -> Project {
        Project {
            entries: vec![SourceEntry {
                id: SourceEntryId {
                    kind: EntryKind::TKey,
                    key: "Q.LetterLabel".into(),
                    def_type: None,
                },
                text: "Quest complete".into(),
                source_locale: "en".into(),
                contexts: vec![SourceContext {
                    file: "Defs/Q.xml".into(),
                    line: Some(11),
                    def_type: Some("QuestScriptDef".into()),
                    role: ContextRole::Effective,
                }],
                provenance: SourceProvenance {
                    version_selected: Some("1.6".into()),
                    conditional_branch: false,
                    patch_stage: PatchStage::Applied,
                    selected_by: None,
                },
                tkey: Some(rimloc_core::TKeyMeta {
                    strategy: "slate_ref".into(),
                    suffix: ".slateRef".into(),
                    def_type: "QuestScriptDef".into(),
                    contexts: 1,
                    locations: Vec::new(),
                }),
                source_ref: None,
            }],
            translations: vec![Translation {
                source_id: SourceEntryId {
                    kind: EntryKind::TKey,
                    key: "Q.LetterLabel".into(),
                    def_type: None,
                },
                locale: "ru".into(),
                text: Some("Задание выполнено".into()),
                completeness: rimloc_domain::canonical::Completeness::Translated,
                review: rimloc_domain::canonical::Review::None,
                validation: rimloc_domain::canonical::ValidationState::Unknown,
                lifecycle: rimloc_domain::canonical::Lifecycle::Active,
                origin: Origin::Human,
                notes: String::new(),
                source_changed: None,
            }],
            ..Default::default()
        }
    }

    /// Serialization is deterministic: two calls, byte-identical output.
    #[test]
    fn serialization_is_deterministic() {
        let a = serialize_project(&sample()).unwrap();
        let b = serialize_project(&sample()).unwrap();
        assert_eq!(a, b);
    }

    /// Save → load → same canonical state (reopen/recovery acceptance §29).
    #[test]
    fn save_load_round_trip_preserves_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let original = sample();
        save_project(&original, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        // Exact model equality proves no field is lost on the round trip.
        assert_eq!(loaded, original);
    }

    #[test]
    // ---------- Adapter identity conformance (§F9/§F14, LOCALIZATION_ADAPTERS) ----------

    #[test]
    fn legacy_envelope_without_adapter_defaults_to_rimworld() {
        let bytes = serialize_project(&sample()).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("\"adapter\""), "new saves record the adapter");
        // Legacy v2 file: strip the adapter object entirely.
        let legacy = strip_adapter_object(&s);
        let loaded = load_project_str(&legacy).unwrap();
        assert_eq!(loaded.adapter.adapter_id, "rimworld");
        assert_eq!(loaded.adapter.adapter_api_version, "1");
    }

    #[test]
    fn adapter_identity_round_trips() {
        let mut p = sample();
        p.adapter.adapter_id = "rimloc-application".to_string();
        p.adapter.adapter_api_version = "1".to_string();
        p.adapter.adapter_project_schema_version = "3".to_string();
        let loaded = load_project_str(&String::from_utf8(serialize_project(&p).unwrap()).unwrap())
            .unwrap();
        assert_eq!(loaded.adapter.adapter_id, "rimloc-application");
        assert_eq!(loaded.adapter.adapter_project_schema_version, "3");
    }

    #[test]
    fn unknown_adapter_fails_cleanly_with_diagnostic() {
        let bytes = serialize_project(&sample()).unwrap();
        let s = String::from_utf8(bytes)
            .unwrap()
            .replace("\"adapter_id\": \"rimworld\"", "\"adapter_id\": \"minecraft\"");
        assert!(s.contains("minecraft"), "tamper must apply");
        let err = load_project_str(&s).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("minecraft"), "{msg}");
        assert!(msg.contains("unknown localization adapter"), "{msg}");
        assert!(msg.contains("rimworld"), "{msg}"); // known list present
    }

    #[test]
    fn rimworld_and_selfloc_adapter_projects_coexist() {
        let rimworld = sample();
        let mut selfloc = sample();
        selfloc.adapter.adapter_id = "rimloc-application".to_string();
        let a = load_project_str(&String::from_utf8(serialize_project(&rimworld).unwrap()).unwrap())
            .unwrap();
        let b = load_project_str(&String::from_utf8(serialize_project(&selfloc).unwrap()).unwrap())
            .unwrap();
        assert_eq!(a.adapter.adapter_id, "rimworld");
        assert_eq!(b.adapter.adapter_id, "rimloc-application");
        // Same generic core operations on both — no RimWorld-specific reads.
        assert_eq!(a.entries.len(), b.entries.len());
    }

    /// Remove the serialized `adapter` object from a project file (legacy
    /// simulation). The block is a fixed-shape pretty-printed object right
    /// after the file's opening fields.
    fn strip_adapter_object(file: &str) -> String {
        let start = file
            .find("  \"adapter\": {")
            .expect("adapter block must exist in new saves");
        let end = file[start..].find("  },\n").map(|i| start + i + 5).unwrap();
        let mut out = String::with_capacity(file.len());
        out.push_str(&file[..start]);
        out.push_str(&file[end..]);
        out
    }

    #[test]
    fn wrong_schema_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let bytes = serialize_project(&sample()).unwrap();
        // Tamper the container version precisely.
        let s = String::from_utf8(bytes)
            .unwrap()
            .replace("\"schema_version\": 2,", "\"schema_version\": 99,");
        assert!(s.contains("99"), "tamper must apply");
        std::fs::write(&path, s).unwrap();
        let err = load_project(&path).unwrap_err();
        assert!(err.to_string().contains("schema_version"), "{err}");
    }

    /// v1 migration, unambiguous evidence: ids gain the def type from the
    /// consistent context/TKey metadata and translation references are
    /// re-pointed together; provenance is preserved.
    #[test]
    fn v1_migration_derives_discriminator_and_repoints_references() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        // Hand-written v1 file: kind+key ids WITHOUT def_type, context and
        // TKey metadata carrying the evidence.
        let v1 = r#"{
  "schema_version": 1,
  "context": { "view": "potential" },
  "entries": [
    {
      "id": { "kind": "def_injected", "key": "Dup.label" },
      "text": "thing label",
      "source_locale": "en",
      "contexts": [
        { "file": "Defs/Thing.xml", "line": 3, "def_type": "ThingDef", "role": "effective" }
      ],
      "provenance": { "patch_stage": "none" }
    },
    {
      "id": { "kind": "t_key", "key": "Q.LetterLabel" },
      "text": "Quest complete",
      "source_locale": "en",
      "contexts": [],
      "provenance": { "patch_stage": "none" },
      "tkey": {
        "strategy": "slate_ref", "suffix": ".slateRef",
        "def_type": "QuestScriptDef", "contexts": 1
      }
    }
  ],
  "translations": [
    {
      "source_id": { "kind": "def_injected", "key": "Dup.label" },
      "locale": "ru", "text": "метка вещи",
      "completeness": "translated", "review": "none",
      "validation": "unknown", "lifecycle": "active", "origin": "human"
    },
    {
      "source_id": { "kind": "t_key", "key": "Q.LetterLabel" },
      "locale": "ru", "text": "Квест завершён",
      "completeness": "translated", "review": "none",
      "validation": "unknown", "lifecycle": "active", "origin": "human"
    }
  ]
}
"#;
        std::fs::write(&path, v1).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(
            loaded.entries[0].id.def_type.as_deref(),
            Some("ThingDef"),
            "discriminator derived from the consistent context evidence"
        );
        assert_eq!(
            loaded.entries[1].id.def_type.as_deref(),
            Some("QuestScriptDef"),
            "TKey metadata is the TKey evidence"
        );
        assert_eq!(loaded.translations.len(), 2);
        for t in &loaded.translations {
            assert!(
                loaded.entries.iter().any(|e| e.id == t.source_id),
                "reference re-pointed with the entry: {:?}",
                t.source_id
            );
        }
        assert_eq!(loaded.translations[0].text.as_deref(), Some("метка вещи"));
        // Re-saving produces the v2 container.
        let resaved = dir.path().join("resaved.rimloc.json");
        save_project(&loaded, &resaved).unwrap();
        let text = std::fs::read_to_string(&resaved).unwrap();
        assert!(text.contains("\"schema_version\": 2"), "{text}");
    }

    /// The v1 collision case (one entry whose contexts show TWO def types)
    /// is rejected with the typed diagnostic; the original file bytes stay
    /// untouched (no silent split, no translation copied into both sides).
    #[test]
    fn v1_collapsed_collision_is_rejected_without_touching_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let v1 = r#"{
  "schema_version": 1,
  "context": { "view": "potential" },
  "entries": [
    {
      "id": { "kind": "def_injected", "key": "Dup.label" },
      "text": "thing label",
      "source_locale": "en",
      "contexts": [
        { "file": "Defs/A.xml", "line": 3, "def_type": "ThingDef", "role": "effective" },
        { "file": "Defs/B.xml", "line": 4, "def_type": "AbilityDef", "role": "overridden" }
      ],
      "provenance": { "patch_stage": "none" }
    }
  ],
  "translations": []
}
"#;
        let original = v1.to_string();
        std::fs::write(&path, &original).unwrap();
        let err = load_project(&path).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("ambiguous v1 entry") && msg.contains("ThingDef"),
            "{msg}"
        );
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            original,
            "failed load must not modify the file"
        );
    }

    /// v2 integrity (028-4): save VALIDATES before any write — an invalid
    /// state (duplicate identity, dangling reference) is rejected with the
    /// typed diagnostic and nothing lands on disk. The loader side is
    /// exercised through hand-crafted raw JSON (tests only — production
    /// save can no longer produce it).
    #[test]
    fn v2_validates_identity_and_reference_uniqueness() {
        let dir = tempfile::tempdir().unwrap();

        // Duplicate identity: rejected by save, nothing written.
        let mut p = sample();
        p.entries.push(SourceEntry {
            id: p.entries[0].id.clone(),
            text: "duplicate identity".into(),
            source_locale: "en".into(),
            contexts: Vec::new(),
            provenance: SourceProvenance::default(),
            tkey: None,
            source_ref: None,
        });
        let path = dir.path().join("dup.rimloc.json");
        let err = save_project(&p, &path).unwrap_err();
        assert!(
            err.to_string().contains("duplicate entry identity"),
            "{err}"
        );
        assert!(!path.exists(), "rejected save must not write the file");

        // Dangling translation reference: rejected by save, nothing written.
        let mut p2 = sample();
        let mut dangling = p2.translations.remove(0);
        dangling.source_id.key = "Ghost.key".into();
        p2.translations.push(dangling);
        let path2 = dir.path().join("dangling.rimloc.json");
        let err = save_project(&p2, &path2).unwrap_err();
        assert!(
            err.to_string().contains("references missing entry"),
            "{err}"
        );
        assert!(!path2.exists(), "rejected save must not write the file");

        // Hand-crafted broken v2 raw JSON (test-only construction) is
        // rejected by the loader; the bytes stay untouched.
        let path3 = dir.path().join("raw-dup.rimloc.json");
        let raw = r#"{
  "schema_version": 2,
  "context": { "view": "potential" },
  "entries": [
    {
      "id": { "kind": "t_key", "key": "Q.LetterLabel", "def_type": "QuestScriptDef" },
      "text": "Quest complete",
      "source_locale": "en",
      "contexts": [],
      "provenance": { "patch_stage": "none" }
    },
    {
      "id": { "kind": "t_key", "key": "Q.LetterLabel", "def_type": "QuestScriptDef" },
      "text": "Quest complete",
      "source_locale": "en",
      "contexts": [],
      "provenance": { "patch_stage": "none" }
    }
  ],
  "translations": []
}
"#;
        std::fs::write(&path3, raw).unwrap();
        let before = std::fs::read_to_string(&path3).unwrap();
        let err = load_project(&path3).unwrap_err();
        assert!(
            err.to_string().contains("duplicate entry identity"),
            "{err}"
        );
        assert_eq!(
            std::fs::read_to_string(&path3).unwrap(),
            before,
            "failed load must not modify the file"
        );
    }

    /// 028-1: structurally distinct identities that a concatenated string
    /// encoding would collide ("A|b"+None vs "A"+"b") are two real entries;
    /// each keeps its own translation through a save/load roundtrip.
    #[test]
    fn structurally_distinct_ids_never_collapse() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let mut p = Project::default();
        let mk = |key: &str, dt: Option<&str>| SourceEntryId {
            kind: EntryKind::DefInjected,
            key: key.into(),
            def_type: dt.map(str::to_string),
        };
        let id_a = mk("A|b", None);
        let id_b = mk("A", Some("b"));
        assert_ne!(id_a, id_b);
        for (id, text) in [(&id_a, "one"), (&id_b, "two")] {
            p.entries.push(SourceEntry {
                id: id.clone(),
                text: text.into(),
                source_locale: "en".into(),
                contexts: vec![],
                provenance: SourceProvenance::default(),
                tkey: None,
                source_ref: None,
            });
            p.update_translation(id.clone(), "ru", Some(format!("{text} ru")), Origin::Human);
        }
        save_project(&p, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(
            loaded.translation(&id_a, "ru").unwrap().text.as_deref(),
            Some("one ru")
        );
        assert_eq!(
            loaded.translation(&id_b, "ru").unwrap().text.as_deref(),
            Some("two ru")
        );
    }

    /// 028-3a: authentic pre-identity v1 entry — context def_type is empty
    /// but the effective context file is the canonical virtual DefInjected
    /// path — migrates to the typed identity, and the translation follows.
    #[test]
    fn v1_legacy_path_evidence_migrates_to_typed_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let v1 = r#"{
  "schema_version": 1,
  "context": { "view": "potential" },
  "entries": [
    {
      "id": { "kind": "def_injected", "key": "Widget.label" },
      "text": "a label",
      "source_locale": "en",
      "contexts": [
        {
          "file": "mod/Languages/English/DefInjected/ThingDef/Widget.xml",
          "line": null,
          "role": "effective"
        }
      ],
      "provenance": { "patch_stage": "none" }
    }
  ],
  "translations": [
    {
      "source_id": { "kind": "def_injected", "key": "Widget.label" },
      "locale": "ru", "text": "метка",
      "completeness": "translated", "review": "none",
      "validation": "unknown", "lifecycle": "active", "origin": "human"
    }
  ]
}
"#;
        std::fs::write(&path, v1).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(
            loaded.entries[0].id.def_type.as_deref(),
            Some("ThingDef"),
            "legacy DefInjected path evidence salvages the type"
        );
        assert_eq!(
            loaded.translations[0].source_id.def_type.as_deref(),
            Some("ThingDef"),
            "the translation reference follows the entry"
        );
    }

    /// 028-3b: TKey metadata contradicting the context def type is
    /// REJECTED (never silently guessed from one side); bytes untouched.
    #[test]
    fn v1_conflicting_evidence_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let v1 = r#"{
  "schema_version": 1,
  "context": { "view": "potential" },
  "entries": [
    {
      "id": { "kind": "t_key", "key": "Sample.LetterLabel" },
      "text": "quest text",
      "source_locale": "en",
      "contexts": [
        { "file": "Defs/Thing.xml", "line": 2, "def_type": "ThingDef", "role": "effective" }
      ],
      "provenance": { "patch_stage": "none" },
      "tkey": {
        "strategy": "slate_ref", "suffix": ".slateRef",
        "def_type": "QuestScriptDef", "contexts": 1
      }
    }
  ],
  "translations": []
}
"#;
        let original = v1.to_string();
        std::fs::write(&path, &original).unwrap();
        let err = load_project(&path).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("contradictory def-type evidence"), "{msg}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    /// 035: an authentic pre-feb v1 collapsed collision carries TWO
    /// untyped contexts whose virtual paths prove TWO def types — the
    /// migration must reject it (both orders), never silently take the
    /// first path's type; the bytes stay untouched.
    #[test]
    fn v1_collapsed_two_path_collision_is_rejected_both_orders() {
        let entry_contexts = |first: &str, second: &str| {
            format!(
                r#"{{
  "schema_version": 1,
  "context": {{ "view": "potential" }},
  "entries": [
    {{
      "id": {{ "kind": "def_injected", "key": "Wild.label" }},
      "text": "some label",
      "source_locale": "en",
      "contexts": [
        {{
          "file": "mod/Languages/English/DefInjected/{first}/Wild.xml",
          "line": null,
          "role": "effective"
        }},
        {{
          "file": "mod/Languages/English/DefInjected/{second}/Wild.xml",
          "line": null,
          "role": "overridden"
        }}
      ],
      "provenance": {{ "patch_stage": "none" }}
    }}
  ],
  "translations": [
    {{
      "source_id": {{ "kind": "def_injected", "key": "Wild.label" }},
      "locale": "ru", "text": "старый перевод",
      "completeness": "translated", "review": "none",
      "validation": "unknown", "lifecycle": "active", "origin": "human"
    }}
  ]
}}
"#
            )
        };
        let dir = tempfile::tempdir().unwrap();
        for (order, first, second) in [
            ("thingdef-first", "ThingDef", "PawnKindDef"),
            ("pawnkinddef-first", "PawnKindDef", "ThingDef"),
        ] {
            let path = dir.path().join(format!("{order}.rimloc.json"));
            let raw = entry_contexts(first, second);
            std::fs::write(&path, &raw).unwrap();
            let err = load_project(&path).unwrap_err();
            let msg = format!("{err:#}");
            assert!(
                msg.contains("ambiguous v1 entry")
                    && msg.contains("ThingDef")
                    && msg.contains("PawnKindDef"),
                "[{order}] {msg}"
            );
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                raw,
                "[{order}] failed load must not modify the file"
            );
        }
    }

    /// Consistent multiple legacy paths (same def type, several contexts)
    /// migrate normally — only genuinely conflicting types fail.
    #[test]
    fn v1_consistent_multiple_legacy_paths_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");
        let v1 = r#"{
  "schema_version": 1,
  "context": { "view": "potential" },
  "entries": [
    {
      "id": { "kind": "def_injected", "key": "Wild.label" },
      "text": "some label",
      "source_locale": "en",
      "contexts": [
        {
          "file": "mod/Languages/English/DefInjected/ThingDef/Wild.xml",
          "line": null,
          "role": "effective"
        },
        {
          "file": "mod/Languages/English/DefInjected/ThingDef/Wild2.xml",
          "line": null,
          "role": "overridden"
        }
      ],
      "provenance": { "patch_stage": "none" }
    }
  ],
  "translations": [
    {
      "source_id": { "kind": "def_injected", "key": "Wild.label" },
      "locale": "ru", "text": "метка",
      "completeness": "translated", "review": "none",
      "validation": "unknown", "lifecycle": "active", "origin": "human"
    }
  ]
}
"#;
        std::fs::write(&path, v1).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded.entries[0].id.def_type.as_deref(), Some("ThingDef"));
        assert_eq!(
            loaded.translations[0].source_id.def_type.as_deref(),
            Some("ThingDef")
        );
    }
    /// Binding envelope (033): the durable id/revision ride the SAME
    /// atomic record as the content; bare saves keep writing legacy-shaped
    /// v2 files that still load.
    #[test]
    fn envelope_meta_round_trips_and_stays_additive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.rimloc.json");

        // Bare save (no meta): file has no envelope fields, loads fine.
        save_project(&sample(), &path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("project_id"), "{text}");
        assert!(!text.contains("\"revision\""), "{text}");
        let bare = load_project(&path).unwrap();
        assert_eq!(bare.entries.len(), sample().entries.len());

        // Meta save: id + revision + display name ride along. The M3
        // source fingerprint rides the same envelope and round-trips.
        let meta = ProjectEnvelopeMeta {
            project_id: Some("proj-abc".into()),
            revision: Some(7),
            display_name: Some("My Mod".into()),
            source_root: None,
            source_fingerprint: Some("a".repeat(64)),
        };
        save_project_with_meta(&sample(), &meta, &path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("\"schema_version\": 2"),
            "still the v2 container: {text}"
        );
        let loaded = load_project_with_meta(&path).unwrap();
        assert_eq!(loaded.meta.project_id.as_deref(), Some("proj-abc"));
        assert_eq!(loaded.meta.revision, Some(7));
        assert_eq!(loaded.meta.display_name.as_deref(), Some("My Mod"));
        assert_eq!(
            loaded.meta.source_fingerprint.as_deref(),
            Some("a".repeat(64).as_str())
        );

        // Old-shaped bare v2 (no envelope fields) still loads with defaults.
        let bare_v2 = "{\"schema_version\": 2, \"context\": {\"view\": \"potential\"}, \"entries\": [], \"translations\": []}\n"
            .to_string();
        std::fs::write(&path, bare_v2).unwrap();
        let loaded = load_project(&path).unwrap();
        assert!(loaded.entries.is_empty());
    }
}
