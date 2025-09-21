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

#[test]
fn validate_detects_issues_in_bad_xml() {
    let mut cmd = bin_cmd();
    cmd.args(["validate", "--root"])
        .arg(fixture("test/TestMod"));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DuplicateKey"))
        .stdout(predicate::str::contains("EmptyKey"));
}

#[test]
fn import_po_requires_target() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"])
        .arg(fixture("test/ok.po"));

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("нужно указать либо --out-xml, либо --mod-root"));
}

#[test]
fn help_in_english_when_ui_lang_en() {
    let mut cmd = bin_cmd();
    cmd.args(["--help", "--ui-lang", "en"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("RimWorld localization toolkit"));
}

#[test]
fn import_error_in_english_when_ui_lang_en() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--ui-lang", "en"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("either --out-xml or --mod-root must be specified"));
}
#[test]
fn validate_po_ok() {
    let mut cmd = bin_cmd();
    cmd.args(["validate-po", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--ui-lang", "en"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Placeholders OK"));
}

#[test]
fn validate_po_strict_mismatch() {
    let mut cmd = bin_cmd();
    cmd.args(["validate-po", "--po"])
        .arg(fixture("test/bad.po"))
        .arg("--strict")
        .args(["--ui-lang", "en"]);

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Total mismatches"));
}

#[test]
fn import_single_file_dry_run_path() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--mod-root"])
        .arg(fixture("test/TestMod"))
        .arg("--single-file")
        .arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Languages/Russian/Keyed/_Imported.xml"));
}

#[test]
fn build_mod_dry_run_prints_header() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out_mod = tmp.path().join("RimLoc_RU");

    let mut cmd = bin_cmd();
    cmd.args(["build-mod", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--out-mod"])
        .arg(&out_mod)
        .args(["--lang", "ru"])
        .arg("--dry-run")
        .args(["--ui-lang", "en"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DRY RUN: building translation mod"));
}

#[test]
fn build_mod_creates_minimal_structure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out_mod = tmp.path().join("RimLoc_RU");

    let mut cmd = bin_cmd();
    cmd.args(["build-mod", "--po"]) 
        .arg(fixture("test/ok.po"))
        .args(["--out-mod"]) 
        .arg(&out_mod)
        .args(["--lang", "ru"])
        // реальная сборка без --dry-run
        .args(["--ui-lang", "en"]);

    cmd.assert().success();

    // Проверяем, что созданы ключевые файлы структуры мода
    let about = out_mod.join("About/About.xml");
    assert!(about.exists(), "About/About.xml must exist in built mod");

    let keyed_any = out_mod.join("Languages/Russian/Keyed");
    assert!(keyed_any.exists(), "Languages/Russian/Keyed folder must exist");

    // Должен появиться хотя бы один XML (в нашем фикстуре — _Imported.xml или Bad.xml)
    let has_any_xml = std::fs::read_dir(&keyed_any)
        .ok()
        .map(|rd| rd.flatten().any(|e| e.path().extension().map(|s| s == "xml").unwrap_or(false)))
        .unwrap_or(false);
    assert!(has_any_xml, "at least one XML file must be generated under Keyed/");
}

#[test]
fn supported_locales_startup_message_matches() {
    // Считываем локали из файловой системы, чтобы тест адаптировался к репозиторию
    let locales_dir = workspace_root()
        .join("crates/rimloc-cli/i18n");
    let mut locales = vec![];
    if let Ok(rd) = fs::read_dir(locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                locales.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }

    // Проверяем известные локали, если они есть
    if locales.iter().any(|l| l == "ru") {
        let mut cmd = bin_cmd();
        // В некоторых CLI глобальные флаги должны идти до сабкоманды
        cmd.args(["--ui-lang", "ru"])
            .args(["validate", "--root"]) // любая команда, чтобы увидеть стартовый лог
            .arg(fixture("test/TestMod"));
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("rimloc запущен"));
    }

    if locales.iter().any(|l| l == "en") {
        let mut cmd = bin_cmd();
        // В некоторых CLI глобальные флаги должны идти до сабкоманды
        cmd.args(["--ui-lang", "en"])
            .args(["validate", "--root"]) 
            .arg(fixture("test/TestMod"));
        cmd.assert()
            .success()
            .stdout(
                predicate::str::contains("rimloc started")
                    .or(predicate::str::contains("rimloc запущен"))
            );
    }
}

fn extract_fluent_vars(s: &str) -> std::collections::BTreeSet<String> {
    // Find tokens like { $var } without regex
    let mut vars = std::collections::BTreeSet::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'{' {
            // skip spaces to potential '$'
            let mut j = i + 1;
            while j + 1 < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'$' {
                // read identifier
                j += 1;
                let start = j;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'-')
                {
                    j += 1;
                }
                if j > start {
                    let name = &s[start..j];
                    vars.insert(name.to_string());
                }
            }
        }
        i += 1;
    }
    vars
}

fn diff_locale_maps(
    en: &std::collections::BTreeMap<String, String>,
    other: &std::collections::BTreeMap<String, String>,
) -> (
    Vec<String>,
    Vec<String>,
    Vec<(
        String,
        std::collections::BTreeSet<String>,
        std::collections::BTreeSet<String>,
    )>,
) {
    use std::collections::BTreeSet;

    let en_keys: BTreeSet<_> = en.keys().cloned().collect();
    let other_keys: BTreeSet<_> = other.keys().cloned().collect();

    let missing: Vec<_> = en_keys.difference(&other_keys).cloned().collect();
    let extra: Vec<_> = other_keys.difference(&en_keys).cloned().collect();

    // Parameter set mismatches on common keys
    let common: Vec<_> = en_keys.intersection(&other_keys).cloned().collect();
    let mut arg_mismatches = Vec::new();
    for k in common {
        let en_vars = extract_fluent_vars(en.get(&k).unwrap());
        let ot_vars = extract_fluent_vars(other.get(&k).unwrap());
        if en_vars != ot_vars {
            arg_mismatches.push((k, en_vars, ot_vars));
        }
    }

    (missing, extra, arg_mismatches)
}

/// Load FTL as key -> value map (trims both sides around '=')
fn load_ftl_map(locale: &str) -> std::collections::BTreeMap<String, String> {
    let ftl_path = workspace_root()
        .join("crates/rimloc-cli/i18n")
        .join(locale)
        .join("rimloc.ftl");
    let content = fs::read_to_string(ftl_path)
        .unwrap_or_else(|_| panic!("Missing FTL file for locale {}", locale));

    let mut map = std::collections::BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let val = line[eq + 1..].trim().to_string();
            map.insert(key, val);
        }
    }
    map
}

#[test]
fn all_locales_have_same_keys() {
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
    let mut locales = vec![];
    if let Ok(rd) = fs::read_dir(locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                locales.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }

    assert!(
        locales.contains(&"en".to_string()),
        "English locale must exist as reference"
    );

    let reference = load_ftl_map("en");

    for loc in locales {
        if loc == "en" {
            continue;
        }
        let map = load_ftl_map(&loc);
        let (missing, extra, arg_mismatches) = diff_locale_maps(&reference, &map);

        if !missing.is_empty() || !extra.is_empty() || !arg_mismatches.is_empty() {
            let mut msg = String::new();
            msg.push_str(&format!("Locale {} has issues:\n", loc));
            if !missing.is_empty() {
                msg.push_str(&format!("  • Missing keys ({}): {:?}\n", missing.len(), missing));
            }
            if !extra.is_empty() {
                msg.push_str(&format!("  • Extra keys ({}): {:?}\n", extra.len(), extra));
            }
            if !arg_mismatches.is_empty() {
                msg.push_str(&format!(
                    "  • Keys with argument set mismatch ({}):\n",
                    arg_mismatches.len()
                ));
                for (k, en_vars, ot_vars) in arg_mismatches {
                    msg.push_str(&format!("    - {}\n      en: {:?}\n      {}: {:?}\n", k, en_vars, loc, ot_vars));
                }
            }
            panic!("{}", msg);
        }
    }
}

#[test]
fn each_locale_runs_help_successfully() {
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
    if let Ok(rd) = fs::read_dir(locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let loc = e.file_name().to_string_lossy().to_string();
                let mut cmd = bin_cmd();
                cmd.args(["--ui-lang", &loc])
                    .arg("--help");
                cmd.assert()
                    .success()
                    .stdout(predicate::str::contains("RimWorld").or(predicate::str::contains("RimLoc")));
            }
        }
    }
}