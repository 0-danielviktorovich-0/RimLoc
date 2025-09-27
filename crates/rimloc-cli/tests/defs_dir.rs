use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;

fn bin_cmd() -> Command {
    let mut cmd = Command::cargo_bin("rimloc-cli").expect("binary built");
    cmd.env("RUST_LOG", "warn");
    cmd
}

#[test]
fn scan_respects_defs_dir_override() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create root/Defs and v1.6/Defs with distinct defNames
    let defs_a = root.join("Defs/ThingDefs_Items");
    fs::create_dir_all(&defs_a).unwrap();
    fs::write(
        defs_a.join("A.xml"),
        r#"<Defs>
  <ThingDef>
    <defName>A1</defName>
    <label>a one</label>
  </ThingDef>
</Defs>
"#,
    )
    .unwrap();

    let defs_b = root.join("v1.6/Defs/ThingDefs_Items");
    fs::create_dir_all(&defs_b).unwrap();
    fs::write(
        defs_b.join("B.xml"),
        r#"<Defs>
  <ThingDef>
    <defName>B1</defName>
    <label>b one</label>
    <title>custom title</title>
  </ThingDef>
</Defs>
"#,
    )
    .unwrap();

    // When include_all_versions is set, we scan from 'root'; override defs_dir to 'Defs'
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]) // stdout JSON
        .arg(root)
        .args(["--format", "json", "--include-all-versions", "--defs-dir", "Defs"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(stdout.contains("A1.label"), "expected A1.label from root/Defs");
    assert!(!stdout.contains("B1.label"), "did not expect B1.label from v1.6/Defs");

    // Now point to v1.6/Defs explicitly
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]) // stdout JSON
        .arg(root)
        .args([
            "--format",
            "json",
            "--include-all-versions",
            "--defs-dir",
            "v1.6/Defs",
        ]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(stdout.contains("B1.label"), "expected B1.label from v1.6/Defs");
    assert!(!stdout.contains("A1.label"), "did not expect A1.label from root/Defs");

    // With extra defs field, custom field should be extracted
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]) // stdout JSON
        .arg(root)
        .args([
            "--format",
            "json",
            "--include-all-versions",
            "--defs-dir",
            "v1.6/Defs",
            "--defs-field",
            "title",
        ]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(stdout.contains("B1.title"), "expected B1.title from v1.6/Defs with --defs-field title");
}
