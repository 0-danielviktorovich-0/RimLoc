use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::collections::BTreeSet;
use std::{fs, path::PathBuf, process::Command};

// clippy: factor complex tuple types
type Paths = Vec<String>;
type Lines = Vec<String>;
type ArgMismatchEntry = (String, BTreeSet<String>, BTreeSet<String>);
type DiffResult = (Paths, Lines, Vec<ArgMismatchEntry>);

// -----------------------------------------------------------------------------
// Test-only i18n loader: reads crates/rimloc-cli/i18n/{loc}/rimloc-tests.ftl
// Minimal parser: "key = value" lines, "#" comments. Supports placeholders
// of the form "{name}" and "{ $name }".
// Locale is chosen via env var RIMLOC_TESTS_LANG (default: "en").
// -----------------------------------------------------------------------------
mod tests_i18n {
    use super::workspace_root;
    use once_cell::sync::Lazy;
    use std::collections::BTreeMap;
    use std::fs;

    // Компиляционный бэкап английского rimloc-tests.ftl на случай отсутствия файлов во время рантайма.
    const EMBEDDED_EN_TESTS_FTL: &str = include_str!("../i18n/en/rimloc-tests.ftl");
    fn tests_locale() -> String {
        std::env::var("RIMLOC_TESTS_LANG").unwrap_or_else(|_| "en".to_string())
    }

    fn parse_ftl_to_map(content: &str) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
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

    fn load_tests_ftl_map(locale: &str) -> BTreeMap<String, String> {
        // 1) пробуем нужную локаль с диска
        let ftl_path = workspace_root()
            .join("crates/rimloc-cli/i18n")
            .join(locale)
            .join("rimloc-tests.ftl");
        if let Ok(content) = fs::read_to_string(&ftl_path) {
            return parse_ftl_to_map(&content);
        }

        // 2) fallback на en с диска
        let en_path = workspace_root()
            .join("crates/rimloc-cli/i18n")
            .join("en")
            .join("rimloc-tests.ftl");
        if let Ok(content) = fs::read_to_string(&en_path) {
            return parse_ftl_to_map(&content);
        }

        // 3) последний fallback — встроенный EN
        parse_ftl_to_map(EMBEDDED_EN_TESTS_FTL)
    }

    static TESTS_FTL: Lazy<BTreeMap<String, String>> =
        Lazy::new(|| load_tests_ftl_map(&tests_locale()));

    /// Very small formatter that replaces "{name}" and "{ $name }" with provided values.
    fn apply_vars(mut template: String, args: &[(&str, String)]) -> String {
        for (name, value) in args {
            // Replace {name}
            let needle1 = format!("{{{}}}", name);
            template = template.replace(&needle1, value);
            // Replace { $name } with optional spaces
            let needle2 = format!("{{ ${} }}", name);
            template = template.replace(&needle2, value);
        }
        template
    }

    pub fn lookup(key: &str, args: &[(&str, String)]) -> String {
        let raw = TESTS_FTL
            .get(key)
            .cloned()
            .unwrap_or_else(|| format!("{{missing-test-i18n:{}}}", key));
        apply_vars(raw, args)
    }
}

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
    cmd.args(["scan", "--root"]).arg(fixture("test/TestMod"));
    cmd.assert()
        .success()
        // Проверяем только заголовок CSV — он не локализуется
        .stdout(predicate::str::contains("key,source,path,line"));
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

    let meta = fs::metadata(&out_po).expect(&ti18n!("test-outpo-exist"));
    assert!(meta.len() > 0, "{}", ti18n!("test-outpo-not-empty"));
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

    // High-level category presence (user-facing labels)
    assert!(
        out.contains("[duplicate]"),
        "{}",
        ti18n!("test-validate-dup-category")
    );
    assert!(
        out.contains("[empty]"),
        "{}",
        ti18n!("test-validate-empty-category")
    );
    assert!(
        out.contains("[placeholder-check]"),
        "{}",
        ti18n!("test-validate-ph-category")
    );

    // Specific validator item names present
    assert!(
        out.contains("DuplicateKey"),
        "{}",
        ti18n!("test-validate-dup-items")
    );
    assert!(
        out.contains("EmptyKey"),
        "{}",
        ti18n!("test-validate-empty-items")
    );
    assert!(
        out.contains("Placeholder"),
        "{}",
        ti18n!("test-validate-ph-items")
    );

    // At least one occurrence of each category
    let dup_count = out.matches("[duplicate]").count();
    let empty_count = out.matches("[empty]").count();
    let ph_count = out.matches("[placeholder-check]").count();
    assert!(
        dup_count >= 1,
        "{}",
        ti18n!(
            "test-validate-atleast-duplicates",
            min = 1,
            count = dup_count
        )
    );
    assert!(
        empty_count >= 1,
        "{}",
        ti18n!("test-validate-atleast-empty", min = 1, count = empty_count)
    );
    assert!(
        ph_count >= 1,
        "{}",
        ti18n!(
            "test-validate-atleast-placeholder",
            min = 1,
            count = ph_count
        )
    );
}

#[test]
fn import_po_requires_target() {
    let mut cmd = bin_cmd();
    cmd.args(["import-po", "--po"]).arg(fixture("test/ok.po"));

    cmd.assert().failure().stderr(predicate::str::contains(
        "нужно указать либо --out-xml, либо --mod-root",
    ));
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

    cmd.assert().failure().stderr(predicate::str::contains(
        "either --out-xml or --mod-root must be specified",
    ));
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

    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let err = String::from_utf8_lossy(assert.get_output().stderr.as_ref()).to_string();
    let combined = format!("{}{}", out, err);
    assert!(
        combined.contains("Languages/Russian/Keyed/_Imported.xml"),
        "{}",
        ti18n!(
            "test-importpo-expected-path-not-found",
            out = out,
            err = err
        )
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

    cmd.assert().success().stdout(predicate::str::contains(
        "DRY RUN: building translation mod",
    ));
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
    assert!(
        about.exists(),
        "{}",
        ti18n!("test-build-path-must-exist", path = "About/About.xml")
    );

    let keyed_any = out_mod.join("Languages/Russian/Keyed");
    assert!(
        keyed_any.exists(),
        "{}",
        ti18n!(
            "test-build-folder-must-exist",
            path = "Languages/Russian/Keyed"
        )
    );

    // Должен появиться хотя бы один XML (в нашем фикстуре — _Imported.xml или Bad.xml)
    let has_any_xml = std::fs::read_dir(&keyed_any)
        .ok()
        .map(|rd| {
            rd.flatten()
                .any(|e| e.path().extension().map(|s| s == "xml").unwrap_or(false))
        })
        .unwrap_or(false);
    assert!(
        has_any_xml,
        "{}",
        ti18n!("test-build-xml-under-path", path = "Keyed/")
    );

    // Validate content of About/About.xml includes expected metadata
    let about_content = fs::read_to_string(&about).expect(&ti18n!("test-build-about-readable"));
    assert!(
        about_content.contains("<name>RimLoc Translation</name>"),
        "{}",
        ti18n!(
            "test-build-should-contain-tag",
            path = "About/About.xml",
            tag = "<name>"
        )
    );
    assert!(
        about_content.contains("<packageId>yourname.rimloc.translation</packageId>"),
        "{}",
        ti18n!(
            "test-build-should-contain-tag",
            path = "About/About.xml",
            tag = "<packageId>"
        )
    );
}

#[test]
fn supported_locales_startup_message_matches() {
    // Считываем локали из файловой системы, чтобы тест адаптировался к репозиторию
    let locales_dir = workspace_root().join("crates/rimloc-cli/i18n");
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
        cmd.assert().success().stdout(
            predicate::str::contains("rimloc started")
                .or(predicate::str::contains("rimloc запущен")),
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

fn diff_locale_maps(
    en: &std::collections::BTreeMap<String, String>,
    other: &std::collections::BTreeMap<String, String>,
) -> DiffResult {
    let en_keys: BTreeSet<_> = en.keys().cloned().collect();
    let other_keys: BTreeSet<_> = other.keys().cloned().collect();

    let missing: Paths = en_keys.difference(&other_keys).cloned().collect();
    let extra: Lines = other_keys.difference(&en_keys).cloned().collect();

    // Parameter set mismatches on common keys
    let common: Vec<_> = en_keys.intersection(&other_keys).cloned().collect();
    let mut arg_mismatches: Vec<ArgMismatchEntry> = Vec::new();
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
    let content = fs::read_to_string(&ftl_path).unwrap_or_else(|_| {
        panic!(
            "{}",
            ti18n!("test-ftl-failed-read", path = ftl_path.display())
        )
    });

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
                use std::collections::BTreeMap;
                let mut by_sec: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
                for k in &missing {
                    by_sec.entry(section_for_key(k)).or_default().push(k);
                }
                msg.push_str(&format!("  • Missing keys ({} total):\n", missing.len()));
                for (sec, items) in by_sec {
                    msg.push_str(&format!(
                        "    - in section {} ({}): {:?}\n",
                        sec,
                        items.len(),
                        items
                    ));
                }
            }
            if !extra.is_empty() {
                use std::collections::BTreeMap;
                let mut by_sec: BTreeMap<&str, Vec<&String>> = BTreeMap::new();
                for k in &extra {
                    by_sec.entry(section_for_key(k)).or_default().push(k);
                }
                msg.push_str(&format!("  • Extra keys ({} total):\n", extra.len()));
                for (sec, items) in by_sec {
                    msg.push_str(&format!(
                        "    - in section {} ({}): {:?}\n",
                        sec,
                        items.len(),
                        items
                    ));
                }
            }
            if !arg_mismatches.is_empty() {
                msg.push_str(&format!(
                    "  • Keys with argument set mismatch ({}):\n",
                    arg_mismatches.len()
                ));
                for (k, en_vars, ot_vars) in arg_mismatches {
                    msg.push_str(&format!(
                        "    - {}\n      en: {:?}\n      {}: {:?}\n",
                        k, en_vars, loc, ot_vars
                    ));
                }
            }
            panic!("{}", ti18n!("test-nonlocalized-found", offenders = msg));
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
                cmd.args(["--ui-lang", &loc]).arg("--help");
                cmd.assert().success().stdout(
                    predicate::str::contains("RimWorld").or(predicate::str::contains("RimLoc")),
                );
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

    // Command should still succeed, but emit a warning about unsupported UI language
    cmd.assert().success().stdout(
        predicate::str::contains("Unsupported UI language")
            .or(predicate::str::contains("Неподдерживаемый язык интерфейса"))
            .or(predicate::str::contains(
                "Неподдерживаемый код языка интерфейса",
            ))
            .or(predicate::str::contains(
                "UI language code is not supported",
            )),
    );
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
    let offenders = scan_for_hardcoded_user_strings_in(&root, true);
    if !offenders.is_empty() {
        panic!(
            "{}",
            ti18n!("test-nonlocalized-found", offenders = offenders.join("\n"))
        );
    }
}
