use assert_cmd::prelude::*;
use serde::Deserialize;
use std::{fs, path::PathBuf, process::Command};

#[derive(Deserialize)]
struct FileStat {
    path: String,
    keys: usize,
    status: String,
    added: Vec<String>,
    changed: Vec<String>,
}

#[derive(Deserialize)]
struct Summary {
    mode: String,
    created: usize,
    updated: usize,
    skipped: usize,
    keys: usize,
    files: Vec<FileStat>,
}

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc-cli built")
}

fn write_xml(root: &PathBuf, content: &str) -> PathBuf {
    let path = root
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join("Sample.xml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

fn make_po(entries: &[(&str, &str)]) -> String {
    // Build a minimal PO: each entry has a reference into Languages/Russian/Keyed/Sample.xml
    let mut out = String::new();
    for (k, v) in entries {
        out.push_str("#: Languages/Russian/Keyed/Sample.xml:1\n");
        out.push_str(&format!("msgctxt \"{}|Languages/Russian/Keyed/Sample.xml:1\"\n", k));
        out.push_str(&format!("msgstr \"{}\"\n\n", v));
    }
    out
}

#[test]
fn import_only_diff_updates_only_changed_keys() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();
    // Existing file with A and B
    write_xml(
        &root,
        r#"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<LanguageData>
  <A>oldA</A>
  <B>oldB</B>
</LanguageData>
"#,
    );

    // PO has A (unchanged), B (changed), C (new)
    let po_text = make_po(&[("A", "oldA"), ("B", "newB"), ("C", "newC")]);
    let po_path = tmp.path().join("diff.po");
    fs::write(&po_path, po_text).unwrap();

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "import-po"]) // JSON report, only-diff
        .args(["--po"]).arg(&po_path)
        .args(["--mod-root"]).arg(&root)
        .args(["--lang", "ru"]) // resolve Languages/Russian
        .args(["--only-diff"]).args(["--report"]).args(["--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    // stdout: last non-empty line is JSON summary
    let json_line = out
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("have json line");
    let rep: Summary = serde_json::from_str(json_line).expect("json summary");

    assert_eq!(rep.updated, 1, "expected exactly one updated file");
    assert_eq!(rep.created, 0);
    assert_eq!(rep.skipped, 0);
    assert_eq!(rep.keys, 2, "only changed/new keys should be written");
    assert_eq!(rep.files.len(), 1);
    let f = &rep.files[0];
    assert_eq!(f.status, "updated");
    // 'B' changed, 'C' added
    assert!(f.changed.contains(&"B".to_string()));
    assert!(f.added.contains(&"C".to_string()));
}

#[test]
fn import_incremental_skips_identical() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();
    // Existing file equals entries in PO
    // Write with the same renderer as CLI to ensure byte-for-byte identity
    let out_path = root
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join("Sample.xml");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    let entries = vec![("A".to_string(), "same".to_string()), ("B".to_string(), "same".to_string())];
    rimloc_import_po::write_language_data_xml(&out_path, &entries).expect("write xml");
    let po_text = make_po(&[("A", "same"), ("B", "same")]);
    let po_path = tmp.path().join("same.po");
    fs::write(&po_path, po_text).unwrap();

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "import-po"]) // incremental compare should skip
        .args(["--po"]).arg(&po_path)
        .args(["--mod-root"]).arg(&root)
        .args(["--lang", "ru"]) // resolve Languages/Russian
        .args(["--incremental"]).args(["--report"]).args(["--format", "json"]);

    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json_line = out
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("have json line");
    let rep: Summary = serde_json::from_str(json_line).expect("json summary");
    assert_eq!(rep.skipped, 1, "expected one skipped file due to identical content");
    assert_eq!(rep.updated + rep.created, 0);
}
