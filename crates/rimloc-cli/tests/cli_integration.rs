use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::{fs, path::PathBuf, process::Command};

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("binary rimloc-cli should be built by cargo")
}

fn workspace_root() -> PathBuf {
    // crates/rimloc-cli -> <workspace root>
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap() // crates/
        .parent().unwrap() // <workspace root>
        .to_path_buf()
}

fn fixture(rel: &str) -> PathBuf {
    workspace_root().join(rel)
}

#[test]
fn help_works() {
    let mut cmd = bin_cmd();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("RimWorld localization toolkit"));
}

#[test]
fn scan_outputs_csv_header() {
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(fixture("test/TestMod"));
    cmd.assert()
        .success()
        // Проверяем только заголовок CSV — он не локализуется
        .stdout(predicate::str::contains("key,source,path,line"));
}

#[test]
fn export_po_creates_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out_po = tmp.path().join("out.po");

    let mut cmd = bin_cmd();
    cmd.args(["export-po", "--root"])
        .arg(fixture("test/TestMod"))
        .args(["--out-po"])
        .arg(&out_po);

    cmd.assert().success();

    let meta = fs::metadata(&out_po).expect("out.po should exist");
    assert!(meta.len() > 0, "out.po should not be empty");
}

#[test]
fn import_po_dry_run_prints_indicator() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"])
        .arg(fixture("test/bad.po"))
        .args(["--mod-root"])
        .arg(fixture("test/TestMod"))
        .arg("--dry-run");

    // В сообщении есть общий токен "DRY-RUN" и в en, и в ru
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DRY-RUN"));
}