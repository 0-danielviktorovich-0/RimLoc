use assert_cmd::prelude::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc-cli built")
}

fn write_rel(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

#[test]
fn build_mod_filters_multiple_versions_from_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src: PathBuf = tmp.path().to_path_buf();
    // Create 1.4 and v1.6 sources under from_root
    write_rel(
        &src,
        "1.4/Languages/Russian/Keyed/A.xml",
        "<LanguageData><Old>старый</Old></LanguageData>",
    );
    write_rel(
        &src,
        "v1.6/Languages/Russian/Keyed/B.xml",
        "<LanguageData><New>новый</New></LanguageData>",
    );

    // Build into out dir, selecting only v1.6
    let out = tempfile::tempdir().expect("out");
    let out_dir = out.path().join("RimLoc_RU");

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "build-mod"]) // simple run, not dry-run
        .args(["--po", "./test/ok.po"]) // required arg; content ignored when --from-root used
        .args(["--out-mod"])
        .arg(&out_dir)
        .args(["--lang", "ru"]) // lang folder name resolution
        .args(["--from-root"])
        .arg(&src)
        .args(["--from-game-version", "v1.6"]);
    cmd.current_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap(),
    );
    cmd.assert().success();

    // Check that only B.xml exists under out Languages
    let b = out_dir
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join("B.xml");
    assert!(b.exists(), "expected B.xml from v1.6");
    let a = out_dir
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join("A.xml");
    assert!(!a.exists(), "A.xml from 1.4 must be excluded");
}
