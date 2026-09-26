//! First-party UI-catalog source adapter (self-localization, owner mandate
//! §12: "RimLoc translates RimLoc" — ONE small vertical slice through the
//! ORDINARY canonical services, no new translation engine).
//!
//! A *catalog source* is a directory that contains the app's own generated
//! bridge file `catalog.en.json` (`docs/development/SELFLOC_BRIDGE.md`,
//! `schema_version: "1"`). When the session create-path is pointed at such a
//! directory, this adapter produces the canonical inventory from the catalog
//! INSTEAD of running any RimWorld mod scanner on it:
//!
//! - each message becomes a [`SourceEntry`] with `kind = EntryKind::Keyed`
//!   (the Keyed FAMILY, deliberately — no new `EntryKind::Application` in
//!   this slice; the decision to introduce one stays with a future wave);
//!   `key = message id`, `text = source_text`, `source_locale = "en"`;
//! - the origin is marked per entry in `provenance.selected_by`
//!   ([`rimloc_core::winner_reason::UI_CATALOG`]) and the real catalog file
//!   is the entry's only source context — evidence is greppable, never
//!   invented;
//! - the M3 source fingerprint covers the generated catalog files
//!   themselves (`catalog.<locale>.json` + `catalog.meta.json`), so an edit
//!   of any source message is honest source-drift under the EXISTING M3
//!   semantics (`Some(...)` verdicts, `None` only for legacy envelopes).
//!
//! Deliberately NOT here (mandate §8 — DefInjected mechanics must never
//! touch application messages): no RimWorld-specific validation, no
//! DefInjected writer, no Keyed-XML scanner on catalog data. The ordinary
//! session `validate` runs the regular placeholder/lost-placeholder checks
//! over translations, and the ordinary export writes only the Keyed-family
//! output — asserted by the integration test below on a REAL generated
//! catalog copy (never the repository files).
//!
//! Kind decision note (recorded per the task): the bridge's
//! `canonical_bridge::kind_for` infers kinds from SCAN PATHS, and a
//! `catalog.en.json` path would fall into its DefInjected fallback arm —
//! the wrong family for application messages. The catalog adapter therefore
//! stamps the kind it knows BY CONSTRUCTION (Keyed family) instead of
//! fabricating a virtual path to steer the inference. `canonical_bridge`
//! remains the single converter of the mod-scan pipeline; the catalog
//! adapter converts its own, non-scan source.

use crate::{Result, TransUnit};
use rimloc_core::winner_reason;
use rimloc_domain::canonical::{
    ContextRole, EntryKind, InventoryContext, PatchStage, Project, SourceContext, SourceEntry,
    SourceEntryId, SourceProvenance, ViewLabel,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The EN source file that MARKS a directory as a catalog source.
pub const CATALOG_SOURCE_FILE: &str = "catalog.en.json";
/// The generated bridge carries its schema in the META file
/// (`catalog.meta.json`), not inside the message lists — SELFLOC_BRIDGE.md.
pub const CATALOG_META_FILE: &str = "catalog.meta.json";
/// Schema version this adapter accepts (SELFLOC_BRIDGE.md: additive-minor
/// policy; a future "2" is a deliberate contract change, not a fallback).
pub const SCHEMA_VERSION: &str = "1";

/// One catalog message — the form gate is STRICT (id / source_text /
/// placeholders are all required by the generated schema; an export that
/// lost its shape is not a source we can translate honestly).
#[derive(Debug, Deserialize)]
struct CatalogMessage {
    /// Flat catalog key `<screen>.<block>.<element>`; unique per catalog.
    id: String,
    /// Verbatim en dictionary value, `{name}`-interpolated at runtime.
    source_text: String,
    /// Sorted unique `{name}` tokens of THIS text (the validation contract).
    #[allow(dead_code)]
    placeholders: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CatalogMessages {
    messages: Vec<CatalogMessage>,
}

#[derive(Debug, Deserialize)]
struct CatalogMeta {
    schema_version: String,
}

fn catalog_en_path(root: &Path) -> PathBuf {
    root.join(CATALOG_SOURCE_FILE)
}

/// Cheap + strict recognition of a catalog source: the directory contains a
/// `catalog.en.json` whose message list parses in full form (every message
/// with id / source_text / placeholders) AND a `catalog.meta.json` declaring
/// schema `"1"` (the version lives in the meta file by bridge contract).
/// Anything else — including a broken catalog file — is NOT a catalog
/// source, and the ordinary mod-scan path runs (an unparseable file must
/// never silently become an empty-looking catalog project). Cost: two small
/// file reads.
pub fn is_catalog_source(root: &Path) -> bool {
    parse_catalog(&catalog_en_path(root)).is_some()
}

fn parse_catalog(en_path: &Path) -> Option<CatalogMessages> {
    let root = en_path.parent()?;
    let meta: CatalogMeta =
        serde_json::from_slice(&std::fs::read(root.join(CATALOG_META_FILE)).ok()?).ok()?;
    if meta.schema_version != SCHEMA_VERSION {
        return None;
    }
    let messages: CatalogMessages = serde_json::from_slice(&std::fs::read(en_path).ok()?).ok()?;
    // Form gate: every message needs a non-empty id (source_text and
    // placeholders are already structurally required by Deserialize).
    messages
        .messages
        .iter()
        .all(|m| !m.id.trim().is_empty())
        .then_some(messages)
}

/// Scan the catalog source into the canonical inventory. Duplicate ids are
/// a REFUSAL (the exporter guarantees uniqueness; a hand-edited catalog
/// must not silently collapse identities), empty source texts are SKIPPED
/// (the same accept rule the Defs pipeline applies to empty sources).
pub fn build_catalog_project(root: &Path) -> Result<Project> {
    let en_path = catalog_en_path(root);
    let file = parse_catalog(&en_path).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "`{}` is not a parseable UI catalog (schema `{}` in `{}`)",
            en_path.display(),
            SCHEMA_VERSION,
            CATALOG_META_FILE
        )
    })?;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut entries: Vec<SourceEntry> = Vec::with_capacity(file.messages.len());
    for m in file.messages {
        if m.source_text.trim().is_empty() {
            continue;
        }
        if !seen.insert(m.id.clone()) {
            return Err(color_eyre::eyre::eyre!(
                "duplicate catalog message id `{}` — identities must stay unique",
                m.id
            ));
        }
        entries.push(SourceEntry {
            id: SourceEntryId {
                kind: EntryKind::Keyed,
                key: m.id,
                def_type: None,
            },
            text: m.source_text,
            source_locale: "en".into(),
            contexts: vec![SourceContext {
                file: en_path.display().to_string(),
                line: None,
                def_type: None,
                role: ContextRole::Effective,
            }],
            provenance: SourceProvenance {
                // No version/view machinery exists for a catalog: the
                // inventory is complete by construction (the exporter
                // enumerates every dictionary entry), hence an EXACT view.
                version_selected: None,
                conditional_branch: false,
                patch_stage: PatchStage::None,
                selected_by: Some(winner_reason::UI_CATALOG.into()),
            },
            tkey: None,
        });
    }
    Ok(Project {
        context: InventoryContext {
            target_version: None,
            view: ViewLabel::Exact,
            ..Default::default()
        },
        entries,
        translations: Vec::new(),
    })
}

/// Scan the catalog into the shared `TransUnit` shape (key = message id,
/// source = source_text, `selected_by` marks the origin). Provided for
/// callers that consume the scan-world unit stream directly; the session
/// create-path uses [`build_catalog_project`].
pub fn scan_catalog_units(root: &Path) -> Result<Vec<TransUnit>> {
    let project = build_catalog_project(root)?;
    Ok(project
        .entries
        .iter()
        .map(|e| TransUnit {
            key: e.id.key.clone(),
            source: Some(e.text.clone()),
            path: catalog_en_path(root),
            line: None,
            selected_by: Some(winner_reason::UI_CATALOG.into()),
            ..Default::default()
        })
        .collect())
}

/// M3 source fingerprint of a catalog source: `(relative path, sha256)` of
/// every generated bridge file in the directory (`catalog.<locale>.json` +
/// `catalog.meta.json`), sorted, `/`-normalized — the same accumulator
/// contract as the mod fingerprint. Editing any source message, adding or
/// removing a catalog file, or re-exporting (the revision moves) each
/// change the value; the same content reproduces it byte-for-byte.
/// Cost note: three small files, one sequential read pass per project
/// (re)start — bounded far under the mod-tree walk.
pub fn source_fingerprint(root: &Path) -> Result<String> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(read) = std::fs::read_dir(root) {
        for entry in read.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with("catalog.") && name.ends_with(".json") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut acc = String::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        acc.push_str(&rel);
        acc.push('\n');
        let bytes = std::fs::read(&path)?;
        acc.push_str(&crate::observability::sha256_hex(&bytes));
        acc.push('\n');
    }
    Ok(crate::observability::sha256_hex(acc.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_catalog(dir: &Path, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(CATALOG_SOURCE_FILE), body).unwrap();
        fs::write(
            dir.join(CATALOG_META_FILE),
            r#"{"schema_version":"1","source_app":"rimloc"}"#,
        )
        .unwrap();
    }

    fn small_catalog() -> String {
        r#"{
  "schema_version": "1",
  "messages": [
    { "id": "common.appName", "source_text": "RimLoc", "placeholders": [] },
    { "id": "home.recent.sourceChanged", "source_text": "source changed: {count}", "placeholders": ["count"] }
  ]
}"#
        .to_string()
    }

    #[test]
    fn plain_dirs_are_not_catalog_sources() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_catalog_source(dir.path()));
        // A mod-shaped dir (Languages tree) is untouched by the adapter.
        fs::create_dir_all(dir.path().join("Languages/English/Keyed")).unwrap();
        fs::write(
            dir.path().join("Languages/English/Keyed/K.xml"),
            "<LanguageData><K>hi</K></LanguageData>",
        )
        .unwrap();
        assert!(!is_catalog_source(dir.path()));
    }

    #[test]
    fn broken_or_versioned_catalogs_are_not_sources() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), "{ not json");
        assert!(!is_catalog_source(dir.path()));
        // Wrong schema in the META file (the bridge's version carrier).
        fs::write(
            dir.path().join(CATALOG_META_FILE),
            r#"{"schema_version":"2"}"#,
        )
        .unwrap();
        assert!(!is_catalog_source(dir.path()));
        // Missing meta at all — an orphaned message list is not a source.
        fs::remove_file(dir.path().join(CATALOG_META_FILE)).unwrap();
        write_catalog(dir.path(), &small_catalog());
        fs::remove_file(dir.path().join(CATALOG_META_FILE)).unwrap();
        assert!(!is_catalog_source(dir.path()));
        // A message without source_text (form loss) is refused by the gate.
        write_catalog(
            dir.path(),
            r#"{"schema_version":"1","messages":[{"id":"a"}]}"#,
        );
        assert!(!is_catalog_source(dir.path()));
    }

    #[test]
    fn catalog_project_maps_messages_to_keyed_entries_with_origin() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        let p = build_catalog_project(dir.path()).unwrap();
        assert_eq!(p.entries.len(), 2);
        let app = p
            .entries
            .iter()
            .find(|e| e.id.key == "common.appName")
            .unwrap();
        assert_eq!(app.id.kind, EntryKind::Keyed);
        assert_eq!(app.text, "RimLoc");
        assert_eq!(app.source_locale, "en");
        assert_eq!(
            app.provenance.selected_by.as_deref(),
            Some(winner_reason::UI_CATALOG)
        );
        assert!(app.contexts[0].file.ends_with(CATALOG_SOURCE_FILE));
    }

    #[test]
    fn duplicate_ids_are_refused_not_collapsed() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(
            dir.path(),
            r#"{"schema_version":"1","messages":[
                {"id":"a","source_text":"one","placeholders":[]},
                {"id":"a","source_text":"two","placeholders":[]}]}"#,
        );
        assert!(build_catalog_project(dir.path()).is_err());
    }

    /// The create-path HOOK: `build_project` on a catalog dir must produce
    /// the catalog inventory (the ordinary mod pipeline would yield ZERO
    /// entries for a dir without Languages/Defs — a zero result here would
    /// mean the hook did not fire).
    #[test]
    fn build_project_routes_catalog_sources_through_the_adapter() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        let p = crate::project::build_project(dir.path(), None).unwrap();
        assert_eq!(p.entries.len(), 2);
        assert!(p.entries.iter().all(|e| e.id.kind == EntryKind::Keyed));
    }

    #[test]
    fn catalog_fingerprint_stable_and_moving_like_m3() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        let a = source_fingerprint(dir.path()).unwrap();
        let b = source_fingerprint(dir.path()).unwrap();
        assert_eq!(a, b, "deterministic for unchanged content");
        // Source-message edit moves the fingerprint.
        write_catalog(dir.path(), &small_catalog().replace("RimLoc", "RimLoc v2"));
        let edited = source_fingerprint(dir.path()).unwrap();
        assert_ne!(a, edited);
        // Added catalog file (ru sidecar) moves it too; removal restores
        // the EDITED baseline (the en edit above stays in place).
        fs::write(dir.path().join("catalog.ru.json"), "{}").unwrap();
        let with_ru = source_fingerprint(dir.path()).unwrap();
        assert_ne!(edited, with_ru);
        fs::remove_file(dir.path().join("catalog.ru.json")).unwrap();
        assert_eq!(edited, source_fingerprint(dir.path()).unwrap());
    }
}
