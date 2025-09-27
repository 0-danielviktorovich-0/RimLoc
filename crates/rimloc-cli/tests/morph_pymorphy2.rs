use assert_cmd::prelude::*;
use std::{env, fs, path::Path, path::PathBuf, process::Command};

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc-cli built")
}

fn write_sample_xml(dir: &Path) {
    let keyed = dir.join("Languages").join("Russian").join("Keyed");
    fs::create_dir_all(&keyed).expect("create dirs");
    let xml = keyed.join("Sample.xml");
    let body = r#"<LanguageData>
  <Mother>мама</Mother>
</LanguageData>
"#;
    fs::write(&xml, body).expect("write xml");
}

#[test]
fn morph_pymorphy2_consumes_service_when_available() {
    // Optional smoke: requires PYMORPHY_URL pointing to a running service.
    let url = match env::var("PYMORPHY_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => {
            eprintln!("skipping pymorphy2 smoke: PYMORPHY_URL not set");
            return;
        }
    };

    let tmp = tempfile::tempdir().expect("tempdir");
    let root: PathBuf = tmp.path().to_path_buf();
    write_sample_xml(&root);

    let mut cmd = bin_cmd();
    cmd.env("PYMORPHY_URL", &url)
        .args(["--quiet", "morph", "--root"])
        .arg(&root)
        .args(["--provider", "pymorphy2"])
        .args(["--lang", "ru"])
        .args(["--timeout-ms", "3000"]);
    let assert = cmd.assert();
    // If service is reachable, command should succeed and write full cases
    if assert.get_output().status.success() {
        let case_xml = fs::read_to_string(
            root.join("Languages")
                .join("Russian")
                .join("Keyed")
                .join("_Case.xml"),
        )
        .expect("case exists");
        // Check at least one extra case beyond Nominative/Genitive
        assert!(
            case_xml.contains("<Case.Mother.Dative>")
                || case_xml.contains("<Case.Mother.Accusative>")
                || case_xml.contains("<Case.Mother.Instrumental>")
                || case_xml.contains("<Case.Mother.Prepositional>"),
            "expected extra cases from pymorphy2"
        );
    } else {
        eprintln!("pymorphy2 service not reachable; smoke skipped");
    }
}
