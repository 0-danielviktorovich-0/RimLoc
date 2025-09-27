use assert_cmd::prelude::*;
use std::{fs, path::Path, path::PathBuf, process::Command};

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc-cli is built")
}

fn write_sample_xml(dir: &Path) {
    let keyed = dir.join("Languages").join("Russian").join("Keyed");
    fs::create_dir_all(&keyed).expect("create dirs");
    let xml = keyed.join("Sample.xml");
    let body = r#"<LanguageData>
  <Hero>герой</Hero>
  <Mother>мама</Mother>
  <Square>площадь</Square>
</LanguageData>
"#;
    fs::write(&xml, body).expect("write xml");
}

#[test]
fn morph_dummy_generates_plural_and_gender() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();
    write_sample_xml(&root);

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "morph", "--root"])
        .arg(&root)
        .args(["--provider", "dummy"])
        .args(["--lang", "ru"]);
    cmd.assert().success();

    // Check that _Plural.xml and _Gender.xml exist and contain expected heuristics
    let keyed = root.join("Languages").join("Russian").join("Keyed");
    let plural = fs::read_to_string(keyed.join("_Plural.xml")).expect("plural exists");
    let gender = fs::read_to_string(keyed.join("_Gender.xml")).expect("gender exists");

    // герой -> герои (й -> и)
    assert!(
        plural.contains("<Plural.Hero>герои</Plural.Hero>"),
        "expected plural for герой"
    );
    // мама -> мамы (а -> ы)
    assert!(
        plural.contains("<Plural.Mother>мамы</Plural.Mother>"),
        "expected plural for мама"
    );
    // площадь -> площади (ь -> и)
    assert!(
        plural.contains("<Plural.Square>площади</Plural.Square>"),
        "expected plural for площадь"
    );

    // Gender
    assert!(gender.contains("<Gender.Mother>Female</Gender.Mother>"));
    assert!(gender.contains("<Gender.Hero>Male</Gender.Hero>"));
    assert!(gender.contains("<Gender.Square>Female</Gender.Square>"));
}
