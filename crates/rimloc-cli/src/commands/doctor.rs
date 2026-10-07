//! `rimloc doctor`: environment diagnostics with OK / WARNING / ERROR /
//! NOT CHECKED statuses and remediation hints (gate L observability).
//!
//! Checks are deliberately secret-free: provider configuration is reported
//! as presence only (env var set or not), never values. The keychain is not
//! enumerated: the doctor probes env vars and points at the keychain as a
//! remediation instead of reading it.
//!
//! The output-writability probe refuses destinations inside the game or mod
//! trees (canonicalized, so symlink aliases are caught) BEFORE any mkdir or
//! write — RimLoc treats RimWorld installs and mod trees as read-only — and
//! creates its probe file with `create_new` under a unique name so an
//! existing file can never be truncated or followed through a symlink.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Check {
    pub name: String,
    /// "ok" | "warning" | "error" | "not_checked"
    pub status: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

impl Check {
    fn ok(name: &str, detail: impl Into<String>) -> Check {
        Check {
            name: name.to_string(),
            status: "ok".to_string(),
            detail: detail.into(),
            remediation: None,
        }
    }
    fn warning(name: &str, detail: impl Into<String>, remediation: &str) -> Check {
        Check {
            name: name.to_string(),
            status: "warning".to_string(),
            detail: detail.into(),
            remediation: Some(remediation.to_string()),
        }
    }
    fn error(name: &str, detail: impl Into<String>, remediation: &str) -> Check {
        Check {
            name: name.to_string(),
            status: "error".to_string(),
            detail: detail.into(),
            remediation: Some(remediation.to_string()),
        }
    }
    fn not_checked(name: &str, remediation: &str) -> Check {
        Check {
            name: name.to_string(),
            status: "not_checked".to_string(),
            detail: "not checked (missing input)".to_string(),
            remediation: Some(remediation.to_string()),
        }
    }
}

const PROVIDERS: [&str; 3] = ["anthropic", "openai", "zai"];

fn is_dir(path: &Path) -> bool {
    path.is_dir()
}

fn check_rw_install(game_root: Option<&Path>) -> Check {
    let Some(root) = game_root else {
        return Check::not_checked(
            "rw_install",
            "pass --game-root <dir> (the folder that contains Data/ and Mods/)",
        );
    };
    if !root.is_dir() {
        return Check::error(
            "rw_install",
            format!("game root does not exist: {}", root.display()),
            "point --game-root at the RimWorld installation directory",
        );
    }
    let data = root.join("Data");
    let mods = root.join("Mods");
    if !is_dir(&data) {
        return Check::error(
            "rw_install",
            "Data/ not found under the given game root".to_string(),
            "point --game-root at the RimWorld installation directory (the folder containing Data/)",
        );
    }
    if !is_dir(&mods) {
        return Check::warning(
            "rw_install",
            "Data/ found but Mods/ is missing",
            "create Mods/ next to Data/ (RimWorld creates it on first mod install)",
        );
    }
    Check::ok(
        "rw_install",
        format!("Data/ and Mods/ found at {}", root.display()),
    )
}

fn check_rw_version(game_root: Option<&Path>) -> Check {
    let Some(root) = game_root else {
        return Check::not_checked("rw_version", "pass --game-root so Version.txt can be read");
    };
    let version_file = root.join("Version.txt");
    match std::fs::read_to_string(&version_file) {
        Ok(text) => {
            let v = text.trim();
            if v.is_empty() {
                Check::warning(
                    "rw_version",
                    "Version.txt exists but is empty",
                    "reinstall or verify the game installation",
                )
            } else {
                Check::ok("rw_version", format!("RimWorld {v}"))
            }
        }
        Err(_) => Check::warning(
            "rw_version",
            "Version.txt not found — game version unknown",
            "if the install layout is unusual, verify the installation; RimLoc treats version as advisory",
        ),
    }
}

fn check_mod_dirs(game_root: Option<&Path>) -> Check {
    let Some(root) = game_root else {
        return Check::not_checked("mod_dirs", "pass --game-root to probe the Mods/ directory");
    };
    let mods = root.join("Mods");
    match std::fs::read_dir(&mods) {
        Ok(entries) => {
            let count = entries.flatten().filter(|e| e.path().is_dir()).count();
            if count == 0 {
                Check::warning(
                    "mod_dirs",
                    "Mods/ is readable but contains no mod directories",
                    "place mods under Mods/ or use --root to point RimLoc at a mod elsewhere",
                )
            } else {
                Check::ok("mod_dirs", format!("{count} mod directories readable"))
            }
        }
        Err(e) => Check::error(
            "mod_dirs",
            format!("cannot read {}: {e}", mods.display()),
            "check filesystem permissions on the Mods/ directory",
        ),
    }
}

fn check_mod_root(mod_root: Option<&Path>) -> Option<Check> {
    let root = mod_root?;
    if !root.is_dir() {
        return Some(Check::error(
            "mod_root",
            format!("mod root does not exist: {}", root.display()),
            "pass --root pointing at the mod folder (the one containing About/, Defs/, Languages/)",
        ));
    }
    let languages = root.join("Languages");
    match std::fs::read_dir(&languages) {
        Ok(entries) => {
            let mut dirs: Vec<String> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .collect();
            dirs.sort();
            if dirs.is_empty() {
                Some(Check::warning(
                    "mod_root",
                    "mod root found but Languages/ has no language folders",
                    "run `rimloc export-po` to scaffold Languages/<lang>/DefInjected + Keyed",
                ))
            } else {
                Some(Check::ok(
                    "mod_root",
                    format!("language folders: {}", dirs.join(", ")),
                ))
            }
        }
        Err(e) => Some(Check::warning(
            "mod_root",
            format!("cannot read {}: {e}", languages.display()),
            "check permissions; a mod without Languages/ cannot hold translations",
        )),
    }
}

fn check_provider_config() -> Check {
    provider_config_status(&|env_name| std::env::var(env_name).ok())
}

/// Testable core of the provider check: the env lookup is injected so tests
/// never mutate the real environment.
fn provider_config_status(env_lookup: &dyn Fn(&str) -> Option<String>) -> Check {
    let mut configured: Vec<String> = Vec::new();
    for provider in PROVIDERS {
        // Presence only — the value is never read into the report.
        let env_name = rimloc_llm::env_key(provider);
        let present = env_lookup(&env_name)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if present {
            configured.push(provider.to_string());
        }
    }
    if configured.is_empty() {
        Check::warning(
            "provider_config",
            "no provider API key found in environment (checked: RIMLOC_ANTHROPIC_API_KEY, RIMLOC_OPENAI_API_KEY, RIMLOC_ZAI_API_KEY)",
            "set one of those env vars or store a key in the OS keychain (service `rimloc-llm`); `rimloc translate --provider mock` works without keys",
        )
    } else {
        Check::ok(
            "provider_config",
            format!(
                "configured via env (presence only): {}; keychain entries (service `rimloc-llm`) are not enumerated by this check",
                configured.join(", ")
            ),
        )
    }
}

fn check_output_writable(out_dir: &Path, forbidden_roots: &[PathBuf]) -> Check {
    // Read-only invariant first: reject destinations inside game/mod/source
    // trees BEFORE any mkdir/write. Containment goes through the shared
    // services helper (canonical views: symlink aliases and parent-traversal
    // of not-yet-existing paths are caught).
    for root in forbidden_roots {
        if root.as_os_str().is_empty() {
            continue;
        }
        if rimloc_services::is_within(out_dir, root) {
            return Check::error(
                "output_writable",
                format!(
                    "output directory {} is inside a read-only source tree ({})",
                    out_dir.display(),
                    root.display()
                ),
                "choose a writable directory outside the RimWorld installation and mod trees for exports and support bundles",
            );
        }
    }

    // The doctor never creates directories: a missing out_dir stays missing.
    // The probe runs in the nearest existing ancestor and the report states
    // explicitly what was (and was not) tested.
    let mut missing_leaves: Vec<std::ffi::OsString> = Vec::new();
    let mut probe_dir = out_dir.to_path_buf();
    while !probe_dir.exists() {
        match (probe_dir.parent(), probe_dir.file_name()) {
            (Some(parent), Some(name)) => {
                missing_leaves.push(name.to_os_string());
                probe_dir = parent.to_path_buf();
            }
            _ => break,
        }
    }
    if !probe_dir.is_dir() {
        return Check::error(
            "output_writable",
            format!(
                "nearest existing ancestor {} is not a directory",
                probe_dir.display()
            ),
            "choose an existing, writable --out-dir for exports and support bundles",
        );
    }

    // Unique probe name + create_new: an existing file or symlink can never
    // be opened/truncated through this path.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let probe = probe_dir.join(format!(
        ".rimloc-doctor-probe-{}-{nanos:x}",
        std::process::id()
    ));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
    {
        Ok(mut f) => {
            let write_ok = f.write_all(b"rimloc doctor probe").is_ok();
            drop(f);
            // Cleanup honesty: a leftover probe file is pollution we caused,
            // so a failed removal must not be reported as a clean OK.
            let cleanup = std::fs::remove_file(&probe);
            let missing_note = if missing_leaves.is_empty() {
                String::new()
            } else {
                let names: Vec<String> = missing_leaves
                    .iter()
                    .rev()
                    .map(|n| n.to_string_lossy().into_owned())
                    .collect();
                format!(
                    "; missing leaf {} NOT created (RimLoc creates it on real writes)",
                    names.join("/")
                )
            };
            if !write_ok {
                return Check::error(
                    "output_writable",
                    format!(
                        "{} accepted file creation but not writes",
                        probe_dir.display()
                    ),
                    "check disk space and permissions",
                );
            }
            match cleanup {
                Ok(()) => Check::ok(
                    "output_writable",
                    format!(
                        "{} is writable{}",
                        out_dir.display(),
                        if missing_leaves.is_empty() {
                            String::new()
                        } else {
                            format!(
                                " (probe ran in nearest existing ancestor {}{})",
                                probe_dir.display(),
                                missing_note
                            )
                        }
                    ),
                ),
                Err(e) => Check::warning(
                    "output_writable",
                    format!(
                        "{} is writable but the probe file {} could not be removed: {e}",
                        probe_dir.display(),
                        probe.display()
                    ),
                    "remove the leftover .rimloc-doctor-probe file manually",
                ),
            }
        }
        Err(e) => Check::error(
            "output_writable",
            format!("cannot write into {}: {e}", out_dir.display()),
            "choose a writable --out-dir for exports and support bundles",
        ),
    }
}

fn status_tag(status: &str) -> &'static str {
    match status {
        "ok" => "[OK]",
        "warning" => "[WARNING]",
        "error" => "[ERROR]",
        _ => "[NOT CHECKED]",
    }
}

/// Entry point for `rimloc doctor`. Always exits 0: doctor is a report, not
/// a gate — statuses carry the verdict.
pub fn run_doctor(
    game_root: Option<PathBuf>,
    mod_root: Option<PathBuf>,
    out_dir: PathBuf,
    format: String,
) -> color_eyre::Result<()> {
    let game_root_ref = game_root.as_deref();
    let mut checks = vec![
        check_rw_install(game_root_ref),
        check_rw_version(game_root_ref),
        check_mod_dirs(game_root_ref),
    ];
    if let Some(c) = check_mod_root(mod_root.as_deref()) {
        checks.push(c);
    }
    checks.push(check_provider_config());

    let mut forbidden: Vec<PathBuf> = Vec::new();
    for root in [game_root.as_ref(), mod_root.as_ref()]
        .into_iter()
        .flatten()
    {
        if !forbidden.contains(root) {
            forbidden.push(root.clone());
        }
    }
    checks.push(check_output_writable(&out_dir, &forbidden));

    if format == "json" {
        let summary = summary_of(&checks);
        let out = serde_json::json!({ "checks": checks, "summary": summary });
        serde_json::to_writer(std::io::stdout().lock(), &out)?;
        println!();
        return Ok(());
    }

    crate::ui_out!("doctor-title");
    for c in &checks {
        let tag = format!("{:<12}", status_tag(&c.status));
        let name = c.name.as_str();
        let detail = c.detail.as_str();
        crate::ui_out!(
            "doctor-line",
            tag = tag.as_str(),
            name = name,
            detail = detail
        );
        if let Some(r) = &c.remediation {
            let hint = r.as_str();
            crate::ui_out!("doctor-remediation", hint = hint);
        }
    }
    let s = summary_of(&checks);
    let ok_n = s["ok"].as_u64().unwrap_or(0);
    let warn_n = s["warning"].as_u64().unwrap_or(0);
    let err_n = s["error"].as_u64().unwrap_or(0);
    let nc_n = s["not_checked"].as_u64().unwrap_or(0);
    crate::ui_out!(
        "doctor-summary",
        ok = ok_n,
        warning = warn_n,
        error = err_n,
        not_checked = nc_n
    );
    Ok(())
}

fn summary_of(checks: &[Check]) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    for status in ["ok", "warning", "error", "not_checked"] {
        m.insert(
            status.to_string(),
            serde_json::Value::from(checks.iter().filter(|c| c.status == status).count()),
        );
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_are_secret_free_and_well_formed() {
        // Provider presence must never embed a value. The lookup is injected
        // — the real environment is never mutated by tests.
        let lookup = |name: &str| {
            if name == "RIMLOC_ANTHROPIC_API_KEY" {
                Some("sk-super-secret-value-123".to_string())
            } else {
                None
            }
        };
        let c = provider_config_status(&lookup);
        assert_eq!(c.status, "ok");
        assert!(
            c.detail.contains("anthropic"),
            "presence reported: {}",
            c.detail
        );
        assert!(!c.detail.contains("sk-super-secret-value-123"));

        let empty = provider_config_status(&|_| None);
        assert_eq!(empty.status, "warning");
    }

    #[test]
    fn missing_game_root_reports_not_checked_with_remediation() {
        for c in [
            check_rw_install(None),
            check_rw_version(None),
            check_mod_dirs(None),
        ] {
            assert_eq!(c.status, "not_checked");
            assert!(c.remediation.is_some());
        }
    }

    #[test]
    fn tmp_mod_fixture_passes_doctor_checks() {
        let tmp = tempfile::tempdir().expect("tmp");
        let game = tmp.path().join("RimWorld");
        std::fs::create_dir_all(game.join("Data")).expect("dirs");
        std::fs::create_dir_all(game.join("Mods/SomeMod")).expect("dirs");
        std::fs::write(game.join("Version.txt"), "1.6.4518\n").expect("version");

        let install = check_rw_install(Some(&game));
        assert_eq!(install.status, "ok");
        let version = check_rw_version(Some(&game));
        assert_eq!(version.status, "ok");
        assert!(version.detail.contains("1.6.4518"));
        let dirs = check_mod_dirs(Some(&game));
        assert_eq!(dirs.status, "ok");

        let mod_root = tmp.path().join("MyMod");
        std::fs::create_dir_all(mod_root.join("Languages/Russian")).expect("dirs");
        let mc = check_mod_root(Some(&mod_root)).expect("check present");
        assert_eq!(mc.status, "ok");
        assert!(mc.detail.contains("Russian"));
    }

    #[test]
    fn empty_path_reports_error() {
        let missing = Path::new("/nonexistent/rimloc/definitely/absent");
        assert_eq!(check_rw_install(Some(missing)).status, "error");
        // Version/mods degrade to warnings rather than errors when the root
        // itself is absent — install check already carries the error.
        assert_eq!(check_rw_version(Some(missing)).status, "warning");
        assert_eq!(check_mod_dirs(Some(missing)).status, "error");

        let tmp = tempfile::tempdir().expect("tmp");
        // Missing out_dir is probed at the nearest existing ancestor and is
        // NOT created as a side effect.
        let missing_out = tmp.path().join("out");
        let writable = check_output_writable(&missing_out, &[]);
        assert_eq!(writable.status, "ok");
        assert!(
            writable.detail.contains("NOT created"),
            "must state what was tested: {}",
            writable.detail
        );
        assert!(!missing_out.exists(), "missing out_dir must stay missing");
    }

    #[test]
    fn write_probe_never_touches_existing_files_and_rejects_source_trees() {
        let tmp = tempfile::tempdir().expect("tmp");

        // Fixture game tree with a sentinel (no real game/install touched).
        let game = tmp.path().join("RimWorld");
        std::fs::create_dir_all(game.join("Data")).expect("dirs");
        std::fs::write(game.join("Version.txt"), "SENTINEL").expect("sentinel");

        // Destination inside the game tree → error, no mkdir/write there.
        let inside = game.join("exports");
        let c = check_output_writable(&inside, std::slice::from_ref(&game));
        assert_eq!(c.status, "error", "got: {:?}", c.detail);
        assert!(c.detail.contains("read-only source tree"));
        assert!(!inside.exists(), "must not mkdir inside the source tree");
        assert_eq!(
            std::fs::read_to_string(game.join("Version.txt")).expect("sentinel"),
            "SENTINEL",
            "sentinel untouched"
        );

        // The mod root itself as destination → also rejected.
        let mod_root = tmp.path().join("MyMod");
        std::fs::create_dir_all(&mod_root).expect("dirs");
        let c = check_output_writable(&mod_root, std::slice::from_ref(&mod_root));
        assert_eq!(c.status, "error");

        // Symlink alias into the game tree → caught via canonical view.
        #[cfg(unix)]
        {
            let alias = tmp.path().join("alias");
            std::os::unix::fs::symlink(&game, &alias).expect("symlink");
            let c = check_output_writable(&alias.join("out"), std::slice::from_ref(&game));
            assert_eq!(c.status, "error", "symlink alias must be rejected");
            assert!(!alias.join("out").exists());
        }

        // Writable destination outside the trees → ok; pre-existing files
        // keep their exact content (probe uses create_new + unique name).
        let out = tmp.path().join("safe-out");
        std::fs::create_dir_all(&out).expect("dirs");
        std::fs::write(out.join("keep.txt"), "keepme").expect("existing file");
        let c = check_output_writable(&out, &[game, mod_root]);
        assert_eq!(c.status, "ok");
        assert_eq!(
            std::fs::read_to_string(out.join("keep.txt")).expect("keep"),
            "keepme"
        );
        // Missing nested leaf under a writable ancestor: probed at the
        // ancestor, leaf stays missing, fixture whole.
        let nested = out.join("deep/leaf");
        let c = check_output_writable(&nested, &[]);
        assert_eq!(c.status, "ok", "got: {:?}", c.detail);
        assert!(c.detail.contains("NOT created"));
        assert!(!nested.exists(), "missing leaf must stay missing");
        assert_eq!(
            std::fs::read_to_string(out.join("keep.txt")).expect("keep"),
            "keepme",
            "fixture whole after nested probe"
        );
    }

    #[test]
    fn summary_counts_all_statuses() {
        let checks = vec![
            Check::ok("a", "d"),
            Check::warning("b", "d", "fix"),
            Check::not_checked("c", "fix"),
        ];
        let s = summary_of(&checks);
        assert_eq!(s["ok"], serde_json::Value::from(1));
        assert_eq!(s["warning"], serde_json::Value::from(1));
        assert_eq!(s["error"], serde_json::Value::from(0));
        assert_eq!(s["not_checked"], serde_json::Value::from(1));
    }
}
