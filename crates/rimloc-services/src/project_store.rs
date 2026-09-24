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
        }
    }
}

impl std::error::Error for ProjectLoadDiagnostic {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectFile {
    /// Persistence container version (transport), independent of the domain
    /// model evolution handled by serde defaults.
    schema_version: u32,
    #[serde(flatten)]
    project: Project,
}

/// Serialize deterministically (BTree/Vec field order is stable; maps sorted
/// via BTreeMap inside the model). Pretty-printed for diffability.
pub fn serialize_project(project: &Project) -> std::io::Result<Vec<u8>> {
    let file = ProjectFile {
        schema_version: PROJECT_FILE_VERSION,
        project: project.clone(),
    };
    let s = serde_json::to_string_pretty(&file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut out = s.into_bytes();
    out.push(b'\n');
    Ok(out)
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
    // fresh typed identities.
    if let Some(c) = entry.contexts.first() {
        if let Some(dt) = crate::canonical_bridge::definjected_def_type_str(&c.file) {
            push(&mut sources, Some(&dt));
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
    use rimloc_domain::canonical::SourceEntryId;

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
}
