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

use rimloc_core::winner_reason;
use rimloc_domain::canonical::{EntryKind, SourceEntryId};
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
    let out = tmp.path().join("out");
    let exp = mgr
        .export_project(&snap.project_id, snap.session_epoch, &out, &fixture.locale)
        .unwrap();
    assert_eq!(exp.reparsed_keys, 3, "{exp:?}");
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

    // 6. sourceChanged: edit a source message in the TMP catalog copy.
    let mut en: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(catalog_dir.join("catalog.en.json")).unwrap())
            .unwrap();
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
    // A restart-recovered session sees the same drift (fresh manager).
    let mgr2 = ProjectSessionManager::new(tmp.path().join("managed")).unwrap();
    let reopened = mgr2.open(&refreshed.project_id).unwrap();
    assert_eq!(reopened.source_changed, Some(true));
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
