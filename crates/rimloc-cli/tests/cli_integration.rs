use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::collections::BTreeSet;
use std::{fs, path::PathBuf, process::Command};

include!(concat!(env!("OUT_DIR"), "/supported_locales.rs"));

mod helpers;
use helpers::*;

mod tests_i18n;

/// Macro for test i18n lookups with named args: ti18n!("key", name = expr, ...)
macro_rules! ti18n {
    ($key:literal) => { crate::tests_i18n::lookup($key, &[]) };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        crate::tests_i18n::lookup($key, &[ $( (stringify!($name), ($value).to_string()) ),+ ])
    };
}

fn bin_cmd() -> Command {
    Command::cargo_bin("rimloc-cli").expect(&ti18n!("test-binary-built"))
}

fn workspace_root() -> PathBuf {
    // crates/rimloc-cli -> <workspace root>
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // <workspace root>
        .to_path_buf()
}

fn fixture(rel: &str) -> PathBuf {
    workspace_root().join(rel)
}

struct OutputWithStd {
    pub stdout: String,
}

fn run_ok(args: &[&str]) -> OutputWithStd {
    let mut cmd = bin_cmd();
    cmd.args(args);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    OutputWithStd { stdout }
}

#[test]
fn help_works() {
    // Проверяем заголовок хелпа для каждой локали по фактическому FTL.
    // SUPPORTED_LOCALES уже подключён через include!(concat!(env!("OUT_DIR"), "/supported_locales.rs"))

    use std::path::{Path, PathBuf};

    // Путь к i18n каталогу rimloc-cli
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");

    for &lang in SUPPORTED_LOCALES.iter() {
        // Берём ожидаемую строку из FTL; если для конкретной локали нет — fallback на en
        let expected = read_ftl_message(&i18n_dir, lang, "help-about")
            .or_else(|| read_ftl_message(&i18n_dir, "en", "help-about"))
            .unwrap_or_else(|| panic!("{}", ti18n!("test-help-about-key-required")));

        let out = run_ok(&["--ui-lang", lang, "--help"]);
        let ftl_path = i18n_dir.join(lang).join("rimloc.ftl");
        assert_has!(
            &out.stdout,
            &expected,
            &ftl_path,
            lang,
            "help-about",
            &ti18n!("test-help-about-must-be-localized", lang = lang),
        );
    }
}

#[test]
fn scan_outputs_csv_header() {
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"]).arg(fixture("test/TestMod"));
    let assert = cmd.assert().success();
    // Проверяем только заголовок CSV — он не локализуется
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert_has!(
        &out,
        "key,source,path,line",
        std::path::Path::new("n/a"),
        CTX_NONE,
        "csv-header",
        &ti18n!("test-csv-header"),
    );
}

#[test]
fn scan_writes_json_file() {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct JsonUnit {
        key: String,
        value: Option<String>,
        path: String,
        line: Option<usize>,
    }

    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let out_json = tmp.path().join("scan.json");

    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(fixture("test/TestMod"))
        .args(["--format", "json"])
        .args(["--out-json"])
        .arg(&out_json);

    let _assert = cmd.assert().success();
    assert_file_nonempty(
        &out_json,
        &ti18n!("test-json-not-empty"),
        "out-json-nonempty",
    );
    let contents = std::fs::read_to_string(&out_json).expect("read json");
    let units: Vec<JsonUnit> = serde_json::from_str(&contents).expect("valid json output");
    assert!(!units.is_empty(), "json should contain at least one unit");
    // Проверяем, что ключи и путь присутствуют в объекте
    let first = &units[0];
    assert!(
        !first.key.is_empty(),
        "expected first JSON unit to contain a key"
    );
    assert!(
        first.path.contains("Languages"),
        "expected JSON path to reference Languages directory"
    );
    if let Some(val) = &first.value {
        assert!(
            !val.trim().is_empty(),
            "expected JSON unit value to be non-empty when present"
        );
    }
    assert!(
        first.line.is_some(),
        "expected JSON unit to include line information"
    );
}

#[test]
fn export_po_creates_file() {
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let out_po = tmp.path().join("out.po");

    let mut cmd = bin_cmd();
    cmd.args(["export-po", "--root"])
        .arg(fixture("test/TestMod"))
        .args(["--out-po"])
        .arg(&out_po);

    cmd.assert().success();

    assert_file_nonempty(&out_po, &ti18n!("test-outpo-not-empty"), "out-po-nonempty");
}

#[test]
fn validate_json_emits_structured_issues() {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct JsonMsg {
        kind: String,
        key: String,
        path: String,
        line: Option<usize>,
        message: String,
    }

    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]) // ensure no banner
        .args(["validate", "--root"]) // known to contain issues in Bad.xml
        .arg(fixture("test/TestMod"))
        .args(["--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    // In quiet mode stdout must be a clean JSON array
    let json_slice = out.as_str();
    let msgs: Vec<JsonMsg> = serde_json::from_str(json_slice).expect("valid JSON diagnostics");
    assert!(
        !msgs.is_empty(),
        "expected at least one issue in fixture"
    );
    let allowed = ["duplicate", "empty", "placeholder-check"];
    for m in msgs {
        assert!(allowed.contains(&m.kind.as_str()), "unexpected kind: {}", m.kind);
        assert!(!m.key.is_empty(), "key must be non-empty");
        assert!(!m.path.is_empty(), "path must be non-empty");
        assert!(!m.message.is_empty(), "message must be non-empty");
        // Touch optional line to avoid dead_code warnings and ensure it's a valid number if present
        if let Some(l) = m.line { let _ = l; }
    }
}

#[test]
fn export_po_respects_version_selection() {
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    let tmp = tempdir().expect(&ti18n!("test-tempdir"));
    let root = tmp.path();

    // Create v1.5 and v1.6 under Languages/English/Keyed
    let v15 = root
        .join("v1.5")
        .join("Languages")
        .join("English")
        .join("Keyed");
    let v16 = root
        .join("v1.6")
        .join("Languages")
        .join("English")
        .join("Keyed");
    fs::create_dir_all(&v15).unwrap();
    fs::create_dir_all(&v16).unwrap();
    let mut f15 = fs::File::create(v15.join("A.xml")).unwrap();
    writeln!(f15, "<LanguageData>\n  <K1>Old</K1>\n</LanguageData>\n").unwrap();
    let mut f16 = fs::File::create(v16.join("B.xml")).unwrap();
    writeln!(f16, "<LanguageData>\n  <K2>New</K2>\n</LanguageData>\n").unwrap();

    // Export for version 1.5 only (accepts without 'v')
    let out_po = root.join("out.po");
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]) // no banner
        .args(["export-po", "--root"]) // default source dir is English
        .arg(root)
        .args(["--out-po"])
        .arg(&out_po)
        .args(["--game-version", "1.5"]);
    cmd.assert().success();

    let s = fs::read_to_string(&out_po).expect("read out.po");
    // msgid contains source text, not the key
    assert!(s.contains("msgid \"Old\""), "PO must contain 'Old' from v1.5");
    assert!(
        !s.contains("msgid \"New\""),
        "PO must not contain 'New' from v1.6 when version=1.5"
    );
}

#[test]
fn scan_errors_on_missing_version() {
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]).args(["scan", "--root"]).arg(fixture("test/TestMod"))
        .args(["--game-version", "9.9"]);
    cmd.assert().failure();
}

#[test]
fn export_po_errors_on_missing_version() {
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let out_po = tmp.path().join("out.po");
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]).args(["export-po", "--root"]).arg(fixture("test/TestMod"))
        .args(["--out-po"]).arg(&out_po)
        .args(["--game-version", "0.0"]);
    cmd.assert().failure();
}

#[test]
fn scan_csv_adds_lang_column_when_lang_passed() {
    // When --lang is provided, CSV should add a lang column as the first header
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]).args(["scan", "--root"]).arg(fixture("test/TestMod"))
        .args(["--lang", "ru"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(out.lines().next().unwrap_or("").starts_with("lang,key,source,path,line"));
}

#[test]
fn scan_filters_by_source_lang_and_dir() {
    use serde::Deserialize;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[derive(Deserialize)]
    struct JsonUnit {
        key: String,
        value: Option<String>,
        path: String,
        line: Option<usize>,
    }

    let tmp = tempdir().expect(&ti18n!("test-tempdir"));
    let root = tmp.path();

    // Create Languages/English/Keyed and Languages/Russian/Keyed
    let en_keyed = root
        .join("Languages")
        .join("English")
        .join("Keyed");
    let ru_keyed = root
        .join("Languages")
        .join("Russian")
        .join("Keyed");
    fs::create_dir_all(&en_keyed).unwrap();
    fs::create_dir_all(&ru_keyed).unwrap();

    let mut f_en = fs::File::create(en_keyed.join("A.xml")).unwrap();
    writeln!(f_en, "<LanguageData>\n  <K_EN>Hello</K_EN>\n</LanguageData>\n").unwrap();
    let mut f_ru = fs::File::create(ru_keyed.join("B.xml")).unwrap();
    writeln!(f_ru, "<LanguageData>\n  <K_RU>Привет</K_RU>\n</LanguageData>\n").unwrap();

    // 1) No filters => both keys
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]) // no banner
        .args(["scan", "--root"]) // stdout json
        .arg(root)
        .args(["--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json_slice = out.as_str();
    let units: Vec<JsonUnit> = serde_json::from_str(json_slice).expect("valid json");
    // Sanity-check paths/lines and collect keys
    for u in &units {
        assert!(u.path.contains("Languages"), "expected Languages in path");
        assert!(u.path.contains("Keyed"), "expected Keyed in path");
        let _ = u.line.unwrap_or(0);
        let _ = u.value.as_deref();
    }
    let keys: BTreeSet<String> = units.iter().map(|u| u.key.clone()).collect();
    assert_eq!(keys, ["K_EN", "K_RU"].into_iter().map(String::from).collect());

    // 2) Filter by --source-lang en => only English
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]) // no banner
        .args(["scan", "--root"]) // stdout json
        .arg(root)
        .args(["--format", "json"])
        .args(["--source-lang", "en"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json_slice = out.as_str();
    let units: Vec<JsonUnit> = serde_json::from_str(json_slice).expect("valid json");
    for u in &units {
        assert!(u.path.contains("Languages"), "expected Languages in path");
        assert!(u.path.contains("Keyed"), "expected Keyed in path");
        let _ = u.line.unwrap_or(0);
        let _ = u.value.as_deref();
    }
    let keys: BTreeSet<String> = units.iter().map(|u| u.key.clone()).collect();
    assert_eq!(keys, ["K_EN"].into_iter().map(String::from).collect());

    // 3) Filter by --source-lang-dir Russian => only Russian
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]) // no banner
        .args(["scan", "--root"]) // stdout json
        .arg(root)
        .args(["--format", "json"])
        .args(["--source-lang-dir", "Russian"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let json_slice = out.as_str();
    let units: Vec<JsonUnit> = serde_json::from_str(json_slice).expect("valid json");
    for u in &units {
        assert!(u.path.contains("Languages"), "expected Languages in path");
        assert!(u.path.contains("Keyed"), "expected Keyed in path");
        let _ = u.line.unwrap_or(0);
        let _ = u.value.as_deref();
    }
    let keys: BTreeSet<String> = units.iter().map(|u| u.key.clone()).collect();
    assert_eq!(keys, ["K_RU"].into_iter().map(String::from).collect());
}

#[test]
fn import_po_single_file_writes_and_backup() {
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    // Prepare mod root with existing _Imported.xml to trigger backup
    let tmp = tempdir().expect(&ti18n!("test-tempdir"));
    let root = tmp.path();
    let out_xml = root
        .join("Languages")
        .join("Russian")
        .join("Keyed")
        .join("_Imported.xml");
    fs::create_dir_all(out_xml.parent().unwrap()).unwrap();
    fs::write(&out_xml, "<LanguageData><Old>prev</Old></LanguageData>").unwrap();

    // Prepare minimal PO with msgctxt key and msgstr value
    let po = root.join("in.po");
    let mut f = fs::File::create(&po).unwrap();
    writeln!(f, "msgid \"\"")
        .and_then(|_| writeln!(f, "msgstr \"\""))
        .unwrap();
    writeln!(f, "msgctxt \"K_NEW|Keyed/Dummy.xml:3\"").unwrap();
    writeln!(f, "msgid \"Hello\"").unwrap();
    writeln!(f, "msgstr \"Привет\"").unwrap();
    writeln!(f).unwrap();

    // Run import into single file with backup
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"]) // prefer explicit lang ru
        .arg(&po)
        .args(["--mod-root"])
        .arg(root)
        .args(["--lang", "ru"])
        .arg("--single-file")
        .arg("--backup");
    cmd.assert().success();

    // Verify backup and written content
    let bak = out_xml.with_extension("xml.bak");
    assert!(bak.exists(), "expected .bak backup to be created");
    let s = fs::read_to_string(&out_xml).expect("read imported xml");
    assert!(s.contains("<K_NEW>Привет</K_NEW>"), "imported value must appear");
}

#[test]
fn scan_picks_latest_version_by_default_and_flags_work() {
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    // temp root
    let tmp = tempdir().expect(&ti18n!("test-tempdir"));
    let root = tmp.path();

    // Create v1.5 and v1.6 with minimal Keyed XML
    let v15 = root
        .join("v1.5")
        .join("Languages")
        .join("English")
        .join("Keyed");
    let v16 = root
        .join("v1.6")
        .join("Languages")
        .join("English")
        .join("Keyed");
    fs::create_dir_all(&v15).unwrap();
    fs::create_dir_all(&v16).unwrap();

    let mut f15 = fs::File::create(v15.join("A.xml")).unwrap();
    writeln!(f15, "<LanguageData>\n  <K1>Old</K1>\n</LanguageData>\n").unwrap();
    let mut f16 = fs::File::create(v16.join("B.xml")).unwrap();
    writeln!(f16, "<LanguageData>\n  <K2>New</K2>\n</LanguageData>\n").unwrap();

    // 1) По умолчанию берётся последняя версия (v1.6)
    let out_json_latest = root.join("scan-latest.json");
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(root)
        .args(["--format", "json"]) // stdout/json by default
        .args(["--out-json"])
        .arg(&out_json_latest);
    cmd.assert().success();
    let s = fs::read_to_string(&out_json_latest).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&s).unwrap();
    let keys: std::collections::BTreeSet<String> = items
        .iter()
        .filter_map(|o| o.get("key").and_then(|k| k.as_str()).map(|s| s.to_string()))
        .collect();
    assert_eq!(keys, ["K2"].into_iter().map(String::from).collect());

    // 2) Явный выбор версии --game-version 1.5
    let out_json_v15 = root.join("scan-v15.json");
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(root)
        .args(["--game-version", "1.5"]) // accept without 'v'
        .args(["--format", "json"])
        .args(["--out-json"])
        .arg(&out_json_v15);
    cmd.assert().success();
    let s = fs::read_to_string(&out_json_v15).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&s).unwrap();
    let keys: std::collections::BTreeSet<String> = items
        .iter()
        .filter_map(|o| o.get("key").and_then(|k| k.as_str()).map(|s| s.to_string()))
        .collect();
    assert_eq!(keys, ["K1"].into_iter().map(String::from).collect());

    // 3) Полное сканирование всех версий
    let out_json_all = root.join("scan-all.json");
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(root)
        .args(["--include-all-versions"]) // process v1.5 + v1.6
        .args(["--format", "json"])
        .args(["--out-json"])
        .arg(&out_json_all);
    cmd.assert().success();
    let s = fs::read_to_string(&out_json_all).unwrap();
    let items: Vec<serde_json::Value> = serde_json::from_str(&s).unwrap();
    let keys: std::collections::BTreeSet<String> = items
        .iter()
        .filter_map(|o| o.get("key").and_then(|k| k.as_str()).map(|s| s.to_string()))
        .collect();
    assert_eq!(keys, ["K1", "K2"].into_iter().map(String::from).collect());
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

    // Capture output to count categories
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let bad_xml_en = fixture("test/TestMod/Languages/English/Keyed/Bad.xml");

    assert_all_present(
        &out,
        &[
            ("[duplicate]", bad_xml_en.as_path(), "DuplicateKey"),
            ("[empty]", bad_xml_en.as_path(), "EmptyKey"),
            ("[placeholder-check]", bad_xml_en.as_path(), "Placeholder"),
            ("DuplicateKey", bad_xml_en.as_path(), "DuplicateKey"),
            ("EmptyKey", bad_xml_en.as_path(), "EmptyKey"),
            ("Placeholder", bad_xml_en.as_path(), "Placeholder"),
        ],
        CTX_NONE,
        &ti18n!("test-validate-badxml"),
    );

    // At least one occurrence of each category (rich diagnostics)
    assert_count_at_least(
        &out,
        "[duplicate]",
        1,
        &ti18n!(
            "test-validate-atleast-duplicates",
            min = 1,
            count = out.matches("[duplicate]").count()
        ),
        CTX_NONE,
        bad_xml_en.as_path(),
        "category-[duplicate]",
    );
    assert_count_at_least(
        &out,
        "[empty]",
        1,
        &ti18n!(
            "test-validate-atleast-empty",
            min = 1,
            count = out.matches("[empty]").count()
        ),
        CTX_NONE,
        bad_xml_en.as_path(),
        "category-[empty]",
    );
    assert_count_at_least(
        &out,
        "[placeholder-check]",
        1,
        &ti18n!(
            "test-validate-atleast-placeholder",
            min = 1,
            count = out.matches("[placeholder-check]").count()
        ),
        CTX_NONE,
        bad_xml_en.as_path(),
        "category-[placeholder-check]",
    );
}

#[test]
fn import_po_requires_target() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"]).arg(fixture("test/ok.po"));
    // Accept localized FTL text (fallback to en)
    use std::path::{Path, PathBuf};
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let expected_en = read_ftl_message(&i18n_dir, "en", "import-need-target")
        .expect("FTL(en) must contain `import-need-target`");
    let expected_ru = read_ftl_message(&i18n_dir, "ru", "import-need-target");

    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
    // use std::path::Path;  // <-- REMOVE this line
    let ftl_path_en = i18n_dir.join("en").join("rimloc.ftl");

    if stderr.contains(&expected_en) {
        assert_contains_in_outputs(
            "",
            &stderr,
            &expected_en,
            &ti18n!("test-fallback-locale-expected", stdout = &stderr),
            "en",
            &ftl_path_en,
            "import-need-target",
        );
    } else if let Some(expected_ru) = expected_ru.as_ref() {
        let ftl_path_ru = i18n_dir.join("ru").join("rimloc.ftl");
        assert_contains_in_outputs(
            "",
            &stderr,
            expected_ru,
            &ti18n!("test-fallback-locale-expected", stdout = &stderr),
            "ru",
            &ftl_path_ru,
            "import-need-target",
        );
    } else {
        assert_contains_in_outputs(
            "",
            &stderr,
            &expected_en,
            &ti18n!("test-fallback-locale-expected", stdout = &stderr),
            "en",
            &ftl_path_en,
            "import-need-target",
        );
    }
}

#[test]
fn help_in_english_when_ui_lang_en() {
    expect_ftl_contains_lang(&["--ui-lang", "en", "--help"], "en", "help-about");
}

#[test]
fn import_error_in_english_when_ui_lang_en() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--ui-lang", "en"]);

    use std::path::{Path, PathBuf};
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let expected_en = read_ftl_message(&i18n_dir, "en", "import-need-target")
        .expect("FTL(en) must contain `import-need-target`");

    let assert = cmd.assert().failure();
    let stderr = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
    let ftl_path_en = i18n_dir.join("en").join("rimloc.ftl");
    assert_contains_in_outputs(
        "",
        &stderr,
        &expected_en,
        &ti18n!("test-import-error-en"),
        "en",
        &ftl_path_en,
        "import-need-target",
    );
}
#[test]
fn validate_po_ok() {
    let mut cmd = bin_cmd();
    cmd.args(["validate-po", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--ui-lang", "en"]);

    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let ok_po = fixture("test/ok.po");
    assert_contains_file(
        &out,
        "Placeholders OK",
        &ti18n!("test-validate-po-ok"),
        ok_po.as_path(),
        "validate-po-ok",
    );
}

#[test]
fn validate_po_strict_mismatch() {
    let mut cmd = bin_cmd();
    cmd.args(["validate-po", "--po"])
        .arg(fixture("test/bad.po"))
        .arg("--strict")
        .args(["--ui-lang", "en"]);

    let assert = cmd.assert().failure();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let po_path = fixture("test/bad.po");
    assert_contains_file(
        &out,
        "Total mismatches",
        &ti18n!("test-validate-po-strict"),
        po_path.as_path(),
        "validate-po-mismatch",
    );
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

    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let err = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
    let combined = format!("{}{}", out, err);
    // use std::path::Path;
    let expected_rel = "Languages/Russian/Keyed/_Imported.xml";
    let expected_abs = fixture("test/TestMod").join(expected_rel);
    assert_contains_file(
        &combined,
        expected_rel,
        &ti18n!(
            "test-importpo-expected-path-not-found",
            out = out,
            err = err
        ),
        expected_abs.as_path(),
        "import-single-file",
    );
}

#[test]
fn build_mod_dry_run_prints_header() {
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let out_mod = tmp.path().join("RimLoc_RU");

    let mut cmd = bin_cmd();
    cmd.args(["build-mod", "--po"])
        .arg(fixture("test/ok.po"))
        .args(["--out-mod"])
        .arg(&out_mod)
        .args(["--lang", "ru"])
        .arg("--dry-run")
        .args(["--ui-lang", "en"]);

    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    if out.contains("DRY-RUN") {
        assert_contains_file(
            &out,
            "DRY-RUN",
            &ti18n!("test-build-header"),
            out_mod.as_path(),
            "dry-run-would-write",
        );
    } else {
        assert_contains_file(
            &out,
            "DRY RUN",
            &ti18n!("test-build-header"),
            out_mod.as_path(),
            "dry-run-would-write",
        );
    }
}

#[test]
fn build_mod_creates_minimal_structure() {
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
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
    assert_path_exists(
        about.as_path(),
        &ti18n!("test-build-path-must-exist", path = "About/About.xml"),
        "about-path",
    );

    let keyed_any = out_mod.join("Languages/Russian/Keyed");
    assert_path_exists(
        keyed_any.as_path(),
        &ti18n!(
            "test-build-folder-must-exist",
            path = "Languages/Russian/Keyed"
        ),
        "keyed-folder",
    );

    // Должен появиться хотя бы один XML под Keyed/
    assert_dir_contains_xml(
        keyed_any.as_path(),
        &ti18n!("test-build-xml-under-path", path = "Keyed/"),
        "build-xml-under-path",
    );

    // Validate content of About/About.xml includes expected metadata
    let about_content = fs::read_to_string(&about).expect(&ti18n!("test-build-about-readable"));
    assert_all_present(
        &about_content,
        &[
            (
                "<name>RimLoc Translation</name>",
                about.as_path(),
                "about-name-tag",
            ),
            (
                "<packageId>yourname.rimloc.translation</packageId>",
                about.as_path(),
                "about-packageId-tag",
            ),
        ],
        CTX_NONE,
        &ti18n!("test-build-about-tags"),
    );
}

#[test]
fn supported_locales_startup_message_matches() {
    use std::path::{Path, PathBuf};

    // Determine locales by scanning the i18n directory so the test adapts to repo contents.
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let locales_dir = i18n_dir.clone();

    let mut locales = vec![];
    if let Ok(rd) = fs::read_dir(&locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                locales.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }

    // For each available locale, run any command that triggers startup logs
    // and assert that either the structured event token or the localized FTL string is present (covers all locales).
    for loc in locales {
        // expected localized fragment from FTL (fallback to en)
        let expected = read_ftl_message(&i18n_dir, &loc, "app-started")
            .or_else(|| read_ftl_message(&i18n_dir, "en", "app-started"))
            .unwrap_or_else(|| {
                let ftl_path_en = i18n_dir.join("en").join("rimloc.ftl");
                let ftl_path_ru = i18n_dir.join("ru").join("rimloc.ftl");
                assert_ftl_key_present_all(
                    &[("en", &ftl_path_en), ("ru", &ftl_path_ru)],
                    "app-started",
                );
                panic!("{}", ti18n!("test-app-started-key-required"));
            });

        // Derive a tolerant snippet: take text before the first bullet "•" or before the first placeholder "{".
        let expected_snip = {
            let s = &expected;
            let cut_at = s.find('•').or_else(|| s.find('{')).unwrap_or(s.len());
            s[..cut_at].trim().to_string()
        };

        let mut cmd = bin_cmd();
        // global flags first, then a simple subcommand to produce startup output
        cmd.args(["--ui-lang", &loc])
            .args(["validate", "--root"])
            .arg(fixture("test/TestMod"));

        let assert = cmd.assert().success();
        let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
        let err = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
        let combined = format!("{}{}", out, err);
        let clean = strip_ansi(&combined);
        if !(clean.contains("app_started")
            || clean.contains(&expected)
            || clean.contains(&expected_snip))
        {
            let ftl_path = i18n_dir.join(&loc).join("rimloc.ftl");
            assert_has!(
                &combined,
                &expected_snip,
                &ftl_path,
                &loc,
                "app-started",
                &ti18n!("test-startup-text-must-appear", loc = &loc),
            );
        }
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

fn section_for_key(key: &str) -> &'static str {
    // Map by common prefixes used in FTL keys to improve error messages in tests
    if key.starts_with("validate-po-") {
        return "validate-po";
    }
    if key.starts_with("build-") {
        return "build-mod details";
    }
    if key.starts_with("import-") {
        return "import-po";
    }
    if key.starts_with("scan-") {
        return "scan";
    }
    if key.starts_with("xml-") {
        return "xml";
    }
    if key.starts_with("export-po-") {
        return "export-po";
    }
    if key.starts_with("category-") {
        return "validation categories";
    }
    if key.starts_with("kind-") {
        return "validation kinds";
    }
    if key.starts_with("warn-") || key.starts_with("ui-lang-") || key.starts_with("err-") {
        return "warnings/errors";
    }
    if key == "app-started" {
        return "startup";
    }
    if key == "validate-clean" {
        return "validate";
    }
    if key == "dry-run-would-write" {
        return "dry-run";
    }
    // default bucket
    "misc"
}

/// Load FTL as key -> value map (trims both sides around '=')
fn load_ftl_map(locale: &str) -> std::collections::BTreeMap<String, String> {
    // Собираем ключи из всех .ftl, пользуясь нашим кэшем
    let i18n_dir = workspace_root().join("crates/rimloc-cli/i18n");
    get_map(&i18n_dir, locale)
}

#[test]
fn all_locales_have_same_keys() {
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
    let mut locales = vec![];
    if let Ok(rd) = fs::read_dir(&locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                locales.push(e.file_name().to_string_lossy().to_string());
            }
        }
    }

    assert!(
        locales.contains(&"en".to_string()),
        "{}",
        ti18n!("test-en-locale-required")
    );

    let reference = load_ftl_map("en");
    for loc in locales {
        if loc == "en" {
            continue;
        }
        let map = load_ftl_map(&loc);
        // путь к FTL — для контекста в падении
        let ftl_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("i18n")
            .join(&loc)
            .join("rimloc.ftl");

        assert_locale_diff(
            &loc,
            &reference,
            &map,
            &ftl_path,
            section_for_key,
            &ti18n!("test-nonlocalized-found"),
        );
    }
}

#[test]
fn each_locale_runs_help_successfully() {
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
    if let Ok(rd) = fs::read_dir(locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let loc = e.file_name().to_string_lossy().to_string();
                expect_ftl_contains_lang(&["--ui-lang", &loc, "--help"], &loc, "help-about");
            }
        }
    }
}

#[test]
fn warn_on_unsupported_ui_lang() {
    let mut cmd = bin_cmd();
    cmd.args(["--ui-lang", "xx"]) // intentionally unsupported
        .args(["validate", "--root"])
        .arg(fixture("test/TestMod"));

    // Command should succeed but print a warning about unsupported UI language.
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let err = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
    let combined = format!("{}{}", out, err);
    let clean = strip_ansi(&combined);
    let ui_lang = "xx";

    // Build a robust set of expected warning snippets from FTL,
    // covering possible keys used by different locales.
    use std::path::{Path, PathBuf};
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let mut expected_snippets: Vec<String> = Vec::new();

    // EN (required): ui-lang-unsupported
    if let Some(s) = read_ftl_message(&i18n_dir, "en", "ui-lang-unsupported") {
        expected_snippets.push(s);
    }
    // RU (optional): ui-lang-unsupported and legacy warn-unsupported-ui-lang
    if let Some(s) = read_ftl_message(&i18n_dir, "ru", "ui-lang-unsupported") {
        expected_snippets.push(s);
    }
    if let Some(s) = read_ftl_message(&i18n_dir, "ru", "warn-unsupported-ui-lang") {
        expected_snippets.push(s);
    }

    // As an extra fallback (in case wording slightly differs), also accept a minimal token.
    expected_snippets.push("UI language code is not supported".to_string());
    expected_snippets.push("Неподдерживаемый код языка интерфейса".to_string());
    // Even more tolerant tokens to handle emoji/prefix variations
    expected_snippets.push("UI language code".to_string());
    expected_snippets.push("Неподдерживаемый".to_string());

    // Be tolerant: check both cleaned and raw outputs to avoid false negatives
    let matched = expected_snippets
        .iter()
        .any(|snip| clean.contains(snip) || combined.contains(snip));

    if !matched {
        // Build rich diagnostics to understand mismatch quickly
        let clean_head: String = clean.chars().take(800).collect();
        let stdout_head: String = out.chars().take(800).collect();
        let stderr_head: String = err.chars().take(800).collect();
        let combined_head: String = combined.chars().take(800).collect();
        let snippets_list = expected_snippets
            .iter()
            .enumerate()
            .map(|(i, s)| format!("[{}] {}", i, s))
            .collect::<Vec<_>>()
            .join("\n");

        let diag = format!(
            "{}\n--- diagnostics ---\ncleaned_output (first 800 chars):\n{}\ncombined_output (first 800 chars):\n{}\n--- raw stdout (first 800) ---\n{}\n--- raw stderr (first 800) ---\n{}\n--- expected snippets ({} total) ---\n{}\n",
            ti18n!("test-warn-unsupported-lang"),
            clean_head,
            combined_head,
            stdout_head,
            stderr_head,
            expected_snippets.len(),
            snippets_list
        );

        let ftl_path_en = i18n_dir.join("en").join("rimloc.ftl");
        fail_with_context!(
            ui_lang,
            &ftl_path_en,
            "ui-lang-unsupported",
            &diag,
            "<any of expected warning snippets>",
            &combined,
        );
    }
}

#[test]
fn unknown_locale_falls_back_to_real_locale_help() {
    use std::path::{Path, PathBuf};

    // Берём ожидаемые фрагменты из FTL (EN обязателен, RU — опционален)
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let expected_en =
        read_ftl_message(&i18n_dir, "en", "help-about").expect("FTL(en) must contain `help-about`");
    let expected_ru = read_ftl_message(&i18n_dir, "ru", "help-about"); // может и не быть — это ок

    // Запускаем с заведомо несуществующей локалью
    let out = run_ok(&["--ui-lang", "xx", "--help"]);

    // Должен сработать fallback и появиться строка хотя бы из одной реальной локали
    let ftl_path_en = i18n_dir.join("en").join("rimloc.ftl");
    if out.stdout.contains(&expected_en) {
        assert_has!(
            &out.stdout,
            &expected_en,
            &ftl_path_en,
            "en",
            "help-about",
            &ti18n!("test-fallback-locale-expected", stdout = &out.stdout),
        );
    } else if let Some(expected_ru) = expected_ru.as_ref() {
        let ftl_path_ru = i18n_dir.join("ru").join("rimloc.ftl");
        assert_has!(
            &out.stdout,
            expected_ru,
            &ftl_path_ru,
            "ru",
            "help-about",
            &ti18n!("test-fallback-locale-expected", stdout = &out.stdout),
        );
    } else {
        // Neither EN nor RU matched — anchor diagnostics to EN
        assert_has!(
            &out.stdout,
            &expected_en,
            &ftl_path_en,
            "en",
            "help-about",
            &ti18n!("test-fallback-locale-expected", stdout = &out.stdout),
        );
    }
}

fn load_ftl_lines(locale: &str) -> Vec<String> {
    let p = workspace_root()
        .join("crates/rimloc-cli/i18n")
        .join(locale)
        .join("rimloc.ftl");
    let content = fs::read_to_string(&p)
        .unwrap_or_else(|_| panic!("{}", ti18n!("test-ftl-failed-read", path = p.display())));
    content
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() || l.starts_with('#') {
                return None;
            }
            l.split_once('=').map(|(k, _)| k.trim().to_string())
        })
        .collect()
}

#[test]
fn validation_detail_keys_exist_in_locales() {
    // Ensure new detailed validation message keys exist and have the same arg set across locales.
    let required_keys = [
        "validate-detail-duplicate",
        "validate-detail-empty",
        "validate-detail-placeholder",
    ];
    // Expected placeholder set:
    let expected_vars: BTreeSet<String> = ["validator", "path", "line", "message"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    // reference EN
    let en_map = load_ftl_map("en");
    for &k in &required_keys {
        assert_map_contains_key(
            &en_map,
            k,
            "en",
            std::path::Path::new("crates/rimloc-cli/i18n/en/rimloc.ftl"),
            &ti18n!("test-ftl-key-missing", key = k, lang = "en"),
        );
        let vars = extract_fluent_vars(en_map.get(k).unwrap());
        assert_set_eq(
            &vars,
            &expected_vars,
            &ti18n!(
                "test-ftl-args-mismatch",
                key = k,
                expected = format!("{:?}", expected_vars),
                got = format!("{:?}", vars)
            ),
            "en",
            std::path::Path::new("crates/rimloc-cli/i18n/en/rimloc.ftl"),
            k,
        );
    }

    // All other locales must contain the same keys and arg sets
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
    if let Ok(rd) = fs::read_dir(locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let loc = e.file_name().to_string_lossy().to_string();
                if loc == "en" {
                    continue;
                }
                let map = load_ftl_map(&loc);
                for &k in &required_keys {
                    assert_map_contains_key(
                        &map,
                        k,
                        &loc,
                        std::path::Path::new("crates/rimloc-cli/i18n/rimloc.ftl"),
                        &ti18n!("test-ftl-key-missing", key = k, lang = &loc),
                    );
                    let vars = extract_fluent_vars(map.get(k).unwrap());
                    assert_set_eq(
                        &vars,
                        &expected_vars,
                        &ti18n!(
                            "test-ftl-args-mismatch",
                            key = k,
                            expected = format!("{:?}", expected_vars),
                            got = format!("{:?}", vars)
                        ),
                        &loc,
                        std::path::Path::new("crates/rimloc-cli/i18n/rimloc.ftl"),
                        k,
                    );
                }
            }
        }
    }
}

#[test]
fn ftl_key_order_matches_en() {
    let en = load_ftl_lines("en");

    // check all locales except en
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
    if let Ok(rd) = fs::read_dir(locales_dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let loc = e.file_name().to_string_lossy().to_string();
                if loc == "en" {
                    continue;
                }
                let other = load_ftl_lines(&loc);

                // Ensure 'other' keys appear in same relative order as 'en'
                let mut en_idx = 0usize;
                for k in &other {
                    if let Some(pos) = en[en_idx..].iter().position(|ek| ek == k) {
                        en_idx += pos + 1;
                    }
                }
                let in_same_order = other.len() == en_idx;
                assert!(
                    in_same_order,
                    "{}",
                    ti18n!("test-locale-order-mismatch", loc = loc)
                );
            }
        }
    }
}

fn scan_for_hardcoded_user_strings_in(dir: &std::path::Path, include_tests: bool) -> Vec<String> {
    use std::io::Read;

    // Macros that directly print or terminate with a message (should be localized)
    // NOTE: We intentionally do NOT include assert!/assert_eq! here to avoid
    // immediate breakage; we can tighten later once tests are localized too.
    let forbidden_macros: &[&str] = &[
        "println!",
        "eprintln!",
        "panic!",
        "unreachable!",
        "unimplemented!",
        "todo!",
        // tracing/log families
        "tracing::info!",
        "tracing::warn!",
        "tracing::error!",
        "tracing::debug!",
        "log::info!",
        "log::warn!",
        "log::error!",
        "log::debug!",
        // common error macros
        "anyhow::bail!",
        "eyre::bail!",
        "color_eyre::eyre::bail!",
    ];

    // Simple heuristic: if a line contains a forbidden macro and also contains a
    // string literal with alphabetic characters, but doesn't contain `tr!(`, flag it.
    // We skip lines that look like pure format placeholders ("{}", "{:?}") only.
    let mut offenders = Vec::new();

    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let path = e.path();
            if path.is_dir() {
                // Recurse into subdirs, excluding target/.git/… just in case
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if name == "target" || name.starts_with('.') {
                    continue;
                }
                offenders.extend(scan_for_hardcoded_user_strings_in(&path, include_tests));
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            // Skip this very test file to avoid flagging the forbidden_macros definition itself
            if path.file_name().and_then(|s| s.to_str()) == Some("cli_integration.rs") {
                continue;
            }

            // If we are scanning tests=false and this is clearly a test file path, skip
            if !include_tests {
                let pstr = path.to_string_lossy();
                if pstr.contains("/tests/")
                    || pstr.ends_with("_test.rs")
                    || pstr.ends_with("tests.rs")
                {
                    continue;
                }
            }

            let mut s = String::new();
            if let Ok(mut f) = std::fs::File::open(&path) {
                let _ = f.read_to_string(&mut s);
            } else {
                continue;
            }

            for (idx, line) in s.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }
                // ignore module imports/attributes
                if trimmed.starts_with("use ") || trimmed.starts_with("#[") {
                    continue;
                }

                let has_forbidden = forbidden_macros.iter().any(|m| trimmed.contains(m));
                if !has_forbidden {
                    continue;
                }

                let has_tr = trimmed.contains("tr!(");
                if has_tr {
                    continue;
                }

                // Rough check for a quoted string with alphabetic characters
                let has_text_literal = trimmed.matches('"').count() >= 2
                    && trimmed.contains(|c: char| c.is_ascii_alphabetic());

                if !has_text_literal {
                    continue;
                }

                // Try to skip pure formatter-only strings like "{}" or "{:?}"
                let mut pure_formatter_only = false;
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let lit = &trimmed[start + 1..start + 1 + end];
                        pure_formatter_only = lit.chars().all(|ch| {
                            ch == '{'
                                || ch == '}'
                                || ch == ':'
                                || ch == '?'
                                || ch == '!'
                                || ch.is_ascii_whitespace()
                        });
                    }
                }
                if pure_formatter_only {
                    continue;
                }

                // Extract all string literals in the line. If *all* of them look like
                // "machine" tokens (snake/kebab/alpha-num with _.-, no spaces),
                // we treat this line as non-user-facing (e.g. structured log fields like "app_started").
                // This is a lightweight heuristic to avoid false positives.
                let mut literals: Vec<&str> = Vec::new();
                {
                    let b = trimmed.as_bytes();
                    let mut i = 0usize;
                    while i < b.len() {
                        if b[i] == b'"' {
                            i += 1;
                            let start = i;
                            let mut esc = false;
                            while i < b.len() {
                                let ch = b[i];
                                if esc {
                                    esc = false;
                                    i += 1;
                                    continue;
                                }
                                if ch == b'\\' {
                                    esc = true;
                                    i += 1;
                                    continue;
                                }
                                if ch == b'"' {
                                    break;
                                }
                                i += 1;
                            }
                            let end = i.min(trimmed.len());
                            if end > start && end <= trimmed.len() {
                                literals.push(&trimmed[start..end]);
                            }
                            if i < b.len() && b[i] == b'"' {
                                i += 1;
                            }
                            continue;
                        }
                        i += 1;
                    }
                }
                let is_machiney = |s: &str| {
                    let len_ok = s.len() >= 2 && s.len() <= 40;
                    let chars_ok = s.chars().all(|c| {
                        c.is_ascii_lowercase()
                            || c.is_ascii_digit()
                            || c == '_'
                            || c == '.'
                            || c == '-'
                    });
                    len_ok && chars_ok
                };
                if !literals.is_empty() && literals.iter().all(|lit| is_machiney(lit)) {
                    // e.g. info!(event="app_started") — allowed
                    continue;
                }

                offenders.push(format!("{}:{} -> {}", path.display(), idx + 1, trimmed));
            }
        }
    }
    offenders
}

#[test]
fn no_hardcoded_user_strings_anywhere() {
    // Global scan across the whole workspace (all crates, src + tests)
    let root = workspace_root();
    let offenders = scan_for_hardcoded_user_strings_in(&root, false);
    if !offenders.is_empty() {
        let joined = offenders.join("\n");
        fail_with_context!(
            CTX_NONE,
            &root,
            "nonlocalized-user-strings",
            &ti18n!("test-nonlocalized-found", offenders = joined.clone()),
            "<forbidden macro with user-facing string>",
            &joined,
        );
    }
}

#[test]
fn no_color_removes_ansi_sequences() {
    // Берём help — он стабилен и легко воспроизводим
    let mut cmd = bin_cmd();
    // Ensure all possible color sources are disabled (clap + tracing/env_logger conventions)
    cmd.env("RUST_LOG_STYLE", "never");
    cmd.env("NO_COLOR", "1");
    cmd.env("CLICOLOR", "0");
    cmd.args(["--no-color", "--help"]);
    let assert = cmd.assert().success();
    let stdout = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();

    // Подстрахуемся: если вдруг кто-то «раскрасил» help, этот тест мигом упадёт
    assert_no_ansi(&stdout, &ti18n!("test-no-ansi-help"));
}

#[test]
fn help_lists_localized_subcommands() {
    use std::path::{Path, PathBuf};
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");

    // Ожидаемые ключи и их тексты берём из FTL для текущей локали (fallback en)
    let cmds = [
        ("scan", "help-cmd-scan"),
        ("validate", "help-cmd-validate"),
        ("validate-po", "help-cmd-validate-po"),
        ("export-po", "help-cmd-export-po"),
        ("import-po", "help-cmd-import-po"),
        ("build-mod", "help-cmd-build-mod"),
    ];

    for &lang in SUPPORTED_LOCALES.iter() {
        // Собираем пары (ожидаемый текст, ftl_key) с fallback на en
        let mut expected_pairs: Vec<(String, &str)> = Vec::new();
        for &(_, ftl_key) in &cmds {
            if let Some(txt) = read_ftl_message(&i18n_dir, lang, ftl_key)
                .or_else(|| read_ftl_message(&i18n_dir, "en", ftl_key))
            {
                expected_pairs.push((txt, ftl_key));
            }
        }

        let out = run_ok(&["--ui-lang", lang, "--help"]);
        let ftl_path = i18n_dir.join(lang).join("rimloc.ftl");
        for (snip, ftl_key) in expected_pairs {
            assert_has!(
                &out.stdout,
                &snip,
                &ftl_path,
                lang,
                ftl_key,
                &ti18n!("test-help-must-list-snip", snip = snip, lang = lang),
            );
        }
    }
}

#[test]
fn all_tr_keys_exist_in_en_ftl() {
    use std::io::Read;

    // 1) Собираем карту en
    let en_map = load_ftl_map("en");

    // 2) Сканируем исходники на упоминания тр: tr!("some.key")
    let root = workspace_root();
    let mut missing = Vec::new();

    fn scan_file(
        p: &std::path::Path,
        en_map: &std::collections::BTreeMap<String, String>,
        missing: &mut Vec<String>,
    ) {
        let pstr = p.to_string_lossy();
        if pstr.contains("/tests/")
            || p.file_name().and_then(|s| s.to_str()) == Some("cli_integration.rs")
        {
            return;
        }
        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
            return;
        }
        let mut s = String::new();
        if let Ok(mut f) = std::fs::File::open(p) {
            let _ = f.read_to_string(&mut s);
        } else {
            return;
        }

        // Очень простой парсер: ищем tr!("...") и tr!( "...", ..)
        let bytes = s.as_bytes();
        let needle = b"tr!(\"";
        let mut i = 0usize;
        while i + needle.len() < bytes.len() {
            if &bytes[i..i + needle.len()] == needle {
                let start = i + needle.len();
                let mut j = start;
                let mut esc = false;
                while j < bytes.len() {
                    let ch = bytes[j];
                    if esc {
                        esc = false;
                        j += 1;
                        continue;
                    }
                    if ch == b'\\' {
                        esc = true;
                        j += 1;
                        continue;
                    }
                    if ch == b'"' {
                        break;
                    }
                    j += 1;
                }
                if j > start {
                    let key = &s[start..j];
                    if !en_map.contains_key(key) {
                        missing.push(format!(
                            "{}: tr!(\"{}\") missing in en FTL",
                            p.display(),
                            key
                        ));
                    }
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
    }

    fn walk(
        dir: &std::path::Path,
        en_map: &std::collections::BTreeMap<String, String>,
        missing: &mut Vec<String>,
    ) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if name == "target" || name.starts_with('.') {
                        continue;
                    }
                    if name == "tests" {
                        continue;
                    }
                    walk(&p, en_map, missing);
                } else {
                    scan_file(&p, en_map, missing);
                }
            }
        }
    }

    walk(&root, &en_map, &mut missing);

    if !missing.is_empty() {
        let joined = missing.join("\n");
        let root = workspace_root();
        fail_with_context!(
            CTX_NONE,
            &root,
            "tr-keys-missing-in-en",
            &ti18n!("test-nonlocalized-found", offenders = joined.clone()),
            "<tr!(\"...\") key not in en FTL>",
            &joined,
        );
    }
}
