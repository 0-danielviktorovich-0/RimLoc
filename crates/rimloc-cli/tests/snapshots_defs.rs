use assert_cmd::prelude::*;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("binary built")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn sanitize_json_units(mut v: Value) -> Value {
    let ws = workspace_root().display().to_string();
    if let Value::Array(arr) = &mut v {
        arr.sort_by(|a, b| {
            let ak = a.get("key").and_then(|x| x.as_str()).unwrap_or("");
            let ap = a.get("path").and_then(|x| x.as_str()).unwrap_or("");
            let bk = b.get("key").and_then(|x| x.as_str()).unwrap_or("");
            let bp = b.get("path").and_then(|x| x.as_str()).unwrap_or("");
            (ak, ap).cmp(&(bk, bp))
        });
        for obj in arr.iter_mut() {
            if let Some(p) = obj.get_mut("path") {
                if let Some(s) = p.as_str() {
                    let norm = s.replace(&ws, "<WS>");
                    *p = Value::String(norm);
                }
            }
        }
    }
    v
}

#[test]
fn snapshot_scan_defs_only_json() {
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"])
        .arg(workspace_root().join("test/DefsOnly"));
    cmd.args(["--format", "json", "--source-lang-dir", "English"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let v: Value = serde_json::from_str(&stdout).expect("valid json");
    let v = sanitize_json_units(v);
    insta::assert_json_snapshot!(v);
}

#[test]
fn snapshot_validate_defs_only_json() {
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "validate", "--root"])
        .arg(workspace_root().join("test/DefsOnly"));
    cmd.args(["--format", "json", "--source-lang-dir", "English"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let v: Value = serde_json::from_str(&stdout).expect("valid json");
    let v = sanitize_json_units(v);
    insta::assert_json_snapshot!(v);
}

#[test]
fn snapshot_diff_defs_only_json() {
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "diff-xml", "--root"])
        .arg(workspace_root().join("test/DefsOnly"));
    cmd.args([
        "--format",
        "json",
        "--source-lang-dir",
        "English",
        "--lang-dir",
        "Russian",
    ]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let v: Value = serde_json::from_str(&stdout).expect("valid json");
    insta::assert_json_snapshot!(v);
}
