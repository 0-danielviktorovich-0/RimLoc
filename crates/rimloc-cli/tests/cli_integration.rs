// Раньше файл скрывался под cfg(not(windows)): CLI падал STATUS_STACK_OVERFLOW на
// любой команде (1MB main-thread стек windows, гигантский derive-кадр augment_subcommands
// на 25 вариантах Commands). Фикс: Commands разбит на 6 flatten-групп — см. lib.rs и
// tests/startup_stack.rs (детерминированный 1MB-репро).

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
fn scan_help_has_use_en_comments_flag_localized() {
    use std::path::{Path, PathBuf};
    let i18n_dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    for &lang in SUPPORTED_LOCALES.iter() {
        // Expected help snippet for the new flag (fallback to EN)
        let expected = read_ftl_message(&i18n_dir, lang, "help-scan-use-en-comments")
            .or_else(|| read_ftl_message(&i18n_dir, "en", "help-scan-use-en-comments"))
            .unwrap_or_else(|| panic!("{}", ti18n!("test-help-about-key-required")));
        let mut cmd = bin_cmd();
        cmd.args(["--ui-lang", lang, "scan", "--help"]);
        let assert = cmd.assert().success();
        let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
        assert_has!(
            &out,
            &expected,
            &i18n_dir.join(lang).join("rimloc.ftl"),
            lang,
            "help-scan-use-en-comments",
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
fn scan_json_contains_schema_version() {
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"]) // stdout JSON
        .arg(fixture("test/TestMod"))
        .args(["--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(
        out.contains("\"schema_version\""),
        "JSON output should include schema_version field"
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
fn export_po_preserves_entity_markup_in_msgid() {
    // MUST_FIX_BEFORE_BETA №2: `&lt;b&gt;value&lt;/b&gt;` в Keyed обязан
    // попасть в msgid распарсенной разметкой `<b>value</b>` — не `bvalue/b`
    // (quick-xml 0.42 выносит entity в GeneralRef, сканер раньше их терял) и
    // не `&amp;lt;` (двойной escape). Конкуренты (RimTranslate, RimTrans-zh)
    // сохраняют разметку — теперь и RimLoc.
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let keyed = tmp.path().join("Languages").join("English").join("Keyed");
    fs::create_dir_all(&keyed).unwrap();
    fs::write(
        keyed.join("Entities.xml"),
        r#"<LanguageData>
	<MarkedKey>&lt;b&gt;value&lt;/b&gt; tail</MarkedKey>
</LanguageData>
"#,
    )
    .unwrap();
    let out_po = tmp.path().join("out.po");

    let mut cmd = bin_cmd();
    cmd.args(["export-po", "--root"])
        .arg(tmp.path())
        .args(["--out-po"])
        .arg(&out_po);
    cmd.assert().success();

    let po = fs::read_to_string(&out_po).unwrap();
    let line = po
        .lines()
        .find(|l| l.starts_with("msgid \"<b>"))
        .unwrap_or_else(|| panic!("msgid with markup not found in:\n{po}"));
    assert_eq!(line, "msgid \"<b>value</b> tail\"");
    assert!(
        !po.contains("bvalue/b"),
        "tag-stripped corruption must not return"
    );
    assert!(
        !po.contains("&amp;"),
        "double-escaped entities must not appear"
    );
}

#[test]
fn export_po_game_version_loadfolders_keeps_root_keyed() {
    // Regression (HugsLib 818773962, 2026-10-06): `export-po --game-version`
    // narrowed a LoadFolders mod to its vN folder via
    // resolve_game_version_root; Keyed lives at the mod root
    // (LoadFolders `<li>/</li>`), the version folder carries no `Languages/`
    // at all, so the PO silently collapsed to a bare header (76 msgid → 1).
    // The export must follow the EFFECTIVE view (root + version content
    // folders) — same rule as the scan command (Gate H).
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    fs::write(
        tmp.path().join("LoadFolders.xml"),
        r#"<loadFolders>
	<v1.6>
		<li>/</li>
		<li>v1.6</li>
	</v1.6>
</loadFolders>
"#,
    )
    .unwrap();
    let keyed = tmp.path().join("Languages").join("English").join("Keyed");
    fs::create_dir_all(&keyed).unwrap();
    fs::write(
        keyed.join("NewKeys.xml"),
        r#"<LanguageData>
	<RootKey1>значение корневого ключа</RootKey1>
	<RootKey2>&lt;b&gt;разметка&lt;/b&gt; работает</RootKey2>
	<RootKey3>третий ключ</RootKey3>
</LanguageData>
"#,
    )
    .unwrap();
    // Version folder with NO Languages — the exact HugsLib shape (DLL/Defs
    // bump only). Without the fix the narrowed scan sees nothing here.
    fs::create_dir_all(tmp.path().join("v1.6")).unwrap();
    let out_po = tmp.path().join("out.po");

    let mut cmd = bin_cmd();
    cmd.args(["export-po", "--root"])
        .arg(tmp.path())
        .args(["--out-po"])
        .arg(&out_po)
        .args(["--game-version", "1.6"]);
    cmd.assert().success();

    let po = fs::read_to_string(&out_po).unwrap();
    for marker in [
        "значение корневого ключа",
        "msgid \"<b>разметка</b> работает\"",
        "третий ключ",
    ] {
        assert!(
            po.contains(marker),
            "root Keyed entry `{marker}` missing from:\n{po}"
        );
    }
    let msgid_count = po
        .lines()
        .filter(|l| l.starts_with("msgid \"") && !l.starts_with("msgid \"\""))
        .count();
    assert!(
        msgid_count >= 3,
        "expected ≥3 msgid from the root Keyed file, got {msgid_count}:\n{po}"
    );
}

#[test]
fn scan_plain_version_dirs_pick_newest_version_per_key() {
    // Wave-5 MUST_FIX (VE Framework 2023507013): a mod with NATIVE version
    // folders (1.0–1.6, no LoadFolders.xml) must resolve every key to its
    // NEWEST defining version. The old whole-root union let a lexicographic
    // path accident (`Stats.xml` sorts after `Stats_Pawns.xml`) surface the
    // stale 1.3 value `verb range factor` where the disk ships 1.6
    // `weapon range factor` — and the default re-root scan silently lost the
    // root Keyed files the game always loads (GAME_SOURCE_FINDINGS §1.3).
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let root = tmp.path();
    // Shared identity, different values per version (the StatDef case).
    let defs_16 = root.join("1.6/Defs");
    fs::create_dir_all(&defs_16).unwrap();
    fs::write(
        defs_16.join("RangeDefs.xml"),
        r#"<Defs>
	<StatDef>
		<defName>Wave5VerbRange</defName>
		<label>weapon range factor</label>
	</StatDef>
</Defs>
"#,
    )
    .unwrap();
    let defs_14 = root.join("1.4/Defs");
    fs::create_dir_all(&defs_14).unwrap();
    fs::write(
        defs_14.join("RangeDefs.xml"),
        r#"<Defs>
	<StatDef>
		<defName>Wave5VerbRange</defName>
		<label>verb range factor</label>
	</StatDef>
</Defs>
"#,
    )
    .unwrap();
    // A key only the OLD version defines: union coverage must keep it,
    // the effective single-version view must not invent it.
    let defs_13 = root.join("1.3/Defs");
    fs::create_dir_all(&defs_13).unwrap();
    fs::write(
        defs_13.join("Legacy.xml"),
        r#"<Defs>
	<StatDef>
		<defName>Wave5LegacyOnly</defName>
		<label>только в 1.3</label>
	</StatDef>
</Defs>
"#,
    )
    .unwrap();
    // Root Keyed: the game loads the root ALWAYS (§1.3) — the default scan
    // must not lose it to re-rooting.
    let keyed = root.join("Languages/English/Keyed");
    fs::create_dir_all(&keyed).unwrap();
    fs::write(
        keyed.join("Root.xml"),
        r#"<LanguageData>
	<Wave5RootKey>корневой ключ</Wave5RootKey>
</LanguageData>
"#,
    )
    .unwrap();

    // Default scan = effective view of the newest version: 1.6 value, root
    // Keyed present, no 1.4 value, no 1.3-only key.
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"])
        .arg(root)
        .args(["--format", "json"]);
    let out =
        String::from_utf8_lossy(cmd.assert().success().get_output().stdout.as_ref()).to_string();
    assert!(
        out.contains("weapon range factor"),
        "auto scan must take the 1.6 value:\n{out}"
    );
    assert!(
        out.contains("корневой ключ"),
        "root Keyed must be scanned with the version folder (game loads both):\n{out}"
    );
    assert!(
        !out.contains("verb range factor"),
        "stale 1.4 value must not leak into the 1.6 view:\n{out}"
    );
    assert!(
        !out.contains("только в 1.3"),
        "the single-version view must not union older folders:\n{out}"
    );

    // --game-version 1.4 → the 1.4 value is the requested-version priority.
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]).arg(root).args([
        "--format",
        "json",
        "--game-version",
        "1.4",
    ]);
    let out =
        String::from_utf8_lossy(cmd.assert().success().get_output().stdout.as_ref()).to_string();
    assert!(
        out.contains("verb range factor"),
        "--game-version 1.4 must take the 1.4 value:\n{out}"
    );
    assert!(
        !out.contains("weapon range factor"),
        "a 1.4 request must not see 1.6 content:\n{out}"
    );

    // --include-all-versions keeps the union COVERAGE but every shared key
    // resolves to its newest defining version (the MUST_FIX semantics).
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]).arg(root).args([
        "--format",
        "json",
        "--include-all-versions",
    ]);
    let out =
        String::from_utf8_lossy(cmd.assert().success().get_output().stdout.as_ref()).to_string();
    assert!(
        out.contains("weapon range factor"),
        "union must resolve the shared key to the newest (1.6) value:\n{out}"
    );
    assert!(
        out.contains("только в 1.3"),
        "union coverage must keep the 1.3-only key:\n{out}"
    );
    assert!(
        out.contains("корневой ключ"),
        "union must keep root Keyed:\n{out}"
    );
    assert!(
        !out.contains("verb range factor"),
        "the stale 1.4 value must never shadow the 1.6 value (wave-5 MUST_FIX):\n{out}"
    );

    // Version validation follows the game fallback (§1.3, same as
    // LoadFolders): a request ABOVE every folder resolves DOWN with a
    // warning (9.9 → 1.6), a request BELOW everything is a loud refusal.
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]).arg(root).args([
        "--format",
        "json",
        "--game-version",
        "9.9",
    ]);
    let out =
        String::from_utf8_lossy(cmd.assert().success().get_output().stdout.as_ref()).to_string();
    assert!(
        out.contains("weapon range factor"),
        "a above-everything request falls back to the newest folder:\n{out}"
    );
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "scan", "--root"]).arg(root).args([
        "--format",
        "json",
        "--game-version",
        "1.0",
    ]);
    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("not found under"));
}

#[test]
fn build_mod_from_root_newest_version_wins_per_key() {
    // Wave-5 MUST_FIX parity for build: the same key in 1.4 and 1.6
    // Languages trees must not be written twice with a scan-order-dependent
    // winner — the newest version owns the key.
    let tmp = tempfile::tempdir().expect("tempdir");
    let src = tmp.path();
    let write = |rel: &str, body: &str| {
        let p = src.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    };
    write(
        "1.4/Languages/Russian/Keyed/Robots.xml",
        "<LanguageData><Wave5Bot>старый робот</Wave5Bot><Wave5OldOnly>только 1.4</Wave5OldOnly></LanguageData>",
    );
    write(
        "1.6/Languages/Russian/Keyed/Robots.xml",
        "<LanguageData><Wave5Bot>новый робот</Wave5Bot></LanguageData>",
    );
    let out = tempfile::tempdir().expect("out");
    let out_dir = out.path().join("RimLoc_RU");

    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "--ui-lang", "en", "build-mod"])
        .args(["--po", "./test/ok.po"])
        .args(["--out-mod"])
        .arg(&out_dir)
        .args(["--lang", "ru"])
        .args(["--from-root"])
        .arg(src);
    cmd.current_dir(workspace_root());
    cmd.assert().success();

    let built = fs::read_to_string(out_dir.join("Languages/Russian/Keyed/Robots.xml")).unwrap();
    assert!(
        built.contains("новый робот"),
        "the 1.6 value must own the shared key:\n{built}"
    );
    assert!(
        !built.contains("старый робот"),
        "the 1.4 value must not shadow the 1.6 value:\n{built}"
    );
    assert!(
        built.contains("только 1.4"),
        "keys only the older version defines stay in the union:\n{built}"
    );
    // Exactly one element per key: no duplicate `<Wave5Bot>` entries.
    assert_eq!(
        built.matches("<Wave5Bot>").count(),
        1,
        "the shared key must be written once:\n{built}"
    );
}

#[test]
fn build_mod_skip_empty_drops_untranslated_keys() {
    // Wave-5 MUST_FIX №2 (HugsLib LEVEL7, finding №2): untranslated PO
    // entries used to be written as empty `<Key></Key>` elements — the game
    // may render them as a MISSING UI string instead of falling back to
    // English. Default keeps the old output; `--skip-empty` drops them.
    let tmp = tempfile::tempdir().expect("tempdir");
    let po = tmp.path().join("in.po");
    fs::write(
        &po,
        r#"msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"

#: Languages/Russian/Keyed/Gear.xml:2
msgctxt "HelmetName"
msgid "Combat helmet"
msgstr "Боевой шлем"

#: Languages/Russian/Keyed/Gear.xml:3
msgctxt "VestName"
msgid "Armored vest"
msgstr ""

#: Languages/Russian/Keyed/Gear.xml:4
msgctxt "PackName"
msgid "Field pack"
msgstr "   "
"#,
    )
    .unwrap();
    let out = tempfile::tempdir().expect("out");
    let out_dir = out.path().join("RimLoc_RU");
    let cwd = workspace_root();

    // Default: both translated and untranslated keys are present (old
    // behaviour, backwards compatible).
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "--ui-lang", "en", "build-mod"])
        .args(["--po"])
        .arg(&po)
        .args(["--out-mod"])
        .arg(&out_dir)
        .args(["--lang", "ru"]);
    cmd.current_dir(&cwd);
    cmd.assert().success();
    let default_xml = fs::read_to_string(out_dir.join("Languages/Russian/Keyed/Gear.xml")).unwrap();
    assert!(
        default_xml.contains("<HelmetName>Боевой шлем</HelmetName>"),
        "translated key present by default:\n{default_xml}"
    );
    assert!(
        default_xml.contains("<VestName></VestName>"),
        "empty element still written by default (backwards compat):\n{default_xml}"
    );

    // --skip-empty: only translated keys survive; a whitespace-only msgstr
    // counts as untranslated too.
    let out2 = tempfile::tempdir().expect("out2");
    let out_dir2 = out2.path().join("RimLoc_RU");
    let mut cmd = bin_cmd();
    cmd.args(["--quiet", "--ui-lang", "en", "build-mod"])
        .args(["--po"])
        .arg(&po)
        .args(["--out-mod"])
        .arg(&out_dir2)
        .args(["--lang", "ru", "--skip-empty"]);
    cmd.current_dir(&cwd);
    cmd.assert().success();
    let skipped_xml =
        fs::read_to_string(out_dir2.join("Languages/Russian/Keyed/Gear.xml")).unwrap();
    assert!(
        skipped_xml.contains("<HelmetName>Боевой шлем</HelmetName>"),
        "translated keys survive --skip-empty:\n{skipped_xml}"
    );
    assert!(
        !skipped_xml.contains("VestName"),
        "empty msgstr must not produce an empty element:\n{skipped_xml}"
    );
    assert!(
        !skipped_xml.contains("PackName"),
        "whitespace-only msgstr counts as untranslated:\n{skipped_xml}"
    );
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
    assert!(!msgs.is_empty(), "expected at least one issue in fixture");
    let allowed = [
        "duplicate",
        "duplicate-global",
        "empty",
        "placeholder-check",
    ];
    for m in msgs {
        assert!(
            allowed.contains(&m.kind.as_str()),
            "unexpected kind: {}",
            m.kind
        );
        assert!(!m.key.is_empty(), "key must be non-empty");
        assert!(!m.path.is_empty(), "path must be non-empty");
        assert!(!m.message.is_empty(), "message must be non-empty");
        // Touch optional line to avoid dead_code warnings and ensure it's a valid number if present
        if let Some(l) = m.line {
            let _ = l;
        }
    }
}

#[test]
fn validate_json_contains_schema_version() {
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"]) // ensure no banner
        .args(["validate", "--root"]) // known to contain issues in Bad.xml
        .arg(fixture("test/TestMod"))
        .args(["--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(
        out.contains("\"schema_version\""),
        "JSON diagnostics should include schema_version field"
    );
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
    assert!(
        s.contains("msgid \"Old\""),
        "PO must contain 'Old' from v1.5"
    );
    assert!(
        !s.contains("msgid \"New\""),
        "PO must not contain 'New' from v1.6 when version=1.5"
    );
}

#[test]
fn scan_errors_on_missing_version() {
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"])
        .args(["scan", "--root"])
        .arg(fixture("test/TestMod"))
        .args(["--game-version", "9.9"]);
    cmd.assert().failure();
}

#[test]
fn export_po_errors_on_missing_version() {
    let tmp = tempfile::tempdir().expect(&ti18n!("test-tempdir"));
    let out_po = tmp.path().join("out.po");
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"])
        .args(["export-po", "--root"])
        .arg(fixture("test/TestMod"))
        .args(["--out-po"])
        .arg(&out_po)
        .args(["--game-version", "0.0"]);
    cmd.assert().failure();
}

#[test]
fn scan_csv_adds_lang_column_when_lang_passed() {
    // When --lang is provided, CSV should add a lang column as the first header
    let mut cmd = bin_cmd();
    cmd.args(["--quiet"])
        .args(["scan", "--root"])
        .arg(fixture("test/TestMod"))
        .args(["--lang", "ru"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(out
        .lines()
        .next()
        .unwrap_or("")
        .starts_with("lang,key,source,path,line"));
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
    let en_keyed = root.join("Languages").join("English").join("Keyed");
    let ru_keyed = root.join("Languages").join("Russian").join("Keyed");
    fs::create_dir_all(&en_keyed).unwrap();
    fs::create_dir_all(&ru_keyed).unwrap();

    let mut f_en = fs::File::create(en_keyed.join("A.xml")).unwrap();
    writeln!(
        f_en,
        "<LanguageData>\n  <K_EN>Hello</K_EN>\n</LanguageData>\n"
    )
    .unwrap();
    let mut f_ru = fs::File::create(ru_keyed.join("B.xml")).unwrap();
    writeln!(
        f_ru,
        "<LanguageData>\n  <K_RU>Привет</K_RU>\n</LanguageData>\n"
    )
    .unwrap();

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
    assert_eq!(
        keys,
        ["K_EN", "K_RU"].into_iter().map(String::from).collect()
    );

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
    assert!(
        s.contains("<K_NEW>Привет</K_NEW>"),
        "imported value must appear"
    );
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
            ("[duplicate/error]", bad_xml_en.as_path(), "DuplicateKey"),
            ("[empty/error]", bad_xml_en.as_path(), "EmptyKey"),
            (
                "[placeholder-check/info]",
                bad_xml_en.as_path(),
                "Placeholder",
            ),
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
        "[duplicate/error]",
        1,
        &ti18n!(
            "test-validate-atleast-duplicates",
            min = 1,
            count = out.matches("[duplicate/error]").count()
        ),
        CTX_NONE,
        bad_xml_en.as_path(),
        "category-[duplicate]",
    );
    assert_count_at_least(
        &out,
        "[empty/error]",
        1,
        &ti18n!(
            "test-validate-atleast-empty",
            min = 1,
            count = out.matches("[empty/error]").count()
        ),
        CTX_NONE,
        bad_xml_en.as_path(),
        "category-[empty]",
    );
    assert_count_at_least(
        &out,
        "[placeholder-check/info]",
        1,
        &ti18n!(
            "test-validate-atleast-placeholder",
            min = 1,
            count = out.matches("[placeholder-check/info]").count()
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
    // Windows announces the plan path with `\` separators; normalize so
    // the fixture-relative needle matches the announced location itself.
    let combined = format!("{}{}", out, err).replace('\\', "/");
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
        // Windows announces the plan path with `\` separators; normalize so
        // the fixture-relative needle matches the announced location itself.
        let combined = format!("{}{}", out, err).replace('\\', "/");
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
    // Windows announces the plan path with `\` separators; normalize so
    // the fixture-relative needle matches the announced location itself.
    let combined = format!("{}{}", out, err).replace('\\', "/");
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
                // Skip vendored third-party code (not subject to our i18n rules).
                // Normalized separators: on Windows the path spelling mixes
                // `/` and `\`, and same-separator contains() missed the skip
                // (windows CI scanned trees unix never did).
                let pstr = path.to_string_lossy().replace('\\', "/");
                if pstr.contains("/src-tauri/vendor/") || pstr.contains("/vendor/") {
                    continue;
                }
                // Skip examples/ directories: acceptance-harness examples are
                // developer tooling with contracted MACHINE output (JSON report
                // + artifact path), not localized product UI. Product i18n
                // rules still apply to every src/ and tests/ file.
                if name == "examples" {
                    continue;
                }
                offenders.extend(scan_for_hardcoded_user_strings_in(&path, include_tests));
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            // Skip cargo build scripts: `println!("cargo:…")` inside build.rs is
            // the build protocol spoken to cargo itself (rerun-if-changed,
            // rustc-env), machine-facing by construction — not product UI.
            if path.file_name().and_then(|s| s.to_str()) == Some("build.rs") {
                continue;
            }
            // Skip this very test file to avoid flagging the forbidden_macros definition itself
            if path.file_name().and_then(|s| s.to_str()) == Some("cli_integration.rs") {
                continue;
            }

            // If we are scanning tests=false and this is clearly a test file path, skip
            if !include_tests {
                // Normalized separators: same-separator contains() never
                // matched the Windows `\tests\` spelling, so windows CI
                // flagged test helpers unix never scans (test-infra bug,
                // not a product i18n violation).
                let pstr = path.to_string_lossy().replace('\\', "/");
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
                    // Skip vendored third-party code
                    let pstr = p.to_string_lossy();
                    if pstr.contains("/src-tauri/vendor/")
                        || pstr.contains("\\src-tauri\\vendor\\")
                        || pstr.contains("/vendor/")
                        || pstr.contains("\\vendor\\")
                    {
                        continue;
                    }
                    if name == "tests" {
                        continue;
                    }
                    walk(&p, en_map, missing);
                } else {
                    let pstr = p.to_string_lossy();
                    if pstr.contains("/src-tauri/vendor/")
                        || pstr.contains("\\src-tauri\\vendor\\")
                        || pstr.contains("/vendor/")
                        || pstr.contains("\\vendor\\")
                    {
                        continue;
                    }
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

// ---------------------------------------------------------------------------
// Language scoping regressions (real-mod dogfood: VWE mixes NL/EN/JA/RU).
// Fixture: test/MultiLangMod — same keys in English/Russian/Japanese + Defs.
// ---------------------------------------------------------------------------

fn scan_lang_json(root: &std::path::Path, args: &[&str]) -> Vec<(String, String)> {
    // Returns (language-folder-or-Defs, key) pairs from scan --format json stdout.
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(root)
        .args(args)
        .args(["--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let units: Vec<serde_json::Value> = serde_json::from_str(&out).expect("scan json");
    units
        .into_iter()
        .map(|u| {
            let path = u["path"].as_str().unwrap_or_default().to_string();
            // Separator-agnostic scope: the JSON path on Windows is spelled
            // with `\` (possibly mixed with `/`), and a pure "/Languages/"
            // split labeled every unit "Defs" there.
            let segments: Vec<&str> = path.split(['/', '\\']).collect();
            let scope = match segments
                .iter()
                .position(|s| s.eq_ignore_ascii_case("Languages"))
            {
                Some(i) if i + 1 < segments.len() => segments[i + 1].to_string(),
                _ => "Defs".to_string(),
            };
            (scope, u["key"].as_str().unwrap_or_default().to_string())
        })
        .collect()
}

#[test]
fn scan_lang_flag_filters_to_requested_language() {
    let root = fixture("test/MultiLangMod");

    let en = scan_lang_json(&root, &["--lang", "en"]);
    assert!(
        en.iter().all(|(scope, _)| scope == "English"),
        "--lang en must collect only the English source set, got: {en:?}"
    );
    // Defs-derived strings surface under the English DefInjected target path;
    // ML_Gadget exists only in Defs (no translation files), so its presence
    // proves Defs were scanned on the English side.
    assert!(
        en.iter().any(|(_, key)| key == "ML_Gadget.description"),
        "--lang en must include Defs-derived source strings"
    );

    let ru = scan_lang_json(&root, &["--lang", "ru"]);
    assert!(
        ru.iter().all(|(scope, _)| scope == "Russian"),
        "--lang ru must collect only Russian folder, got: {ru:?}"
    );
    assert!(
        !ru.iter().any(|(_, key)| key.starts_with("ML_Gadget")),
        "--lang ru must not include untranslated Defs-only strings"
    );
}

#[test]
fn coverage_accepts_full_language_dir_paths() {
    // Regression T6: full paths as --source-lang-dir/--target-lang-dir were
    // compared as bare folder names, so coverage always reported 0.
    let ws = workspace_root();
    let root = ws.join("test/MultiLangMod");
    let mut cmd = bin_cmd();
    cmd.args(["coverage", "--root"])
        .arg(&root)
        .arg("--source-lang-dir")
        .arg(root.join("Languages/English"))
        .arg("--target-lang-dir")
        .arg(root.join("Languages/Russian"));
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let line = out
        .lines()
        .find(|l| l.starts_with("Coverage:"))
        .expect("coverage summary line");
    // English side: Defs (label+description) + Keyed(2) + DefInjected(1) — at least 4 source keys
    assert!(
        line.contains("source="),
        "unexpected coverage output: {line}"
    );
    assert!(
        !line.contains("source=0"),
        "coverage must be non-zero with full-path args: {line}"
    );
    assert!(
        !line.contains("translated=0"),
        "Russian translation must be recognized: {line}"
    );
}

#[test]
fn validate_does_not_flag_same_key_across_language_folders() {
    // Regression T7: ML_Greeting exists in English, Russian and Japanese Keyed
    // folders — that is normal RimWorld layout, not a duplicate.
    let root = fixture("test/MultiLangMod");
    let mut cmd = bin_cmd();
    cmd.args(["validate", "--root"]).arg(&root);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    assert!(
        !out.contains("duplicate-global"),
        "same key across language folders must not be reported as duplicate-global: {out}"
    );
}

#[test]
fn version_config_falls_back_for_flat_mod_declaring_support() {
    // Regression (iteration 2): MultiLangMod is flat (no 1.x/ dirs) but About
    // declares 1.5; a configured game_version=1.5 must resolve to the root.
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(fixture("test/MultiLangMod"))
        .args(["--game-version", "1.5", "--lang", "en", "--format", "json"]);
    cmd.assert().success();

    // Unknown version on a flat mod must still fail (typo protection).
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(fixture("test/MultiLangMod"))
        .args(["--game-version", "9.9"]);
    cmd.assert().failure();
}

#[test]
fn coverage_treats_todo_placeholder_as_missing() {
    // RimWorld parity (1.6 LoadedLanguage): "TODO" is not a translation.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("Languages/English/Keyed")).unwrap();
    std::fs::create_dir_all(root.join("Languages/Russian/Keyed")).unwrap();
    std::fs::write(
        root.join("Languages/English/Keyed/K.xml"),
        "<LanguageData>\n  <A>Hello world</A>\n  <B>Two words</B>\n  <C>Three word text</C>\n</LanguageData>\n",
    )
    .unwrap();
    std::fs::write(
        root.join("Languages/Russian/Keyed/K.xml"),
        "<LanguageData>\n  <A>Привет мир</A>\n  <B>TODO</B>\n  <C>todo</C>\n</LanguageData>\n",
    )
    .unwrap();

    let mut cmd = bin_cmd();
    cmd.args(["coverage", "--root"])
        .arg(root)
        .arg("--source-lang-dir")
        .arg(root.join("Languages/English"))
        .arg("--target-lang-dir")
        .arg(root.join("Languages/Russian"));
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    let line = out.lines().find(|l| l.starts_with("Coverage:")).unwrap();
    // B and C are TODO → missing; only A translated.
    assert!(line.contains("translated=1"), "{line}");
    assert!(line.contains("missing=2"), "{line}");
}

#[test]
fn scan_never_emits_nontranslatable_technical_fields() {
    // NoTranslate evidence: extraction is allowlist-driven, so technical/
    // identity fields (defName, texPath, workerClass, defaultDamage) can never
    // become translatable entries — this is the construction-level guarantee
    // behind the NoTranslate classification (see RIMWORLD_REFERENCE_AUDIT.md).
    let mut cmd = bin_cmd();
    cmd.args(["scan", "--root"])
        .arg(fixture("test/MultiLangMod"))
        .args(["--game-version", "1.5", "--lang", "en", "--format", "json"]);
    let assert = cmd.assert().success();
    let out = String::from_utf8_lossy(assert.get_output().stdout.as_ref()).to_string();
    for forbidden in ["defName", "texPath", "workerClass", "defaultDamage"] {
        let bad: Vec<&str> = out
            .lines()
            .filter(|l| l.contains(&format!(".{forbidden}\"")))
            .map(|l| &l[..l.len().min(120)])
            .collect();
        assert!(
            bad.is_empty(),
            "technical field `{forbidden}` must never be extracted: {bad:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// P1-4 TKey regressions (fixture: test/TKeyMod with RU DefInjected).
// All three proven serialization shapes must match through the registry-gated
// fallback: bare (TipSetDef li), .slateRef (direct field) and
// .value.slateRef (parms descendant of QuestNode_SubScript).
// ---------------------------------------------------------------------------
#[test]
fn coverage_matches_tkey_serialization_shapes_on_tkey_fixture() {
    let root = fixture("test/TKeyMod");
    let out = run_ok(&[
        "coverage",
        "--root",
        root.to_str().unwrap(),
        "--source-lang-dir",
        "English",
        "--target-lang-dir",
        "Russian",
        "--format",
        "json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out.stdout).expect("coverage json");
    // 6 source units = 6 TKey identities. The canonical inventory (B
    // unification) removed the double extraction the old parsers path had:
    // a TKey-attributed node no longer ALSO appears as an ordinary
    // `SampleQuest.label` unit. translated=4: bare, .slateRef and
    // .value.slateRef all match; missing=2 = TODO placeholder (case C,
    // RimWorld LoadedLanguage parity) + AbsentTip with no RU entry at all
    // (case D absent).
    assert_eq!(v["source_total"].as_u64(), Some(6), "{v}");
    assert_eq!(v["target_total"].as_u64(), Some(5), "{v}");
    assert_eq!(v["translated"].as_u64(), Some(4), "{v}");
    assert_eq!(v["missing"].as_u64(), Some(2), "{v}");
}

#[test]
fn scan_emits_tkey_metadata_strategies_on_tkey_fixture() {
    let root = fixture("test/TKeyMod");
    let units = scan_lang_json(&root, &["--lang", "English"]);
    let tkeys: Vec<(String, String)> = units
        .iter()
        .filter(|(scope, _)| scope == "Defs")
        .cloned()
        .collect();
    // The parms shape must be present alongside bare and slate_ref shapes.
    let keys: Vec<&str> = tkeys.iter().map(|(_, k)| k.as_str()).collect();
    assert!(keys.contains(&"SampleQuest.LetterTextParms"), "{keys:?}");
    assert!(keys.contains(&"SampleTips.DismissLetters"), "{keys:?}");
}

// ---------------------------------------------------------------------------
// P1-2 TKey round-trip (gate A): source Defs TKey -> PO with proven target
// paths -> TM-prefilled build -> correct DefInjected output for all three
// serialization strategies (bare / .slateRef / .value.slateRef).
// ---------------------------------------------------------------------------
#[test]
fn tkey_round_trip_export_tm_build_produces_valid_definjected() {
    let root = fixture("test/TKeyMod");
    let tmp = tempfile::TempDir::new().unwrap();
    let po = tmp.path().join("rt.po");
    let out_mod = tmp.path().join("rt-mod");

    run_ok(&[
        "export-po",
        "--root",
        root.to_str().unwrap(),
        "--out-po",
        po.to_str().unwrap(),
        "--tm-root",
        root.join("Languages/Russian").to_str().unwrap(),
    ]);
    run_ok(&[
        "build-mod",
        "--po",
        po.to_str().unwrap(),
        "--out-mod",
        out_mod.to_str().unwrap(),
        "--lang",
        "ru",
        "--name",
        "TKey RT",
        "--package-id",
        "rt.tkey.test",
        "--rw-version",
        "1.6",
        "--lang-dir",
        "Russian",
    ]);

    let quest = std::fs::read_to_string(
        out_mod.join("Languages/Russian/DefInjected/QuestScriptDef/SampleQuest.xml"),
    )
    .unwrap();
    // Proven serialization paths, not guessed shapes:
    assert!(
        quest.contains("<SampleQuest.LetterLabelFavorReceiver.slateRef>Метка услуги</"),
        "{quest}"
    );
    assert!(
        quest.contains("<SampleQuest.LetterTextParms.value.slateRef>Пармс-текст.</"),
        "{quest}"
    );
    assert!(
        quest.contains("<SampleQuest.LetterTextSample.slateRef>Текст квеста.</"),
        "{quest}"
    );
    // TODO placeholder survives the round-trip as TODO.
    assert!(
        quest.contains("<SampleQuest.ExpiryTip.slateRef>TODO</"),
        "{quest}"
    );

    let tips = std::fs::read_to_string(
        out_mod.join("Languages/Russian/DefInjected/TipSetDef/SampleTips.xml"),
    )
    .unwrap();
    // Bare strategy: no suffix for TipSetDef li.
    assert!(
        tips.contains("<SampleTips.DismissLetters>Подсказки"),
        "{tips}"
    );
}

// ---------------------------------------------------------------------------
// Gate H evidence: effective RimWorld source precedence.
// H3 — LoadFolders/version: changing the target version changes the
// effective winner (real-mod analogue: VWE 1814383360, 195 units @1.5 vs
// 247 @1.6 — recorded in CANONICAL_INVENTORY.md).
// ---------------------------------------------------------------------------
#[test]
fn loadfolders_scan_is_version_scoped() {
    let root = fixture("test/LoadFoldersMod");
    let run = |ver: &str| {
        let out = run_ok(&[
            "scan",
            "--root",
            root.to_str().unwrap(),
            "--game-version",
            ver,
            "--lang",
            "English",
            "--format",
            "json",
        ]);
        let units: Vec<serde_json::Value> = serde_json::from_str(&out.stdout).unwrap();
        units
    };
    let v15 = run("1.5");
    let v16 = run("1.6");

    let value = |units: &[serde_json::Value], key: &str| -> Option<String> {
        units
            .iter()
            .find(|u| u["key"].as_str() == Some(key))
            .and_then(|u| u["value"].as_str().map(String::from))
    };

    // Def identity lives in BOTH version roots; the effective winner is the
    // requested version's file, not the union and not a sorted accident.
    assert_eq!(
        value(&v15, "Versioned.label").as_deref(),
        Some("Label from 1.5"),
        "1.5 view: {v15:?}"
    );
    assert_eq!(
        value(&v16, "Versioned.label").as_deref(),
        Some("Label from 1.6"),
        "1.6 view: {v16:?}"
    );
    // Content dirs not active in 1.5 do not contribute.
    assert!(value(&v15, "OnlySixteen").is_none(), "1.5 view: {v15:?}");
    assert_eq!(
        value(&v16, "OnlySixteen").as_deref(),
        Some("sixteen-only"),
        "{v16:?}"
    );
    // Common languages stay present in both.
    assert_eq!(value(&v15, "Greeting").as_deref(), Some("hello"));
}

/// H1: CLI language folders are joined into write paths — traversal,
/// absolute and backslash shapes are refused with a typed error, and
/// nothing is written outside the mod root.
#[test]
fn cli_lang_dir_traversal_is_refused_on_write_commands() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mod");
    fs::create_dir_all(root.join("Defs")).unwrap();

    // Minimal PO for the import/build commands.
    let po = tmp.path().join("t.po");
    fs::write(
        &po,
        "msgctxt \"ThingDef/Dup.label\"\nmsgid \"thing\"\nmsgstr \"вещь\"\n",
    )
    .unwrap();

    let evil_dirs = [
        "../../evil-init",
        "/tmp/rimloc-cli-absolute-evil",
        "C:\\evil",
        "ru/../../evil-x",
    ];
    for dir in evil_dirs {
        // init: refused before any scan or write.
        bin_cmd()
            .args(["init", "--root"])
            .arg(&root)
            .args(["--lang", "ru", "--lang-dir", dir])
            .assert()
            .failure()
            .stderr(predicates::str::contains("malformed language folder"));

        // import-po: refused before anything is written into the tree.
        bin_cmd()
            .args(["import-po", "--po"])
            .arg(&po)
            .args(["--mod-root"])
            .arg(&root)
            .args(["--lang-dir", dir])
            .assert()
            .failure()
            .stderr(predicates::str::contains("malformed language folder"));

        // build-mod: refused before the output tree is created.
        bin_cmd()
            .args([
                "build-mod",
                "--po",
                po.to_str().unwrap(),
                "--out-mod",
                tmp.path().join("out").to_str().unwrap(),
                "--lang",
                "ru",
                "--lang-dir",
                dir,
            ])
            .assert()
            .failure()
            .stderr(predicates::str::contains("malformed language folder"));
    }

    // Zero side effects outside the mod root.
    assert!(!tmp.path().join("evil-init").exists());
    assert!(!tmp.path().join("evil-x").exists());
    assert!(!std::path::Path::new("/tmp/rimloc-cli-absolute-evil").exists());
}

/// H1 containment branch: a bare folder name passes the form check, but
/// `Languages/<name>` being a symlink pointing OUT of the mod root is
/// refused (deny-direction containment, symlink aliases included).
#[test]
#[cfg(unix)]
fn cli_lang_dir_symlink_escape_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mod");
    fs::create_dir_all(root.join("Languages")).unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("Languages").join("Evil")).unwrap();

    bin_cmd()
        .args(["init", "--root"])
        .arg(&root)
        .args(["--lang", "ru", "--lang-dir", "Evil"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("resolves outside the mod root"));

    assert!(
        !outside.join("Languages").exists(),
        "nothing was written through the symlink"
    );
}

/// H2: a LoadFolders.xml `<li>` entry pointing OUTSIDE the mod root is a
/// typed scan refusal — the external content (LEAKED key) is never read.
#[test]
fn cli_scan_refuses_loadfolders_entry_outside_root() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(outside.join("Languages/English/Keyed")).unwrap();
    fs::write(
        outside.join("Languages/English/Keyed/S.xml"),
        "<LanguageData><SecretKey.Outside>LEAKED-KEYED-TEXT-77</SecretKey.Outside></LanguageData>",
    )
    .unwrap();

    let escmod = tmp.path().join("escmod");
    fs::create_dir_all(escmod.join("1.6")).unwrap();
    fs::write(
        escmod.join("LoadFolders.xml"),
        format!(
            "<loadFolders><v1.6><li>1.6</li><li>{}</li></v1.6></loadFolders>",
            outside.display()
        ),
    )
    .unwrap();

    bin_cmd()
        .args(["scan", "--root"])
        .arg(&escmod)
        .args(["--game-version", "1.6", "--format", "json"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("outside the mod root"));

    // The leaked key never reached the output.
    let output = bin_cmd()
        .args(["scan", "--root"])
        .arg(&escmod)
        .args(["--game-version", "1.6", "--format", "json"])
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(!out.contains("LEAKED-KEYED-TEXT-77"));
}

/// M1: a non-existent mod root is a loud refusal (non-zero exit), never a
/// green "all clean" / empty "[]" report over nothing.
#[test]
fn cli_validate_and_scan_refuse_nonexistent_root() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no-such-mod");

    bin_cmd()
        .args(["validate", "--root"])
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicates::str::contains("does not exist"));

    bin_cmd()
        .args(["scan", "--root"])
        .arg(&missing)
        .assert()
        .failure()
        .stderr(predicates::str::contains("does not exist"));
}

/// Regression (diff vs Text Grabber, workshop mod 3242000764): a Steam
/// workshop content folder is named by its numeric workshop id. The numeric
/// name must not be mistaken for a RimWorld game version, otherwise
/// `scan --game-version 1.5` silently resolves to the mod root and ships
/// 1.6 content for a 1.5 request (probe before the fix: 0 units from 1.5,
/// 40 units from 1.6 on the corpus copy).
#[test]
fn scan_honors_game_version_under_numeric_workshop_id_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("3242000764");
    fs::create_dir_all(root.join("About")).unwrap();
    fs::write(
        root.join("About").join("About.xml"),
        "<ModMetaData><name>numeric-root</name>\
         <supportedVersions><li>1.5</li><li>1.6</li></supportedVersions>\
         </ModMetaData>",
    )
    .unwrap();
    fs::create_dir_all(
        root.join("1.5")
            .join("Languages")
            .join("English")
            .join("Keyed"),
    )
    .unwrap();
    fs::write(
        root.join("1.5")
            .join("Languages")
            .join("English")
            .join("Keyed")
            .join("A.xml"),
        "<LanguageData><From15>old</From15></LanguageData>",
    )
    .unwrap();
    fs::create_dir_all(
        root.join("1.6")
            .join("Languages")
            .join("English")
            .join("Keyed"),
    )
    .unwrap();
    fs::write(
        root.join("1.6")
            .join("Languages")
            .join("English")
            .join("Keyed")
            .join("B.xml"),
        "<LanguageData><From16>new</From16></LanguageData>",
    )
    .unwrap();

    let output = bin_cmd()
        .args(["--quiet", "scan", "--root"])
        .arg(&root)
        .args(["--game-version", "1.5", "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "scan must succeed");
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.contains("From15"),
        "1.5 content must be scanned, got: {out}"
    );
    assert!(
        !out.contains("From16"),
        "1.6 content must not leak into a --game-version 1.5 scan, got: {out}"
    );
}
