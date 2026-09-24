//! Bridge: scan-world `TransUnit` → canonical `SourceEntry` (Gate I2).
//!
//! The scan pipeline stays the single producer of inventory; this module is
//! the ONLY place that converts its output into the canonical model, so kind
//! inference and provenance rules live in one spot.

use rimloc_core::TransUnit;
use rimloc_domain::canonical::{
    ContextRole, EntryKind, PatchStage, Project, SourceContext, SourceEntry, SourceEntryId,
    SourceProvenance, ViewLabel,
};
use std::collections::BTreeMap;
use std::path::Path;

fn kind_for(path: &Path, tkey: bool) -> EntryKind {
    let s = path.to_string_lossy().replace('\\', "/");
    if tkey {
        return EntryKind::TKey;
    }
    if s.contains("/Keyed/") {
        EntryKind::Keyed
    } else if s.contains("/DefInjected/") {
        EntryKind::DefInjected
    } else if s.contains("/Strings/") {
        EntryKind::Strings
    } else if s.contains("/Backstories/") {
        EntryKind::Backstories
    } else if s.contains("/Patches/") {
        EntryKind::PatchDerived
    } else {
        // Defs-derived units (already rewritten to DefInjected target paths by
        // the canonical scan) are ordinary DefInjected entries.
        EntryKind::DefInjected
    }
}

/// Convert an effective inventory into canonical source entries. Units that
/// share one identity (same kind+key) collapse into one entry whose first
/// context is Effective and the rest Overridden (Gate H diagnostics).
///
/// `selected_by` records WHY the effective occurrence won (winner-reason
/// provenance vocabulary: "version-selected", "loadfolders", "first-file-wins",
/// "keyed-last-wins", "patch-applied"). Pass `None` when the batch-level
/// winner reason is per-family and unknown at this granularity — never a
/// label that would be true for only part of the inventory.
pub fn source_entries(
    units: &[TransUnit],
    patch_stage: PatchStage,
    version: Option<&str>,
    selected_by: Option<&str>,
) -> Vec<SourceEntry> {
    let mut order: Vec<SourceEntryId> = Vec::new();
    let mut by_id: BTreeMap<SourceEntryId, SourceEntry> = BTreeMap::new();
    for u in units {
        let kind = kind_for(&u.path, u.tkey.is_some());
        let id = SourceEntryId {
            kind,
            key: u.key.clone(),
        };
        let ctx = SourceContext {
            file: u.path.display().to_string(),
            line: u.line,
            def_type: u.tkey.as_ref().map(|m| m.def_type.clone()),
            role: ContextRole::Effective,
        };
        let text = u.source.clone().unwrap_or_default();
        match by_id.get_mut(&id) {
            Some(e) => {
                // Later duplicate occurrences in the effective inventory keep
                // the effective-first ordering; record as overridden context.
                e.contexts.push(SourceContext {
                    role: ContextRole::Overridden,
                    ..ctx
                });
            }
            None => {
                order.push(id.clone());
                by_id.insert(
                    id.clone(),
                    SourceEntry {
                        id,
                        text,
                        source_locale: "en".into(),
                        contexts: vec![ctx],
                        provenance: SourceProvenance {
                            version_selected: version.map(String::from),
                            conditional_branch: false,
                            patch_stage,
                            selected_by: selected_by.map(String::from),
                        },
                        tkey: u.tkey.clone(),
                    },
                );
            }
        }
    }
    order
        .into_iter()
        .map(|id| by_id.remove(&id).unwrap())
        .collect()
}

/// Assemble a canonical project snapshot from an effective inventory.
pub fn project_from_inventory(
    units: &[TransUnit],
    patch_stage: PatchStage,
    version: Option<&str>,
    selected_by: Option<&str>,
    context: rimloc_domain::canonical::InventoryContext,
) -> Project {
    let mut p = Project {
        context,
        entries: source_entries(units, patch_stage, version, selected_by),
        translations: Vec::new(),
    };
    if p.context.view == ViewLabel::Potential && p.context.target_version.is_none() {
        // Without a known version the view can never be Exact.
        p.context.view = ViewLabel::Potential;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use rimloc_core::TKeyMeta;

    fn unit(key: &str, text: &str, path: &str, tkey: Option<TKeyMeta>) -> TransUnit {
        TransUnit {
            key: key.into(),
            source: Some(text.into()),
            path: Path::new(path).to_path_buf(),
            line: Some(7),
            tkey,
        }
    }

    #[test]
    fn kinds_and_tkey_metadata_survive_the_bridge() {
        let units = vec![
            unit(
                "Greeting",
                "hello",
                "/Mod/Languages/English/Keyed/K.xml",
                None,
            ),
            unit(
                "Widget.label",
                "a label",
                "/Mod/Languages/English/DefInjected/ThingDef/W.xml",
                None,
            ),
            unit(
                "SampleQuest.LetterLabel.slateRef",
                "letter",
                "/Mod/Defs/Q.xml",
                Some(TKeyMeta {
                    strategy: "slate_ref".into(),
                    suffix: ".slateRef".into(),
                    def_type: "QuestScriptDef".into(),
                    contexts: 1,
                }),
            ),
        ];
        let entries = source_entries(&units, PatchStage::None, Some("1.6"), Some("loadfolders"));
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id.kind, EntryKind::Keyed);
        assert_eq!(entries[1].id.kind, EntryKind::DefInjected);
        assert_eq!(entries[2].id.kind, EntryKind::TKey);
        assert_eq!(entries[2].tkey.as_ref().unwrap().suffix, ".slateRef");
        assert_eq!(
            entries[2].provenance.version_selected.as_deref(),
            Some("1.6")
        );
        // Winner-reason provenance reaches every entry of the batch.
        assert!(entries
            .iter()
            .all(|e| e.provenance.selected_by.as_deref() == Some("loadfolders")));
    }

    /// One identity, two occurrences -> ONE entry, second context Overridden.
    #[test]
    fn duplicate_occurrences_collapse_into_contexts() {
        let units = vec![
            unit(
                "Dup.key",
                "first",
                "/Mod/Languages/English/Keyed/A.xml",
                None,
            ),
            unit(
                "Dup.key",
                "second",
                "/Mod/Languages/English/Keyed/Z.xml",
                None,
            ),
        ];
        let entries = source_entries(&units, PatchStage::None, None, None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "first");
        assert_eq!(entries[0].contexts.len(), 2);
        assert_eq!(entries[0].contexts[0].role, ContextRole::Effective);
        assert_eq!(entries[0].contexts[1].role, ContextRole::Overridden);
    }

    #[test]
    fn project_snapshot_labels_view_potential_without_version() {
        let units = vec![unit("K", "v", "/Mod/Languages/English/Keyed/K.xml", None)];
        let p = project_from_inventory(
            &units,
            PatchStage::None,
            None,
            None,
            rimloc_domain::canonical::InventoryContext::default(),
        );
        assert_eq!(p.context.view, ViewLabel::Potential);
        assert!(p.entries[0].provenance.selected_by.is_none());
        let p2 = project_from_inventory(
            &units,
            PatchStage::Applied,
            Some("1.6"),
            Some("version-selected"),
            rimloc_domain::canonical::InventoryContext {
                target_version: Some("1.6".into()),
                view: ViewLabel::Exact,
                ..Default::default()
            },
        );
        assert_eq!(p2.context.view, ViewLabel::Exact);
        assert_eq!(
            p2.entries[0].provenance.selected_by.as_deref(),
            Some("version-selected")
        );
    }
}
