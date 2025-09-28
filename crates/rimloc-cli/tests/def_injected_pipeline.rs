use assert_cmd::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cargo_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("binary built")
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ImportFileStat {
    path: String,
    keys: usize,
    status: String,
    added: Vec<String>,
    changed: Vec<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ImportSummary {
    mode: String,
    created: usize,
    updated: usize,
    skipped: usize,
    keys: usize,
    files: Vec<ImportFileStat>,
}

#[test]
fn scan_detects_defs_without_english_definj() {
    let mut cmd = cargo_cmd();
    let root = workspace_root().join("test/DefInjectedOnly");
    cmd.args(["--quiet", "scan", "--root"]).arg(&root);
    cmd.args(["--format", "json", "--source-lang-dir", "English"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json: Value = serde_json::from_str(&stdout).expect("valid json");
    let arr = json.as_array().expect("array");
    let paths: Vec<&str> = arr
        .iter()
        .filter_map(|item| item.get("path").and_then(|p| p.as_str()))
        .collect();
    let keys: Vec<&str> = arr
        .iter()
        .filter_map(|item| item.get("key").and_then(|k| k.as_str()))
        .collect();
    assert!(keys.contains(&"Meal_Simple.label"));
    assert!(keys.contains(&"Meal_Fine.description"));
    assert!(
        paths
            .iter()
            .any(|p| p.contains("Languages/English/DefInjected/ThingDef/Food.xml")),
        "scan should surface DefInjected target path for learned defs",
    );
}

#[test]
fn scan_reports_both_keyed_and_defs() {
    let mut cmd = cargo_cmd();
    let root = workspace_root().join("test/MixedMod");
    cmd.args(["--quiet", "scan", "--root"]).arg(&root);
    cmd.args(["--format", "json", "--source-lang-dir", "English"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json: Value = serde_json::from_str(&stdout).expect("valid json");
    let arr = json.as_array().expect("array");
    let mut has_keyed = false;
    let mut has_def = false;
    let mut has_definj_path = false;
    for item in arr {
        if let Some(key) = item.get("key").and_then(|k| k.as_str()) {
            if key == "Greeting" {
                has_keyed = true;
            }
            if key == "Weapon_Bow.description" {
                has_def = true;
                if let Some(path) = item.get("path").and_then(|p| p.as_str()) {
                    if path.contains("Languages/English/DefInjected/ThingDef/Weapons.xml") {
                        has_definj_path = true;
                    }
                }
            }
        }
    }
    assert!(has_keyed, "Greeting from Keyed should be present");
    assert!(
        has_def,
        "Weapon_Bow.description from Defs should be present"
    );
    assert!(
        has_definj_path,
        "DefInjected entries should point to the canonical English path"
    );
}

#[test]
fn export_po_emits_definj_entries_and_hint() {
    let tmp = TempDir::new().expect("temp dir");
    let out_po = tmp.path().join("out.po");
    let mut cmd = cargo_cmd();
    let root = workspace_root().join("test/DefInjectedOnly");
    cmd.args(["--quiet", "export-po", "--root"]).arg(&root);
    cmd.args(["--out-po"]).arg(&out_po);
    cmd.args(["--lang", "ru", "--source-lang-dir", "English"]);
    let assert = cmd.assert().success();
    let stderr = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
    assert!(
        stderr.contains("_learn/suggested.xml"),
        "should hint about suggested.xml"
    );
    let po = fs::read_to_string(&out_po).expect("po written");
    assert!(po.contains("Meal_Fine.description"));
    assert!(po.contains("Languages/English/DefInjected/ThingDef/Food.xml"));
}

#[test]
fn learn_defs_produces_ready_templates() {
    let tmp = TempDir::new().expect("temp dir");
    let out_dir = tmp.path().join("_learn");
    let mut cmd = cargo_cmd();
    let root = workspace_root().join("test/DefInjectedOnly");
    cmd.args(["--quiet", "learn-defs", "--mod-root"]).arg(&root);
    cmd.args(["--out-dir"]).arg(&out_dir);
    cmd.args(["--no-ml"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(
        stdout.contains("suggested"),
        "summary should mention suggested file"
    );

    let suggested = out_dir.join("suggested.xml");
    let suggested_content = fs::read_to_string(&suggested).expect("suggested.xml");
    assert!(suggested_content.contains("Meal_Fine.description"));
    assert!(suggested_content.contains("A more refined dish."));

    let tree_file = out_dir
        .join("Languages")
        .join("English")
        .join("DefInjected")
        .join("ThingDef")
        .join("Food.xml");
    let tree_content = fs::read_to_string(&tree_file).expect("definj tree file");
    assert!(tree_content.contains("Meal_Simple.label"));
    assert!(tree_content.contains("simple meal"));
}

#[test]
fn scan_keyed_only_mod_still_works() {
    let mut cmd = cargo_cmd();
    let root = workspace_root().join("test/TestMod");
    cmd.args(["--quiet", "scan", "--root"]).arg(&root);
    cmd.args(["--format", "json", "--source-lang-dir", "English"]);
    cmd.assert().success();
}

#[test]
fn import_po_updates_definj_files() {
    let tmp = TempDir::new().expect("temp dir");
    let root = tmp.path();

    let russian_definj = root
        .join("Languages")
        .join("Russian")
        .join("DefInjected")
        .join("ThingDef");
    std::fs::create_dir_all(&russian_definj).expect("definj tree");
    let food_xml = russian_definj.join("Food.xml");
    std::fs::write(
        &food_xml,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<LanguageData>
  <Meal_Simple.label>Старое блюдо</Meal_Simple.label>
</LanguageData>
"#,
    )
    .expect("write initial xml");

    let po_path = root.join("update.po");
    std::fs::write(
        &po_path,
        r#"#: Languages/Russian/DefInjected/ThingDef/Food.xml:2
msgctxt "Meal_Simple.label|DefInjected/ThingDef/Food.xml:2"
msgstr "Новое блюдо"

#: Languages/Russian/DefInjected/ThingDef/Food.xml:3
msgctxt "Meal_Fine.description|DefInjected/ThingDef/Food.xml:3"
msgstr "Изящное описание"
"#,
    )
    .expect("write po");

    let mut cmd = cargo_cmd();
    cmd.args(["--quiet", "import-po", "--po"]).arg(&po_path);
    cmd.args(["--mod-root"]).arg(root);
    cmd.args(["--lang", "ru", "--report", "--format", "json"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("json summary line");
    let summary: ImportSummary = serde_json::from_str(json_line).expect("summary json");

    assert_eq!(summary.mode, "import");
    assert_eq!(summary.updated, 1);
    assert_eq!(summary.created, 0);
    assert_eq!(summary.skipped, 0);
    assert_eq!(summary.keys, 2);
    let stat = summary
        .files
        .iter()
        .find(|f| {
            f.path
                .replace('\\', "/")
                .ends_with("DefInjected/ThingDef/Food.xml")
        })
        .expect("stats for Food.xml");
    assert_eq!(stat.status, "updated");
    assert_eq!(stat.keys, 2);

    let updated = std::fs::read_to_string(&food_xml).expect("read updated xml");
    assert!(updated.contains("Новое блюдо"));
    assert!(updated.contains("Изящное описание"));
}
