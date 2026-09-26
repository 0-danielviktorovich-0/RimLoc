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

/// M6: a rebuild into an EXISTING non-empty out dir silently merged with the
/// previous build, shipping dead keys from removed source files. The build
/// now refuses a non-empty target unless `--merge` is passed consciously;
/// `--merge` warns that stale files are kept (no cleanup).
#[test]
fn build_mod_refuses_nonempty_out_without_merge_flag() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src: PathBuf = tmp.path().join("src");
    write_rel(
        &src,
        "Languages/Russian/Keyed/A.xml",
        "<LanguageData><Old>старый</Old></LanguageData>",
    );
    let out = tempfile::tempdir().expect("out");
    let out_dir = out.path().join("RimLoc_RU");
    let cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // First build into a fresh dir succeeds.
    let mut cmd = bin_cmd();
    // --ui-lang en: the M6 messages under assertion are localized.
    cmd.args(["--quiet", "--ui-lang", "en", "build-mod"])
        .args(["--po", "./test/ok.po"])
        .args(["--out-mod"])
        .arg(&out_dir)
        .args(["--lang", "ru"])
        .args(["--from-root"])
        .arg(&src);
    cmd.current_dir(&cwd);
    cmd.assert().success();
    assert!(out_dir.join("Languages/Russian/Keyed/A.xml").exists());

    // Source changed (B added): a plain rebuild into the same non-empty dir
    // is a refusal, never a silent merge.
    write_rel(
        &src,
        "Languages/Russian/Keyed/B.xml",
        "<LanguageData><New>новый</New></LanguageData>",
    );

    let mut cmd = bin_cmd();
    // --ui-lang en: the M6 messages under assertion are localized.
    cmd.args(["--quiet", "--ui-lang", "en", "build-mod"])
        .args(["--po", "./test/ok.po"])
        .args(["--out-mod"])
        .arg(&out_dir)
        .args(["--lang", "ru"])
        .args(["--from-root"])
        .arg(&src);
    cmd.current_dir(&cwd);
    cmd.assert().failure();

    // With --merge the rebuild is a conscious overwrite-with-keep: succeeds
    // and warns that stale files are kept (A.xml survives).
    let mut cmd = bin_cmd();
    // --ui-lang en: the M6 messages under assertion are localized.
    cmd.args(["--quiet", "--ui-lang", "en", "build-mod"])
        .args(["--po", "./test/ok.po"])
        .args(["--out-mod"])
        .arg(&out_dir)
        .args(["--lang", "ru"])
        .args(["--from-root"])
        .arg(&src)
        .args(["--merge"]);
    cmd.current_dir(&cwd);
    cmd.assert()
        .success()
        .stderr(predicates::str::contains("stale"));

    assert!(out_dir.join("Languages/Russian/Keyed/B.xml").exists());
    // Merge keeps stale files: A.xml from the FIRST build is still there.
    assert!(
        out_dir.join("Languages/Russian/Keyed/A.xml").exists(),
        "merge must keep stale files, never clean up"
    );
}
