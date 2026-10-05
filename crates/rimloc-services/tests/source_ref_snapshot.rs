//! Live Source Inspector bridge (wave 12): session snapshots decorate
//! entries with the OPTIONAL `source_ref` projection — the EFFECTIVE
//! context's file relative to the project root, the parser-guaranteed line
//! and the winner reason. This is the mod-project half of the evidence;
//! the ui-catalog half lives in `ui_catalog_session.rs`.

use rimloc_core::winner_reason;
use rimloc_services::ProjectSessionManager;
use std::fs;
use std::path::Path;

fn write(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

#[test]
fn mod_project_snapshot_carries_source_ref() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("MyMod");
    write(
        &root.join("Languages/English/Keyed/K.xml"),
        "<LanguageData>\n  <Greeting>hello</Greeting>\n</LanguageData>\n",
    );

    let mgr = ProjectSessionManager::new(tmp.path().join("managed")).unwrap();
    let snap = mgr.create(&root, None).unwrap();

    let e = snap
        .project
        .entries
        .iter()
        .find(|e| e.id.key == "Greeting")
        .expect("keyed_entry_present");
    let sr = e.source_ref.as_ref().expect("snapshot_carries_source_ref");
    // Path is RELATIVE to the mod root; the line mirrors the recorded
    // effective context (never fabricated, never recomputed differently).
    assert_eq!(sr.file, "Languages/English/Keyed/K.xml", "{sr:?}");
    assert!(sr.line.is_some(), "keyed_parser_records_lines: {sr:?}");
    assert_eq!(sr.line, e.contexts[0].line, "projection_mirrors_context");
    assert_eq!(sr.selected_by, winner_reason::KEYED_FIRST_IN_FILE);

    // The wire snapshot carries the projection as data (this is what the
    // SourceTab live mode renders).
    let json = serde_json::to_value(&snap).unwrap();
    let entry_json = json["project"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"]["key"] == "Greeting")
        .expect("entry_json_present");
    assert_eq!(
        entry_json["source_ref"]["file"],
        "Languages/English/Keyed/K.xml"
    );
    assert_eq!(
        entry_json["source_ref"]["selected_by"],
        serde_json::json!(winner_reason::KEYED_FIRST_IN_FILE)
    );

    // The decoration is a snapshot-time view: the durable managed state
    // stays projection-free (no denormalized drift copy on disk).
    let persisted = fs::read_to_string(mgr.managed_path(&snap.project_id).unwrap()).unwrap();
    assert!(
        !persisted.contains("source_ref"),
        "persisted_bytes_stay_projection_free"
    );
}
