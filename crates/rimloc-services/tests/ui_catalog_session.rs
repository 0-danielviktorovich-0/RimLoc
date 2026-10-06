//! Session-level E2E on the REAL generated UI catalog (self-localization
//! wave B4, owner mandate §12): create → snapshot → apply (with one
//! deliberately broken placeholder) → validate finds it → fix → validate
//! clean → export (DefInjected paths MUST NOT apply, §8) → source edit →
//! M3 source-drift verdict.
//!
//! The repository's generated files are NEVER mutated: the test works on a
//! tmp copy and reads the shared translation fixture
//! (`gui/tauri-app/frontend-v2/tests/fixtures/selfloc-e2e-translations.json`)
//! that the TS chain test (`tests/selfloc-catalog-chain.test.ts`) uses too.
//!
//! The SECOND test here is the real junction between the session world and
//! the TS contribution world (SF-5): the session's canonical export is
//! written into a KNOWN deterministic directory and summarized into
//! `chain.json` from a REPARSE OF THE WRITTEN FILES — the TS side consumes
//! that artifact as-is instead of rebuilding the expected data from the
//! fixture.

use rimloc_core::winner_reason;
use rimloc_domain::canonical::{EntryKind, Origin, SourceEntryId};
use rimloc_services::contract::{
    ApplyIntentsRequest, IntentAction, ProjectSnapshot, Revision, SessionEpoch, TranslationIntent,
};
use rimloc_services::{ui_catalog, ProjectSessionManager};
use std::fs;
use std::path::PathBuf;

const GENERATED_REL: &str = "gui/tauri-app/frontend-v2/src/i18n/generated";
const FIXTURE_REL: &str = "gui/tauri-app/frontend-v2/tests/fixtures/selfloc-e2e-translations.json";

#[derive(serde::Deserialize)]
struct FixtureChange {
    id: String,
    value: String,
}

#[derive(serde::Deserialize)]
struct SelflocFixture {
    locale: String,
    changes: Vec<FixtureChange>,
    broken_first: FixtureChange,
    fixed: FixtureChange,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("repo root exists")
}

fn keyed(id: &str) -> SourceEntryId {
    SourceEntryId {
        kind: EntryKind::Keyed,
        key: id.into(),
        def_type: None,
    }
}

fn intent(locale: &str, id: &str, text: &str) -> TranslationIntent {
    TranslationIntent {
        entry: keyed(id),
        locale: locale.into(),
        action: IntentAction::SetTranslation,
        text: Some(text.into()),
    }
}

fn apply_req(
    pid: &str,
    epoch: SessionEpoch,
    rev: Revision,
    intents: Vec<TranslationIntent>,
) -> ApplyIntentsRequest {
    ApplyIntentsRequest {
        project_id: pid.into(),
        expected_revision: rev,
        session_epoch: epoch,
        origin: None,
        intents,
    }
}

fn ids(s: &ProjectSnapshot) -> std::collections::HashSet<String> {
    s.project.entries.iter().map(|e| e.id.key.clone()).collect()
}

#[test]
fn catalog_project_end_to_end_through_ordinary_sessions() {
    let generated = repo_root().join(GENERATED_REL);
    let fixture: SelflocFixture =
        serde_json::from_str(&fs::read_to_string(repo_root().join(FIXTURE_REL)).unwrap())
            .expect("shared selfloc fixture parses");

    // REAL generated catalog, copied — the repository files stay intact.
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = tmp.path().join("catalog");
    fs::create_dir_all(&catalog_dir).unwrap();
    for name in ["catalog.en.json", "catalog.ru.json", "catalog.meta.json"] {
        fs::copy(generated.join(name), catalog_dir.join(name)).unwrap();
    }

    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).unwrap();

    // 1. create via the ORDINARY session path + snapshot evidence.
    let snap = mgr.create(&catalog_dir, None).unwrap();
    let en_count = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(catalog_dir.join("catalog.en.json")).unwrap(),
    )
    .unwrap()["messages"]
        .as_array()
        .unwrap()
        .len();
    assert!(
        snap.project.entries.len() > 1000,
        "the real catalog must exceed 1000 entries, got {}",
        snap.project.entries.len()
    );
    assert_eq!(snap.project.entries.len(), en_count);
    // SF-10: the create imported the existing ru catalog as ORDINARY data —
    // the translator opens the project with their previous work in place,
    // not 1284 empty slots. The import rides the same create flow; the
    // snapshot carries it (persist round-trip asserted after the restart
    // at the bottom of this test).
    let ru_count = count_nonempty_translated(&catalog_dir);
    assert!(
        snap.project.translations.len() > 1000,
        "the existing ru catalog must be imported, got {} translations",
        snap.project.translations.len()
    );
    assert_eq!(snap.project.translations.len(), ru_count);
    let imported = snap
        .project
        .translations
        .iter()
        .find(|t| t.source_id.key == "common.appName")
        .expect("imported ru record for common.appName");
    assert_eq!(imported.locale, "ru");
    assert_eq!(imported.origin, Origin::Imported);
    // M3 invariant: a fresh create is honestly in sync, never legacy None.
    assert_eq!(snap.source_changed, Some(false));
    // Identity + origin evidence on real data.
    let app = snap
        .project
        .entries
        .iter()
        .find(|e| e.id.key == "common.appName")
        .expect("real catalog id present");
    assert_eq!(app.id.kind, EntryKind::Keyed);
    assert_eq!(
        app.provenance.selected_by.as_deref(),
        Some(winner_reason::UI_CATALOG)
    );
    assert!(app.contexts[0]
        .file
        .ends_with(ui_catalog::CATALOG_SOURCE_FILE));
    // Live Source Inspector bridge (wave 12): the snapshot projects the
    // entry's `source_ref` — for a ui-catalog project the file is exactly
    // the catalog-root-relative source (catalog.en.json), the line stays
    // None (the catalog carries no line numbers), the winner reason names
    // the catalog origin.
    let sr = app
        .source_ref
        .as_ref()
        .expect("catalog_entry_projects_source_ref");
    assert_eq!(sr.file, ui_catalog::CATALOG_SOURCE_FILE, "{sr:?}");
    assert_eq!(sr.line, None, "catalog_carries_no_line_numbers");
    assert_eq!(sr.selected_by, winner_reason::UI_CATALOG);
    // Ids are stable across two creates (the catalog order is the dict order).
    let snap2 = mgr.create(&catalog_dir, None).unwrap();
    assert_eq!(ids(&snap), ids(&snap2));

    // 2. apply the shared translations + the deliberately broken one.
    let mut intents: Vec<TranslationIntent> = fixture
        .changes
        .iter()
        .map(|c| intent(&fixture.locale, &c.id, &c.value))
        .collect();
    intents.push(intent(
        &fixture.locale,
        &fixture.broken_first.id,
        &fixture.broken_first.value,
    ));
    let res = mgr
        .apply(&apply_req(
            &snap.project_id,
            snap.session_epoch,
            snap.revision,
            intents,
        ))
        .unwrap();
    assert_eq!(res.applied, 3);
    assert_eq!(res.revision, 2);

    // 3. validate: the dropped placeholder is found (negative evidence).
    let v = mgr
        .validate_project(&snap.project_id, snap.session_epoch, Some(&fixture.locale))
        .unwrap();
    assert_eq!(v.status, "failed", "{v:?}");
    let lost = v
        .findings
        .iter()
        .find(|f| f.kind == "lost-placeholder")
        .expect("lost-placeholder finding for the broken translation");
    assert_eq!(lost.key, fixture.broken_first.id);
    assert!(v.error_count >= 1);

    // 4. fix: re-apply the corrected value → validate clean.
    let res = mgr
        .apply(&apply_req(
            &snap.project_id,
            snap.session_epoch,
            2,
            vec![intent(
                &fixture.locale,
                &fixture.fixed.id,
                &fixture.fixed.value,
            )],
        ))
        .unwrap();
    assert_eq!(res.applied, 1);
    assert_eq!(res.revision, 3);
    let v = mgr
        .validate_project(&snap.project_id, snap.session_epoch, Some(&fixture.locale))
        .unwrap();
    assert_eq!(v.status, "succeeded", "{v:?}");
    assert_eq!(v.error_count, 0);

    // 5. export into tmp — ordinary Keyed-family output; DefInjected
    //    mechanics must NOT apply to application messages (mandate §8).
    //    SF-10: the export now round-trips the WHOLE imported ru set (the
    //    three session-applied intents overwrite their imported records in
    //    place — the count stays the ru file's), so the reparse guard is
    //    checked against the full catalog, not just the touched slice.
    let out = tmp.path().join("out");
    let exp = mgr
        .export_project(&snap.project_id, snap.session_epoch, &out, &fixture.locale)
        .unwrap();
    assert_eq!(
        exp.reparsed_keys, ru_count,
        "the export must round-trip every imported+applied ru translation: {exp:?}"
    );
    assert!(exp.skipped_unknown_type.is_empty());
    let definj = out
        .join("Languages")
        .join(&fixture.locale)
        .join("DefInjected");
    assert!(
        !definj.exists(),
        "DefInjected paths must never be written for a catalog project"
    );
    let keyed_xml = fs::read_to_string(
        out.join("Languages")
            .join(&fixture.locale)
            .join("Keyed")
            .join("Translation.xml"),
    )
    .unwrap();
    assert!(keyed_xml.contains(&fixture.fixed.value), "{keyed_xml}");

    // 6. sourceChanged — SEMANTIC SOURCE ONLY (SF-09 contract change):
    //    the fingerprint covers catalog.en.json + catalog.meta.json;
    //    target catalogs (catalog.ru.json) are EXPORTS of the source, so a
    //    translator finishing a locale must NEVER surface as source-drift
    //    (the old fingerprint accumulated every catalog.<locale>.json and
    //    flagged finished translations for review on a pure target change).
    //    6a. an en SOURCE edit is drift.
    let original_en = fs::read_to_string(catalog_dir.join("catalog.en.json")).unwrap();
    let mut en: serde_json::Value = serde_json::from_str(&original_en).unwrap();
    en["messages"][0]["source_text"] = serde_json::json!("RimLoc (edited)");
    fs::write(
        catalog_dir.join("catalog.en.json"),
        serde_json::to_string_pretty(&en).unwrap(),
    )
    .unwrap();
    // refresh is a session (re)start: the drift verdict re-evaluates.
    let refreshed = mgr.refresh(&snap.project_id).unwrap();
    assert_eq!(refreshed.source_changed, Some(true));
    let v = mgr
        .validate_project(
            &snap.project_id,
            refreshed.session_epoch,
            Some(&fixture.locale),
        )
        .unwrap();
    assert_eq!(v.status, "succeeded", "drift is a WARNING (M3 invariant)");
    assert!(v.findings.iter().any(|f| f.kind == "source-drift"), "{v:?}");
    // 6b. the en edit reverted + the ru TARGET edited: no drift. The same
    //     edit moved the fingerprint under the old contract — here it is
    //     honestly "in sync": only the source counts.
    fs::write(catalog_dir.join("catalog.en.json"), &original_en).unwrap();
    let mut ru: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(catalog_dir.join("catalog.ru.json")).unwrap())
            .unwrap();
    ru["messages"][0]["translated"] = serde_json::json!("РимЛок (переведено)");
    fs::write(
        catalog_dir.join("catalog.ru.json"),
        serde_json::to_string_pretty(&ru).unwrap(),
    )
    .unwrap();
    let refreshed = mgr.refresh(&snap.project_id).unwrap();
    assert_eq!(
        refreshed.source_changed,
        Some(false),
        "a target-catalog edit is not source drift (SF-09 boundary)"
    );
    // A restart-recovered session sees the same verdicts (fresh manager).
    let mgr2 = ProjectSessionManager::new(tmp.path().join("managed")).unwrap();
    let reopened = mgr2.open(&refreshed.project_id).unwrap();
    assert_eq!(reopened.source_changed, Some(false));
    // ...and the SF-10 import survives the restart as ordinary data.
    assert_eq!(reopened.project.translations.len(), ru_count);
}

/// Count the importable records of the copied ru target catalog (SF-10):
/// non-empty `translated` values, every id of which resolves to an en
/// entry for the real generated files.
fn count_nonempty_translated(catalog_dir: &std::path::Path) -> usize {
    serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(catalog_dir.join("catalog.ru.json")).unwrap(),
    )
    .unwrap()["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| {
            !m["translated"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty()
        })
        .count()
}

/// The repository's generated catalog stays byte-intact after the E2E run:
/// the fixture copy lives in tmp, so the committed bridge files can never be
/// a silent victim of the test.
#[test]
fn repository_generated_catalog_is_never_the_fixture() {
    let generated = repo_root().join(GENERATED_REL);
    let en = fs::read_to_string(generated.join("catalog.en.json")).unwrap();
    assert!(en.contains("\"source_text\": \"RimLoc\""));
}

/// Self-localization audit §5: a catalog translation that RENAMES a
/// `{name}` placeholder is an ERROR finding from `project_validate` — the
/// strict set comparison against the base message. The generic
/// lost-placeholder pass cannot see this shape (the translation still
/// carries A placeholder), which is exactly the audit's hole: the i18n
/// runtime would render the typo'd `{…}` as literal text.
#[test]
fn validate_flags_a_renamed_catalog_placeholder_as_set_mismatch() {
    let generated = repo_root().join(GENERATED_REL);
    let fixture: SelflocFixture =
        serde_json::from_str(&fs::read_to_string(repo_root().join(FIXTURE_REL)).unwrap())
            .expect("shared selfloc fixture parses");

    // REAL generated catalog, copied — the repository files stay intact.
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = tmp.path().join("catalog");
    fs::create_dir_all(&catalog_dir).unwrap();
    for name in ["catalog.en.json", "catalog.ru.json", "catalog.meta.json"] {
        fs::copy(generated.join(name), catalog_dir.join(name)).unwrap();
    }

    // A REAL catalog message that carries the placeholder contract.
    let en: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(catalog_dir.join("catalog.en.json")).unwrap())
            .unwrap();
    let (id, source_text, placeholders) = en["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| {
            let ph = m["placeholders"].as_array()?;
            if ph.is_empty() {
                return None;
            }
            let names: Vec<String> = ph
                .iter()
                .filter_map(|p| p.as_str().map(str::to_string))
                .collect();
            Some((
                m["id"].as_str()?.to_string(),
                m["source_text"].as_str()?.to_string(),
                names,
            ))
        })
        .next()
        .expect("the real catalog has placeholder-bearing messages");

    // RENAME every {name} → {typo_name}: still well-formed braces, wrong set.
    let mut broken = source_text.clone();
    for p in &placeholders {
        broken = broken.replace(&format!("{{{p}}}"), &format!("{{typo_{p}}}"));
    }
    assert_ne!(broken, source_text);

    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).unwrap();
    let snap = mgr.create(&catalog_dir, None).unwrap();
    let res = mgr
        .apply(&apply_req(
            &snap.project_id,
            snap.session_epoch,
            snap.revision,
            vec![intent(&fixture.locale, &id, &broken)],
        ))
        .unwrap();
    assert_eq!(res.applied, 1);

    let v = mgr
        .validate_project(&snap.project_id, snap.session_epoch, Some(&fixture.locale))
        .unwrap();
    assert_eq!(v.status, "failed", "{v:?}");
    let hit = v
        .findings
        .iter()
        .find(|f| f.key == id && f.kind == "placeholder-check" && f.severity == "error")
        .expect("error-severity placeholder-check finding for the renamed placeholder");
    assert_eq!(hit.severity, "error");
    assert!(
        hit.message.contains("missing") && hit.message.contains("unexpected"),
        "{:?}",
        hit.message
    );
    for p in &placeholders {
        assert!(
            hit.message.contains(&format!("{{{p}}}")),
            "message must name the lost {{{p}}}: {:?}",
            hit.message
        );
        assert!(
            hit.message.contains(&format!("{{typo_{p}}}")),
            "message must name the invented {{typo_{p}}}: {:?}",
            hit.message
        );
    }
    // The rename shape is invisible to the generic lost-placeholder pass
    // (the translation still carries a token): no duplicate finding.
    assert!(
        !v.findings
            .iter()
            .any(|f| f.key == id && f.kind == "lost-placeholder"),
        "{v:?}"
    );
}

// ---------------------------------------------------------------------------
// SF-5: the real session-export -> TS junction.
// ---------------------------------------------------------------------------

/// Env override for the chain directory; BOTH halves must agree on the same
/// default so a plain `cargo test ui_catalog` + `npm test` is the whole
/// recipe.
const CHAIN_DIR_ENV: &str = "RIMLOC_SELFLOC_CHAIN_DIR";
const CHAIN_DIR_DEFAULT: &str = "/tmp/rimloc-selfloc-chain";

fn chain_dir() -> PathBuf {
    std::env::var_os(CHAIN_DIR_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Дефолт `/tmp/…` unix-специфичен: на windows это относительный
            // путь, и write-guard честно отказывает (InvalidOutputPath) —
            // контракт обязанен. Переносим во временный каталог платформы.
            if cfg!(windows) {
                std::env::temp_dir().join("rimloc-selfloc-chain")
            } else {
                PathBuf::from(CHAIN_DIR_DEFAULT)
            }
        })
}

/// Git-sha of the working copy that built the junction (wave 12, junction
/// race fix): the TS chain test compares this marker against ITS checkout
/// and SKIPS honestly on a mismatch instead of failing against foreign
/// bytes (a cargo run of ANOTHER worktree/version overwrites the shared
/// chain dir). `git rev-parse HEAD` identifies the checkout; std Command
/// is acceptable in tests.
fn builder_marker() -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root())
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "unknown_builder".to_string(),
    }
}

/// Minimal serialization helper for the chain manifest. It exists ONLY in
/// this test: `chain.json` is a test-to-test artifact, not a production
/// contract, so production code stays untouched.
///
/// Wave 12 (junction race): the manifest carries the `builder` git-sha and
/// is written ATOMICALLY — sibling temp file + rename in the SAME directory.
/// The caller writes it strictly AFTER every export file is on disk, so the
/// TS side can never observe (or race on) a half-written manifest: rename
/// is atomic within a directory, and until it lands the previous complete
/// manifest — if any — stays visible as a whole.
fn write_chain_manifest(
    chain_root: &std::path::Path,
    locale: &str,
    files: &serde_json::Value,
    values: std::collections::BTreeMap<String, String>,
) {
    let manifest = serde_json::json!({
        "builder": builder_marker(),
        "exported_at_run": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        "locale": locale,
        "files": files,
        "values": values,
    });
    let tmp_path = chain_root.join("chain.json.tmp");
    fs::write(&tmp_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    fs::rename(&tmp_path, chain_root.join("chain.json")).unwrap();
}

#[test]
fn chain_export_writes_real_artifacts_for_the_ts_side() {
    let generated = repo_root().join(GENERATED_REL);
    let fixture: SelflocFixture =
        serde_json::from_str(&fs::read_to_string(repo_root().join(FIXTURE_REL)).unwrap())
            .expect("shared selfloc fixture parses");

    // Deterministic chain directory, cleaned at the start of every run so a
    // stale artifact can never be mistaken for a fresh export.
    let chain_root = chain_dir();
    let export_dir = chain_root.join("export");
    if chain_root.exists() {
        fs::remove_dir_all(&chain_root).unwrap();
    }
    fs::create_dir_all(&export_dir).unwrap();

    // Compact ordinary session on a tmp catalog copy: create -> apply
    // (translations, one carrying a {placeholder}) -> validate clean.
    let tmp = tempfile::tempdir().unwrap();
    let catalog_dir = tmp.path().join("catalog");
    fs::create_dir_all(&catalog_dir).unwrap();
    for name in ["catalog.en.json", "catalog.ru.json", "catalog.meta.json"] {
        fs::copy(generated.join(name), catalog_dir.join(name)).unwrap();
    }
    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).unwrap();
    let snap = mgr.create(&catalog_dir, None).unwrap();
    let intents: Vec<TranslationIntent> = fixture
        .changes
        .iter()
        .chain(std::iter::once(&fixture.fixed))
        .map(|c| intent(&fixture.locale, &c.id, &c.value))
        .collect();
    let res = mgr
        .apply(&apply_req(
            &snap.project_id,
            snap.session_epoch,
            snap.revision,
            intents,
        ))
        .unwrap();
    assert_eq!(res.applied, 3);
    let v = mgr
        .validate_project(&snap.project_id, snap.session_epoch, Some(&fixture.locale))
        .unwrap();
    assert_eq!(v.status, "succeeded", "{v:?}");

    // THE canonical service export — this is what physically flows onward.
    let exp = mgr
        .export_project(
            &snap.project_id,
            snap.session_epoch,
            &export_dir,
            &fixture.locale,
        )
        .unwrap();
    assert!(exp.skipped_unknown_type.is_empty(), "{exp:?}");

    // The written file exists and physically contains the session-applied
    // text (Rust-side correctness; the TS side gets values from the reparse
    // below, not from this fixture comparison).
    let keyed_rel = std::path::Path::new("Languages")
        .join(&fixture.locale)
        .join("Keyed")
        .join("Translation.xml");
    let keyed_path = export_dir.join(&keyed_rel);
    assert!(keyed_path.exists(), "Keyed output missing: {keyed_path:?}");
    let keyed_xml = fs::read_to_string(&keyed_path).unwrap();
    assert!(keyed_xml.contains(&fixture.fixed.value), "{keyed_xml}");

    // chain.json values come from REPARSING THE WRITTEN FILES with the
    // ordinary scanner — the same evidence `export_project` itself accepts
    // on — never from the fixture.
    let units = rimloc_parsers_xml::scan_keyed_xml(&export_dir).unwrap();
    assert_eq!(units.len(), exp.reparsed_keys, "{exp:?}");
    assert!(!units.is_empty());
    let mut values = std::collections::BTreeMap::new();
    for u in units {
        let text = u.source.clone().unwrap_or_default();
        assert!(
            !text.trim().is_empty(),
            "exported key {} has no text",
            u.key
        );
        values.insert(u.key, text);
    }
    // What the session applied is what the disk now carries.
    assert_eq!(
        values.get(&fixture.fixed.id).map(String::as_str),
        Some(fixture.fixed.value.as_str())
    );

    // At least one exported value carries a {placeholder}: the placeholder
    // contract must survive the whole chain (validated here, asserted by the
    // TS pack/bundle gates on the same bytes).
    assert!(
        values.values().any(|t| t.contains('{') && t.contains('}')),
        "chain artifact must carry at least one placeholder-bearing value"
    );

    // The manifest path is relative to the CHAIN ROOT (not to the export
    // mod) so chain.json + the files it lists form one self-contained
    // artifact directory for the TS side.
    let files = serde_json::json!([{
        "path": std::path::Path::new("export")
            .join(&keyed_rel)
            .to_string_lossy()
            .replace('\\', "/"),
        "locale": fixture.locale,
        "key_count": values.len(),
    }]);
    write_chain_manifest(&chain_root, &fixture.locale, &files, values);

    // The manifest parses back (the TS side will read exactly this file)
    // and carries the builder marker the TS side matches its checkout
    // against (non-empty: a real sha, or the honest unknown fallback).
    let back: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(chain_root.join("chain.json")).unwrap()).unwrap();
    assert!(
        !back["builder"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .is_empty(),
        "chain_manifest_carries_builder_marker"
    );
}
