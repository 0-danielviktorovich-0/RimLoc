//! Gate L acceptance: a REAL, deterministic validate failure captured into a
//! sanitized support bundle through the public CLI.
//!
//! The fixture mod has a genuinely broken translation (an empty label, plus
//! a placeholder mismatch under --compare-placeholders). The test learns the
//! failing keys from the CLI's own stdout, then verifies that the support
//! bundle ALONE lets an independent reader diagnose the same cause — the
//! operation id, the unfinished (failed) state, the affected keys and the
//! actual validator messages — while the source tree stays byte-identical.
//!
//! No network, no provider calls, no paid APIs.

use std::collections::BTreeMap;
use std::path::Path;

use assert_cmd::Command;

fn bin() -> Command {
    Command::cargo_bin("rimloc-cli").expect("rimloc binary")
}

fn tree_snapshot(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).expect("readdir").flatten() {
            let p = e.path();
            let name = format!("{prefix}/{}", e.file_name().to_string_lossy().into_owned());
            if p.is_dir() {
                walk(&p, &name, out);
            } else {
                out.insert(name, std::fs::read(&p).expect("read"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, "", &mut out);
    out
}

#[test]
fn validate_failure_flows_into_sanitized_support_bundle() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mod_root = tmp.path().join("MyMod");
    let eng = mod_root.join("Languages/English/Keyed");
    let rus = mod_root.join("Languages/Russian/Keyed");
    std::fs::create_dir_all(&eng).expect("dirs");
    std::fs::create_dir_all(&rus).expect("dirs");

    // Real defects for the REAL validator:
    // 1) an empty translation (per-row `empty` check), and
    // 2) a placeholder mismatch surfaced via --compare-placeholders.
    std::fs::write(
        eng.join("Actions.xml"),
        "<LanguageData>\n  <Greet.label>Hi {0}!</Greet.label>\n  <Bye.label>Bye</Bye.label>\n  <Tip.label>Tip {0}!</Tip.label>\n</LanguageData>\n",
    )
    .expect("write en");
    std::fs::write(
        rus.join("Actions.xml"),
        "<LanguageData>\n  <Greet.label></Greet.label>\n  <Bye.label>Пока</Bye.label>\n  <Tip.label>Совет {0}!</Tip.label>\n</LanguageData>\n",
    )
    .expect("write ru");
    let sentinel = mod_root.join("sentinel.txt");
    std::fs::write(&sentinel, b"KEEP").expect("sentinel");
    let before = tree_snapshot(&mod_root);

    let bundle_out = tmp.path().join("bundle-out");
    let output = bin()
        .args([
            "--quiet",
            "validate",
            "--root",
            mod_root.to_str().expect("utf8 root"),
            "--source-lang-dir",
            "English",
            "--lang-dir",
            "Russian",
            "--compare-placeholders",
            "--support-bundle",
            bundle_out.to_str().expect("utf8 out"),
        ])
        .output()
        .expect("run rimloc validate");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // The real validator actually ran and found real issues (so the captured
    // failure is genuine, not a hand-seeded one).
    assert!(
        stdout.contains("[empty/error]") || stdout.contains("[placeholder-check/error]"),
        "validator must report real issues, got: {stdout}"
    );

    // Bundle written and complete.
    for name in [
        "report.md",
        "diagnostics.json",
        "environment.json",
        "manifest.json",
    ] {
        assert!(bundle_out.join(name).exists(), "{name} must exist");
    }

    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(bundle_out.join("manifest.json")).expect("manifest"),
    )
    .expect("manifest json");
    let operation_id = manifest["operation_id"]
        .as_str()
        .expect("op id")
        .to_string();
    assert!(operation_id.starts_with("op-"));

    let diagnostics: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(bundle_out.join("diagnostics.json")).expect("diagnostics"),
    )
    .expect("diagnostics json");

    // The preserved operation carries the SAME id and the REAL results.
    let operation = &diagnostics["operation"];
    assert_eq!(
        operation["operation_id"],
        serde_json::Value::String(operation_id.clone())
    );
    assert_eq!(
        operation["name"],
        serde_json::Value::String("validate".to_string())
    );
    // Severity split of the REAL findings (026): 2 errors — the empty
    // Russian label and the placeholder mismatch — plus 1 info (the
    // placeholder presence hint on the well-formed Tip.label): an
    // Info+Error mixture in one run.
    let counters = &operation["stages"][0]["counters"];
    assert_eq!(counters["errors"], serde_json::Value::from(2), "2 errors");
    assert_eq!(
        counters["warnings"],
        serde_json::Value::from(0),
        "0 warnings"
    );
    assert_eq!(counters["info"], serde_json::Value::from(1), "1 info hint");
    assert_eq!(
        counters["findings_total"],
        serde_json::Value::from(3),
        "all findings counted"
    );
    // Failed run stays failed — never a synthetic success. The derived
    // status agrees: unfinished with errors = failed.
    assert!(operation["finished_at"].is_null());
    assert_eq!(
        operation["status"],
        serde_json::Value::String("failed".to_string())
    );

    // Every finding (both severities) survives in the sanitized structured
    // context; only errors become causal entries.
    let findings = diagnostics["project_meta"]["extra"]["validation_findings"]
        .as_array()
        .expect("validation_findings")
        .clone();
    assert_eq!(findings.len(), 3, "all findings preserved");
    let severities: Vec<&str> = findings
        .iter()
        .filter_map(|f| f["severity"].as_str())
        .collect();
    // Emission order of the real validator (Tip.info comes from the
    // per-row pass ordering; both errors from the empty label and the
    // cross-language mismatch).
    assert_eq!(severities, vec!["error", "info", "error"]);
    // Causal entries carry only the Error findings.
    let causal = operation["errors"].as_array().expect("errors");
    assert_eq!(causal.len(), 2, "only error findings in the causal chain");

    // Affected keys derive from the actual validator output.
    let stdout_keys: Vec<&str> = stdout
        .lines()
        .filter_map(|l| {
            // text format: "[kind/severity] key (path:line) — message"
            let rest = l.split("] ").nth(1)?;
            let key = rest.split(" (").next()?;
            if key.is_empty() {
                None
            } else {
                Some(key.trim())
            }
        })
        .collect();
    assert!(
        stdout_keys.contains(&"Greet.label"),
        "stdout keys parsed from real output: {stdout_keys:?}"
    );
    let affected = diagnostics["affected"].as_array().expect("affected");
    assert!(
        affected.iter().any(|a| a
            .as_str()
            .map(|s| s.contains("Greet.label"))
            .unwrap_or(false)),
        "affected carries the real failing key: {affected:?}"
    );

    // Independent reader: report.md ALONE names the cause.
    let report = std::fs::read_to_string(bundle_out.join("report.md")).expect("report");
    assert!(report.contains("Failed operation context"));
    assert!(report.contains("NOT finished"));
    assert!(report.contains(&operation_id));
    assert!(report.contains("Greet.label"), "affected key in report");
    assert!(
        report.contains("empty") || report.contains("placeholder-check"),
        "real validator kinds present"
    );

    // Source tree byte-identical.
    assert_eq!(tree_snapshot(&mod_root), before, "source untouched");
    assert_eq!(
        std::fs::read(&sentinel).expect("sentinel"),
        b"KEEP",
        "sentinel untouched"
    );
}

// Round-021: with --format json the WHOLE stdout is machine-readable —
// the support-bundle notice must go to stderr and never pollute it.
#[test]
fn json_stdout_stays_parseable_with_support_bundle() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mod_root = tmp.path().join("MyMod");
    let eng = mod_root.join("Languages/English/Keyed");
    let rus = mod_root.join("Languages/Russian/Keyed");
    std::fs::create_dir_all(&eng).expect("dirs");
    std::fs::create_dir_all(&rus).expect("dirs");
    std::fs::write(
        eng.join("Actions.xml"),
        "<LanguageData>\n  <Greet.label>Hi {0}!</Greet.label>\n</LanguageData>\n",
    )
    .expect("write en");
    std::fs::write(
        rus.join("Actions.xml"),
        "<LanguageData>\n  <Greet.label></Greet.label>\n</LanguageData>\n",
    )
    .expect("write ru");

    let bundle_out = tmp.path().join("bundle-out");
    let output = bin()
        .args([
            "--quiet",
            "validate",
            "--format",
            "json",
            "--root",
            mod_root.to_str().expect("utf8 root"),
            "--source-lang-dir",
            "English",
            "--lang-dir",
            "Russian",
            "--support-bundle",
            bundle_out.to_str().expect("utf8 out"),
        ])
        .output()
        .expect("run rimloc validate");

    // The WHOLE stdout must parse as the documented JSON array of issues.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let issues: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("whole stdout is valid JSON");
    let items = issues.as_array().expect("json array");
    assert!(!items.is_empty(), "real validator issues present: {stdout}");
    assert!(
        items
            .iter()
            .all(|i| i.get("kind").is_some() && i.get("key").is_some()),
        "issue objects keep their documented shape"
    );

    // The localized bundle notice went to stderr, not stdout.
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        stderr.contains(&bundle_out.display().to_string()),
        "bundle path announced on stderr: {stderr}"
    );

    // The bundle itself exists and preserves the causal results.
    assert!(bundle_out.join("manifest.json").exists());
    let diagnostics: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(bundle_out.join("diagnostics.json")).expect("diagnostics"),
    )
    .expect("json");
    assert_eq!(
        diagnostics["operation"]["stages"][0]["counters"]["errors"],
        serde_json::Value::from(items.len() as u64),
        "actual error count captured"
    );
    assert_eq!(
        diagnostics["operation"]["stages"][0]["counters"]["findings_total"],
        serde_json::Value::from(items.len() as u64),
        "findings total captured"
    );
    assert!(
        diagnostics["operation"]["finished_at"].is_null(),
        "failed run stays failed"
    );
}

// 026 positive control (lead's CleanFixture shape): a VALID {0} pair is
// only an Info finding — validation succeeds, the finding survives in the
// bundle, and the whole JSON stdout stays parseable.
#[test]
fn placeholder_info_keeps_validation_successful_with_bundle() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mod_root = tmp.path().join("MyMod");
    let eng = mod_root.join("Languages/English/Keyed");
    let rus = mod_root.join("Languages/Russian/Keyed");
    std::fs::create_dir_all(&eng).expect("dirs");
    std::fs::create_dir_all(&rus).expect("dirs");
    std::fs::write(
        eng.join("Actions.xml"),
        "<LanguageData>\n  <Greeting.label>Hello {0}!</Greeting.label>\n</LanguageData>\n",
    )
    .expect("write en");
    std::fs::write(
        rus.join("Actions.xml"),
        "<LanguageData>\n  <Greeting.label>Привет {0}!</Greeting.label>\n</LanguageData>\n",
    )
    .expect("write ru");
    let sentinel = mod_root.join("sentinel.txt");
    std::fs::write(&sentinel, b"KEEP").expect("sentinel");
    let before = tree_snapshot(&mod_root);

    let bundle_out = tmp.path().join("bundle-out");
    let output = bin()
        .args([
            "--quiet",
            "validate",
            "--format",
            "json",
            "--root",
            mod_root.to_str().expect("utf8 root"),
            "--lang-dir",
            "Russian",
            "--compare-placeholders",
            "--support-bundle",
            bundle_out.to_str().expect("utf8 out"),
        ])
        .output()
        .expect("run rimloc validate");
    assert!(output.status.success(), "info-only run must succeed");

    // Whole stdout parses as the JSON contract.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let issues: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("whole stdout is valid JSON");
    let items = issues.as_array().expect("array").clone();
    assert!(!items.is_empty(), "the Info finding is still reported");
    for item in &items {
        assert_eq!(
            item["severity"].as_str(),
            Some("info"),
            "valid pair yields only info findings: {items:?}"
        );
    }

    // Validation with warnings/info = SUCCESSFUL; the finding survives in
    // the structured context and the human report.
    let diagnostics: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(bundle_out.join("diagnostics.json")).expect("diagnostics"),
    )
    .expect("json");
    let operation = &diagnostics["operation"];
    assert_eq!(
        operation["status"],
        serde_json::Value::String("succeeded".to_string())
    );
    let counters = &operation["stages"][0]["counters"];
    assert_eq!(counters["errors"], serde_json::Value::from(0));
    assert_eq!(counters["info"], serde_json::Value::from(1));
    assert_eq!(counters["findings_total"], serde_json::Value::from(1));
    let findings = diagnostics["project_meta"]["extra"]["validation_findings"]
        .as_array()
        .expect("findings");
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0]["severity"],
        serde_json::Value::String("info".to_string())
    );
    assert!(
        findings[0]["message"]
            .as_str()
            .map(|m| m.contains("Placeholders present"))
            .unwrap_or(false),
        "presence hint preserved: {findings:?}"
    );
    let report = std::fs::read_to_string(bundle_out.join("report.md")).expect("report");
    assert!(
        report.contains("Placeholders present"),
        "info finding in report"
    );

    // Source tree byte-identical.
    assert_eq!(tree_snapshot(&mod_root), before, "source untouched");
    assert_eq!(std::fs::read(&sentinel).expect("sentinel"), b"KEEP");
}

#[test]
fn validate_clean_run_support_bundle_marks_operation_finished() {
    let tmp = tempfile::tempdir().expect("tmp");
    let mod_root = tmp.path().join("MyMod");
    let eng = mod_root.join("Languages/English/Keyed");
    let rus = mod_root.join("Languages/Russian/Keyed");
    std::fs::create_dir_all(&eng).expect("dirs");
    std::fs::create_dir_all(&rus).expect("dirs");
    std::fs::write(
        eng.join("Actions.xml"),
        "<LanguageData>\n  <Bye.label>Bye</Bye.label>\n</LanguageData>\n",
    )
    .expect("write en");
    std::fs::write(
        rus.join("Actions.xml"),
        "<LanguageData>\n  <Bye.label>Пока</Bye.label>\n</LanguageData>\n",
    )
    .expect("write ru");

    let bundle_out = tmp.path().join("bundle-out");
    bin()
        .args([
            "--quiet",
            "validate",
            "--root",
            mod_root.to_str().expect("utf8 root"),
            "--lang-dir",
            "Russian",
            "--support-bundle",
            bundle_out.to_str().expect("utf8 out"),
        ])
        .assert()
        .success();

    let diagnostics: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(bundle_out.join("diagnostics.json")).expect("diagnostics"),
    )
    .expect("json");
    // A clean run is a successful operation: finish() happened.
    assert!(diagnostics["operation"]["finished_at"].is_string());
    assert_eq!(
        diagnostics["operation"]["status"],
        serde_json::Value::String("succeeded".to_string()),
        "clean run derives succeeded"
    );
    assert_eq!(
        diagnostics["operation"]["stages"][0]["counters"]["findings_total"],
        serde_json::Value::from(0)
    );
}
