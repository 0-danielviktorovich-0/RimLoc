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
//! - the M3 source fingerprint covers the SEMANTIC SOURCE only
//!   (`catalog.en.json` + `catalog.meta.json`, SF-09 boundary): a target
//!   catalog (`catalog.<locale>.json`, locale ≠ source) is an EXPORT of the
//!   source — a translator finishing a locale must never surface as
//!   source-drift. Verdicts stay honest M3 (`Some(...)`; `None` only when
//!   the source cannot be fingerprinted right now).
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
    ContextRole, EntryKind, InventoryContext, Origin, PatchStage, Project, SourceContext,
    SourceEntry, SourceEntryId, SourceProvenance, ViewLabel,
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
/// `source_app` this adapter accepts when the meta field is present. The
/// field is OPTIONAL (older meta may lack it — absence is fine); a DIFFERENT
/// value means the directory belongs to another producer and the catalog is
/// not ours to scan.
pub const SOURCE_APP: &str = "rimloc";
/// `source_locale` this adapter accepts when the meta field is present (the
/// same optionality rule: the en file is the source by bridge contract).
pub const SOURCE_LOCALE: &str = "en";

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

/// The parsed EN message list of a recognized catalog source (the bridge's
/// strict full form; Deserialize IS the form gate).
#[derive(Debug, Deserialize)]
pub struct CatalogMessages {
    messages: Vec<CatalogMessage>,
}

#[derive(Debug, Deserialize)]
struct CatalogMeta {
    schema_version: String,
    /// Optional in older meta (SELFLOC_BRIDGE.md carried only schema_version
    /// at first). Present-but-different is a typed refusal, not a fallback.
    #[serde(default)]
    source_app: Option<String>,
    #[serde(default)]
    source_locale: Option<String>,
}

/// One TARGET catalog message (`catalog.<locale>.json`, bridge form: the
/// value lives under `translated`; `placeholders` of the target text are
/// ignored here — the ordinary session validation owns that contract).
#[derive(Debug, Deserialize)]
struct CatalogTargetMessage {
    id: String,
    translated: String,
}

#[derive(Debug, Deserialize)]
struct CatalogTargetMessages {
    messages: Vec<CatalogTargetMessage>,
}

fn catalog_en_path(root: &Path) -> PathBuf {
    root.join(CATALOG_SOURCE_FILE)
}

/// Typed catalog recognition (SF-09): the three cases every caller must be
/// able to tell apart. The OLD `Option` shape made a BROKEN catalog
/// indistinguishable from "no catalog here", so the project entry point
/// silently fell back to the mod scan and a corrupted source looked like a
/// legitimately empty mod.
pub enum CatalogStatus {
    /// No catalog marker here at all (no `catalog.en.json`): the ordinary
    /// RimWorld mod pipeline is the right scanner for this directory.
    NotCatalog,
    /// The marker is present and the whole source parses in full form.
    Valid(CatalogMessages),
    /// The marker IS here, but the source it promises is broken. The
    /// string carries the exact reason (schema mismatch, unreadable/
    /// unparseable file, empty id, missing meta while en is present,
    /// foreign `source_app`/`source_locale`) — a typed refusal, never a
    /// silent fallback.
    Invalid(String),
}

/// THE boundary (recorded per the SF-09 task): `catalog.en.json` is the
/// MARKER of a catalog source — without it the directory is simply not a
/// catalog, whatever else it holds. Once the marker is present, everything
/// it implies but fails to deliver is [`CatalogStatus::Invalid`]: an
/// orphaned message list without meta, a schema this adapter does not
/// speak, a meta from another producer, a hand-broken file. Cost: two
/// small file reads.
pub fn recognize_catalog(root: &Path) -> CatalogStatus {
    let en_path = catalog_en_path(root);
    if !en_path.is_file() {
        return CatalogStatus::NotCatalog;
    }
    let meta_path = root.join(CATALOG_META_FILE);
    let meta: CatalogMeta = match std::fs::read(&meta_path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(meta) => meta,
            Err(e) => {
                return CatalogStatus::Invalid(format!(
                    "`{}` is present but unreadable: {e}",
                    meta_path.display()
                ));
            }
        },
        Err(e) => {
            // The ask names this shape explicitly: en without meta is an
            // ORPHANED message list, not a source.
            return CatalogStatus::Invalid(format!(
                "`{}` is present but `{}` is missing ({e}) — an orphaned message list is not a catalog source",
                en_path.display(),
                meta_path.display()
            ));
        }
    };
    if meta.schema_version != SCHEMA_VERSION {
        return CatalogStatus::Invalid(format!(
            "meta declares schema `{}`, this adapter speaks `{}` only (a future schema is a deliberate contract change, not a fallback)",
            meta.schema_version, SCHEMA_VERSION
        ));
    }
    if let Some(app) = &meta.source_app {
        if app != SOURCE_APP {
            return CatalogStatus::Invalid(format!(
                "meta declares source_app `{expected}`, found `{found}` — not a RimLoc catalog",
                expected = SOURCE_APP,
                found = app
            ));
        }
    }
    if let Some(locale) = &meta.source_locale {
        if locale != SOURCE_LOCALE {
            return CatalogStatus::Invalid(format!(
                "meta declares source_locale `{expected}`, found `{found}` — the en catalog is the source by bridge contract",
                expected = SOURCE_LOCALE,
                found = locale
            ));
        }
    }
    let en_bytes = match std::fs::read(&en_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            return CatalogStatus::Invalid(format!(
                "`{}` is present but unreadable: {e}",
                en_path.display()
            ));
        }
    };
    let messages: CatalogMessages = match serde_json::from_slice(&en_bytes) {
        Ok(messages) => messages,
        Err(e) => {
            return CatalogStatus::Invalid(format!(
                "`{}` is present but does not parse in full form (every message needs id / source_text / placeholders): {e}",
                en_path.display()
            ));
        }
    };
    // Form gate over ids: the empty-id offender is NAMED, not averaged
    // into a boolean.
    for (i, m) in messages.messages.iter().enumerate() {
        if m.id.trim().is_empty() {
            return CatalogStatus::Invalid(format!(
                "`{}` message #{i} has an empty id — identities must stay addressable",
                en_path.display()
            ));
        }
    }
    CatalogStatus::Valid(messages)
}

/// Bool compatibility wrapper over [`recognize_catalog`] for callers that
/// only route ("is this a catalog dir?"). A bool CANNOT distinguish
/// NotCatalog from Invalid by construction — any caller that must not
/// silently fall back on a broken catalog (the project create-path) uses
/// [`recognize_catalog`] directly.
pub fn is_catalog_source(root: &Path) -> bool {
    matches!(recognize_catalog(root), CatalogStatus::Valid(_))
}

/// Scan the catalog source into the canonical inventory. Duplicate ids are
/// a REFUSAL (the exporter guarantees uniqueness; a hand-edited catalog
/// must not silently collapse identities), empty source texts are SKIPPED
/// (the same accept rule the Defs pipeline applies to empty sources).
///
/// SF-10: when the directory also carries a target catalog
/// (`catalog.<locale>.json`, locale ≠ source), the existing translation is
/// imported as ORDINARY project data — the translator opens a project with
/// their previous work in place, not 1253 empty slots. Policy (fail-closed,
/// recorded per the task):
/// - the target file must parse in the bridge form (`messages[{id,
///   translated}]`) — an unparseable target is a typed refusal naming the
///   file, because silently ignoring it would pretend existing user work
///   does not exist;
/// - every target id must exist in the en inventory. A target id that is
///   not in en means the two generated files drifted apart (stale export or
///   a hand edit) — the refusal names the id and the file; the fix is
///   mechanical (re-run `npm run export:catalog`). A silent skip would
///   strand real translations with no diagnostic;
/// - a duplicate id inside one target file is refused, mirroring the
///   en-side identity rule (no last-write-wins);
/// - an EMPTY/whitespace `translated` value is not a translation: the slot
///   is skipped (no record), the same accept rule the PO import applies to
///   empty msgstr. Placeholder correctness is NOT judged at import — the
///   ordinary session validation owns that contract.
pub fn build_catalog_project(root: &Path) -> Result<Project> {
    let en_path = catalog_en_path(root);
    let file = match recognize_catalog(root) {
        CatalogStatus::Valid(file) => file,
        other => {
            let reason = match other {
                CatalogStatus::NotCatalog => format!(
                    "`{}` is not a UI catalog (no `{}` marker)",
                    en_path.display(),
                    CATALOG_SOURCE_FILE
                ),
                CatalogStatus::Invalid(reason) => reason,
                CatalogStatus::Valid(_) => unreachable!("matched_above"),
            };
            return Err(color_eyre::eyre::eyre!(
                "`{}` is not a usable UI catalog (schema `{}` in `{}`): {reason}",
                en_path.display(),
                SCHEMA_VERSION,
                CATALOG_META_FILE
            ));
        }
    };
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
    let mut project = Project {
        context: InventoryContext {
            target_version: None,
            view: ViewLabel::Exact,
            ..Default::default()
        },
        entries,
        translations: Vec::new(),
    };
    // SF-10: import every existing target catalog as ordinary data (the
    // single `update_translation` write path; origin Imported marks the
    // values as imported work, not fresh human edits).
    let ids_by_key: std::collections::HashMap<String, SourceEntryId> = project
        .entries
        .iter()
        .map(|e| (e.id.key.clone(), e.id.clone()))
        .collect();
    for (locale, path) in target_catalog_files(root)? {
        let bytes = std::fs::read(&path).map_err(|e| {
            color_eyre::eyre::eyre!("target catalog `{}` is unreadable: {e}", path.display())
        })?;
        let file: CatalogTargetMessages = serde_json::from_slice(&bytes).map_err(|e| {
            color_eyre::eyre::eyre!(
                "target catalog `{}` does not parse in the bridge form (messages[{{id, translated}}]): {e}",
                path.display()
            )
        })?;
        let mut target_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for m in file.messages {
            if m.translated.trim().is_empty() {
                continue;
            }
            if !target_seen.insert(m.id.clone()) {
                return Err(color_eyre::eyre::eyre!(
                    "target catalog `{}` repeats id `{}` — identities must stay unique",
                    path.display(),
                    m.id
                ));
            }
            let Some(entry_id) = ids_by_key.get(&m.id) else {
                return Err(color_eyre::eyre::eyre!(
                    "target catalog `{}` translates id `{}` which does not exist in `{}` — the generated files drifted apart; re-run the catalog export (fail-closed: a silent skip would strand existing translations)",
                    path.display(),
                    m.id,
                    en_path.display()
                ));
            };
            project.update_translation(
                entry_id.clone(),
                &locale,
                Some(m.translated),
                Origin::Imported,
            );
        }
    }
    Ok(project)
}

/// Target catalogs of the source directory: `(locale, path)` for every
/// `catalog.<locale>.json` whose locale is neither empty nor the source
/// locale, sorted by path for deterministic import order. The meta file is
/// not a target (its "locale" position holds the literal word `meta`).
fn target_catalog_files(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let read = std::fs::read_dir(root).map_err(|e| {
        color_eyre::eyre::eyre!("catalog source dir `{}` is unreadable: {e}", root.display())
    })?;
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    for entry in read {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == CATALOG_SOURCE_FILE || name == CATALOG_META_FILE {
            continue;
        }
        let Some(locale) = name
            .strip_prefix("catalog.")
            .and_then(|r| r.strip_suffix(".json"))
        else {
            continue;
        };
        if locale.is_empty() || locale.eq_ignore_ascii_case(SOURCE_LOCALE) {
            continue;
        }
        out.push((locale.to_string(), path));
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
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

/// M3 source fingerprint of a catalog source — SEMANTIC SOURCE ONLY (SF-09
/// boundary): `(relative path, sha256)` of `catalog.en.json` +
/// `catalog.meta.json`, sorted, `/`-normalized — the same accumulator
/// contract as the mod fingerprint.
///
/// Contract change vs the first slice (recorded per the task, updates the
/// M3-drift meaning for selfloc): the OLD fingerprint accumulated EVERY
/// `catalog.<locale>.json`, so finishing a translation or moving the export
/// revision surfaced as "source drift" and flagged finished translations
/// for review. Drift now means "the SOURCE changed, not the export": a
/// target catalog is an EXPORT of the source and is deliberately NOT
/// accumulated here. The meta file stays in: it is the source's schema /
/// identity carrier (its `catalog_revision` pin moves when the tree the
/// bridge was generated from moves — the bridge regenerates en + meta
/// atomically, so that movement is an honest provenance signal). Editing a
/// source message, adding or removing a source file each change the value;
/// the same content reproduces it byte-for-byte.
///
/// A `read_dir` failure is an explicit Err — an unreadable source must
/// never fold into a stable empty fingerprint that would read as "in sync".
pub fn source_fingerprint(root: &Path) -> Result<String> {
    let read = std::fs::read_dir(root).map_err(|e| {
        color_eyre::eyre::eyre!("catalog source dir `{}` is unreadable: {e}", root.display())
    })?;
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in read {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == CATALOG_SOURCE_FILE || name == CATALOG_META_FILE {
            files.push(path);
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

    /// SF-09: the typed statuses. Every broken-catalog shape is Invalid
    /// WITH a reason — never a silent NotCatalog — while a plain dir (and a
    /// mod-shaped dir) stays honestly NotCatalog.
    #[test]
    fn recognize_catalog_distinguishes_not_catalog_from_invalid() {
        use CatalogStatus::{Invalid, NotCatalog, Valid};

        // Plain dir and mod-shaped dir: genuinely not a catalog.
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(recognize_catalog(dir.path()), NotCatalog));
        fs::create_dir_all(dir.path().join("Languages/English/Keyed")).unwrap();
        fs::write(
            dir.path().join("Languages/English/Keyed/K.xml"),
            "<LanguageData><K>hi</K></LanguageData>",
        )
        .unwrap();
        assert!(matches!(recognize_catalog(dir.path()), NotCatalog));

        // Broken JSON in the en marker: the marker is here, the source is
        // Invalid with a reason.
        write_catalog(dir.path(), "{ not json");
        assert!(matches!(recognize_catalog(dir.path()), Invalid(_)));

        // Schema mismatch in the meta carrier.
        fs::write(
            dir.path().join(CATALOG_META_FILE),
            r#"{"schema_version":"2","source_app":"rimloc","source_locale":"en"}"#,
        )
        .unwrap();
        match recognize_catalog(dir.path()) {
            Invalid(reason) => assert!(reason.contains("schema"), "{reason}"),
            _ => panic!("schema_mismatch_must_be_invalid"),
        }

        // Missing meta while en is present (the ask's named boundary).
        write_catalog(dir.path(), &small_catalog());
        fs::remove_file(dir.path().join(CATALOG_META_FILE)).unwrap();
        match recognize_catalog(dir.path()) {
            Invalid(reason) => assert!(reason.contains(CATALOG_META_FILE), "{reason}"),
            _ => panic!("orphaned_message_list_must_be_invalid"),
        }

        // Empty message id: the offender is named.
        write_catalog(
            dir.path(),
            r#"{"schema_version":"1","messages":[
                {"id":"a","source_text":"one","placeholders":[]},
                {"id":"  ","source_text":"two","placeholders":[]}]}"#,
        );
        match recognize_catalog(dir.path()) {
            Invalid(reason) => assert!(reason.contains("#1"), "{reason}"),
            _ => panic!("empty_id_must_be_invalid"),
        }

        // Meta identity checks: OPTIONAL fields may be absent (older meta);
        // present-but-different is Invalid. (The valid en list goes back
        // first — the empty-id list above must not interfere.)
        write_catalog(dir.path(), &small_catalog());
        fs::write(
            dir.path().join(CATALOG_META_FILE),
            r#"{"schema_version":"1"}"#,
        )
        .unwrap();
        assert!(matches!(recognize_catalog(dir.path()), Valid(_)));
        for (field, value) in [("source_app", "other-app"), ("source_locale", "ru")] {
            fs::write(
                dir.path().join(CATALOG_META_FILE),
                format!(r#"{{"schema_version":"1","{field}":"{value}"}}"#),
            )
            .unwrap();
            match recognize_catalog(dir.path()) {
                Invalid(reason) => assert!(reason.contains(field), "{reason}"),
                _ => panic!("{field}_mismatch_must_be_invalid"),
            }
        }
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

    /// SF-10: a target catalog riding along the source is imported as
    /// ordinary project data — EXACTLY the ids the target file carries
    /// (subset import), under the locale token of the FILE name, marked
    /// origin=Imported.
    #[test]
    fn build_imports_existing_target_catalog_subset() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        fs::write(
            dir.path().join("catalog.ru.json"),
            r#"{"messages":[
                {"id":"common.appName","translated":"РимЛок","placeholders":[]}
            ]}"#,
        )
        .unwrap();
        let p = build_catalog_project(dir.path()).unwrap();
        let app_id = p.entries[0].id.clone();
        let tr = p.translation(&app_id, "ru").expect("ru imported");
        assert_eq!(tr.text.as_deref(), Some("РимЛок"));
        assert_eq!(p.translations.len(), 1, "exactly the subset ids");
        assert_eq!(tr.origin, Origin::Imported);
        // An id present in en but ABSENT from the target stays untranslated.
        let other_id = &p.entries[1].id;
        assert!(p.translation(other_id, "ru").is_none());
    }

    /// SF-10: a target id that does not exist in en is a TYPED refusal
    /// naming the id — fail-closed (a silent skip would strand real
    /// translations with no diagnostic).
    #[test]
    fn target_id_not_in_en_is_a_typed_refusal() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        fs::write(
            dir.path().join("catalog.ru.json"),
            r#"{"messages":[{"id":"ghost.id","translated":"призрак","placeholders":[]}]}"#,
        )
        .unwrap();
        let err = format!("{}", build_catalog_project(dir.path()).unwrap_err());
        assert!(err.contains("ghost.id"), "{err}");
        assert!(err.contains("catalog.ru.json"), "{err}");
    }

    /// SF-10: a duplicate id inside one target file is refused (mirrors the
    /// en-side identity rule; no last-write-wins).
    #[test]
    fn target_duplicate_id_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        fs::write(
            dir.path().join("catalog.ru.json"),
            r#"{"messages":[
                {"id":"common.appName","translated":"первый","placeholders":[]},
                {"id":"common.appName","translated":"второй","placeholders":[]}
            ]}"#,
        )
        .unwrap();
        assert!(build_catalog_project(dir.path()).is_err());
    }

    /// SF-10: an empty target value is "not translated" — skipped, the same
    /// accept rule the PO import applies to empty msgstr; and a target file
    /// that lost the bridge shape (`translated` field) is a typed refusal.
    #[test]
    fn empty_target_value_is_skipped_and_broken_form_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        fs::write(
            dir.path().join("catalog.ru.json"),
            r#"{"messages":[{"id":"common.appName","translated":"  ","placeholders":[]}]}"#,
        )
        .unwrap();
        let p = build_catalog_project(dir.path()).unwrap();
        assert!(p.translations.is_empty(), "empty value = no record");

        fs::write(
            dir.path().join("catalog.ja.json"),
            r#"{"messages":[{"id":"common.appName","value":"リムロック"}]}"#,
        )
        .unwrap();
        let err = format!("{}", build_catalog_project(dir.path()).unwrap_err());
        assert!(err.contains("translated"), "{err}");
    }

    /// SF-10 backward compatibility: no target file -> empty translations,
    /// exactly the pre-SF-10 shape.
    #[test]
    fn build_without_target_catalogs_keeps_translations_empty() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        let p = build_catalog_project(dir.path()).unwrap();
        assert_eq!(p.entries.len(), 2);
        assert!(p.translations.is_empty());
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

    /// SF-09: the project build-path does NOT silently fall back to the mod
    /// scan on a broken catalog — the marker is here, so the build refuses
    /// with the typed reason (the mod pipeline would return a misleadingly
    /// empty inventory instead).
    #[test]
    fn build_project_refuses_an_invalid_catalog_instead_of_falling_back() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), "{ not json");
        let err = crate::project::build_project(dir.path(), None)
            .expect_err("broken catalog must be a typed refusal");
        let text = format!("{err}");
        // The refusal names the root AND carries the recognition reason.
        assert!(text.contains("invalid and was not scanned"), "{text}");
        assert!(text.contains("does not parse in full form"), "{text}");
    }

    /// SF-09 boundary: the M3 fingerprint is SEMANTIC SOURCE ONLY
    /// (catalog.en.json + catalog.meta.json). Contract change vs the first
    /// slice, updated here DELIBERATELY: the old fingerprint accumulated
    /// every catalog.<locale>.json, so a translator finishing a locale (or
    /// a re-export moving the revision) surfaced as source-drift and flagged
    /// finished translations for review. Drift now = the SOURCE changed;
    /// target catalogs are exports of the source and do not move it.
    #[test]
    fn catalog_fingerprint_tracks_semantic_source_only() {
        let dir = tempfile::tempdir().unwrap();
        write_catalog(dir.path(), &small_catalog());
        let a = source_fingerprint(dir.path()).unwrap();
        let b = source_fingerprint(dir.path()).unwrap();
        assert_eq!(a, b, "deterministic for unchanged content");

        // Source-message edit moves the fingerprint.
        write_catalog(dir.path(), &small_catalog().replace("RimLoc", "RimLoc v2"));
        let edited = source_fingerprint(dir.path()).unwrap();
        assert_ne!(a, edited);

        // A target catalog appearing, CHANGING, and disappearing must NOT
        // move the fingerprint in any of the three steps: the translator's
        // work is not source drift.
        fs::write(
            dir.path().join("catalog.ru.json"),
            r#"{"messages":[{"id":"common.appName","translated":"РимЛок","placeholders":[]}]}"#,
        )
        .unwrap();
        let with_ru = source_fingerprint(dir.path()).unwrap();
        assert_eq!(edited, with_ru, "adding a target catalog is not drift");
        fs::write(
            dir.path().join("catalog.ru.json"),
            r#"{"messages":[{"id":"common.appName","translated":"РимЛок v2","placeholders":[]}]}"#,
        )
        .unwrap();
        assert_eq!(
            edited,
            source_fingerprint(dir.path()).unwrap(),
            "editing a target catalog is not drift"
        );
        fs::remove_file(dir.path().join("catalog.ru.json")).unwrap();
        assert_eq!(
            edited,
            source_fingerprint(dir.path()).unwrap(),
            "removing a target catalog is not drift"
        );

        // The meta file is part of the source contract (schema/identity
        // carrier): a meta change moves the fingerprint.
        fs::write(
            dir.path().join(CATALOG_META_FILE),
            r#"{"schema_version":"1","catalog_revision":"abc-dirty"}"#,
        )
        .unwrap();
        assert_ne!(edited, source_fingerprint(dir.path()).unwrap());

        // An unreadable source dir is an explicit Err — never a stable
        // empty fingerprint that would read as "in sync" (SF-09).
        let not_a_dir = tempfile::tempdir().unwrap();
        let plain_file = not_a_dir.path().join("plain");
        fs::write(&plain_file, "x").unwrap();
        assert!(source_fingerprint(&plain_file).is_err());
    }
}
