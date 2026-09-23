//! Canonical project workflows (Gate I4): build a project from a mod,
//! apply an existing translation, and write RimWorld output — WITHOUT any
//! PO intermediate. PO import/export stays a separate adapter concern.

use crate::Result;
use rimloc_domain::canonical as dom;
use rimloc_domain::canonical::{
    Completeness, EntryKind, InventoryContext, Origin, PatchStage, Project, SourceEntryId,
    Translation, ViewLabel,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Build a canonical project snapshot for a mod (Gate I4 shared entry).
pub fn build_project(mod_root: &Path, target_version: Option<&str>) -> Result<Project> {
    let auto = crate::autodiscover_defs_context(mod_root)?;
    let (units, patch_report) = if mod_root.join("LoadFolders.xml").is_file() {
        let units =
            crate::scan_units_effective(mod_root, target_version, &auto.dict, &auto.extra_fields)?;
        // scan_units_effective already applied patches internally; derive the
        // stage marker from whether any Patches dir exists in content dirs.
        (
            units,
            if mod_root.join("Patches").is_dir() {
                crate::patches_effect::PatchCoverage::Partial
            } else {
                crate::patches_effect::PatchCoverage::None
            },
        )
    } else {
        let units =
            crate::scan_units_with_defs_and_dict(mod_root, None, &auto.dict, &auto.extra_fields)?;
        (
            units,
            if mod_root.join("Patches").is_dir() {
                crate::patches_effect::PatchCoverage::Partial
            } else {
                crate::patches_effect::PatchCoverage::None
            },
        )
    };
    let stage = match patch_report {
        crate::patches_effect::PatchCoverage::None => PatchStage::None,
        crate::patches_effect::PatchCoverage::Full => PatchStage::Applied,
        crate::patches_effect::PatchCoverage::Partial => PatchStage::Partial,
    };
    // Exact view requires a known version; otherwise honest POTENTIAL.
    let view = if target_version.is_some() {
        ViewLabel::Exact
    } else {
        ViewLabel::Potential
    };
    let context = InventoryContext {
        target_version: target_version.map(String::from),
        view,
        ..Default::default()
    };
    Ok(crate::canonical_bridge::project_from_inventory(
        &units,
        stage,
        target_version,
        context,
    ))
}

/// Workflow C seed: import an existing translation pack into the project.
/// Resolution goes through the canonical matcher (SourceMatcher): exact,
/// proven aliases, then TKey-suffix fallback — never shape stripping.
/// Every match is recorded with origin=Imported; nothing is overwritten
/// silently (existing translations win only if the slot was empty).
pub fn apply_existing_translation(
    project: &mut Project,
    pack_root: &Path,
    locale: &str,
) -> Result<usize> {
    use crate::matching::SourceMatcher;

    // Source-side units reconstructed from canonical entries.
    let src_units: Vec<rimloc_core::TransUnit> = project
        .entries
        .iter()
        .map(|e| rimloc_core::TransUnit {
            key: e.id.key.clone(),
            source: Some(e.text.clone()),
            path: PathBuf::from(
                e.contexts
                    .first()
                    .map(|c| c.file.clone())
                    .unwrap_or_default(),
            ),
            line: None,
            tkey: None,
        })
        .collect();
    let registry = crate::matching::TKeyRegistry::from_identities(
        project
            .entries
            .iter()
            .filter(|e| e.id.kind == EntryKind::TKey)
            .map(|e| e.id.key.clone()),
    );
    let matcher = SourceMatcher::new(&src_units, &registry);

    let pack_units = rimloc_parsers_xml::scan_keyed_xml(pack_root)?;
    let mut applied = 0usize;
    for u in &pack_units {
        let Some(text) = u.source.as_deref().map(str::trim).filter(|t| !t.is_empty()) else {
            continue;
        };
        if let Some(source_key) = matcher.source_for_target(&u.key) {
            let kind = if project
                .entries
                .iter()
                .any(|e| e.id.kind == EntryKind::TKey && e.id.key == source_key)
            {
                EntryKind::TKey
            } else {
                EntryKind::DefInjected
            };
            let id = SourceEntryId {
                kind,
                key: source_key,
            };
            if project.translation(&id, locale).is_none() {
                project.translations.push(Translation {
                    source_id: id,
                    locale: locale.to_string(),
                    text: Some(text.to_string()),
                    completeness: if text.eq_ignore_ascii_case("TODO") {
                        Completeness::Todo
                    } else {
                        Completeness::Translated
                    },
                    review: dom::Review::None,
                    validation: dom::ValidationState::Unknown,
                    lifecycle: dom::Lifecycle::Active,
                    origin: Origin::Imported,
                    notes: String::new(),
                    source_changed: None,
                });
                applied += 1;
            }
        }
    }
    Ok(applied)
}

/// Workflow A final step: write RimWorld translation output straight from
/// the canonical project — NO PO intermediate anywhere.
///
/// Layout mirrors the official pack conventions:
/// - Keyed entries → `Keyed/<locale>.xml` (flat elements, key = entry key);
/// - DefInjected/TKey entries → `DefInjected/<DefType>/<defName>.xml`,
///   element name = key (+ TKey suffix when the kind is TKey).
pub fn write_rimworld_translation(
    project: &Project,
    out_mod: &Path,
    lang_dir: &str,
    mod_name: &str,
    package_id: &str,
    rw_version: &str,
) -> Result<PathBuf> {
    use std::fmt::Write as _;

    let base = out_mod.join("Languages").join(lang_dir);
    // defName -> file; collected per def type from entry keys/contexts.
    let mut keyed: BTreeMap<String, String> = BTreeMap::new();
    let mut definj: BTreeMap<(String, String, String), BTreeMap<String, String>> = BTreeMap::new(); // (def_type, def_name, file) -> (element, text)

    for t in &project.translations {
        if t.locale != lang_dir && !t.locale.is_empty() {
            // Locale is the target folder name; keep entries for it only.
        }
        if t.locale != lang_dir {
            continue;
        }
        let Some(text) = t.text.as_deref().filter(|s| !s.trim().is_empty()) else {
            continue;
        };
        let Some(entry) = project.entries.iter().find(|e| e.id == t.source_id) else {
            continue;
        };
        if t.lifecycle == dom::Lifecycle::Obsolete {
            continue;
        }
        match entry.id.kind {
            EntryKind::Keyed => {
                keyed.insert(entry.id.key.clone(), text.to_string());
            }
            EntryKind::TKey | EntryKind::DefInjected => {
                let def_type = entry
                    .tkey
                    .as_ref()
                    .map(|m| m.def_type.clone())
                    .or_else(|| def_type_from_contexts(entry))
                    .unwrap_or_else(|| "Misc".into());
                let def_name = entry
                    .id
                    .key
                    .split('.')
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let element = match &entry.tkey {
                    Some(m) => format!("{}{}", entry.id.key, m.suffix),
                    None => entry.id.key.clone(),
                };
                let file = format!("{def_name}.xml");
                definj
                    .entry((def_type, def_name, file))
                    .or_default()
                    .insert(element, text.to_string());
            }
            _ => {
                // Strings/Backstories/PatchDerived writers land with their
                // gates; not part of this acceptance.
            }
        }
    }

    // About.xml
    let about = out_mod.join("About");
    std::fs::create_dir_all(&about)?;
    let mut about_xml = String::new();
    let _ = writeln!(
        about_xml,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<RimWorldManifest>\n  <name>{mod_name}</name>\n  <packageId>{package_id}</packageId>\n  <supportedVersions>\n    <li>{rw_version}</li>\n  </supportedVersions>\n</RimWorldManifest>"
    );
    crate::write_atomic(&about.join("About.xml"), about_xml.as_bytes())?;

    // Keyed
    if !keyed.is_empty() {
        let dir = base.join("Keyed");
        std::fs::create_dir_all(&dir)?;
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<LanguageData>\n");
        for (k, v) in &keyed {
            let _ = writeln!(xml, "  <{k}>{}</{k}>", escape_xml(v));
        }
        xml.push_str("</LanguageData>\n");
        crate::write_atomic(&dir.join("Translation.xml"), xml.as_bytes())?;
    }

    // DefInjected
    for ((def_type, _def_name, file), items) in &definj {
        let dir = base.join("DefInjected").join(def_type);
        std::fs::create_dir_all(&dir)?;
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<LanguageData>\n");
        for (element, text) in items {
            let _ = writeln!(xml, "  <{element}>{}</{element}>", escape_xml(text));
        }
        xml.push_str("</LanguageData>\n");
        crate::write_atomic(&dir.join(file), xml.as_bytes())?;
    }

    Ok(out_mod.to_path_buf())
}

fn def_type_from_contexts(entry: &rimloc_domain::canonical::SourceEntry) -> Option<String> {
    for c in &entry.contexts {
        let s = c.file.replace('\\', "/");
        if let Some(i) = s.find("/DefInjected/") {
            let rest = &s[i + "/DefInjected/".len()..];
            return Some(rest.split('/').next().unwrap_or_default().to_string());
        }
    }
    None
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Source provenance stays reachable for diagnostics without per-entry noise.
pub fn provenance_summary(project: &Project) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for e in &project.entries {
        let key = format!(
            "version={};conditional={};patch={}",
            e.provenance.version_selected.as_deref().unwrap_or("-"),
            e.provenance.conditional_branch,
            match e.provenance.patch_stage {
                PatchStage::None => "none",
                PatchStage::Applied => "applied",
                PatchStage::Partial => "partial",
            }
        );
        *out.entry(key).or_default() += 1;
    }
    out
}

#[cfg(test)]
mod gate_i_tests {
    use super::*;
    use rimloc_domain::canonical::SourceProvenance;
    use rimloc_domain::canonical::{SourceEntry, Translation};

    fn project_with_translation(
        key: &str,
        kind: EntryKind,
        text: &str,
        tkey: Option<rimloc_core::TKeyMeta>,
    ) -> Project {
        let id = SourceEntryId {
            kind,
            key: key.into(),
        };
        Project {
            entries: vec![SourceEntry {
                id: id.clone(),
                text: "source".into(),
                source_locale: "en".into(),
                contexts: vec![],
                provenance: SourceProvenance::default(),
                tkey,
            }],
            translations: vec![Translation {
                source_id: id,
                locale: "Russian".into(),
                text: Some(text.into()),
                completeness: Completeness::Translated,
                review: dom::Review::None,
                validation: dom::ValidationState::Unknown,
                lifecycle: dom::Lifecycle::Active,
                origin: Origin::Human,
                notes: String::new(),
                source_changed: None,
            }],
            ..Default::default()
        }
    }

    /// Workflow A core proof: project -> RimWorld output WITHOUT any PO.
    #[test]
    fn write_rimworld_emits_definjected_and_keyed_from_project() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("mod");
        let tk = rimloc_core::TKeyMeta {
            strategy: "slate_ref".into(),
            suffix: ".slateRef".into(),
            def_type: "QuestScriptDef".into(),
            contexts: 1,
        };
        let mut p = project_with_translation(
            "SampleQuest.LetterLabel",
            EntryKind::TKey,
            "Метка",
            Some(tk.clone()),
        );
        p.translations.push(Translation {
            source_id: SourceEntryId {
                kind: EntryKind::Keyed,
                key: "Greeting".into(),
            },
            locale: "Russian".into(),
            text: Some("Привет".into()),
            completeness: Completeness::Translated,
            review: dom::Review::None,
            validation: dom::ValidationState::Unknown,
            lifecycle: dom::Lifecycle::Active,
            origin: Origin::Human,
            notes: String::new(),
            source_changed: None,
        });
        p.entries.push(SourceEntry {
            id: SourceEntryId {
                kind: EntryKind::Keyed,
                key: "Greeting".into(),
            },
            text: "hello".into(),
            source_locale: "en".into(),
            contexts: vec![],
            provenance: SourceProvenance::default(),
            tkey: None,
        });
        let out = write_rimworld_translation(&p, &out, "Russian", "T", "t.test", "1.6").unwrap();
        let definj = std::fs::read_to_string(
            out.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
        )
        .unwrap();
        assert!(
            definj.contains("<SampleQuest.LetterLabel.slateRef>Метка</"),
            "{definj}"
        );
        let keyed =
            std::fs::read_to_string(out.join("Languages/Russian/Keyed/Translation.xml")).unwrap();
        assert!(keyed.contains("<Greeting>Привет</Greeting>"), "{keyed}");
    }
}

#[cfg(test)]
mod gate_i4_acceptance {
    use super::*;
    use rimloc_domain::canonical::EntryKind;

    const FIXTURE: &str = "test/TKeyMod";

    fn fixture_path() -> PathBuf {
        // Integration-style: run against the repo fixture (crate dir -> repo
        // root -> test/).
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(FIXTURE)
            .canonicalize()
            .expect("fixture exists")
    }

    /// THE Gate I acceptance: all three workflows over ONE canonical model.
    ///
    /// A. native/no-PO:  source -> project -> apply existing RU -> write.
    /// B. PO interop:    project -> PO file -> import -> project -> write.
    /// C. existing pack: covered by A's import step (preserve + TODO parity),
    ///    plus reopen via the persistence store between workflows.
    #[test]
    fn three_workflows_over_one_canonical_project() {
        let root = fixture_path();
        let tmp = tempfile::tempdir().unwrap();

        // ---------- Workflow A: build + import existing RU + write ----------
        let mut project = build_project(&root, Some("1.6")).unwrap();
        // The view is EXACT for a known version; patches absent -> stage None.
        assert_eq!(project.context.view, ViewLabel::Exact);
        let ru_dir = root.join("Languages/Russian");
        let applied = apply_existing_translation(&mut project, &ru_dir, "Russian").unwrap();
        assert!(applied >= 4, "existing RU entries must map, got {applied}");

        // Save/reopen between workflows (project persistence, Gate I3).
        let project_file = tmp.path().join("project.rimloc.json");
        crate::project_store::save_project(&project, &project_file).unwrap();
        let project = crate::project_store::load_project(&project_file).unwrap();

        let out_a = tmp.path().join("out-a");
        write_rimworld_translation(&project, &out_a, "Russian", "T", "t.a", "1.6").unwrap();
        let quest_a = std::fs::read_to_string(
            out_a.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
        )
        .unwrap();
        assert!(
            quest_a.contains("<SampleQuest.LetterLabelFavorReceiver.slateRef>Метка услуги<"),
            "{quest_a}"
        );
        assert!(
            quest_a.contains("<SampleQuest.LetterTextParms.value.slateRef>Пармс-текст.<"),
            "{quest_a}"
        );
        assert!(
            quest_a.contains("<SampleQuest.ExpiryTip.slateRef>TODO</"),
            "{quest_a}"
        );
        let tips_a = std::fs::read_to_string(
            out_a.join("Languages/Russian/DefInjected/TipSetDef/SampleTips.xml"),
        )
        .unwrap();
        assert!(
            tips_a.contains("<SampleTips.DismissLetters>Подсказки"),
            "{tips_a}"
        );

        // ---------- Workflow B: PO interoperability adapter ----------
        // project -> PO file (adapter export over canonical translations)...
        let po_path = tmp.path().join("interop.po");
        let po_units: Vec<rimloc_core::TransUnit> = project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .filter_map(|t| {
                project
                    .entries
                    .iter()
                    .find(|e| e.id == t.source_id)
                    .map(|e| rimloc_core::TransUnit {
                        key: match (&e.id.kind, &e.tkey) {
                            (EntryKind::TKey, Some(m)) => format!("{}{}", e.id.key, m.suffix),
                            _ => e.id.key.clone(),
                        },
                        source: Some(e.text.clone()),
                        path: PathBuf::from(format!(
                            "Languages/Russian/DefInjected/{}.xml",
                            e.tkey
                                .as_ref()
                                .map(|m| m.def_type.clone())
                                .unwrap_or_else(|| "Misc".into())
                        )),
                        line: None,
                        tkey: None,
                    })
            })
            .collect();
        // msgstr filled from the project's own translations (as if a
        // translator had completed them in an external PO editor).
        let tm_map: std::collections::HashMap<String, String> = project
            .translations
            .iter()
            .filter(|t| t.locale == "Russian")
            .filter_map(|t| {
                project
                    .entries
                    .iter()
                    .find(|e| e.id == t.source_id)
                    .and_then(|e| {
                        t.text.clone().map(|text| {
                            let key = match (&e.id.kind, &e.tkey) {
                                (EntryKind::TKey, Some(m)) => {
                                    format!("{}{}", e.id.key, m.suffix)
                                }
                                _ => e.id.key.clone(),
                            };
                            (key, text)
                        })
                    })
            })
            .collect();
        rimloc_export_po::write_po_with_tm(&po_path, &po_units, Some("ru"), Some(&tm_map)).unwrap();
        // ...external-like import into a FRESH project (same source scan)...
        let mut project_b = build_project(&root, Some("1.6")).unwrap();
        let entries = rimloc_import_po::read_po_entries(&po_path).unwrap();
        let matcher_units: Vec<rimloc_core::TransUnit> = project_b
            .entries
            .iter()
            .map(|e| rimloc_core::TransUnit {
                key: e.id.key.clone(),
                source: Some(e.text.clone()),
                path: PathBuf::new(),
                line: None,
                tkey: None,
            })
            .collect();
        let registry = crate::matching::TKeyRegistry::from_identities(
            project_b
                .entries
                .iter()
                .filter(|e| e.id.kind == EntryKind::TKey)
                .map(|e| e.id.key.clone()),
        );
        let matcher = crate::matching::SourceMatcher::new(&matcher_units, &registry);
        let mut merged = 0;
        for e in &entries {
            let v = e.value.trim();
            if v.is_empty() {
                continue;
            }
            if let Some(source_key) = matcher.source_for_target(&e.key) {
                let kind = if project_b
                    .entries
                    .iter()
                    .any(|en| en.id.kind == EntryKind::TKey && en.id.key == source_key)
                {
                    EntryKind::TKey
                } else {
                    EntryKind::DefInjected
                };
                project_b.update_translation(
                    SourceEntryId {
                        kind,
                        key: source_key,
                    },
                    "Russian",
                    Some(v.to_string()),
                    dom::Origin::Imported,
                );
                merged += 1;
            }
        }
        assert!(
            merged >= 4,
            "PO interop must restore translations, got {merged}"
        );
        // ...and the PO round trip writes the SAME RimWorld output.
        let out_b = tmp.path().join("out-b");
        write_rimworld_translation(&project_b, &out_b, "Russian", "T", "t.b", "1.6").unwrap();
        let quest_b = std::fs::read_to_string(
            out_b.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
        )
        .unwrap();
        assert!(quest_b.contains("<SampleQuest.LetterLabelFavorReceiver.slateRef>Метка услуги<"));
    }
}

/// Maintenance pipeline (Gate K): after the source mod updated, rebuild the
/// effective inventory and compare per canonical identity. Translations whose
/// source text changed are flagged (sourceChanged + Pending review); new
/// source identities become plain untranslated entries; vanished identities
/// keep their translations but are marked obsolete in diagnostics count.
/// Nothing is deleted — user work is preserved (mandate 4 §3).
pub fn detect_source_changes(
    project: &mut Project,
    updated_mod_root: &Path,
    target_version: Option<&str>,
) -> Result<SourceChangeReport> {
    let fresh = build_project(updated_mod_root, target_version)?;
    let mut report = SourceChangeReport::default();

    let fresh_by_key: std::collections::HashMap<&str, &rimloc_domain::canonical::SourceEntry> =
        fresh
            .entries
            .iter()
            .map(|e| (e.id.key.as_str(), e))
            .collect();
    let old_by_key: std::collections::HashMap<&str, &rimloc_domain::canonical::SourceEntry> =
        project
            .entries
            .iter()
            .map(|e| (e.id.key.as_str(), e))
            .collect();

    // 1. Changed source text for translated entries.
    for t in &mut project.translations {
        if t.text.is_none() {
            continue;
        }
        if let Some(old_entry) = old_by_key.get(t.source_id.key.as_str()) {
            match fresh_by_key.get(t.source_id.key.as_str()) {
                Some(new_entry) => {
                    if new_entry.text != old_entry.text && t.source_changed.is_none() {
                        t.source_changed = Some("source text changed".into());
                        t.review = dom::Review::Pending;
                        report.source_changed += 1;
                    }
                }
                None => {
                    t.lifecycle = dom::Lifecycle::Obsolete;
                    report.obsolete += 1;
                }
            }
        }
    }

    // 2. New source identities -> fresh untranslated entries in the project.
    // Owned keys end the borrow of project.entries before we push into it.
    let old_keys: std::collections::HashSet<String> =
        old_by_key.keys().map(|k| k.to_string()).collect();
    for e in &fresh.entries {
        if !old_keys.contains(&e.id.key) {
            let mut entry = e.clone();
            entry.provenance.version_selected = target_version
                .map(String::from)
                .or_else(|| entry.provenance.version_selected.clone());
            project.entries.push(entry);
            report.new_source += 1;
        }
    }

    // 3. Reusable: translations whose source text is unchanged stay as-is.
    report.reusable = project
        .translations
        .iter()
        .filter(|t| t.text.is_some() && t.source_changed.is_none())
        .count();

    Ok(report)
}

/// Classification result of a source update (mandate 4 §3).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SourceChangeReport {
    pub reusable: usize,
    pub source_changed: usize,
    pub new_source: usize,
    pub obsolete: usize,
}

#[cfg(test)]
mod gate_k_tests {
    use super::*;
    use rimloc_domain::canonical::{
        Completeness, EntryKind, Origin, Project, SourceEntry, SourceEntryId, SourceProvenance,
    };

    fn entry(key: &str, text: &str) -> SourceEntry {
        SourceEntry {
            id: SourceEntryId {
                kind: EntryKind::Keyed,
                key: key.into(),
            },
            text: text.into(),
            source_locale: "en".into(),
            contexts: vec![],
            provenance: SourceProvenance::default(),
            tkey: None,
        }
    }

    /// A source text change flags the translation for review; unchanged work
    /// stays reusable; a vanished identity becomes obsolete; nothing is lost.
    #[test]
    fn source_update_classifies_translations() {
        let updated = tempfile::tempdir().unwrap();
        let defs = updated.path().join("Defs");
        std::fs::create_dir_all(&defs).unwrap();
        std::fs::write(
            defs.join("K.xml"),
            r#"<Defs><ThingDef><defName>A</defName><label>CHANGED text</label></ThingDef>
  <ThingDef><defName>C</defName><label>brand new</label></ThingDef>
  <ThingDef><defName>D</defName><label>untouched</label></ThingDef></Defs>"#,
        )
        .unwrap();
        // Disable patches dir interference: none exists.

        let mut p = Project::default();
        // A: translated, source changed now.
        p.entries.push(entry("A.label", "old text"));
        // B: translated, source vanished.
        p.entries.push(entry("B.label", "gone source"));
        // C: new identity (added below by detector).
        p.entries.push(entry("D.label", "untouched"));

        p.update_translation(
            SourceEntryId {
                kind: EntryKind::Keyed,
                key: "A.label".into(),
            },
            "Russian",
            Some("старый перевод".into()),
            Origin::Human,
        );
        p.update_translation(
            SourceEntryId {
                kind: EntryKind::Keyed,
                key: "B.label".into(),
            },
            "Russian",
            Some("перевод осиротел".into()),
            Origin::Human,
        );
        p.update_translation(
            SourceEntryId {
                kind: EntryKind::Keyed,
                key: "D.label".into(),
            },
            "Russian",
            Some("не трогать".into()),
            Origin::Human,
        );

        let report = detect_source_changes(&mut p, updated.path(), None).unwrap();
        assert_eq!(report.source_changed, 1, "{report:?}");
        assert_eq!(report.new_source, 1, "{report:?}");
        assert_eq!(report.obsolete, 1, "{report:?}");
        assert!(report.reusable >= 1, "{report:?}");

        let a = p
            .translation(
                &SourceEntryId {
                    kind: EntryKind::Keyed,
                    key: "A.label".into(),
                },
                "Russian",
            )
            .unwrap();
        assert!(a.source_changed.is_some());
        assert_eq!(a.review, dom::Review::Pending);
        assert_eq!(
            p.translation(
                &SourceEntryId {
                    kind: EntryKind::Keyed,
                    key: "B.label".into()
                },
                "Russian"
            )
            .unwrap()
            .lifecycle,
            dom::Lifecycle::Obsolete
        );
        // Old translation text preserved even though source changed.
        assert_eq!(a.text.as_deref(), Some("старый перевод"));
    }
}
