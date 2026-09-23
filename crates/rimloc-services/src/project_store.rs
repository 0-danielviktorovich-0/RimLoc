//! Project persistence POC (Gate I3): canonical `Project` ⇄ disk.
//!
//! Evidence-based choice for the POC: a single versioned JSON file with
//! deterministic serialization — portable, human-readable, zero new
//! dependencies, trivially diffable. SQLite stays DEFERRED until real
//! query/scale needs appear (documented in CANONICAL_PROJECT_MODEL.md);
//! the store interface below is the seam that would change.
//!
//! Crash safety: writes go through `write_atomic` (tmp → rename), so a
//! crash never leaves a half-written project file.

use crate::Result;
use rimloc_domain::canonical::Project;
use std::path::Path;

pub const PROJECT_FILE_VERSION: u32 = 1;

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

/// Save atomically: tmp file → rename, never a torn project file.
pub fn save_project(project: &Project, path: &Path) -> std::io::Result<()> {
    let bytes = serialize_project(project)?;
    crate::write_atomic(path, &bytes)
}

/// Load a project file, checking the container version. Unknown domain
/// fields are rejected by default (the model is frozen before GUI binding;
/// loosening that is a deliberate later decision with a migration note).
pub fn load_project(path: &Path) -> Result<Project> {
    let text = std::fs::read_to_string(path)?;
    let file: ProjectFile = serde_json::from_str(&text)?;
    if file.schema_version != PROJECT_FILE_VERSION {
        return Err(color_eyre::eyre::eyre!(
            "project file schema_version {} not supported (expected {})",
            file.schema_version,
            PROJECT_FILE_VERSION
        ));
    }
    Ok(file.project)
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
                },
                tkey: Some(rimloc_core::TKeyMeta {
                    strategy: "slate_ref".into(),
                    suffix: ".slateRef".into(),
                    def_type: "QuestScriptDef".into(),
                    contexts: 1,
                }),
            }],
            translations: vec![Translation {
                source_id: SourceEntryId {
                    kind: EntryKind::TKey,
                    key: "Q.LetterLabel".into(),
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
        // Tamper the container version.
        let s = String::from_utf8(bytes).unwrap().replacen("1,", "99,", 1);
        std::fs::write(&path, s).unwrap();
        let err = load_project(&path).unwrap_err();
        assert!(err.to_string().contains("schema_version"), "{err}");
    }
}
