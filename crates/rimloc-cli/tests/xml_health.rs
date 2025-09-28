use assert_cmd::prelude::*;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
#[allow(dead_code)]
struct HealthOut {
    checked: usize,
    issues: Vec<Issue>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Issue {
    path: String,
    category: String,
    error: String,
}

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc-cli built")
}

fn write(root: &Path, rel: &str, content: &str) {
    let p = root
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

#[test]
fn xml_health_detects_doctype_encoding_mismatch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();
    write(
        &root,
        "doctype.xml",
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE foo>\n<LanguageData><Key>v</Key></LanguageData>",
    );
    write(
        &root,
        "enc.xml",
        "<?xml version=\"1.0\" encoding=\"windows-1251\"?>\n<LanguageData><K>v</K></LanguageData>",
    );
    write(
        &root,
        "mismatch.xml",
        "<LanguageData><A>1</B></LanguageData>",
    );

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "xml-health", "--root"])
        .arg(&root)
        .args(["--format", "json"]) // restrict to key categories
        .args([
            "--only",
            "unexpected-doctype,encoding-detected,tag-mismatch,parse",
        ]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let parsed: HealthOut = serde_json::from_str(&out).expect("valid json");
    let cats: std::collections::HashSet<_> =
        parsed.issues.iter().map(|i| i.category.as_str()).collect();
    assert!(cats.contains("encoding-detected"));
    assert!(cats.contains("unexpected-doctype"));
    assert!(
        cats.contains("tag-mismatch") || cats.contains("parse"),
        "expected tag structure error category"
    );
}

#[test]
fn xml_health_detects_invalid_char() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();
    // Insert ASCII ESC (0x1B) control char into content
    let p = root
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join("ctrl.xml");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let bytes = b"<LanguageData><X>bad\x1Bchar</X></LanguageData>".to_vec();
    fs::write(&p, &bytes).unwrap();

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "xml-health", "--root"])
        .arg(&root)
        .args(["--format", "json"]) // restrict to invalid-char
        .args(["--only", "invalid-char"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let parsed: HealthOut = serde_json::from_str(&out).expect("valid json");
    assert_eq!(
        parsed.issues.len(),
        1,
        "expected exactly one invalid-char issue"
    );
    assert_eq!(parsed.issues[0].category, "invalid-char");
}
