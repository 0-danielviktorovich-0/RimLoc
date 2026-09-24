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

/// Def type segment of a DefInjected path (real sidecar or the canonical
/// virtual output path the scan builds for Defs-derived units):
/// `.../DefInjected/ThingDef/X.xml` -> "ThingDef". Exact case — def types
/// that differ only by case are different types, never folded.
pub(crate) fn definjected_def_type(path: &Path) -> Option<String> {
    definjected_def_type_str(&path.to_string_lossy())
}

/// String form of [`definjected_def_type`] for callers holding raw path
/// text (persistence migration evidence).
pub(crate) fn definjected_def_type_str(path: &str) -> Option<String> {
    let s = path.replace('\\', "/");
    let i = s.find("/DefInjected/")?;
    s[i + "/DefInjected/".len()..]
        .split('/')
        .next()
        .filter(|seg| !seg.is_empty())
        .map(str::to_string)
}

/// The def-type discriminator of a unit, for kinds that are def-type-scoped
/// (DefInjected, TKey). Everything else stays `None` — a Keyed key has no
/// def-type scope, and an unknown type is never turned into a fake
/// identity.
fn def_type_discriminator(kind: EntryKind, u: &TransUnit) -> Option<String> {
    match kind {
        EntryKind::DefInjected | EntryKind::TKey => u
            .tkey
            .as_ref()
            .map(|m| m.def_type.clone())
            .or_else(|| definjected_def_type(&u.path)),
        _ => None,
    }
}

/// Source context of one unit: the REAL effective source file when the scan
/// recorded one (Defs-derived units carry the virtual output path in
/// `path`), honest location only — a line is present where the parser
/// computed one, never fabricated (Source Inspector mandate §1).
fn context_parts(u: &TransUnit) -> (String, Option<usize>) {
    match &u.src {
        Some(src) => (src.file.display().to_string(), src.line.or(u.line)),
        None => (u.path.display().to_string(), u.line),
    }
}

/// Convert an effective inventory into canonical source entries. Units that
/// share one identity (same kind+key) collapse into one entry whose first
/// context is Effective and the rest Overridden (Gate H diagnostics).
///
/// Winner-reason provenance is per entry: a unit's own `selected_by`
/// (stamped by the scan pipeline at the decision point) is the truth; the
/// `selected_by` parameter is only a batch-level FALLBACK for inventories
/// whose winner reason is genuinely uniform (e.g. LoadFolders resolution
/// chose every root). Pass `None` rather than a label that would be true for
/// only part of the inventory.
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
        // Def-type discriminator from REAL provenance only (TKey metadata or
        // the DefInjected path segment). Two def types sharing one logical
        // key are two distinct Defs — separate entries, never a collapsed
        // one. Kinds without a def-type scope (Keyed) stay `None`; an
        // unknown type is never guessed into a "Misc" identity.
        let id = SourceEntryId {
            def_type: def_type_discriminator(kind, u),
            kind,
            key: u.key.clone(),
        };
        let (file, line) = context_parts(u);
        let ctx = SourceContext {
            file,
            line,
            // TKey metadata wins; otherwise the DefInjected path segment
            // names the owning def type (sidecar or canonical virtual path).
            def_type: u
                .tkey
                .as_ref()
                .map(|m| m.def_type.clone())
                .or_else(|| definjected_def_type(&u.path)),
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
                // Mandate §14 (TKey/multi-context): Primary location + Other
                // usages — never pretend to be one source. A TKey identity
                // shared by several same-file nodes keeps every node's real
                // location; the primary (last assignment, effective text)
                // leads, earlier nodes follow as overridden usages. Rejected
                // cross-file duplicates are NOT here (the parser never
                // records their locations).
                let mut contexts = vec![ctx];
                if let Some(meta) = &u.tkey {
                    if meta.locations.len() > 1 {
                        let def_type = Some(meta.def_type.clone());
                        for loc in &meta.locations[..meta.locations.len() - 1] {
                            contexts.push(SourceContext {
                                file: loc.file.display().to_string(),
                                line: loc.line,
                                def_type: def_type.clone(),
                                role: ContextRole::Overridden,
                            });
                        }
                    }
                }
                order.push(id.clone());
                by_id.insert(
                    id.clone(),
                    SourceEntry {
                        id,
                        text,
                        source_locale: "en".into(),
                        contexts,
                        provenance: SourceProvenance {
                            version_selected: version.map(String::from),
                            conditional_branch: u.conditional,
                            patch_stage,
                            selected_by: u
                                .selected_by
                                .clone()
                                .or_else(|| selected_by.map(String::from)),
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
            ..Default::default()
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
                    locations: Vec::new(),
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
