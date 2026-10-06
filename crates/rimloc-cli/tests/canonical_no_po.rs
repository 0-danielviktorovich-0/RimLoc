//! Canonical no-PO coverage (release/architecture-po, mandate §3):
//! PO stops being a mandatory internal seam. Three E2E scenarios:
//! - NO-PO-1: scan → project → apply → validate → build without a single
//!   PO byte (services API, the same create/apply the GUI uses);
//! - NO-PO-2: CLI translate (MockProvider) → canonical JSON report →
//!   review → build; the default run writes NO .po file anywhere;
//! - PO-INTEROP: canonical → export-po → import-po → the SAME identities
//!   (the kept adapters round-trip canonical state losslessly by key).
//!
//! Modeled after `export_po_game_version_loadfolders_keeps_root_keyed`
//! (fixture-in-tempdir + real binary), plus a services-level loop where the
//! session API already covers the flow.

use assert_cmd::prelude::*;
use rimloc_services::contract::{
    ApplyExistingRequest, ApplyIntentsRequest, IntentAction, TranslationIntent,
};
use rimloc_services::ProjectSessionManager;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc-cli binary built")
}

/// A minimal flat mod: two Keyed entries, nothing else translatable.
fn write_keyed_mod(root: &Path) {
    let keyed = root.join("Languages").join("English").join("Keyed");
    fs::create_dir_all(&keyed).unwrap();
    fs::write(
        keyed.join("NoPo.xml"),
        r#"<LanguageData>
	<GreetKey>Hello world</GreetKey>
	<FarewellKey>Goodbye friend</FarewellKey>
</LanguageData>
"#,
    )
    .unwrap();
    let about = root.join("About");
    fs::create_dir_all(&about).unwrap();
    fs::write(
        about.join("About.xml"),
        "<ModMetaData><name>NoPo</name><packageId>test.nopo</packageId>\
         <supportedVersions><li>1.6</li></supportedVersions></ModMetaData>\n",
    )
    .unwrap();
}

/// Recursively assert no .po file exists anywhere under `root`.
fn assert_no_po(root: &Path, context: &str) {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|x| x.to_str()) == Some("po") {
                out.push(p);
            }
        }
    }
    let mut found = Vec::new();
    walk(root, &mut found);
    assert!(
        found.is_empty(),
        "{context}: unexpected .po artifacts: {found:?}"
    );
}

/// The (identity → text) map of one locale's translations in a snapshot.
fn russian_translations(
    snap: &rimloc_services::contract::ProjectSnapshot,
) -> std::collections::BTreeMap<String, String> {
    snap.project
        .translations
        .iter()
        .filter(|t| t.locale == "Russian")
        .filter_map(|t| {
            t.text
                .clone()
                .map(|text| (t.source_id.display_identity(), text))
        })
        .collect()
}

/// NO-PO-1: scan → project → apply → validate → build. The full canonical
/// loop over the session API (the same one the GUI drives) without a single
/// PO byte — and the built output re-verified to carry the translations.
#[test]
fn no_po_1_scan_project_apply_validate_build() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mod_root = tmp.path().join("Mod");
    write_keyed_mod(&mod_root);
    let managed = tmp.path().join("managed");

    // scan → project: create builds the canonical inventory from the
    // read-only source and persists the managed record.
    let manager = ProjectSessionManager::new(&managed).unwrap();
    let snap = manager.create(&mod_root, None).unwrap();
    let keyed: Vec<_> = snap
        .project
        .entries
        .iter()
        .filter(|e| {
            e.id.kind == rimloc_domain::canonical::EntryKind::Keyed && !e.text.trim().is_empty()
        })
        .cloned()
        .collect();
    assert_eq!(keyed.len(), 2, "fixture must yield two Keyed entries");

    // apply: typed intents, the single write path.
    let intents: Vec<TranslationIntent> = keyed
        .iter()
        .map(|e| TranslationIntent {
            entry: e.id.clone(),
            locale: "Russian".into(),
            action: IntentAction::SetTranslation,
            text: Some(format!("«{}»", e.text)),
        })
        .collect();
    let res = manager
        .apply(&ApplyIntentsRequest {
            project_id: snap.project_id.clone(),
            expected_revision: snap.revision,
            session_epoch: snap.session_epoch,
            origin: None,
            intents,
        })
        .unwrap();
    assert_eq!(res.applied, 2, "both intents must apply: {:?}", res.skipped);
    assert!(res.skipped.is_empty());

    // validate: the trusted-state validator over the applied locale.
    let verdict = manager
        .validate_project(&snap.project_id, snap.session_epoch, Some("Russian"))
        .unwrap();
    assert_eq!(
        verdict.status, "succeeded",
        "findings: {:?}",
        verdict.findings
    );

    // build: the full mod package straight from canonical state.
    let out_dir = tmp.path().join("out_mod");
    let built = manager
        .build_mod_project(&snap.project_id, snap.session_epoch, &out_dir, "Russian")
        .unwrap();
    assert!(built.files_written >= 1);

    // The built Keyed file carries the applied translations.
    let xml = fs::read_to_string(
        out_dir
            .join("Languages")
            .join("Russian")
            .join("Keyed")
            .join("Translation.xml"),
    )
    .unwrap();
    assert!(
        xml.contains("«Hello world»"),
        "missing translation in:\n{xml}"
    );
    assert!(
        xml.contains("«Goodbye friend»"),
        "missing translation in:\n{xml}"
    );

    // NO PO anywhere: neither the output nor the managed store, which does
    // hold the persisted canonical project record.
    assert_no_po(tmp.path(), "NO-PO-1");
    let managed_has_record = fs::read_dir(&managed)
        .unwrap()
        .flatten()
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"));
    assert!(managed_has_record, "canonical record must be persisted");
}

/// NO-PO-2: CLI translate (MockProvider) → canonical JSON report → review
/// → build. The default run creates the canonical project and the versioned
/// JSON report — and NO .po file. Dry-run parity: the dry run reports
/// exactly the unit count the real run applies, and writes nothing.
#[test]
fn no_po_2_mock_translate_review_build_without_po() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mod_root = tmp.path().join("Mod");
    write_keyed_mod(&mod_root);
    let managed = tmp.path().join("managed");
    let out_json = tmp.path().join("results.json");

    // Dry run: scope report only — no provider call, no managed store, no
    // output files.
    let mut dry = bin_cmd();
    dry.args(["translate", "--root"])
        .arg(&mod_root)
        .args(["--provider", "mock", "--ui-lang", "en"])
        .arg("--managed-root")
        .arg(&managed)
        .arg("--out-json")
        .arg(&out_json)
        .arg("--dry-run");
    let dry_out = dry.assert().success();
    let dry_stderr = String::from_utf8_lossy(dry_out.get_output().stderr.as_ref()).to_string();
    let dry_units: usize = parse_dry_run_units(&dry_stderr);
    assert_eq!(dry_units, 2, "dry run must report the fixture scope");
    assert!(
        !managed.exists(),
        "dry run must not create the managed store"
    );
    assert!(!out_json.exists(), "dry run must not write results");

    // Real run: canonical apply + JSON report; CWD confined to tmp so a
    // stray default artifact would be visible there.
    let mut cmd = bin_cmd();
    cmd.current_dir(tmp.path());
    cmd.args(["translate", "--root"])
        .arg(&mod_root)
        .args(["--provider", "mock", "--ui-lang", "en"])
        .arg("--managed-root")
        .arg(&managed)
        .arg("--out-json")
        .arg(&out_json);
    cmd.assert().success();

    // The seam is gone: no .po anywhere in the run directory.
    assert_no_po(tmp.path(), "NO-PO-2");
    assert!(!tmp.path().join("rimloc-translated.po").exists());

    // Review: the canonical JSON report (schema v1).
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_json).unwrap()).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["locale"], "Russian");
    assert_eq!(report["provider"], "mock");
    let rows = report["results"].as_array().unwrap();
    // Dry-run parity: the scope the dry run reported is exactly what the
    // real run produced.
    assert_eq!(
        rows.len(),
        dry_units,
        "dry-run units must equal applied rows"
    );
    for row in rows {
        assert_eq!(row["status"], "applied", "row: {row}");
        assert_eq!(row["locale"], "Russian");
        assert_eq!(row["provenance"]["provider"], "mock");
        assert_eq!(row["provenance"]["origin"], "llm");
        let target = row["target"].as_str().unwrap();
        assert!(
            target.starts_with("ru: "),
            "mock translation expected, got {target}"
        );
    }
    assert_eq!(report["summary"]["applied"], 2);

    // Build straight from the canonical project the CLI created.
    let manager = ProjectSessionManager::new(&managed).unwrap();
    let pid = report["project_id"].as_str().unwrap().to_string();
    let snap = manager.open(&pid).unwrap();
    let translations = russian_translations(&snap);
    assert_eq!(translations.len(), 2);

    let out_dir = tmp.path().join("built");
    manager
        .build_mod_project(&pid, snap.session_epoch, &out_dir, "Russian")
        .unwrap();
    let xml = fs::read_to_string(
        out_dir
            .join("Languages")
            .join("Russian")
            .join("Keyed")
            .join("Translation.xml"),
    )
    .unwrap();
    assert!(xml.contains("ru: Hello world"), "missing in:\n{xml}");
    assert_no_po(tmp.path(), "NO-PO-2 build");
}

/// PO-INTEROP: canonical → export-po → import-po → the same identities.
/// The kept PO adapters must round-trip canonical state losslessly by key:
/// every identity translated by the canonical run comes back with the same
/// text through the PO files.
#[test]
fn po_interop_canonical_export_import_keeps_identity() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mod_root = tmp.path().join("Mod");
    write_keyed_mod(&mod_root);
    let managed = tmp.path().join("managed");
    let out_json = tmp.path().join("results.json");

    // 1) Canonical translate (MockProvider → "ru: <source>", Russian).
    let mut cmd = bin_cmd();
    cmd.current_dir(tmp.path());
    cmd.args(["translate", "--root"])
        .arg(&mod_root)
        .args(["--provider", "mock", "--ui-lang", "en"])
        .arg("--managed-root")
        .arg(&managed)
        .arg("--out-json")
        .arg(&out_json);
    cmd.assert().success();
    let report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_json).unwrap()).unwrap();
    let pid1 = report["project_id"].as_str().unwrap().to_string();

    // 2) Canonical state → the built mod package (Languages/Russian tree).
    let manager = ProjectSessionManager::new(&managed).unwrap();
    let snap1 = manager.open(&pid1).unwrap();
    let out_mod = tmp.path().join("out_mod");
    manager
        .build_mod_project(&pid1, snap1.session_epoch, &out_mod, "Russian")
        .unwrap();

    // 3) export-po adapter: msgid from the Russian source side, msgstr
    //    prefilled from the same tree via the TM root. Composite msgctxt
    //    (`Key|path`) is the adapter's identity carrier.
    let interop_po = tmp.path().join("interop.po");
    let mut exp = bin_cmd();
    exp.args(["export-po", "--root"])
        .arg(&out_mod)
        .arg("--out-po")
        .arg(&interop_po)
        .args(["--source-lang", "ru"])
        .arg("--tm-root")
        .arg(out_mod.join("Languages").join("Russian"));
    exp.assert().success();
    let po_text = fs::read_to_string(&interop_po).unwrap();
    assert!(
        po_text.contains("ru: Hello world"),
        "export-po must carry the translation:\n{po_text}"
    );

    // 4) import-po adapter: the PO back into a FRESH copy of the source
    //    mod as its Russian pack.
    let mod2 = tmp.path().join("Mod2");
    copy_dir(&mod_root, &mod2);
    let mut imp = bin_cmd();
    imp.args(["import-po", "--po"])
        .arg(&interop_po)
        .arg("--mod-root")
        .arg(&mod2)
        .args(["--lang", "ru"]);
    imp.assert().success();
    let pack = fs::read_to_string(
        mod2.join("Languages")
            .join("Russian")
            .join("Keyed")
            .join("Translation.xml"),
    )
    .unwrap();
    assert!(
        pack.contains("ru: Hello world"),
        "import-po output:\n{pack}"
    );

    // 5) The pack lands in a fresh canonical project via apply_existing.
    let snap2 = manager.create(&mod2, None).unwrap();
    let applied = manager
        .apply_existing(&ApplyExistingRequest {
            project_id: snap2.project_id.clone(),
            expected_revision: snap2.revision,
            session_epoch: snap2.session_epoch,
            existing_dir: rimloc_services::contract::PathBufDto::new(
                mod2.join("Languages").join("Russian").display().to_string(),
            ),
            locale: "Russian".into(),
        })
        .unwrap();
    assert!(applied.applied >= 2, "pack import must apply: {applied:?}");

    // 6) THE SAME identities, THE SAME texts — canonical → PO → canonical.
    let t1 = russian_translations(&manager.open(&pid1).unwrap());
    let t2 = russian_translations(&manager.open(&snap2.project_id).unwrap());
    assert_eq!(t1.len(), 2);
    assert_eq!(t1, t2, "PO round-trip must preserve identity and text");
}

fn parse_dry_run_units(stderr: &str) -> usize {
    // "Dry run: { $units } units, ~{ $batches } batches; ..."
    for line in stderr.lines() {
        if let Some(rest) = line.strip_prefix("ℹ Dry run:") {
            let digits: String = rest
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = digits.parse() {
                return n;
            }
        }
    }
    panic!("dry-run unit count not found in stderr:\n{stderr}");
}

fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap().flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        if from.is_dir() {
            copy_dir(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}
